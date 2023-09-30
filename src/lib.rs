//! This crate provides routines for finding the closest match between a regex-like pattern and a
//! text. The pattern and text do not have to match exactly, or even closely: `fuzzy` finds the
//! best fit it can by skipping text or pattern characters where necesary.
//!
//! In lieu of better documentation, see the project README for more discussion about the regex
//! features we support and how well the "closest match" works in practice.
//!
//! This crate is very early in it's development, it's API is akward, and will likely be changed in
//! breaking ways several times before it matures. We don't currently implement any convenience
//! functions which match a pattern against a text in one call. Instead, the crate provides
//! implementations of the following three traits, which can be combined to do the match:
//!
//! - a [`Question`] produces a [`Problem`] to be solved.
//! - a [`Solution`] calculates the optimal match and provides the corresponding
//!   [`score`](Solution::score) and [`trace`](Solution::trace).
//! - an [`Output`] displays [`Problem`] and [`Solution`] info to the user.
//!
//! Implementations can be combined as follows:
//!
//! ```rust
//! use fuzzy::{Question, Solution, Output};
//! use fuzzy::regex_question::RegexQuestion;
//! use fuzzy::table_solution::TableSolution;
//! use fuzzy::diff_output::DiffOutput;
//! use fuzzy::error::Error;
//!
//! fn fuzzy_match(pattern_regex: String, text: String) -> Result<(), Error> {
//!     let question = RegexQuestion { pattern_regex, text };
//!     let problem = question.ask()?;
//!     let solution = TableSolution::solve(&problem)?;
//!     let output = DiffOutput::new(&problem, solution.score(), solution.trace());
//!     println!("{}", output);
//!     Ok(())
//! }
//! ```
//!
//! # Overview
//!
//! The main three traits in our API are [`Question`], [`Solution`], and [`Output`]. See
//! submodules for the various implementations.
//!
//! In addition to these traits:
//!
//! - The [`Problem`] contains the parsed [`pattern`](Problem::pattern) and [`text`](Problem::text).
//!    - A [`Patt`] is a single item from the parsed pattern.
//!    - A [`Text`] is a single character from the text.
//! - From the [`Solution`]:
//!    - The [`score`](Solution::score) is a simple `usize`.
//!    - A [`Step`] is a single item from the optimal [`trace`](Solution::trace).

use std::fmt::Display;
use regex_syntax::hir;

pub mod regex_question;
pub mod lattice_solution;
pub mod map_solution;
pub mod table_solution;
pub mod debug_output;
pub mod diff_output;
pub mod error;

/// A builder of [`Problem`] values.
///
/// Questions are built from some specification of a pattern and text, but the details are not part
/// of this API: different Question implementations can do this differently.
pub trait Question<Error> {
    /// Try to build a [`Problem`].
    fn ask(&self) -> Result<Problem, Error>;
}

/// Calculates the optimal solution for a [`Problem`].
///
/// In practice, our solution implementations to date are simply structs directly storing the final
/// calculated `score` and `trace`. We will probably change this API in the future.
pub trait Solution<Error> : Sized {
    /// Try to figure out the solution for a [`Problem`].
    fn solve(problem: &Problem) -> Result<Self, Error>;

    /// Return the final score for the solution.
    ///
    /// This score represents the cost of mismatches: `0` is best, higher worse.
    fn score(&self) -> &usize;

    /// Return the [`Step`]s followed by the optimal match between pattern and text.
    fn trace(&self) -> &Vec<Step>;
}

/// Displays the final solution.
///
/// Output implementations are just types that implement
/// [`Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html) and can be constructed out of
/// a [`Problem`], [`score`](Solution::score), and [`trace`](Solution::trace).
///
/// If the [`Solution`] API changes, we will probably change this API as well.
pub trait Output : Display {
    /// Build the display. This value will have a user-friendly string representation.
    fn new(problem: &Problem, score: &usize, trace: &Vec<Step>) -> Self;
}

/// Represents a parsed pattern and the text it is meant to match.
#[derive(Clone, Debug)]
pub struct Problem {
    /// The individual [`Patt`] values in the parsed pattern.
    pub pattern: Vec<Patt>,

    /// The individual [`Text`] values in the text to match.
    pub text: Vec<Text>,
}

