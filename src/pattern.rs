use regex_syntax;
use regex_syntax::hir::{Capture, Hir, HirKind, Literal, Repetition};
use crate::error::Error;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Patt {
    Lit(char), // TODO modify to take bytes like regex library. For now, assuming ascii
    Any,
    GroupStart,
    GroupEnd,
    KleeneStart(usize), // the offset of the end
    KleeneEnd(usize),   // the offset of the start
    End,
}

#[derive(Eq, PartialEq, Debug)]
pub struct Pattern {
    pub items: Vec<Patt>,
}

impl Pattern {
    pub fn parse(pattern: &str) -> Result<Self, Error> {
        let hir = regex_syntax::parse(pattern)?;
        let mut items = vec![];
        Self::parse_impl(&hir, &mut items)?;
        items.push(Patt::End);
        Ok(Self { items })
    }

    fn parse_impl(hir: &Hir, items: &mut Vec<Patt>) -> Result<usize, Error> {
        let wildcard_class = match regex_syntax::parse(".")?.into_kind() {
            HirKind::Class(c) => Ok(c),
            unsupported => Err(Error::UnexpectedRegexRepr(format!("{:?}", unsupported))),
        }?;

        match hir.kind() {
            HirKind::Literal(Literal(ref bytes)) => {
                // TODO modify Patt::Lit to use bytes rather then chars. For now, assuming ascii
                for byte in bytes.iter() {
                    items.push(Patt::Lit(*byte as char));
                }
                Ok(bytes.len())
            }
            HirKind::Class(class) if *class == wildcard_class => {
                items.push(Patt::Any);
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
        parse_test(".", vec![Patt::Any]);
    }

    #[test]
    fn parse_concat_1() {
        parse_test("a.", vec![Patt::Lit('a'), Patt::Any]);
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
        let mut expected_with_end = expected.clone();
        expected_with_end.push(Patt::End);
        let expected_pattern = Pattern { items: expected_with_end };

        let actual_pattern = Pattern::parse(pattern).expect("Cannot parse pattern");
        assert_eq!(expected_pattern, actual_pattern);
    }

}
