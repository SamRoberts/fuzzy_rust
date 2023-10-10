//! An implementation of [`Question`](crate::Question) that parses the pattern using
//! [`regex_syntax`](https://docs.rs/regex-syntax).
//!
//! [`regex_syntax`](https://docs.rs/regex-syntax) sometimes uses bytes in their API, while this
//! crate currently operates on unicode characters. For now, we are getting around this by naively
//! assuming all characters are ASCII. We will change this in the future.

use regex_syntax;
use regex_syntax::hir::{Capture, Hir, HirKind, Literal, Repetition};
use crate::{Class, Patt, Problem, Question, Text};
use crate::error::Error;

pub struct RegexQuestion {
    pub pattern_regex: String,
    pub text: String,
}

impl Question<Error> for RegexQuestion {
    fn ask(&self) -> Result<Problem, Error> {
        let text = Self::new_text(&self.text);
        let pattern = Self::parse_pattern(&self.pattern_regex)?;
        Ok(Problem { pattern, text })
    }
}

impl RegexQuestion {

    pub fn new_text(text: &str) -> Vec<Text> {
        let mut text_vec: Vec<Text> = text.chars().map(|c| Text::Lit(c)).collect();
        text_vec.push(Text::End);
        text_vec
    }

    fn parse_pattern(pattern: &str) -> Result<Vec<Patt>, Error> {
        let hir = regex_syntax::parse(pattern)?;
        let mut items = vec![];
        Self::parse_impl(&hir, &mut items)?;
        items.push(Patt::End);
        Ok(items)
    }

    fn parse_impl(hir: &Hir, items: &mut Vec<Patt>) -> Result<usize, Error> {
        match hir.kind() {
            HirKind::Literal(Literal(ref bytes)) => {
                // TODO modify Patt::Lit to use bytes rather then chars. For now, assuming ascii
                for byte in bytes.iter() {
                    items.push(Patt::Lit(*byte as char));
                }
                Ok(bytes.len())
            }
            HirKind::Class(class) => {
                items.push(Patt::Class(Class::from(class.clone())));
                Ok(1)
            }
            HirKind::Capture(Capture { sub, .. }) => {
                items.push(Patt::GroupStart);
                let num_children = Self::parse_impl(sub, items)?;
                items.push(Patt::GroupEnd);
                Ok(num_children + 2)
            }
            HirKind::Alternation(children) => {
                match &children[..] {
                    [] => Ok(0),
                    [left, right @ ..] => Self::parse_alternation_impl(left, right, items),
                }
            }
            HirKind::Repetition(Repetition { min: 0, max: None, sub, .. }) => {
                let start_ix = items.len();
                items.push(Patt::RepetitionStart(0)); // replaced with proper offset later
                let num_children = Self::parse_impl(sub, items)?;
                let offset = num_children + 1;
                items[start_ix] = Patt::RepetitionStart(offset);
                items.push(Patt::RepetitionEnd(offset));
                Ok(num_children + 2)
            }
            HirKind::Concat(children) => {
                let mut sum = 0;
                for child in children {
                    sum += Self::parse_impl(child, items)?;
                }
                Ok(sum)
            }
            unsupported => {
                Err(Error::PatternUnsupported(format!("{:?}", unsupported)))
            }
        }
    }

    fn parse_alternation_impl(left: &Hir, right: &[Hir], items: &mut Vec<Patt>) -> Result<usize, Error> {
        match right {
            [] => Self::parse_impl(left, items),
            [next_left, next_right @ ..] => {
                let left_ix = items.len();
                items.push(Patt::AlternativeLeft(0)); // replaced with proper offset later
                let num_left = Self::parse_impl(left, items)?;
                let right_ix = items.len();
                items.push(Patt::AlternativeRight(0)); // replaced with proper offset later
                let num_right = Self::parse_alternation_impl(next_left, next_right, items)?;
                let next_ix = items.len();

                items[left_ix] = Patt::AlternativeLeft(right_ix - left_ix);
                items[right_ix] = Patt::AlternativeRight(next_ix - right_ix);

                Ok(num_left + num_right + 2)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::patt_class;

    #[test]
    fn parse_lit_1() {
        parse_test("a", vec![Patt::Lit('a')]);
    }

    #[test]
    fn parse_lit_2() {
        parse_test("abc", vec![Patt::Lit('a'), Patt::Lit('b'), Patt::Lit('c')]);
    }

    #[test]
    fn parse_wildcard() {
        parse_test(".", vec![patt_class(".")])
    }

    #[test]
    fn parse_concat_1() {
        parse_test("a.", vec![Patt::Lit('a'), patt_class(".")]);
    }

    #[test]
    fn parse_repetition_1() {
        parse_test("a*", vec![
            Patt::RepetitionStart(2),
            Patt::Lit('a'),
            Patt::RepetitionEnd(2),
        ]);
    }

    #[test]
    fn parse_group_1() {
        parse_test("(a)", vec![Patt::GroupStart, Patt::Lit('a'), Patt::GroupEnd]);
    }

    #[test]
    fn parse_alternative_1() {
        parse_test("ab|cd", vec![
            Patt::AlternativeLeft(3),
            Patt::Lit('a'),
            Patt::Lit('b'),
            Patt::AlternativeRight(3),
            Patt::Lit('c'),
            Patt::Lit('d'),
        ]);
    }

    #[test]
    fn parse_alternative_2() {
        parse_test("ab|cd|wxyz", vec![
            Patt::AlternativeLeft(3),
            Patt::Lit('a'),
            Patt::Lit('b'),
            Patt::AlternativeRight(9),
            Patt::AlternativeLeft(3),
            Patt::Lit('c'),
            Patt::Lit('d'),
            Patt::AlternativeRight(5),
            Patt::Lit('w'),
            Patt::Lit('x'),
            Patt::Lit('y'),
            Patt::Lit('z'),
        ]);
    }

    fn parse_test(pattern: &str, expected: Vec<Patt>) {
        // TODO see if we can avoid this unnecesary copying?
        let mut expected_pattern = expected.clone();
        expected_pattern.push(Patt::End);

        let actual_pattern = RegexQuestion::parse_pattern(&pattern).expect("Cannot parse pattern");
        assert_eq!(expected_pattern, actual_pattern);
    }
}
