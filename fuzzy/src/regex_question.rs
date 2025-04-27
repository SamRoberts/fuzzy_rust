//! Parses pattern using [`regex_syntax`](https://docs.rs/regex-syntax).
//!
//! [`regex_syntax`](https://docs.rs/regex-syntax) sometimes uses bytes in their API, while this
//! crate currently operates on unicode characters. For now, we are getting around this by naively
//! assuming all characters are ASCII. We will change this in the future.

use regex_syntax::hir;
use crate::{Class, Element, Match, Pattern, Repetition};
use crate::error::Error;

pub fn parse_pattern(pattern: &str) -> Result<Pattern<Element>, Error> {
    let hir = regex_syntax::parse(pattern)?;
    return wrap(parse_impl(&hir));
}

fn wrap(try_elems: Result<Vec<Element>, Error>) -> Result<Pattern<Element>, Error> {
    try_elems.map(|elems| Pattern { elems })
}


fn parse_impl(hir: &hir::Hir) -> Result<Vec<Element>, Error>
{
    match hir.kind() {
        hir::HirKind::Literal(hir::Literal(ref bytes)) => {
            // TODO modify Patt::Lit to use bytes rather then chars. For now, assuming ascii
            Ok(bytes.iter().map(|b| Element::Match(Match::Lit(*b as char))).collect())
        }
        hir::HirKind::Class(class) => {
            Ok(vec![Element::Match(Match::Class(Class::from(class.clone())))])
        }
        hir::HirKind::Capture(hir::Capture { sub, .. }) => {
           let pattern = wrap(parse_impl(sub))?;
           Ok(vec![Element::Capture(pattern)])
        }
        hir::HirKind::Alternation(children) => {
            match &children[..] {
                [] => Ok(vec![]),
                [sub] => parse_impl(sub),
                [sub1, sub2, subs @ ..] => {
                    let try_p1 = wrap(parse_impl(sub1));
                    let try_p2 = wrap(parse_impl(sub2));
                    let mut try_ps = subs.iter().map(|sub| wrap(parse_impl(sub)));

                    let try_init = try_p1.and_then(|p1| try_p2.map(|p2| Element::Alternative(p1, p2)));

                    let try_alternative = try_init.and_then(|init|
                        try_ps.try_fold(init, |elem, try_p|
                            try_p.map(|p| Element::Alternative(Pattern { elems: vec![elem] }, p))
                        )
                    );

                    try_alternative.map(|alt| vec![alt])
                }
            }
        }
        hir::HirKind::Repetition(hir::Repetition { min, max, sub, .. }) => {
            Result::from_iter(
                wrap(parse_impl(sub)).map(|inner| {
                    let minimum = (*min).try_into().map_err(|_| Error::RegexBoundTooLarge)?;
                    let maximum = max.map_or(Ok(None), |max|
                        max.try_into().map(|m| Some(m)).map_err(|_| Error::RegexBoundTooLarge)
                    )?;
                    Ok(Element::Repetition(Repetition { minimum, maximum, inner }))
                })
            )
        }
        hir::HirKind::Concat(subs) => {
            let try_nested: Result<Vec<Vec<Element>>, Error> =
                Result::from_iter(subs.iter().map(|sub| parse_impl(sub)));
            try_nested.map(|nested| nested.into_iter().flatten().collect())
        }
        unsupported => {
            Err(Error::PatternUnsupported(format!("{:?}", unsupported)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::{alt, class, capture, lit, lits, rep, rep_min, rep_bound};
    use proptest::prelude::*;

    #[test]
    fn parse_lit_1() {
        parse_test("a", lits("a"));
    }

    #[test]
    fn parse_lit_2() {
        parse_test("abc", lits("abc"));
    }

    #[test]
    fn parse_wildcard() {
        parse_test(".", vec![class(".")])
    }

    #[test]
    fn parse_concat_1() {
        parse_test("a.", vec![lit('a'), class(".")]);
    }

    #[test]
    fn parse_repetition_1() {
        parse_test("a*", vec![rep(lits("a"))]);
    }

    #[test]
    fn parse_repetition_2() {
        parse_test("a+", vec![rep_min(1, lits("a"))]);
    }

    #[test]
    fn parse_repetition_3() {
        parse_test("a{2,}", vec![rep_min(2, lits("a"))]);
    }

    #[test]
    fn parse_repetition_4() {
        parse_test("a{0,3}", vec![rep_bound(0, 3, lits("a"))]);
    }

    #[test]
    fn parse_repetition_5() {
        parse_test("a{4}", vec![rep_bound(4, 4, lits("a"))]);
    }

    #[test]
    fn parse_group_1() {
        parse_test("(a)", vec![capture(lits("a"))]);
    }

    #[test]
    fn parse_alternative_1() {
        parse_test("ab|cd", vec![alt(lits("ab"), lits("cd"))]);
    }

    #[test]
    fn parse_alternative_2() {
        parse_test("ab|cd|wxyz", vec![alt(vec![alt(lits("ab"), lits("cd"))], lits("wxyz"))]);
    }

    fn parse_test(pattern: &str, expected_elems: Vec<Element>) {
        let expected_pattern = Pattern { elems: expected_elems };
        let actual_pattern = parse_pattern(&pattern).expect("Cannot parse pattern");
        assert_eq!(expected_pattern, actual_pattern);
    }

    // TODO support parsing empty patterns
    // TODO more accurate range of literal patterns here
    const LITERAL_PATTERN_REGEX: &str = "[[:alnum:]]+";

    proptest! {
        #[test]
        fn smoketest(pattern in "\\PC*") {
            let _ = parse_pattern(&pattern);
        }

        #[test]
        fn literals(pattern in LITERAL_PATTERN_REGEX) {
            let expected_pattern = Pattern { elems: lits(&pattern) };
            let actual_pattern = parse_pattern(&pattern).expect("Cannot parse pattern");
            prop_assert_eq!(expected_pattern, actual_pattern);
        }

        #[test]
        fn captures(inner in LITERAL_PATTERN_REGEX) {
            let wrapped = format!("({})", inner);
            let Pattern { elems: actual_inner } = parse_pattern(&inner).expect("Cannot parse inner");
            let Pattern { elems: actual_wrapped } = parse_pattern(&wrapped).expect("Cannot parse wrapped");
            prop_assert_eq!( actual_wrapped, vec![capture(actual_inner)]);
        }

        #[test]
        fn alternatives(inners in prop::collection::vec(LITERAL_PATTERN_REGEX, 2..5)) {
            // the regex lib is smart enough to turn an alternative of single characters into a
            // character class ... which is good, but annoying for this particular test
            prop_assume!(inners.iter().any(|inner| inner.len() > 1));

            let alt_pattern = inners.join("|");
            let expected_alt = inners.iter()
                .map(|inner| lits(&inner))
                .reduce(|acc, right| vec![alt(acc, right)]).expect("Cannot be empty");

            let expected_pattern = Pattern { elems: expected_alt };
            let actual_pattern = parse_pattern(&alt_pattern).expect("Cannot parse pattern");
            prop_assert_eq!(expected_pattern, actual_pattern);
        }
    }
}
