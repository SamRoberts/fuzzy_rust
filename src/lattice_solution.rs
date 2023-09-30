use crate::{Patt, Problem, Solution, Step, StepKind, Text};
use crate::error::Error;
use std::fmt::Debug;

// An initial naively recursive family of lattice solutions
//
// Implementations define a mutable state space
// and an index type that addresses it.
//
// Each index has a set of child indices that it can reach
// by matching the text to the pattern, or skipping one or
// the other. These links form a lattice. The implementation
// must ensure that Ix::can_restart is implemented correctly
// so that there are no loops in the lattice. All paths
// through the lattice begin at Ix::start and end at Ix::end.
//
// The solve method in this code traverses the state space by
// naively recursing from each node to it's child nodes,
// stopping when it reaches the end node.

pub trait LatticeSolution : Sized {
    type Conf: LatticeConfig<Self::Ix>;
    type Ix: LatticeIx<Self::Conf>;
    type State: LatticeState<Self::Conf, Self::Ix>;

    fn new(score: usize, trace: Vec<Step>) -> Self;

    fn score_lattice(&self) -> &usize;
    fn trace_lattice(&self) -> &Vec<Step>;

    fn solve_lattice(problem: &Problem) -> Result<Self, Error> {
        let conf = Self::Conf::new(problem);
        let mut state = Self::State::new(&conf);

        let start_ix = Self::Ix::start();
        let end_ix = Self::Ix::end(&conf);

        let start_lead = Next { cost: 0, next: start_ix, kind: StepKind::NoOp };
        let _ = Self::solve_ix(&conf, &mut state, end_ix, start_lead);

        let score = match state.get(start_ix) {
            Node::Done(Done { score, .. }) => Ok(score),
            _ => Err(Error::IncompleteFinalState),
        }?;

        let mut trace = vec![];
        let mut from = start_ix;
        while let Node::Done(done) = state.get(from) {
            if from == end_ix { break; }
            let step = Self::Ix::to_step(&conf, &from, &done);
            trace.push(step);
            from = done.next;
        }
        if from != end_ix {
            return Err(Error::IncompleteFinalState);
        }

        Ok(LatticeSolution::new(score, trace))
    }

    fn solve_ix(
        conf: &Self::Conf,
        state: &mut Self::State,
        end_ix: Self::Ix,
        lead: Next<Self::Ix>,
     ) -> Result<Done<Self::Ix>, Error> {
        let Next { cost, kind, next: ix } = lead; // the step's lead is our current ix

        match state.get(ix) {
            Node::Working =>
                Err(Error::InfiniteLoop(format!("{:?}", ix))),
            Node::Done(done) =>
                Ok(Done { score: done.score + cost, next: ix, kind }),
            Node::Ready => {
                state.set(ix, Node::Working);

                let mut maybe_score = None;
                let (patt, text) = conf.get(ix);

                match (patt, text) {
                    (Patt::Class(class), Text::Lit(c)) if class.matches(*c) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.hit())?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    (Patt::Lit(a), Text::Lit(b)) if *a == *b => {
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.hit())?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    _ =>
                        (),
                }

                match text {
                    Text::Lit(_) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.skip_text())?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Text::End =>
                        (),
                }

                match patt {
                    Patt::Lit(_) | Patt::Class(_) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.skip_patt())?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::GroupStart => {
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.start_group())?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::GroupEnd => {
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.stop_group())?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::KleeneEnd(off) if ix.can_restart() => {
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.restart_kleene(*off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::KleeneEnd(_) => { // cannot restart
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.end_kleene())?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::KleeneStart(off) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.start_kleene())?;
                        maybe_score = Self::update(maybe_score, outcome);
                        let outcome = Self::solve_ix(conf, state, end_ix, ix.pass_kleene(*off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    }
                    Patt::End =>
                        (),
                }

                let score = match maybe_score {
                    Some(score) => score,
                    None if ix == end_ix =>
                        Done { score: 0, next: end_ix, kind: StepKind::NoOp },
                    None =>
                        return Err(Error::Blocked(format!("{:?}", ix))),
                };

                state.set(ix, Node::Done(score));
                Ok(Done { score: score.score + cost, next: ix, kind })
            }
        }
    }

    fn update(current: Option<Done<Self::Ix>>, new: Done<Self::Ix>) -> Option<Done<Self::Ix>> {
        Some(current.map_or(new, |c| Done::optimal(c, new)))
    }
}

