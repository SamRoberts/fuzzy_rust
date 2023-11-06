//! Provides a sub-trait of [`Solution`] with a generic [`Solution::solve`] implementation.

use crate::{ElementCore, Match, Problem, Solution, Step};
use crate::flat_pattern::Flat;
use crate::error::Error;
use nonempty::{NonEmpty, nonempty};
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

        let _ = Self::solve_ix(&conf, &mut state)?;

        let start_node = state.get(start_ix);
        let score = start_node.done_info()
            .map(|i| i.0)
            .map_err(|_| Error::IncompleteFinalState)?;

        let mut trace = vec![];
        let mut from = start_ix;
        loop {
            let node = state.get(from);
            if !node.is_done() || from == end_ix { break; }
            let (patt, text) = conf.get(from);
            let (_, step_type, next) = node.done_info()?;
            if let Some(step) =  step_type.step() {
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
            from = next;
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
     ) -> Result<(), Error> {
        let start_ix = conf.start();
        let end_ix = conf.end();

        let mut loop_state = LoopState::Down(Down {
            parent: Default::default(),
            current: start_ix,
        });

        let mut loop_counter = 0;

        loop {
            loop_counter += 1;
            if loop_counter >= 1000000000 { // TODO make this max configurable
                return Err(Error::ExceededMaxSteps(loop_counter));
            }
            let new_parent = match &loop_state {
                LoopState::Down(down) if state.get(down.current).is_ready() => {
                    let (flat, text) = conf.get(down.current);
                    let opt_node_type = NodeType::get(flat, text, &down.current);
                    let node_state = state.get_mut(down.current);
                    node_state.initialise(end_ix, down.parent, down.current, opt_node_type)?;
                    down.parent
                }
                LoopState::Down(down) => down.parent,
                LoopState::Back(back) => {
                    let new_child = back.child;
                    let (new_score, _, _) = state.get(new_child).done_info()?;
                    let node_state = state.get_mut(back.current);
                    let new_parent = node_state.update(new_child, back.current, new_score)?;
                    new_parent
                }
            };

            let current_ix = loop_state.current();
            let final_state = state.get(current_ix);
            if current_ix == start_ix && final_state.is_done() {
                break;
            } else if final_state.is_done() {
                loop_state = LoopState::Back(Back {
                    current: new_parent,
                    child: current_ix,
                });
            } else if final_state.is_working() {
                let current_step_type = final_state.current_step_type()?;
                let child = conf.step(current_ix, current_step_type);
                loop_state = LoopState::Down(Down {
                    parent: current_ix,
                    current: child,
                });
            } else {
                return Err(Error::NoNodeProgress(format!("{:?}", current_ix)));
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
enum LoopState<Ix> {
    Down(Down<Ix>),
    Back(Back<Ix>),
}

impl <Ix: Copy + Clone> LoopState<Ix> {
    fn current(&self) -> Ix {
        match self {
            LoopState::Down(down) => down.current,
            LoopState::Back(back) => back.current,
        }
    }
}

#[derive(Debug)]
struct Down<Ix> {
    parent: Ix,
    current: Ix,
}

#[derive(Debug)]
struct Back<Ix> {
    current: Ix,
    child: Ix,
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

    fn step(&self, ix: Ix, step_type: StepType) -> Ix;
}

pub trait LatticeState<Conf, Ix: Clone> {
    fn new(conf: &Conf) -> Self;
    fn get(&self, ix: Ix) -> &Node<Ix>;
    fn get_mut(&mut self, ix: Ix) -> &mut Node<Ix>;
    fn set(&mut self, ix: Ix, node: Node<Ix>);
}

// TODO Ix turns out to be a sizable struct, remove Copy and pass by reference where possible
pub trait LatticeIx<Conf> : Eq + PartialEq + Copy + Clone + Debug + Sized + Default {
    fn can_restart(&self) -> bool;
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Node<Ix: Clone + Sized> {
    parent: Ix,
    score: usize,
    step_type: StepType,
    next: Ix,
    current: usize,
    step_types: Vec<StepType>,
}

impl <Ix: Copy + Clone + Debug + Eq + Sized + Default> Node<Ix> {
    pub fn new() -> Self {
        Self {
            current: 0,
            parent: Default::default(),
            score: 0,
            step_type: StepType::Hit,
            next: Default::default(),
            step_types: vec![],
        }
    }

    fn is_ready(&self) -> bool {
        self.current == 0
    }

    fn is_working(&self) -> bool {
        self.current > 0 && self.current <= self.step_types.len()
    }

    fn is_done(&self) -> bool {
        self.current > self.step_types.len()
    }

    fn current_step_type(&self) -> Result<StepType, Error> {
        if self.is_working() {
            Ok(self.step_types[self.current - 1])
        } else {
            Err(Error::CannotGetNodeField("current_step_type", "working"))
        }
    }

    fn done_info(&self) -> Result<(usize, StepType, Ix), Error> {
        if self.is_done() {
            Ok((self.score, self.step_type, self.next))
        } else {
            Err(Error::CannotGetNodeField("score/step_type/next", "done"))
        }
    }

    fn initialise(&mut self, end_ix: Ix, parent_ix: Ix, ix: Ix, opt_node_type: Option<NodeType>) -> Result<(), Error>{
        if self.is_ready() {
            match opt_node_type {
                Some(node_type) => {
                    let step_types = Vec::from(node_type.step_types());
                    self.parent = parent_ix;
                    self.current += 1;
                    self.step_types = step_types;
                    Ok(())
                }
                None if ix == end_ix => { // end_ix: insert dummy done value
                    self.parent = parent_ix;
                    self.current += 1;
                    Ok(())
                }
                None => {
                    Err(Error::NoNodeType(format!("{:?}", ix)))
                }
            }
        } else {
            Err(Error::CannotInitialiseNode(format!("{:?}", ix)))
        }
    }

    fn update(&mut self, new_child: Ix, ix: Ix, new_score: usize) -> Result<Ix, Error> {
        if self.is_working() {
            let parent_ix = self.parent;
            let current_step_type = self.current_step_type()?;
            let new_score = new_score + current_step_type.cost();
            if self.current <= 1 || new_score < self.score {
                self.step_type = current_step_type;
                self.score = new_score;
                self.next = new_child;
                self.current += 1;
            } else {
                self.current += 1;
            }
            Ok(parent_ix)
        } else {
            Err(Error::CannotUpdateNode(format!("{:?}", ix)))
        }
   }
}

#[derive(Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub enum NodeType {
    FinishedPattern,
    FinishedText,
    Hit,
    NoHit,
    StartGroup,
    EndGroup,
    AlternativeLeft(usize),
    AlternativeRight(usize),
    RepetitionStart(usize),
    RepetitionRestart(usize),
    RepetitionEnd,
}

impl NodeType {
    fn get<Conf, Ix: LatticeIx<Conf>>(opt_flat: Option<&Flat>, opt_text: Option<&char>, ix: &Ix) -> Option<Self> {
        // TODO this is really, really hard to follow for something conceptually simple. Can I make it nicer?
        match opt_flat {
            None if opt_text == None => None,
            None => Some(NodeType::FinishedPattern),
            Some(flat) => Some(match flat {
                Flat::Lit(c) if opt_text == Some(c) => NodeType::Hit,
                Flat::Lit(_) if opt_text == None => NodeType::FinishedText,
                Flat::Lit(_) => NodeType::NoHit,
                Flat::Class(class) if opt_text.map_or(false, |t| class.matches(*t)) => NodeType::Hit,
                Flat::Class(_) if opt_text == None => NodeType::FinishedText,
                Flat::Class(_) => NodeType::NoHit,
                Flat::GroupStart => NodeType::StartGroup,
                Flat::GroupEnd => NodeType::EndGroup,
                Flat::AlternativeLeft(off) => NodeType::AlternativeLeft(*off),
                Flat::AlternativeRight(off) => NodeType::AlternativeRight(*off),
                Flat::RepetitionStart(off) => NodeType::RepetitionStart(*off),
                Flat::RepetitionEnd(off) if ix.can_restart() => NodeType::RepetitionRestart(*off),
                Flat::RepetitionEnd(_) => NodeType::RepetitionEnd,
            })
        }
    }

    fn step_types(&self) -> NonEmpty<StepType> {
        use StepType::*;
        match self {
            Self::FinishedPattern => nonempty![SkipText],
            Self::FinishedText => nonempty![SkipPattern],
            Self::Hit => nonempty![Hit, SkipPattern, SkipText],
            Self::NoHit => nonempty![SkipPattern, SkipText],
            Self::StartGroup => nonempty![StartGroup],
            Self::EndGroup => nonempty![EndGroup],
            Self::AlternativeLeft(off) => nonempty![StartLeft, StartRight(*off)],
            Self::AlternativeRight(off) => nonempty![PassRight(*off)],
            Self::RepetitionStart(off) => nonempty![StartRepetition, PassRepetition(*off)],
            Self::RepetitionRestart(off) => nonempty![RestartRepetition(*off)],
            Self::RepetitionEnd => nonempty![EndRepetition],
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum StepType {
    SkipText,
    SkipPattern,
    Hit,
    StartGroup,
    EndGroup,
    StartLeft,
    StartRight(usize),
    PassRight(usize),
    StartRepetition,
    PassRepetition(usize),
    EndRepetition,
    RestartRepetition(usize),
}

impl StepType {
    fn cost(&self) -> usize {
        match self {
            Self::SkipPattern => 1,
            Self::SkipText    => 1,
            _                 => 0,
        }
    }

    fn step(&self) -> Option<Step<(),()>> {
        match self {
            Self::Hit         => Some(Step::Hit((), ())),
            Self::SkipPattern => Some(Step::SkipPattern(())),
            Self::SkipText    => Some(Step::SkipText(())),
            Self::StartGroup  => Some(Step::StartCapture),
            Self::EndGroup    => Some(Step::StopCapture),
            _                 => None,
        }
    }
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