/// An individual element in [`Problem::pattern`].
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Patt {
    /// Matches a specific character.
    ///
    /// Although this API implies this crate operates on unicode characters, the current code
    /// sometimes naively converts bytes to characters, assuming ASCII.
    Lit(char),
    /// Matches a class of characters, e.g. `.` or `[a-z]`.
    Class(Class),
    GroupStart,
    GroupEnd,
    /// Starts a repetition.
    ///
    /// This stores the offset between this item and the corresponding future
    /// [`KleeneEnd`](Patt::KleeneEnd) item.
    KleeneStart(usize),
    /// Ends a repetition.
    ///
    /// This stores the offset between this item and the corresponding past
    /// [`KleeneStart`](Patt::KleeneStart) item.
    KleeneEnd(usize),
    /// Ends the pattern.
    ///
    /// Although this is redundant, fuzzy currently requires the pattern vector to end with
    /// this value. We will probably remove it in the future.
    End,
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

/// An individual element in [`Problem::text`].
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Text {
    /// A character.
    ///
    /// Although this API implies the crate operates on unicode characters, the current code
    /// sometimes naively converts bytes to characters, assuming ASCII.
    Lit(char),
    /// Ends the text.
    ///
    /// Although this is redundant, fuzzy currently requires the text vector to end with
    /// this value. We will probably remove it in the future.
    End
}

/// An individual element in [`Solution::trace`].
///
/// Each step represents a transition from one [`Patt`] to another, or one [`Text`] to another, or
/// both.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct Step {
    /// The index into [`Problem::pattern`] we transitioned from.
    pub from_patt: usize,
    /// The index into [`Problem::text`] we transitioned into.
    pub from_text: usize,
    /// The index into [`Problem::pattern`] we transitioned to.
    pub to_patt: usize,
    /// The index into [`Problem::text`] we transitioned to.
    pub to_text: usize,
    /// The cumulative score from this step to the end of [`Solution::trace`].
    pub score: usize,
    /// The type of step (e.g. did the pattern and text match? Did we skip something?)
    pub kind: StepKind,
}

/// Included in [`Step`] to represent the type of step taken.
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum StepKind {
    /// This step did not involve matching or skipping any characters in the text or pattern.
    ///
    /// This is typically used when navigating between control [`Patt`] elements (e.g. skip past a
    /// repetition).
    NoOp,
    Hit,
    SkipText,
    SkipPattern,
    StartCapture,
    StopCapture,
}

#[cfg(test)]
pub mod test_cases {
    use super::*;
    use regex_syntax::hir::HirKind;

    // A test case may or may not have a well defined trace
    pub struct TestCase<Trace> {
        pub problem: Problem,
        pub score: usize,
        pub trace: Trace
    }

