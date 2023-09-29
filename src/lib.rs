use std::fmt::Display;

pub mod regex_question;
pub mod lattice_solution;
pub mod map_solution;
pub mod table_solution;
pub mod debug_output;
pub mod diff_output;
pub mod error;

pub trait Question<Error> {
    fn ask(&self) -> Result<Problem, Error>;
}

pub trait Solution<Error> : Sized {
    fn solve(problem: &Problem) -> Result<Self, Error>;
    fn score(&self) -> &usize;
    fn trace(&self) -> &Vec<Step>;
}

pub trait Output : Display {
    fn new(problem: &Problem, score: &usize, trace: &Vec<Step>) -> Self;
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub pattern: Vec<Patt>,
    pub text: Vec<Text>,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Patt {
    Lit(char), // TODO modify to take bytes like regex library. For now assuming ascii
    Any,
    GroupStart,
    GroupEnd,
    KleeneStart(usize), // the offset of the end
    KleeneEnd(usize),   // the offset of the start
    End,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Text {
    Lit(char), // TODO modify to take bytes like regex library. For now assuming ascii
    End
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct Step {
    pub from_patt: usize,
    pub from_text: usize,
    pub to_patt: usize,
    pub to_text: usize,
    pub score: usize,
    pub kind: StepKind,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum StepKind {
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
}