impl <Sln> Solution<Error> for Sln where
    Sln: LatticeSolution,
{
    fn score(&self) -> &usize {
        LatticeSolution::score_lattice(self)
    }

    fn trace(&self) -> &Vec<Step> {
        LatticeSolution::trace_lattice(self)
    }

    fn solve(problem: &Problem) -> Result<Self, Error> {
        LatticeSolution::solve_lattice(problem)
    }
}

pub trait LatticeConfig<Ix> {
    fn new(problem: &Problem) -> Self;
    fn get(&self, ix: Ix) -> (&Patt, &Text);
}

pub trait LatticeState<Conf, Ix> {
    fn new(conf: &Conf) -> Self;
    fn get(&self, ix: Ix) -> Node<Ix>;
    fn set(&mut self, ix: Ix, node: Node<Ix>);
}

pub trait LatticeIx<Conf> : Eq + PartialEq + Copy + Clone + Debug + Sized {
    fn start() -> Self;
    fn end(conf: &Conf) -> Self;

    fn skip_text(&self) -> Next<Self>;
    fn skip_patt(&self) -> Next<Self>;
    fn hit(&self) -> Next<Self>;
    fn start_group(&self) -> Next<Self>;
    fn stop_group(&self) -> Next<Self>;
    fn start_kleene(&self) -> Next<Self>;
    fn end_kleene(&self) -> Next<Self>;
    fn pass_kleene(&self, off: usize) -> Next<Self>;
    fn restart_kleene(&self, off: usize) -> Next<Self>;

    fn can_restart(&self) -> bool;

    fn to_step(conf: &Conf, from: &Self, done: &Done<Self>) -> Step;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Node<Ix: Sized> {
    Ready,
    Working,
    Done(Done<Ix>),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Done<Ix: Sized> {
    pub score: usize,
    pub next: Ix,
    pub kind: StepKind,
}

impl <Ix: Sized> Done<Ix> {
    fn optimal(left: Self, right: Self) -> Self {
        if left.score <= right.score { left } else { right }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Next<Ix> {
    pub cost: usize,
    pub next: Ix,
    pub kind: StepKind,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::test_cases::TestCase;

    pub fn test_solve_match_empty<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_empty());
    }

    pub fn test_solve_match_lit_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_lit_1());
    }

    pub fn test_solve_match_lit_2<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_lit_2());
    }

    pub fn test_solve_match_class_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_class_1());
    }

    pub fn test_solve_match_class_2<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_class_2());
    }

    pub fn test_solve_match_class_3<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_class_3());
    }

    pub fn test_solve_match_kleene_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_kleene_1());
    }

    pub fn test_solve_match_kleene_2<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_kleene_2());
    }

    pub fn test_solve_match_kleene_3<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_kleene_3());
    }

    pub fn test_solve_fail_empty_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::fail_empty_1());
    }

    pub fn test_solve_fail_empty_2<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::fail_empty_2());
    }

    pub fn test_solve_fail_lit_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::fail_lit_1());
    }

    pub fn test_solve_fail_lit_2<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::fail_lit_2());
    }

    pub fn test_solve_fail_lit_3<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::fail_lit_3());
    }

    pub fn test_solve_fail_class_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::fail_class_1());
    }

    pub fn test_solve_fail_kleene_1<Sln: LatticeSolution>() {
        test_solve_for_test_case_with_ambiguous_trace::<Sln>(TestCase::fail_kleene_1());
    }

    pub fn test_solve_for_test_case<Sln: LatticeSolution>(test_case: TestCase<Vec<Step>>) {
        let actual = Sln::solve(&test_case.problem).unwrap();
        assert_eq!(test_case.score, *actual.score());
        assert_eq!(test_case.trace, *actual.trace());
    }

    pub fn test_solve_for_test_case_with_ambiguous_trace<Sln: LatticeSolution>(test_case: TestCase<()>) {
        let actual = Sln::solve(&test_case.problem).unwrap();
        assert_eq!(test_case.score, *actual.score());
    }
}
