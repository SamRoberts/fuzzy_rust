//! Provides a sub-trait of [`Solution`] with a generic [`Solution::solve`] implementation.

use crate::{ElementCore, Match, Problem, Solution, Step};
use crate::flat_pattern::Flat;
use crate::error::Error;
use std::fmt::Debug;

/// A naive family of "recurse through a lattice" [`Solution`] implementations.
///
/// [`LatticeSolution`] implementations get [`Solution::solve`] defined automatically. Instead,
/// implementations are required to specify a mutable [`State`](LatticeSolution::State) space
/// and an [`Ix`](LatticeSolution::Ix) type which addresses it.
///
/// Each index links to child indices which represent the next possible steps we can take to match
/// the pattern to the text (e.g. match a character, skip a character from the text or pattern,
/// etc.). There is a defined [`start`](LatticeConfig::start) index, when no progress has been made,
/// and an [`end`](LatticeConfig::end) index, when both the entire pattern and text have been matched.
/// Implementation must ensure that [`can_restart`](LatticeIx::can_restart) is implemented
/// correctly, so that these links never form a loop. These links form a
/// [lattice](https://en.wikipedia.org/wiki/Lattice_(order)).
///
/// [`LatticeSolution`] implements [`Solution::solve`] by naively recursing through this lattice,
/// recording the optimal score for each index in [`State`](LatticeSolution::State) as it goes.
pub trait LatticeSolution : Sized  + Solution<Error> {
    /// Carries immutable information derived from the [`Problem`](crate::Problem) being solved.
    type Conf: LatticeConfig<Self::Ix>;
    /// Mutable state being updated while solving.
    type State: LatticeState<Self::Conf, Self::Ix>;
    /// The type used to index into [`State`](LatticeSolution::State) and
    /// [`Conf`](LatticeSolution::Conf).
    type Ix: LatticeIx<Self::Conf>;

    fn new(score: usize, trace: Vec<Step<Match, char>>) -> Self;

    fn score_lattice(&self) -> &usize;
    fn trace_lattice(&self) -> &Vec<Step<Match, char>>;

    /// [`Solution::solve`] implementation.
    fn solve_lattice(problem: &Problem<ElementCore>) -> Result<Self, Error> {
        let conf = Self::Conf::new(problem);
        let mut state = Self::State::new(&conf);

        let start_ix = conf.start();
        let end_ix = conf.end();

        let start_lead = Next { cost: 0, next: start_ix, step: None };
        let _ = Self::solve_ix(&conf, &mut state, end_ix, start_lead)?;

        let score = match state.get(start_ix) {
            Node::Done(Done { score, .. }) => Ok(score),
            _ => Err(Error::IncompleteFinalState),
        }?;

        let mut trace = vec![];
        let mut from = start_ix;
        while let Node::Done(done) = state.get(from) {
            if from == end_ix { break; }
            for step in done.step.iter() {
                let (patt, text) = conf.get(from);
                let final_step = step.map(
                    |_| match patt {
                        Some(Flat::Lit(c))   => Match::Lit(*c),
                        Some(Flat::Class(c)) => Match::Class(c.clone()),
                        unexpected           => panic!("Unexpected trace pattern {:?}", unexpected),
                    },
                    |_| match text {
                        Some(c) => *c,
                        unexpected         => panic!("Unexpected trace text {:?}", unexpected),
                    }
                );
                trace.push(final_step);
            }
            from = done.next;
        }
        if from != end_ix {
            return Err(Error::IncompleteFinalState);
        }

        Ok(LatticeSolution::new(score, trace))
    }

