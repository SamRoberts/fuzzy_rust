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
    fn trace(&self) -> &Vec<Step<Patt, Text>>;
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
    fn new(problem: &Problem, score: &usize, trace: &Vec<Step<Patt, Text>>) -> Self;
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
    /// Starts the first branch of an alternation.
    ///
    /// This stores the offset between this item and the corresponding
    /// [`AlternativeRight`](Patt::AlternativeRight) branch.
    AlternativeLeft(usize),
    /// Starts the second branch of an alternation.
    ///
    /// This stores the offset between this item and the element immediately after the alternation.
    AlternativeRight(usize),
    /// Starts a repetition.
    ///
    /// This stores the offset between this item and the corresponding future
    /// [`RepetitionEnd`](Patt::RepetitionEnd) item.
    RepetitionStart(usize),
    /// Ends a repetition.
    ///
    /// This stores the offset between this item and the corresponding past
    /// [`RepetitionStart`](Patt::RepetitionStart) item.
    RepetitionEnd(usize),
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
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Step<P, T> {
    // NOTE: making P and T generic seems overkill right now, but will be useful when I completely
    // separate Patt/Text in solutions from Patt/Text in top-level api
    Hit(P, T),
    SkipPattern(P),
    SkipText(T),
    StartCapture,
    StopCapture,
}

impl <P, T> Step<P, T> {
    fn with<Q, U>(&self, q: Q, u: U) -> Step<Q, U> {
        match self {
            Self::Hit(_, _) => Step::Hit(q, u),
            Self::SkipPattern(_) => Step::SkipPattern(q),
            Self::SkipText(_) => Step::SkipText(u),
            Self::StartCapture => Step::StartCapture,
            Self::StopCapture => Step::StopCapture,
        }
    }
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

    impl TestCase<Vec<Step<Patt, Text>>> {
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
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
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
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::Hit(Patt::Lit('b'), Text::Lit('b')),
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
                    Step::Hit(patt_class("."), Text::Lit('a')),
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
                    Step::Hit(patt_class("[a-zA-Z]"), Text::Lit('a')),
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
                    Step::Hit(patt_class("[a-zA-Z]"), Text::Lit('X')),
                ],
            }
        }

        pub fn match_alternative_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![
                        Patt::AlternativeLeft(3),
                        Patt::Lit('a'),
                        Patt::Lit('b'),
                        Patt::AlternativeRight(3),
                        Patt::Lit('c'),
                        Patt::Lit('d'),
                        Patt::End
                    ],
                    text: vec![Text::Lit('a'), Text::Lit('b'), Text::End],
                },
                score: 0,
                trace: vec![
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::Hit(Patt::Lit('b'), Text::Lit('b')),
                ],
            }
        }

        pub fn match_alternative_2() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![
                        Patt::AlternativeLeft(3),
                        Patt::Lit('a'),
                        Patt::Lit('b'),
                        Patt::AlternativeRight(3),
                        Patt::Lit('c'),
                        Patt::Lit('d'),
                        Patt::End
                    ],
                    text: vec![Text::Lit('c'), Text::Lit('d'), Text::End],
                },
                score: 0,
                trace: vec![
                    Step::Hit(Patt::Lit('c'), Text::Lit('c')),
                    Step::Hit(Patt::Lit('d'), Text::Lit('d')),
                ],
            }
        }

        pub fn match_alternative_3() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![
                        Patt::AlternativeLeft(2),
                        Patt::Lit('a'),
                        Patt::AlternativeRight(8),
                        Patt::AlternativeLeft(2),
                        Patt::Lit('b'),
                        Patt::AlternativeRight(5),
                        Patt::AlternativeLeft(2),
                        Patt::Lit('c'),
                        Patt::AlternativeRight(2),
                        Patt::Lit('d'),
                        Patt::Lit('z'),
                        Patt::End
                    ],
                    text: vec![Text::Lit('c'), Text::Lit('z'), Text::End],
                },
                score: 0,
                trace: vec![
                    Step::Hit(Patt::Lit('c'), Text::Lit('c')),
                    Step::Hit(Patt::Lit('z'), Text::Lit('z')),
                ],
            }
        }

        pub fn match_repetition_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::RepetitionStart(2), Patt::Lit('a'), Patt::RepetitionEnd(2), Patt::End],
                    text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
                },
                score: 0,
                trace: vec![
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                ],
            }
        }

        pub fn match_repetition_2() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![
                        Patt::RepetitionStart(5),
                        Patt::Lit('a'),
                        Patt::RepetitionStart(2),
                        Patt::Lit('b'),
                        Patt::RepetitionEnd(2),
                        Patt::RepetitionEnd(5),
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
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::Hit(Patt::Lit('b'), Text::Lit('b')),
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::Hit(Patt::Lit('b'), Text::Lit('b')),
                    Step::Hit(Patt::Lit('b'), Text::Lit('b')),
                ],
            }
        }

        pub fn match_repetition_3() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::RepetitionStart(2), patt_class("[0-9]"), Patt::RepetitionEnd(2), Patt::End],
                    text:    vec![Text::Lit('0'), Text::Lit('4'), Text::Lit('5'), Text::Lit('1'), Text::End],
                },
                score: 0,
                trace: vec![
                    Step::Hit(patt_class("[0-9]"), Text::Lit('0')),
                    Step::Hit(patt_class("[0-9]"), Text::Lit('4')),
                    Step::Hit(patt_class("[0-9]"), Text::Lit('5')),
                    Step::Hit(patt_class("[0-9]"), Text::Lit('1')),
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
                    Step::SkipText(Text::Lit('a')),
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
                    Step::SkipPattern(Patt::Lit('a')),
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
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::SkipText(Text::Lit('a')),
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
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::SkipPattern(Patt::Lit('b')),
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
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
                    Step::SkipText(Text::Lit('z')),
                    Step::Hit(Patt::Lit('a'), Text::Lit('a')),
                    Step::Hit(Patt::Lit('b'), Text::Lit('b')),
                    // TODO handle valid possibility that the order of next three steps is changed
                    Step::SkipText(Text::Lit('k')),
                    Step::SkipPattern(Patt::Lit('c')),
                    Step::SkipPattern(Patt::Lit('d')),
                    Step::Hit(Patt::Lit('e'), Text::Lit('e')),
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
                    Step::SkipText(Text::Lit('a')),
                    Step::SkipPattern(patt_class("[^a]")),
                ],
            }
        }

        pub fn fail_alternative_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![
                        Patt::AlternativeLeft(3),
                        Patt::Lit('a'),
                        Patt::Lit('b'),
                        Patt::AlternativeRight(3),
                        Patt::Lit('c'),
                        Patt::Lit('d'),
                        Patt::End
                    ],
                    text: vec![Text::Lit('a'), Text::Lit('c'), Text::Lit('d'), Text::End],
                },
                score: 1,
                trace: vec![
                    Step::SkipText(Text::Lit('a')),
                    Step::Hit(Patt::Lit('c'), Text::Lit('c')),
                    Step::Hit(Patt::Lit('d'), Text::Lit('d')),
                ],
            }
        }
    }

    // these cases have multiple optimal traces so can't easily check trace
    impl TestCase<()> {
        pub fn fail_repetition_1() -> Self {
            Self {
                problem: Problem {
                    pattern: vec![Patt::RepetitionStart(2), Patt::Lit('a'), Patt::RepetitionEnd(2), Patt::End],
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
