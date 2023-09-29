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
            HirKind::Repetition(Repetition { min: 0, max: None, sub, .. }) => {
                items.push(Patt::KleeneStart(0)); // replaced with proper offset later
                let num_children = Self::parse_impl(sub, items)?;
                let offset = num_children + 1;
                let start_ix = items.len() - offset;
                items[start_ix] = Patt::KleeneStart(offset);
                items.push(Patt::KleeneEnd(offset));
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
    fn parse_kleene_1() {
        parse_test("a*", vec![
            Patt::KleeneStart(2),
            Patt::Lit('a'),
            Patt::KleeneEnd(2),
        ]);
    }

    #[test]
    fn parse_group_1() {
        parse_test("(a)", vec![Patt::GroupStart, Patt::Lit('a'), Patt::GroupEnd]);
    }

    fn parse_test(pattern: &str, expected: Vec<Patt>) {
        // TODO see if we can avoid this unnecesary copying?
        let mut expected_pattern = expected.clone();
        expected_pattern.push(Patt::End);

        let actual_pattern = RegexQuestion::parse_pattern(&pattern).expect("Cannot parse pattern");
        assert_eq!(expected_pattern, actual_pattern);
    }
}
