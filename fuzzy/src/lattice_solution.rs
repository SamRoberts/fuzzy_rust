//! Provides a sub-trait of [`Solution`] with a generic [`Solution::solve`] implementation.

use crate::{Class, Element, Match, Pattern, Problem, Solution, Step};
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

    fn new(score: usize, trace: Vec<Step<Patt, Text>>) -> Self;

    fn score_lattice(&self) -> &usize;
    fn trace_lattice(&self) -> &Vec<Step<Patt, Text>>;

    /// [`Solution::solve`] implementation.
    fn solve_lattice(problem: &Problem) -> Result<Self, Error> {
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
                trace.push(step.with(patt, text));
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
                    (Patt::Class(class), Text::Lit(c)) if class.matches(*c) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.hit(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    (Patt::Lit(a), Text::Lit(b)) if *a == *b => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.hit(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    _ =>
                        (),
                }

                match text {
                    Text::Lit(_) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.skip_text(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Text::End =>
                        (),
                }

                match patt {
                    Patt::Lit(_) | Patt::Class(_) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.skip_patt(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::GroupStart => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.start_group(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::GroupEnd => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.stop_group(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::AlternativeLeft(off) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.start_left(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.start_right(ix, *off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::AlternativeRight(off) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.pass_right(ix, *off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::RepetitionEnd(off) if ix.can_restart() => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.restart_repetition(ix, *off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::RepetitionEnd(_) => { // cannot restart
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.end_repetition(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    },
                    Patt::RepetitionStart(off) => {
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.start_repetition(ix))?;
                        maybe_score = Self::update(maybe_score, outcome);
                        let outcome = Self::solve_ix(conf, state, end_ix, conf.pass_repetition(ix, *off))?;
                        maybe_score = Self::update(maybe_score, outcome);
                    }
                    Patt::End =>
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
    fn score(&self) -> usize {
        *LatticeSolution::score_lattice(self)
    }

    fn trace(&self) -> Vec<Step<Match, char>> {
        // TODO temporarily keeping Patt/Text for implementations
        LatticeSolution::trace_lattice(self).into_iter()
            .map(|step| step.map(
                |p| match p {
                    Patt::Lit(c)   => Match::Lit(*c),
                    Patt::Class(c) => Match::Class(c.clone()),
                    unexpected     => panic!("Unexpected trace pattern {:?}", unexpected),
                },
                |t| match  t {
                    Text::Lit(c) => *c,
                    unexpected   => panic!("Unexpected trace text {:?}", unexpected),
                }
            ))
            .collect()
    }

    fn solve(problem: &Problem) -> Result<Self, Error> {
        LatticeSolution::solve_lattice(&problem)
    }
}

pub trait LatticeConfig<Ix> {
    fn new(problem: &Problem) -> Self;
    fn get(&self, ix: Ix) -> (&Patt, &Text);

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

impl Patt {
    pub fn extract(problem: &Problem) -> Vec<Self> {
        Self::extract_custom(problem, 0)
    }

    pub fn extract_custom(problem: &Problem, rep_incr: usize) -> Vec<Patt> {
        let mut result = vec![];
        Self::pattern_patts(&mut result, &problem.pattern, 1, rep_incr);
        Self::single_patt(&mut result, Patt::End, 1);
        result
    }

    fn pattern_patts(result: &mut Vec<Patt>, pattern: &Pattern, reps: usize, rep_incr: usize) {
        for elem in pattern.elems.iter() {
            Self::elem_patts(result, elem, reps, rep_incr)
        }
    }

    fn elem_patts(result: &mut Vec<Patt>, elem: &Element, reps: usize, rep_incr: usize) {
        match elem {
            Element::Match(Match::Lit(c)) =>
                Self::single_patt(result, Patt::Lit(*c), reps),
            Element::Match(Match::Class(class)) =>
                Self::single_patt(result, Patt::Class(class.clone()), reps),
            Element::Capture(inner) => {
                Self::single_patt(result, Patt::GroupStart, reps);
                Self::pattern_patts(result, inner, reps, rep_incr);
                Self::single_patt(result, Patt::GroupEnd, reps);
            }
            Element::Repetition(inner) => {
                let next_reps = reps + rep_incr;
                let start_ix = result.len();
                Self::single_patt(result, Patt::RepetitionStart(0), reps);
                Self::pattern_patts(result, inner, next_reps, rep_incr);
                let end_ix = result.len();
                Self::single_patt(result, Patt::RepetitionEnd(0), next_reps);

                let off = end_ix - start_ix;
                Self::update_patt(result, Patt::RepetitionStart(off), start_ix, reps);
                Self::update_patt(result, Patt::RepetitionEnd(off), end_ix, next_reps);
            }
            Element::Alternative(p1, p2) => {
                let left_ix = result.len();
                Self::single_patt(result, Patt::AlternativeLeft(0), reps);
                Self::pattern_patts(result, p1, reps, rep_incr);
                let right_ix = result.len();
                Self::single_patt(result, Patt::AlternativeRight(0), reps);
                Self::pattern_patts(result, p2, reps, rep_incr);
                let next_ix = result.len();

                let left_off = right_ix - left_ix;
                let right_off = next_ix - right_ix;
                Self::update_patt(result, Patt::AlternativeLeft(left_off), left_ix, reps);
                Self::update_patt(result, Patt::AlternativeRight(right_off), right_ix, reps);
            }
        }
    }

    fn single_patt(result: &mut Vec<Patt>, elem: Patt, reps: usize) {
        for _ in 0..reps {
            result.push(elem.clone());
        }
    }

    fn update_patt(result: &mut Vec<Patt>, elem: Patt, ix: usize, reps: usize) {
        for i in 0..reps {
            result[ix + i] = elem.clone();
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

impl Text {
    pub fn extract(problem: &Problem) -> Vec<Self> {
        problem.text.atoms.iter()
            .map(|c| Text::Lit(*c))
            .chain(vec![Text::End])
            .collect()
    }
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

    pub fn test_solve_match_alternative_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_alternative_1());
    }

    pub fn test_solve_match_alternative_2<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_alternative_2());
    }

    pub fn test_solve_match_alternative_3<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_alternative_3());
    }

    pub fn test_solve_match_repetition_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_repetition_1());
    }

    pub fn test_solve_match_repetition_2<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_repetition_2());
    }

    pub fn test_solve_match_repetition_3<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::match_repetition_3());
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

    pub fn test_solve_fail_alternative_1<Sln: LatticeSolution>() {
        test_solve_for_test_case::<Sln>(TestCase::fail_alternative_1());
    }

    pub fn test_solve_fail_repetition_1<Sln: LatticeSolution>() {
        test_solve_for_test_case_with_ambiguous_trace::<Sln>(TestCase::fail_repetition_1());
    }

    pub fn test_solve_for_test_case<Sln: LatticeSolution>(test_case: TestCase<Vec<Step<Match, char>>>) {
        let actual = Sln::solve(&test_case.problem).unwrap();
        assert_eq!(test_case.score, actual.score());
        assert_eq!(test_case.trace, actual.trace());
    }

    pub fn test_solve_for_test_case_with_ambiguous_trace<Sln: LatticeSolution>(test_case: TestCase<()>) {
        let actual = Sln::solve(&test_case.problem).unwrap();
        assert_eq!(test_case.score, actual.score());
    }
}
