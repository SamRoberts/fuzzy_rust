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

        let _ = Self::solve_ix(&conf, &mut state, end_ix, start_ix, 0, StepKind::NoOp)?;

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
        ix: Self::Ix,
        cost: usize,
        kind: StepKind,
     ) -> Result<Done<Self::Ix>, Error> {
        match state.get(ix) {
            Node::Working =>
                Err(Error::InfiniteLoop(format!("{:?}", ix))),
            Node::Done(done) =>
                Ok(Done { score: done.score + cost, next: ix, kind }),
            Node::Ready => {
                state.set(ix, Node::Working);
                let steps = Self::succ(conf, ix);

                let maybe_score = steps.iter()
                    .map(|step| Self::solve_ix(conf, state, end_ix, step.next, step.cost, step.kind))
                    .reduce(Done::combine_result);

                let score = maybe_score.unwrap_or_else(|| {
                    if ix == end_ix {
                        Ok(Done { score: 0, next: end_ix, kind: StepKind::NoOp })
                    } else {
                        Err(Error::Blocked(format!("{:?}", ix)))
                    }
                })?;

                state.set(ix, Node::Done(score));
                Ok(Done { score: score.score + cost, next: ix, kind })
            }
        }
    }

    fn succ(conf: &Self::Conf, ix: Self::Ix) -> Vec<Next<Self::Ix>> {
        let (patt, text) = conf.get(ix);

        let mut steps = vec![];

        match (patt, text) {
            (Patt::Any, Text::Lit(_)) =>
                steps.push(ix.hit()),
            (Patt::Lit(a), Text::Lit(b)) if a == b =>
                steps.push(ix.hit()),
            _ =>
                (),
        }

        match text {
            Text::Lit(_) =>
                steps.push(ix.skip_text()),
            Text::End =>
                (),
        }

        match patt {
            Patt::Lit(_) | Patt::Any =>
                steps.push(ix.skip_patt()),
            Patt::GroupStart =>
                steps.push(ix.start_group()),
            Patt::GroupEnd =>
                steps.push(ix.stop_group()),
            Patt::KleeneEnd(off) if ix.can_restart() =>
                steps.push(ix.restart_kleene(off)),
            Patt::KleeneEnd(_) => // cannot restart
                steps.push(ix.end_kleene()),
            Patt::KleeneStart(off) => {
                steps.push(ix.start_kleene());
                steps.push(ix.pass_kleene(off));
            }
            Patt::End =>
                (),
        }

        steps
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
    fn get(&self, ix: Ix) -> (Patt, Text);
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
    fn combine_result<E>(left: Result<Self, E>, right: Result<Self, E>) -> Result<Self, E> {
        match (left, right) {
            (Ok(l), Ok(r)) => Ok(Self::combine(l, r)),
            (Err(l), _)    => Err(l),
            (_, Err(r))    => Err(r),
        }
    }

    fn combine(left: Self, right: Self) -> Self {
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

    pub fn test_solve_match_empty<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::End],
            text:    vec![Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 0;
        let expected_trace: Vec<Step> = vec![];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_match_lit_1<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::Lit('a'), Patt::End],
            text:    vec![Text::Lit('a'), Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 0;
        let expected_trace = vec![
            step(0, 0, 1, 1, 0, StepKind::Hit),
        ];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_match_lit_2<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::Lit('a'), Patt::Lit('b'), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('b'), Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 0;
        let expected_trace = vec![
            step(0, 0, 1, 1, 0, StepKind::Hit),
            step(1, 1, 2, 2, 0, StepKind::Hit),
        ];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_match_kleene_1<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::KleeneStart(2), Patt::Lit('a'), Patt::KleeneEnd(2), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 0;
        let expected_trace = vec![
            step(0, 0, 1, 0, 0, StepKind::NoOp),
            step(1, 0, 2, 1, 0, StepKind::Hit),
            step(2, 1, 0, 1, 0, StepKind::NoOp),
            step(0, 1, 1, 1, 0, StepKind::NoOp),
            step(1, 1, 2, 2, 0, StepKind::Hit),
            step(2, 2, 0, 2, 0, StepKind::NoOp),
            step(0, 2, 3, 2, 0, StepKind::NoOp),
        ];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_match_kleene_2<Sln: LatticeSolution>() {
        let problem = Problem {
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
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 0;
        let expected_trace = vec![
            step(0, 0, 1, 0, 0, StepKind::NoOp),
            step(1, 0, 2, 1, 0, StepKind::Hit),
            step(2, 1, 5, 1, 0, StepKind::NoOp),
            step(5, 1, 0, 1, 0, StepKind::NoOp),
            step(0, 1, 1, 1, 0, StepKind::NoOp),
            step(1, 1, 2, 2, 0, StepKind::Hit),
            step(2, 2, 3, 2, 0, StepKind::NoOp),
            step(3, 2, 4, 3, 0, StepKind::Hit),
            step(4, 3, 2, 3, 0, StepKind::NoOp),
            step(2, 3, 5, 3, 0, StepKind::NoOp),
            step(5, 3, 0, 3, 0, StepKind::NoOp),
            step(0, 3, 1, 3, 0, StepKind::NoOp),
            step(1, 3, 2, 4, 0, StepKind::Hit),
            step(2, 4, 3, 4, 0, StepKind::NoOp),
            step(3, 4, 4, 5, 0, StepKind::Hit),
            step(4, 5, 2, 5, 0, StepKind::NoOp),
            step(2, 5, 3, 5, 0, StepKind::NoOp),
            step(3, 5, 4, 6, 0, StepKind::Hit),
            step(4, 6, 2, 6, 0, StepKind::NoOp),
            step(2, 6, 5, 6, 0, StepKind::NoOp),
            step(5, 6, 0, 6, 0, StepKind::NoOp),
            step(0, 6, 6, 6, 0, StepKind::NoOp),
        ];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_fail_empty_1<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::End],
            text:    vec![Text::Lit('a'), Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 1;
        let expected_trace = vec![
            step(0, 0, 0, 1, 1, StepKind::SkipText),
        ];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_fail_empty_2<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::Lit('a'), Patt::End],
            text:    vec![Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 1;
        let expected_trace = vec![
            step(0, 0, 1, 0, 1, StepKind::SkipPattern),
        ];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_fail_lit_1<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::Lit('a'), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 1;
        let expected_trace = vec![
            step(0, 0, 1, 1, 1, StepKind::Hit),
            step(1, 1, 1, 2, 1, StepKind::SkipText),
        ];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_fail_lit_2<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::Lit('a'), Patt::Lit('b'), Patt::Lit('a'), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 1;
        let expected_trace = vec![
            step(0, 0, 1, 1, 1, StepKind::Hit),
            step(1, 1, 2, 1, 1, StepKind::SkipPattern),
            step(2, 1, 3, 2, 0, StepKind::Hit),
        ];
        assert_eq!(expected_score, *actual.score());
        assert_eq!(expected_trace, *actual.trace());
    }

    pub fn test_solve_fail_kleene_1<Sln: LatticeSolution>() {
        let problem = Problem {
            pattern: vec![Patt::KleeneStart(2), Patt::Lit('a'), Patt::KleeneEnd(2), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('b'), Text::Lit('a'), Text::End],
        };
        let actual = Sln::solve(&problem).unwrap();
        let expected_score = 1;
        // there are multiple valid traces so can't check expected_trace
        assert_eq!(expected_score, *actual.score());
    }

    fn step(from_patt: usize, from_text: usize, to_patt: usize, to_text: usize, score: usize, kind: StepKind) -> Step {
        Step { from_patt, from_text, to_patt, to_text, score, kind }
    }
}
