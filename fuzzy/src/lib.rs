//! This crate provides routines for finding the closest match between a regex-like pattern and a
//! text. The pattern and text do not have to match exactly, or even closely: `fuzzy` finds the
//! best fit it can by skipping text or pattern characters where necesary.
//!
//! In lieu of better documentation, see the project README for more discussion about the regex
//! features we support and how well the "closest match" works in practice.
//!
//! This crate is very early in it's development, it's API is akward, and will likely be changed in
//! breaking ways several times before it matures. We don't currently implement any convenience
//! functions which match a pattern against a text in one call.
//!
//! Implementations can be combined as follows:
//!
//! ```rust
//! use fuzzy::Output;
//! use fuzzy::regex_question::RegexQuestion;
//! use fuzzy::table_solution::TableSolution;
//! use fuzzy::diff_output::DiffOutput;
//! use fuzzy::error::Error;
//!
//! fn fuzzy_match(pattern_regex: String, text: String) -> Result<(), Error> {
//!     let question = RegexQuestion { pattern_regex, text };
//!     let problem = question.ask()?;
//!     let problem_core = problem.desugar();
//!     let solution = TableSolution::solve(&problem_core)?;
//!     let output = DiffOutput::new(&solution.score, &solution.trace);
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```

use std::fmt::Display;
use regex_syntax::hir;

pub mod regex_question;
pub mod table_solution;
pub mod debug_output;
pub mod diff_output;
pub mod flat_pattern;
pub mod error;

/// Displays the final solution.
///
/// Output implementations are just types that implement
/// [`Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html) and can be constructed out of
/// the [`score`](TableSolution::score) and [`trace`](TableSolution::trace).
///
/// If the [`TableSolution`] API changes, we will probably change this API as well.
pub trait Output : Display {
    /// Build the display. This value will have a user-friendly string representation.
    fn new(score: &usize, trace: &Vec<Step<Match, char>>) -> Self;
}

/// A problem to be solved: contains the pattern we are matching text against, as well as the text
/// which may or may not match it.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Problem<E> {
    pub pattern: Pattern<E>,
    pub text: Atoms,
}

