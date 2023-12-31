//! A theoretically faster implementation of [`Solution`](crate::Solution).
//!
//! This implementation pre-allocates a [vector](State) storing state for all [nodes](Ix), so in
//! theory it should be relatively efficient, although we haven't done any benchmarks yet. We will
//! do these in the future.

use crate::{ElementCore, Match, Problem, Step};
use crate::flat_pattern::{Flat, FlatPattern};
use crate::lattice_solution::{LatticeConfig, LatticeIx, LatticeSolution, LatticeState, Node, StepType};

#[derive(Eq, PartialEq, Debug)]
pub struct TableSolution {
    score: usize,
    trace: Vec<Step<Match, char>>,
}

impl LatticeSolution for TableSolution {
    type Conf = Config;
    type Ix = Ix;
    type State = State;

    fn new(score: usize, trace: Vec<Step<Match, char>>) -> Self {
        TableSolution { score, trace }
    }

    fn score_lattice(&self) -> &usize {
        &self.score
    }

    fn trace_lattice(&self) -> &Vec<Step<Match, char>> {
        &self.trace
    }
}

/// Stores the text and pattern from the original [`Problem`](crate::Problem).
///
/// Our state stores an array of nodes. This array forms a table, with one dimension representing
/// the text, while the other dimension represents an expanded pattern, per [`FlatPattern::custom`].
pub struct Config {
    text: Vec<char>,
    pattern: FlatPattern,
}

impl LatticeConfig<Ix> for Config {
    fn new(problem: &Problem<ElementCore>) -> Self {
        let pattern = FlatPattern::custom(&problem.pattern, 1);
        let text = problem.text.atoms.clone();
        Config { text, pattern }
    }

    fn get(&self, ix: Ix) -> (Option<&Flat>, Option<&char>) {
        (self.pattern.get(ix.pattern), self.text.get(ix.text))
    }

    fn start(&self) -> Ix {
        Ix { text: 0, pattern: 0, reps: 1, rep_off: 0 }
    }

    fn end(&self) -> Ix {
        Ix {
            text: self.text.len(),
            pattern: self.pattern.len(),
            reps: 1,
            rep_off: 0,
        }
    }

    fn step(&self, ix: Ix, step_type: StepType) -> Ix {
        match step_type {
            StepType::Hit =>
                Ix {
                    pattern: ix.pattern + ix.reps,
                    text: ix.text + 1,
                    rep_off: 0,
                    ..ix
                },
            StepType::SkipText =>
                Ix {
                    text: ix.text + 1,
                    rep_off: 0,
                    ..ix
                },
            StepType::SkipPattern | StepType::StartGroup | StepType::EndGroup | StepType::StartLeft =>
                Ix {
                    pattern: ix.pattern + ix.reps,
                    ..ix
                },
            StepType::StartRight(off) =>
                Ix {
                    pattern: ix.pattern + off + ix.reps,
                    ..ix
                },
            StepType::PassRight(off) =>
                Ix {
                    pattern: ix.pattern + off,
                    ..ix
                },
            StepType::StartRepetition =>
                Ix {
                    pattern: ix.pattern + ix.reps,
                    reps: ix.reps + 1,
                    rep_off: ix.rep_off + 1,
                    ..ix
                },
            StepType::EndRepetition =>
                Ix {
                    pattern: ix.pattern + ix.reps,
                    reps: ix.reps - 1,
                    rep_off: ix.rep_off - 1,
                    ..ix
                },
            StepType::PassRepetition(off) =>
                Ix {
                    pattern: ix.pattern + off + ix.reps + 1,
                    ..ix
                },
            StepType::RestartRepetition(off) =>
                Ix {
                    pattern: ix.pattern - off,
                    reps: ix.reps - 1,
                    ..ix
                },
        }
    }
}

pub struct State {
    nodes: Vec<Node<Ix>>,
    pattern_len: usize,
}

impl State {
    fn node(&self, ix: Ix) -> usize {
        ix.text * self.pattern_len + ix.pattern + ix.rep_off
    }
}

impl LatticeState<Config, Ix> for State {
    fn new(conf: &Config) -> Self {
        // we need an extra row/col for indices at the end of pattern and text
        let pattern_len = conf.pattern.len() + 1;
        let text_len = conf.text.len() + 1;
        let num_nodes = text_len * pattern_len;
        let nodes = Vec::from_iter((0..num_nodes).into_iter().map(|_| Node::new()));
        State {
            nodes,
            pattern_len,
        }
    }

    fn get(&self, ix: Ix) -> &Node<Ix> {
        let node_ix = self.node(ix);
        &self.nodes[node_ix]
    }

    fn get_mut(&mut self, ix: Ix) -> &mut Node<Ix> {
        let node_ix = self.node(ix);
        &mut self.nodes[node_ix]
    }

    fn set(&mut self, ix: Ix, node: Node<Ix>) {
        let node_ix = self.node(ix);
        self.nodes[node_ix] = node;
    }
}

/// Indexes into [`State`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct Ix {
    /// The index into the [flattened `Problem::pattern`](crate::flat_pattern::FlatPattern).
    pub pattern: usize,
    /// The index into [`Problem::text`](crate::Problem::text).
    pub text: usize,
    /// This field tracks how many times we are repeating each pattern element.
    pub reps: usize,
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
    use super::TableSolution;
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
        test_logic::test_solve::<TableSolution>(test);
    }
}
