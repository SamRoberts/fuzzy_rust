use std::fmt::Display;

pub mod regex_question;
pub mod lattice_solution;
pub mod map_solution;
pub mod table_solution;
pub mod debug_output;
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

#[derive(Clone)]
pub struct Problem {
    pub pattern: Vec<Patt>,
    pub text: Vec<Text>,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Patt {
    Lit(char), // TODO modify to take bytes like regex library. For now assuming ascii
    Any,
    GroupStart,
    GroupEnd,
    KleeneStart(usize), // the offset of the end
    KleeneEnd(usize),   // the offset of the start
    End,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
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
