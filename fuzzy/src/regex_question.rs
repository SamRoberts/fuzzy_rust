//! An implementation of [`Question`](crate::Question) that parses the pattern using
//! [`regex_syntax`](https://docs.rs/regex-syntax).
//!
//! [`regex_syntax`](https://docs.rs/regex-syntax) sometimes uses bytes in their API, while this
//! crate currently operates on unicode characters. For now, we are getting around this by naively
//! assuming all characters are ASCII. We will change this in the future.

use regex_syntax;
use regex_syntax::hir;
use regex_syntax::hir::{Capture, Hir, HirKind, Literal};
use crate::{Atoms, Class, Element, Match, Pattern, Problem, Question, Repetition};
use crate::error::Error;

pub struct RegexQuestion {
    pub pattern_regex: String,
    pub text: String,
}

impl Question<Error> for RegexQuestion {
    fn ask(&self) -> Result<Problem, Error> {
        let pattern = Self::parse_pattern(&self.pattern_regex)?;
        let text = Atoms { atoms: self.text.chars().collect() };
        Ok(Problem { pattern, text })
    }
}

impl RegexQuestion {
    fn parse_pattern(pattern: &str) -> Result<Pattern, Error> {
        let hir = regex_syntax::parse(pattern)?;
        Self::pattern(Self::parse_impl(&hir))
    }

    fn pattern(try_elems: Result<Vec<Element>, Error>) -> Result<Pattern, Error> {
        try_elems.map(|elems| Pattern { elems })
    }

    fn parse_impl(hir: &Hir) -> Result<Vec<Element>, Error>
    {
        match hir.kind() {
            HirKind::Literal(Literal(ref bytes)) => {
                // TODO modify Patt::Lit to use bytes rather then chars. For now, assuming ascii
                Ok(bytes.iter().map(|b| Element::Match(Match::Lit(*b as char))).collect())
            }
            HirKind::Class(class) => {
                Ok(vec![Element::Match(Match::Class(Class::from(class.clone())))])
            }
            HirKind::Capture(Capture { sub, .. }) => {
               Self::pattern(Self::parse_impl(sub)).map(|p| vec![Element::Capture(p)])
            }
            HirKind::Alternation(children) => {
                match &children[..] {
                    [] => Ok(vec![]),
                    [sub] => Self::parse_impl(sub),
                    [sub1, sub2, subs @ ..] => {
                        let try_p1 = Self::pattern(Self::parse_impl(sub1));
                        let try_p2 = Self::pattern(Self::parse_impl(sub2));
                        let mut try_ps = subs.iter().map(|sub| Self::pattern(Self::parse_impl(sub)));

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
            HirKind::Repetition(hir::Repetition { min: 0, max: None, sub, .. }) => {
                Self::pattern(Self::parse_impl(sub)).map(|p|
                    vec![Element::Repetition(Repetition { minimum: 0, inner: p})]
                )
            }
            HirKind::Concat(subs) => {
                let try_nested: Result<Vec<Vec<Element>>, Error> =
                    Result::from_iter(subs.iter().map(|sub| Self::parse_impl(sub)));
                try_nested.map(|nested| nested.into_iter().flatten().collect())
            }
            unsupported => {
                Err(Error::PatternUnsupported(format!("{:?}", unsupported)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::{alt, class, capture, lit, lits, rep};

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
        let actual_pattern = RegexQuestion::parse_pattern(&pattern).expect("Cannot parse pattern");
        assert_eq!(expected_pattern, actual_pattern);
    }
}
