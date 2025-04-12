use crate::{Match, Step};
use std::fmt;

// NOTE: because we do character by character diffs, this won't be the real diff format
// for now. Instead, we will mimic the git diff format, expect we print out all matching
// lines and don't print any line numbers.
//
// The wording in these structs treat the patttern as the original, and text as new. So
// this diff is the change required to go from something complying with pattern, to the
// actual text.

// TODO make this configurable
const ANY: char = '?';

pub struct DiffOutput {
    pub chunks: Vec<Chunk>,
}

#[derive(Eq, PartialEq, Debug)]
pub enum Chunk {
    Same(Same),
    Diff(Diff),
}

impl Chunk {
    fn new_same(c: char) -> Self {
        Chunk::Same(Same { text: vec![c] })
    }

    fn new_added(c: char) -> Self {
        Chunk::Diff(Diff { taken: vec![], added: vec![c] })
    }

    fn new_taken(c: char) -> Self {
        Chunk::Diff(Diff { taken: vec![c], added: vec![] })
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct Same { pub text: Vec<char> }

/// This structure records consecutive characters skipped in either the text or pattern.
///
/// It's not necesary to remember the particular order these characters were skipped, and we get
/// nicer output if we consolidate them like this.
#[derive(Eq, PartialEq, Debug)]
pub struct Diff { pub taken: Vec<char>, pub added: Vec<char> }

impl DiffOutput {
    pub fn new(_score: &usize, trace: &Vec<Step<Match, char>>) -> Self {
        let mut chunks = vec![];
        for step in trace.iter() {
            let current_chunk = chunks.last_mut();
            match (step, current_chunk) {
                (Step::Hit(_, c),                    Some(Chunk::Same(same))) => same.text.push(*c),
                (Step::Hit(_, c),                    _)                       => chunks.push(Chunk::new_same(*c)),
                (Step::SkipText(c),                  Some(Chunk::Diff(diff))) => diff.added.push(*c),
                (Step::SkipText(c),                  _)                       => chunks.push(Chunk::new_added(*c)),
                (Step::SkipPattern(Match::Lit(c)),   Some(Chunk::Diff(diff))) => diff.taken.push(*c),
                (Step::SkipPattern(Match::Class(_)), Some(Chunk::Diff(diff))) => diff.taken.push(ANY),
                (Step::SkipPattern(Match::Lit(c)),   _)                       => chunks.push(Chunk::new_taken(*c)),
                (Step::SkipPattern(Match::Class(_)), _)                       => chunks.push(Chunk::new_taken(ANY)),
                _                                                             => {},
            }
        }
        DiffOutput { chunks }
    }
}

impl fmt::Display for DiffOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.chunks.iter() {
            match chunk {
                // TODO turn vec<char> into string
                Chunk::Same(same) =>
                    write!(f, "{}", Self::as_str(&same.text))?,
                Chunk::Diff(diff) if diff.added.is_empty() =>
                    write!(f, "[-{}-]", Self::as_str(&diff.taken))?,
                Chunk::Diff(diff) if diff.taken.is_empty() =>
                    write!(f, "{{+{}+}}", Self::as_str(&diff.added))?,
                Chunk::Diff(diff) =>
                    write!(f, "[-{}-]{{+{}+}}", Self::as_str(&diff.taken), Self::as_str(&diff.added))?,
            }
        }
        Ok(())
    }
}

impl DiffOutput {
    fn as_str(vec: &Vec<char>) -> String {
        vec.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_cases::TestCase;

    #[test]
    fn test_new_match_empty() {
        let test_case = TestCase::match_empty();
        let expected = "";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_match_lit_1() {
        let test_case = TestCase::match_lit_1();
        let expected = "a";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_match_lit_2() {
        let test_case = TestCase::match_lit_2();
        let expected = "ab";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_match_class_1() {
        let test_case = TestCase::match_class_1();
        let expected = "a";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_match_class_2() {
        let test_case = TestCase::match_class_2();
        let expected = "a";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_match_class_3() {
        let test_case = TestCase::match_class_3();
        let expected = "X";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_match_repetition_1() {
        let test_case = TestCase::match_repetition_1();
        let expected = "aa";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_match_repetition_2() {
        let test_case = TestCase::match_repetition_2();
        let expected = "aababb";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_match_repetition_3() {
        let test_case = TestCase::match_repetition_3();
        let expected = "0451";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_fail_empty_1() {
        let test_case = TestCase::fail_empty_1();
        let expected = "{+a+}";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_fail_empty_2() {
        let test_case = TestCase::fail_empty_2();
        let expected = "[-a-]";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_fail_lit_1() {
        let test_case = TestCase::fail_lit_1();
        let expected = "a{+a+}";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_fail_lit_2() {
        let test_case = TestCase::fail_lit_2();
        let expected = "a[-b-]a";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_fail_lit_3() {
        let test_case = TestCase::fail_lit_3();
        let expected = "{+z+}ab[-cd-]{+k+}e";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_fail_class_1() {
        let test_case = TestCase::fail_class_1();
        let expected = "[-?-]{+a+}";
        let actual = format!("{}", DiffOutput::new(&test_case.score, &test_case.trace));
        assert_eq!(expected, actual);
    }
}