    impl TestCase<Vec<Step>> {
        pub fn match_empty() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::End],
                    text:    vec![Text::End],
                },
                score: 0,
                trace: vec![],
            }
        }

        pub fn match_lit_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::Lit('a'), Patt::End],
                    text:    vec![Text::Lit('a'), Text::End],
                },
                score: 0,
                trace: vec![
                    Self::step(0, 0, 1, 1, 0, StepKind::Hit),
                ],
            }
        }

        pub fn match_lit_2() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::Lit('a'), Patt::Lit('b'), Patt::End],
                    text:    vec![Text::Lit('a'), Text::Lit('b'), Text::End],
                },
                score: 0,
                trace: vec![
                    Self::step(0, 0, 1, 1, 0, StepKind::Hit),
                    Self::step(1, 1, 2, 2, 0, StepKind::Hit),
                ],
            }
        }

        pub fn match_class_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![patt_class("."), Patt::End],
                    text:    vec![Text::Lit('a'), Text::End],
                },
                score: 0,
                trace: vec![
                    Self::step(0, 0, 1, 1, 0, StepKind::Hit),
                ],
            }
        }

        pub fn match_class_2() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![patt_class("[a-zA-Z]"), Patt::End],
                    text:    vec![Text::Lit('a'), Text::End],
                },
                score: 0,
                trace: vec![
                    Self::step(0, 0, 1, 1, 0, StepKind::Hit),
                ],
            }
        }

        pub fn match_class_3() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![patt_class("[a-zA-Z]"), Patt::End],
                    text:    vec![Text::Lit('X'), Text::End],
                },
                score: 0,
                trace: vec![
                    Self::step(0, 0, 1, 1, 0, StepKind::Hit),
                ],
            }
        }

        pub fn match_kleene_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::KleeneStart(2), Patt::Lit('a'), Patt::KleeneEnd(2), Patt::End],
                    text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
                },
                score: 0,
                trace: vec![
                    Self::step(0, 0, 1, 0, 0, StepKind::NoOp),
                    Self::step(1, 0, 2, 1, 0, StepKind::Hit),
                    Self::step(2, 1, 0, 1, 0, StepKind::NoOp),
                    Self::step(0, 1, 1, 1, 0, StepKind::NoOp),
                    Self::step(1, 1, 2, 2, 0, StepKind::Hit),
                    Self::step(2, 2, 0, 2, 0, StepKind::NoOp),
                    Self::step(0, 2, 3, 2, 0, StepKind::NoOp),
                ],
            }
        }

        pub fn match_kleene_2() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![
                        Patt::KleeneStart(5),
                        Patt::Lit('a'),
                        Patt::KleeneStart(2),
                        Patt::Lit('b'),
                        Patt::KleeneEnd(2),
                        Patt::KleeneEnd(5),
                        Patt::End
                    ],
                    text: vec![
                        Text::Lit('a'),
                        Text::Lit('a'),
                        Text::Lit('b'),
                        Text::Lit('a'),
                        Text::Lit('b'),
                        Text::Lit('b'),
                        Text::End
                    ],
                },
                score: 0,
                trace: vec![
                    Self::step(0, 0, 1, 0, 0, StepKind::NoOp),
                    Self::step(1, 0, 2, 1, 0, StepKind::Hit),
                    Self::step(2, 1, 5, 1, 0, StepKind::NoOp),
                    Self::step(5, 1, 0, 1, 0, StepKind::NoOp),
                    Self::step(0, 1, 1, 1, 0, StepKind::NoOp),
                    Self::step(1, 1, 2, 2, 0, StepKind::Hit),
                    Self::step(2, 2, 3, 2, 0, StepKind::NoOp),
                    Self::step(3, 2, 4, 3, 0, StepKind::Hit),
                    Self::step(4, 3, 2, 3, 0, StepKind::NoOp),
                    Self::step(2, 3, 5, 3, 0, StepKind::NoOp),
                    Self::step(5, 3, 0, 3, 0, StepKind::NoOp),
                    Self::step(0, 3, 1, 3, 0, StepKind::NoOp),
                    Self::step(1, 3, 2, 4, 0, StepKind::Hit),
                    Self::step(2, 4, 3, 4, 0, StepKind::NoOp),
                    Self::step(3, 4, 4, 5, 0, StepKind::Hit),
                    Self::step(4, 5, 2, 5, 0, StepKind::NoOp),
                    Self::step(2, 5, 3, 5, 0, StepKind::NoOp),
                    Self::step(3, 5, 4, 6, 0, StepKind::Hit),
                    Self::step(4, 6, 2, 6, 0, StepKind::NoOp),
                    Self::step(2, 6, 5, 6, 0, StepKind::NoOp),
                    Self::step(5, 6, 0, 6, 0, StepKind::NoOp),
                    Self::step(0, 6, 6, 6, 0, StepKind::NoOp),
                ],
            }
        }

        pub fn match_kleene_3() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::KleeneStart(2), patt_class("[0-9]"), Patt::KleeneEnd(2), Patt::End],
                    text:    vec![Text::Lit('0'), Text::Lit('4'), Text::Lit('5'), Text::Lit('1'), Text::End],
                },
                score: 0,
                trace: vec![
                    Self::step(0, 0, 1, 0, 0, StepKind::NoOp),
                    Self::step(1, 0, 2, 1, 0, StepKind::Hit),
                    Self::step(2, 1, 0, 1, 0, StepKind::NoOp),
                    Self::step(0, 1, 1, 1, 0, StepKind::NoOp),
                    Self::step(1, 1, 2, 2, 0, StepKind::Hit),
                    Self::step(2, 2, 0, 2, 0, StepKind::NoOp),
                    Self::step(0, 2, 1, 2, 0, StepKind::NoOp),
                    Self::step(1, 2, 2, 3, 0, StepKind::Hit),
                    Self::step(2, 3, 0, 3, 0, StepKind::NoOp),
                    Self::step(0, 3, 1, 3, 0, StepKind::NoOp),
                    Self::step(1, 3, 2, 4, 0, StepKind::Hit),
                    Self::step(2, 4, 0, 4, 0, StepKind::NoOp),
                    Self::step(0, 4, 3, 4, 0, StepKind::NoOp),
                ],
            }
        }

        pub fn fail_empty_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::End],
                    text:    vec![Text::Lit('a'), Text::End],
                },
                score: 1,
                trace: vec![
                    Self::step(0, 0, 0, 1, 1, StepKind::SkipText),
                ],
            }
        }

        pub fn fail_empty_2() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::Lit('a'), Patt::End],
                    text:    vec![Text::End],
                },
                score: 1,
                trace: vec![
                    Self::step(0, 0, 1, 0, 1, StepKind::SkipPattern),
                ],
            }
        }

        pub fn fail_lit_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::Lit('a'), Patt::End],
                    text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
                },
                score: 1,
                trace: vec![
                    Self::step(0, 0, 1, 1, 1, StepKind::Hit),
                    Self::step(1, 1, 1, 2, 1, StepKind::SkipText),
                ],
            }
        }

        pub fn fail_lit_2() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::Lit('a'), Patt::Lit('b'), Patt::Lit('a'), Patt::End],
                    text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
                },
                score: 1,
                trace: vec![
                    Self::step(0, 0, 1, 1, 1, StepKind::Hit),
                    Self::step(1, 1, 2, 1, 1, StepKind::SkipPattern),
                    Self::step(2, 1, 3, 2, 0, StepKind::Hit),
                ],
            }
        }

        pub fn fail_lit_3() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::Lit('a'), Patt::Lit('b'), Patt::Lit('c'), Patt::Lit('d'), Patt::Lit('e'), Patt::End],
                    text:    vec![Text::Lit('z'), Text::Lit('a'), Text::Lit('b'), Text::Lit('k'), Text::Lit('e'), Text::End],
                },
                score: 4,
                trace: vec![
                    Self::step(0, 0, 0, 1, 4, StepKind::SkipText),
                    Self::step(0, 1, 1, 2, 3, StepKind::Hit),
                    Self::step(1, 2, 2, 3, 3, StepKind::Hit),
                    // TODO handle valid possibility that the order of next two steps is reversed
                    Self::step(2, 3, 2, 4, 3, StepKind::SkipText),
                    Self::step(2, 4, 3, 4, 2, StepKind::SkipPattern),
                    Self::step(3, 4, 4, 4, 1, StepKind::SkipPattern),
                    Self::step(4, 4, 5, 5, 0, StepKind::Hit),
                ],
            }
        }

        pub fn fail_class_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![patt_class("[^a]"), Patt::End],
                    text:    vec![Text::Lit('a'), Text::End],
                },
                score: 2,
                trace: vec![
                    // TODO handle valid possibility that the order of next two steps is reversed
                    Self::step(0, 0, 0, 1, 2, StepKind::SkipText),
                    Self::step(0, 1, 1, 1, 1, StepKind::SkipPattern),
                ],
            }
        }

        fn step(from_patt: usize, from_text: usize, to_patt: usize, to_text: usize, score: usize, kind: StepKind) -> Step {
            Step { from_patt, from_text, to_patt, to_text, score, kind }
        }

    }

    // these cases have multiple optimal traces so can't easily check trace
    impl TestCase<()> {
        pub fn fail_kleene_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::KleeneStart(2), Patt::Lit('a'), Patt::KleeneEnd(2), Patt::End],
                    text:    vec![Text::Lit('a'), Text::Lit('b'), Text::Lit('a'), Text::End],
                },
                score: 1,
                trace: (),
            }
        }
    }

    pub fn patt_class(regex: &str) -> Patt {
        let wildcard_class = match regex_syntax::parse(regex).unwrap().into_kind() {
            HirKind::Class(c) => c,
            unsupported => panic!("Unexpected regex_syntax for class: {:?}", unsupported),
        };

        Patt::Class(Class::from(wildcard_class))
    }
}
