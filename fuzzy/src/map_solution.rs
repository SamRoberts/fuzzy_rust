//! An implementation of [`Solution`](crate::Solution) that should be relatively easy to develop new features for.
//!
//! This implementation uses a [map](State) to store state for each [node](Ix), so it should be
//! easy to change node representation and expand the state space over time.

use crate::{ElementCore, Match, Problem, Step};
use crate::flat_pattern::{Flat, FlatPattern};
use crate::lattice_solution::{LatticeConfig, LatticeIx, LatticeSolution, LatticeState, Node, StepType};
use std::collections::hash_map::HashMap;

#[derive(Eq, PartialEq, Debug)]
pub struct MapSolution {
    score: usize,
    trace: Vec<Step<Match, char>>,
}

impl LatticeSolution for MapSolution {
    type Conf = Config;
    type Ix = Ix;
    type State = State;

    fn new(score: usize, trace: Vec<Step<Match, char>>) -> Self {
        MapSolution { score, trace }
    }

    fn score_lattice(&self) -> &usize {
        &self.score
    }

    fn trace_lattice(&self) -> &Vec<Step<Match, char>> {
        &self.trace
    }
}

pub struct Config {
    pattern: FlatPattern,
    text: Vec<char>,
}

impl LatticeConfig<Ix> for Config {
    fn new(problem: &Problem<ElementCore>) -> Self {
        let pattern = FlatPattern::new(&problem.pattern);
        let text = problem.text.atoms.clone();
        Config { pattern, text }
    }

    fn get(&self, ix: Ix) -> (Option<&Flat>, Option<&char>) {
        (self.pattern.get(ix.pattern), self.text.get(ix.text))
    }

    fn start(&self) -> Ix {
        Ix { pattern: 0, text: 0, rep_off: 0 }
    }

    fn end(&self) -> Ix {
        Ix { pattern: self.pattern.len(), text: self.text.len(), rep_off: 0 }
    }

    fn step(&self, ix: Ix, step_type: StepType) -> Ix {
        match step_type {
            StepType::Hit =>
                Ix { pattern: ix.pattern + 1, text: ix.text + 1, rep_off: 0, ..ix },
            StepType::SkipText =>
                Ix { text: ix.text + 1, rep_off: 0, ..ix },
            StepType::SkipPattern | StepType::StartGroup | StepType::EndGroup | StepType::StartLeft =>
                Ix { pattern: ix.pattern + 1, ..ix },
            StepType::StartRight(off) =>
                Ix { pattern: ix.pattern + off + 1, ..ix },
            StepType::PassRight(off) =>
                Ix { pattern: ix.pattern + off, ..ix },
            StepType::StartRepetition =>
                Ix { pattern: ix.pattern + 1, rep_off: ix.rep_off + 1, ..ix },
            StepType::EndRepetition =>
                Ix { pattern: ix.pattern + 1, rep_off: ix.rep_off - 1, ..ix },
            StepType::PassRepetition(off) =>
                Ix { pattern: ix.pattern + off + 1, ..ix},
            StepType::RestartRepetition(off) =>
                Ix { pattern: ix.pattern - off, ..ix },
        }
    }
}

pub struct State {
  nodes: HashMap<Ix, Node<Ix>>,
  default: Node<Ix>,
}

impl LatticeState<Config, Ix> for State {
    fn new(_conf: &Config) -> Self {
        State { nodes: HashMap::new(), default: Node::new(), }
    }

    fn get(&self, ix: Ix) -> &Node<Ix> {
        match self.nodes.get(&ix) {
            Some(node) => node,
            None => &self.default,
        }
    }

    fn get_mut(&mut self, ix: Ix) -> &mut Node<Ix> {
        self.nodes.entry(ix).or_insert(self.default.clone())
    }

    fn set(&mut self, ix: Ix, node: Node<Ix>) {
        let _ = self.nodes.insert(ix, node);
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Ix {
    /// The index into the [flattened `Problem::pattern`](crate::flat_pattern::FlatPattern).
    pub pattern: usize,
    /// The index into [`Problem::text`](crate::Problem::text).
    pub text: usize,
    /// This field represents our "repetition depth since we last changed text index".
    ///
    /// To avoid infinite loops, we have to avoid repeating a repetition group if that would take us
    /// back to the same index we started at. We keep track of how many repetition groups we entered
    /// since we last matched or skipped a text character, and avoid looping back unless this is 0.
    /// This ix the "repetition depth". Because the "repetition depth" affects future jumps, it also
    /// affects the future score, and so we have a separate score and a separate index for each
    /// repetition depth value.
    pub rep_off: usize,
}

impl LatticeIx<Config> for Ix {
    fn can_restart(&self) -> bool {
        self.rep_off == 0
    }
}

#[cfg(test)]
mod tests {
    use super::MapSolution;
    use crate::test_cases::TestCase;
    use crate::lattice_solution::test_logic;
    use test_case::test_case;

    #[test_case(TestCase::match_empty())]
    #[test_case(TestCase::fail_empty_1())]
    #[test_case(TestCase::fail_empty_2())]
    #[test_case(TestCase::match_lit_1())]
    #[test_case(TestCase::match_lit_2())]
    #[test_case(TestCase::fail_lit_1())]
    #[test_case(TestCase::fail_lit_2())]
    #[test_case(TestCase::fail_lit_3())]
    #[test_case(TestCase::match_class_1())]
    #[test_case(TestCase::match_class_2())]
    #[test_case(TestCase::match_class_3())]
    #[test_case(TestCase::fail_class_1())]
    #[test_case(TestCase::match_alternative_1())]
    #[test_case(TestCase::match_alternative_2())]
    #[test_case(TestCase::match_alternative_3())]
    #[test_case(TestCase::fail_alternative_1())]
    #[test_case(TestCase::match_repetition_1())]
    #[test_case(TestCase::match_repetition_2())]
    #[test_case(TestCase::match_repetition_3())]
    #[test_case(TestCase::match_repetition_4())]
    #[test_case(TestCase::match_repetition_5())]
    #[test_case(TestCase::fail_repetition_1())]
    #[test_case(TestCase::fail_repetition_2())]
    #[test_case(TestCase::fail_repetition_3())]
    fn test_solve(test: TestCase) {
        test_logic::test_solve::<MapSolution>(test);
    }
}