impl Problem<Element> {
    pub fn desugar(&self) -> Problem<ElementCore> {
        let pattern = self.pattern.desugar();
        let text = self.text.clone();
        Problem { pattern, text }
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Pattern<E> {
    elems: Vec<E>,
}

impl Pattern<Element> {
    pub fn desugar(&self) -> Pattern<ElementCore> {
        let mut elems = vec![];
        for elem in &self.elems {
            match elem {
                Element::Match(m) => {
                    elems.push(ElementCore::Match(m.clone()));
                }
                Element::Capture(sugar) => {
                    let inner = sugar.desugar();
                    elems.push(ElementCore::Capture(inner));
                }
                Element::Repetition(Repetition { maximum: None, minimum, inner: sugar }) => {
                    let inner = sugar.desugar();
                    for _ in 0..*minimum {
                        elems.extend(inner.elems.iter().cloned());
                    }
                    elems.push(ElementCore::Repetition(inner));
                }
                Element::Repetition(Repetition { maximum: Some(maximum), minimum, inner: sugar }) => {
                    // We desugar a repetition with a maximum bound as a massive alternative branch
                    // TODO surely there be a better desugared output
                    // TODO surely there must also be a better algorithm to build the output

                    let inner = sugar.desugar();
                    for _ in 0..*minimum {
                        elems.extend(inner.elems.iter().cloned());
                    }

                    let empty = Pattern { elems: vec![] };
                    let mut bounded_loop = empty.clone();
                    for _ in *minimum..*maximum {
                        let mut at_least_one_elems = vec![];
                        at_least_one_elems.extend(inner.elems.iter().cloned());
                        at_least_one_elems.extend(bounded_loop.elems.iter().cloned());

                        let at_least_one = Pattern { elems: at_least_one_elems };
                        bounded_loop = Pattern { elems: vec![ElementCore::Alternative(empty.clone(), at_least_one)] };
                    }
                    elems.extend(bounded_loop.elems.into_iter())
                }
                Element::Alternative(sugar1, sugar2) => {
                    let inner1 = sugar1.desugar();
                    let inner2 = sugar2.desugar();
                    elems.push(ElementCore::Alternative(inner1, inner2));
                }
            }
        }
        Pattern { elems }
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Element {
    Match(Match),
    Capture(Pattern<Element>),
    Repetition(Repetition),
    Alternative(Pattern<Element>, Pattern<Element>),
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum ElementCore {
    Match(Match),
    Capture(Pattern<ElementCore>),
    Repetition(Pattern<ElementCore>),
    Alternative(Pattern<ElementCore>, Pattern<ElementCore>),
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Match {
    Lit(char),
    Class(Class),
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Repetition {
    minimum: usize,
    maximum: Option<usize>,
    inner: Pattern<Element>,
}

// using the term atom as we might eventually match words/lines/etc.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Atoms {
    atoms: Vec<char>,
}

/// Represents a class of characters, e.g. `.` or `[a-z]`.
///
/// Currently we implement this by re-using
/// [regex_syntax's `Class`](https://docs.rs/regex-syntax/latest/regex_syntax/hir/enum.Class.html)
/// struct. We will change this in the future.
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Class {
    hir_class: hir::Class,
}

impl From<hir::Class> for Class {
    fn from(hir_class: hir::Class) -> Class {
        Class { hir_class }
    }
}

impl Class {
    pub fn matches(&self, c: char) -> bool {
        match &self.hir_class {
            hir::Class::Unicode(ranges) =>
                ranges.iter().any(|range| range.start() <= c && c <= range.end()),
            hir::Class::Bytes(ranges) =>
                // TODO As in other places in the code, for now, we are treating the u8 vs char
                // distinctiom by naively assuming all our text is ASCII
                ranges.iter().any(|range| {
                    let start = range.start() as char;
                    let end = range.end() as char;
                    start <= c && c <= end
                }),
        }
    }
}

/// An individual element in [`TableSolution::trace`].
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Step<P, T> {
    Hit(P, T),
    SkipPattern(P),
    SkipText(T),
    StartCapture,
    StopCapture,
}

impl <P, T> Step<P, T> {
    fn map<Q, U, FQ: Fn(&P) -> Q, FU: Fn(&T) -> U>(&self, fq: FQ, fu: FU) -> Step<Q, U> {
        match self {
            Self::Hit(p, t) => Step::Hit(fq(p), fu(t)),
            Self::SkipPattern(p) => Step::SkipPattern(fq(p)),
            Self::SkipText(t) => Step::SkipText(fu(t)),
            Self::StartCapture => Step::StartCapture,
            Self::StopCapture => Step::StopCapture,
        }
    }

}

#[cfg(test)]
pub mod test_cases {
    use super::*;
    use regex_syntax::hir::HirKind;

    pub struct TestCase {
        pub problem: Problem<Element>,
        pub score: usize,
        pub trace: Vec<Step<Match, char>>,
    }

    impl TestCase {
        pub fn match_empty() -> Self {
            Self {
                problem: problem(vec![], ""),
                score: 0,
                trace: vec![],
            }
        }

        pub fn fail_empty_1() -> Self {
            Self {
                problem: problem(vec![], "a"),
                score: 1,
                trace: vec![
                    Step::SkipText('a'),
                ],
            }
        }

        pub fn fail_empty_2() -> Self {
            Self {
                problem: problem(lits("a"), ""),
                score: 1,
                trace: vec![
                    Step::SkipPattern(Match::Lit('a')),
                ],
            }
        }

        pub fn match_lit_1() -> Self {
            Self {
                problem: problem(lits("a"), "a"),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                ],
            }
        }

        pub fn match_lit_2() -> Self {
            Self {
                problem: problem(lits("ab"), "ab"),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('b'), 'b'),
                ],
            }
        }

        pub fn fail_lit_1() -> Self {
            Self {
                problem: problem(lits("a"), "aa"),
                score: 1,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::SkipText('a'),
                ],
            }
        }

        pub fn fail_lit_2() -> Self {
            Self {
                problem: problem(lits("aba"), "aa"),
                score: 1,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::SkipPattern(Match::Lit('b')),
                    Step::Hit(Match::Lit('a'), 'a'),
                ],
            }
        }

        pub fn fail_lit_3() -> Self {
            Self {
                problem: problem(lits("abcde"), "zabke"),
                score: 4,
                trace: vec![
                    Step::SkipText('z'),
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('b'), 'b'),
                    // TODO handle valid possibility that the order of next three steps is changed
                    Step::SkipPattern(Match::Lit('c')),
                    Step::SkipPattern(Match::Lit('d')),
                    Step::SkipText('k'),
                    Step::Hit(Match::Lit('e'), 'e'),
                ],
            }
        }

        pub fn match_class_1() -> Self {
            Self {
                problem: problem(vec![class(".")], "a"),
                score: 0,
                trace: vec![
                    Step::Hit(patt_class("."), 'a'),
                ],
            }
        }

        pub fn match_class_2() -> Self {
            Self {
                problem: problem(vec![class("[a-zA-Z]")], "a"),
                score: 0,
                trace: vec![
                    Step::Hit(patt_class("[a-zA-Z]"), 'a'),
                ],
            }
        }

        pub fn match_class_3() -> Self {
            Self {
                problem: problem(vec![class("[a-zA-Z]")], "X"),
                score: 0,
                trace: vec![
                    Step::Hit(patt_class("[a-zA-Z]"), 'X'),
                ],
            }
        }

        pub fn fail_class_1() -> Self {
            Self {
                problem: problem(vec![class("[^a]")], "a"),
                score: 2,
                trace: vec![
                    // TODO handle valid possibility that the order of next two steps is reversed
                    Step::SkipPattern(patt_class("[^a]")),
                    Step::SkipText('a'),
                ],
            }
        }

        pub fn match_alternative_1() -> Self {
            Self {
                problem: problem(vec![alt(lits("ab"), lits("cd"))], "ab"),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('b'), 'b'),
                ],
            }
        }

        pub fn match_alternative_2() -> Self {
            Self {
                problem: problem(vec![alt(lits("ab"), lits("cd"))], "cd"),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('c'), 'c'),
                    Step::Hit(Match::Lit('d'), 'd'),
                ],
            }
        }

        pub fn match_alternative_3() -> Self {
            Self {
                problem: problem(
                    vec![
                        alt(lits("a"), vec![alt(lits("b"), vec![alt(lits("c"), lits("d"))])]),
                        lit('z')
                    ],
                    "cz"
                ),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('c'), 'c'),
                    Step::Hit(Match::Lit('z'), 'z'),
                ],
            }
        }

        pub fn fail_alternative_1() -> Self {
            Self {
                problem: problem(vec![alt(lits("ab"), lits("cd"))], "acd"),
                score: 1,
                trace: vec![
                    Step::SkipText('a'),
                    Step::Hit(Match::Lit('c'), 'c'),
                    Step::Hit(Match::Lit('d'), 'd'),
                ],
            }
        }

        pub fn match_repetition_1() -> Self {
            Self {
                problem: problem(vec![rep(lits("a"))], "aa"),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('a'), 'a'),
                ],
            }
        }

        pub fn match_repetition_2() -> Self {
            Self {
                problem: problem(vec![rep(vec![lit('a'), rep(lits("b"))])], "aababb"),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('b'), 'b'),
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('b'), 'b'),
                    Step::Hit(Match::Lit('b'), 'b'),
                ],
            }
        }

        pub fn match_repetition_3() -> Self {
            Self {
                problem: problem(vec![rep(vec![class("[0-9]")])], "0451"),
                score: 0,
                trace: vec![
                    Step::Hit(patt_class("[0-9]"), '0'),
                    Step::Hit(patt_class("[0-9]"), '4'),
                    Step::Hit(patt_class("[0-9]"), '5'),
                    Step::Hit(patt_class("[0-9]"), '1'),
                ],
            }
        }

        pub fn match_repetition_4() -> Self {
            Self {
                problem: problem(vec![rep_min(1, lits("a"))], "a"),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                ],
            }
        }

        pub fn match_repetition_5() -> Self {
            Self {
                problem: problem(vec![rep_bound(0, 5, lits("a"))], "aaaa"),
                score: 0,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::Hit(Match::Lit('a'), 'a'),
                ],
            }
        }

        pub fn fail_repetition_1() -> Self {
            Self {
                problem: problem(vec![rep(lits("a"))], "aba"),
                score: 1,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::SkipText('b'),
                    Step::Hit(Match::Lit('a'), 'a'),
                ],
            }
        }

        pub fn fail_repetition_2() -> Self {
            Self {
                problem: problem(vec![rep_min(1, lits("a"))], ""),
                score: 1,
                trace: vec![
                    Step::SkipPattern(Match::Lit('a')),
                ],
            }
        }

        pub fn fail_repetition_3() -> Self {
            Self {
                problem: problem(vec![rep_bound(0, 1, lits("a"))], "aa"),
                score: 1,
                trace: vec![
                    Step::Hit(Match::Lit('a'), 'a'),
                    Step::SkipText('a'),
                ],
            }
        }
    }

    pub fn patt_class(regex: &str) -> Match {
        let wildcard_class = match regex_syntax::parse(regex).unwrap().into_kind() {
            HirKind::Class(c) => c,
            unsupported => panic!("Unexpected regex_syntax for class: {:?}", unsupported),
        };

        Match::Class(Class::from(wildcard_class))
    }

    pub fn problem(elems: Vec<Element>, text: &str) -> Problem<Element> {
        let atoms = text.chars().collect();
        Problem {
            pattern: Pattern { elems },
            text:    Atoms { atoms },
        }
    }

    pub fn lits(cs: &str) -> Vec<Element> {
        cs.chars().map(|c| lit(c)).collect()
    }

    pub fn lit(c: char) -> Element {
        Element::Match(Match::Lit(c))
    }

    pub fn class(regex: &str) -> Element {
        let wildcard_class = match regex_syntax::parse(regex).unwrap().into_kind() {
            HirKind::Class(c) => c,
            unsupported => panic!("Unexpected regex_syntax for class: {:?}", unsupported),
        };

        Element::Match(Match::Class(Class::from(wildcard_class)))
    }

    pub fn rep(elems: Vec<Element>) -> Element {
        rep_min(0, elems)
    }

    pub fn rep_min(minimum: usize, elems: Vec<Element>) -> Element {
        let maximum = None;
        let inner = Pattern { elems };
        Element::Repetition(Repetition { minimum, maximum, inner })
    }

    pub fn rep_bound(minimum: usize, maximum: usize, elems: Vec<Element>) -> Element {
        let max_opt = Some(maximum);
        let inner = Pattern { elems };
        Element::Repetition(Repetition { minimum, maximum: max_opt, inner })
    }

    pub fn alt(left: Vec<Element>, right: Vec<Element>) -> Element {
        Element::Alternative(Pattern { elems: left }, Pattern { elems: right })
    }

    pub fn capture(elems: Vec<Element>) -> Element {
        Element::Capture(Pattern { elems })
    }
}