    /// Update [`State`](LatticeSolution::State) with the optimal steps from the current
    /// [`Ix`](LatticeSolution::Ix) onwards.
    ///
    /// `lead` is the step taken to arrive at the [`Ix`](LatticeSolution::Ix) we are solving.
    fn solve_ix(
        conf: &Self::Conf,
        state: &mut Self::State,
        end_ix: Self::Ix,
        lead: Next<Self::Ix>,
     ) -> Result<Done<Self::Ix>, Error> {
        let Next { cost, step, next: ix } = lead; // the step's lead is our current ix

        match state.get(ix) {
            Node::Working =>
                Err(Error::InfiniteLoop(format!("{:?}", ix))),
            Node::Done(done) =>
                Ok(Done { score: done.score + cost, next: ix, step }),
            Node::Ready => {
                state.set(ix, Node::Working);

                let mut maybe_score = None;
                let (patt, text) = conf.get(ix);

                match (patt, text) {
                    (Some(Flat::Class(class)), Some(c)) if class.matches(*c) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.hit(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    (Some(Flat::Lit(a)), Some(b)) if *a == *b => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.hit(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    _ =>
                        (),
                }

                match text {
                    Some(_) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.skip_text(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    None =>
                        (),
                }

                match patt {
                    Some(Flat::Lit(_) | Flat::Class(_)) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.skip_patt(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Some(Flat::GroupStart) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.start_group(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Some(Flat::GroupEnd) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.stop_group(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Some(Flat::AlternativeLeft(off)) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.start_left(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.start_right(ix, *off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Some(Flat::AlternativeRight(off)) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.pass_right(ix, *off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Some(Flat::RepetitionEnd(off)) if ix.can_restart() => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.restart_repetition(ix, *off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Some(Flat::RepetitionEnd(_)) => { // cannot restart
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.end_repetition(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Some(Flat::RepetitionStart(off)) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.start_repetition(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.pass_repetition(ix, *off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    }
                    None =>
                        (),
                }

                let score = match maybe_score {
                    Some(score) => score,
                    None if ix == end_ix =>
                        Done { score: 0, next: end_ix, step: None },
                    None =>
                        return Err(Error::Blocked(format!("{:?}", ix))),
                };

                state.set(ix, Node::Done(score));
                Ok(Done { score: score.score + cost, next: ix, step })
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

    fn trace(&self) -> &Vec<Step<Match, char>> {
        LatticeSolution::trace_lattice(self)
    }

    fn solve(problem: &Problem<ElementCore>) -> Result<Self, Error> {
        LatticeSolution::solve_lattice(&problem)
    }
}

pub trait LatticeConfig<Ix> {
    fn new(problem: &Problem<ElementCore>) -> Self;
    fn get(&self, ix: Ix) -> (Option<&Flat>, Option<&char>);

    fn start(&self) -> Ix;
    fn end(&self) -> Ix;

    fn skip_text(&self, ix: Ix) -> Next<Ix>;
    fn skip_patt(&self, ix: Ix) -> Next<Ix>;
    fn hit(&self, ix: Ix) -> Next<Ix>;
    fn start_group(&self, ix: Ix) -> Next<Ix>;
    fn stop_group(&self, ix: Ix) -> Next<Ix>;
    fn start_left(&self, ix: Ix) -> Next<Ix>;
    fn start_right(&self, ix: Ix, off: usize) -> Next<Ix>;
    fn pass_right(&self, ix: Ix, off: usize) -> Next<Ix>;
    fn start_repetition(&self, ix: Ix) -> Next<Ix>;
    fn end_repetition(&self, ix: Ix) -> Next<Ix>;
    fn pass_repetition(&self, ix: Ix, off: usize) -> Next<Ix>;
    fn restart_repetition(&self, ix: Ix, off: usize) -> Next<Ix>;
}

pub trait LatticeState<Conf, Ix> {
    fn new(conf: &Conf) -> Self;
    fn get(&self, ix: Ix) -> Node<Ix>;
    fn set(&mut self, ix: Ix, node: Node<Ix>);
}

pub trait LatticeIx<Conf> : Eq + PartialEq + Copy + Clone + Debug + Sized {
    fn can_restart(&self) -> bool;
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
    pub step: Option<Step<(),()>>,
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
    pub step: Option<Step<(),()>>,
}

#[cfg(test)]
pub mod test_logic {
    use super::*;
    use crate::test_cases::TestCase;

    pub fn test_solve<Sln: LatticeSolution>(test_case: TestCase) {
        let desugared = test_case.problem.desugar();
        let actual = Sln::solve(&desugared).unwrap();
        assert_eq!(test_case.score, *actual.score());
        assert_eq!(test_case.trace, *actual.trace());
    }
}
