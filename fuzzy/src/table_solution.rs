//! A theoretically faster implementation of [`Solution`](crate::Solution).
//!
//! This implementation pre-allocates a [vector](State) storing state for all [nodes](Ix), so in
//! theory it should be relatively efficient, although we haven't done any benchmarks yet. We will
//! do these in the future.

use crate::{Problem, Step};
use crate::lattice_solution::{LatticeConfig, LatticeIx, LatticeSolution, LatticeState, Next, Node, Patt, Text};

#[derive(Eq, PartialEq, Debug)]
pub struct TableSolution {
    score: usize,
    trace: Vec<Step<Patt, Text>>,
}

impl LatticeSolution for TableSolution {
    type Conf = Config;
    type Ix = Ix;
    type State = State;

    fn new(score: usize, trace: Vec<Step<Patt, Text>>) -> Self {
        TableSolution { score, trace }
    }

    fn score_lattice(&self) -> &usize {
        &self.score
    }

    fn trace_lattice(&self) -> &Vec<Step<Patt, Text>> {
        &self.trace
    }
}

/// Stores the text and pattern from the original [`Problem`](crate::Problem).
///
/// Our state stores an array of nodes. This array forms a table, with one dimension representing
/// the text, while the other dimension represents an expanded pattern.
///
/// The pattern needs to be expanded because of the ["repetition depth"](Ix::rep_off)
/// concept: we need extra nodes for pattern elements inside repetition groups. We don't actually need
/// to store the expanded pattern, but we do need it's larger offsets and length.
///
/// ```ignore
/// Original pattern: abc<d e f < g  h  i  >  j k l > mno, offsets: 12,  4,  4, and 12, length: 19
/// Expanded pattern: abc<ddeeff<<ggghhhiii>>>jjkkll>>mno, offsets: 27, 11, 11, and 27, length: 35
///
/// (In this example, < and > represent the start and end of repetitions.)
/// ```
pub struct Config {
    text: Vec<Text>,
    pattern: Vec<Patt>,
}

impl LatticeConfig<Ix> for Config {
    fn new(problem: &Problem) -> Self {
        let pattern = Patt::extract_custom(problem, 1);
        let text = Text::extract(problem);
        Config {
            text: text,
            pattern: pattern,
        }
    }

    fn get(&self, ix: Ix) -> (&Patt, &Text) {
        (&self.pattern[ix.pattern], &self.text[ix.text])
    }

    fn start(&self) -> Ix {
        Ix { text: 0, pattern: 0, reps: 1, rep_off: 0 }
    }

    fn end(&self) -> Ix {
        Ix {
            text: self.text.len() - 1,
            pattern: self.pattern.len() - 1,
            reps: 1,
            rep_off: 0,
        }
    }

    fn skip_text(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            text: ix.text + 1,
            rep_off: 0,
            ..ix
        };
        Next { cost: 1, next, step: Some(Step::SkipText(())) }
    }

    fn skip_patt(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + ix.reps,
            ..ix
        };
        Next { cost: 1, next, step: Some(Step::SkipPattern(())) }
    }

    fn hit(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            text: ix.text + 1,
            pattern: ix.pattern + ix.reps,
            rep_off: 0,
            ..ix
        };
        Next { cost: 0, next, step: Some(Step::Hit((),())) }
    }

    fn start_group(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + ix.reps,
            ..ix
        };
        Next { cost: 0, next, step: Some(Step::StartCapture) }
    }

    fn stop_group(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + ix.reps,
            ..ix
        };
        Next { cost: 0, next, step: Some(Step::StopCapture) }
    }

    fn start_left(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + ix.reps,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn start_right(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off + ix.reps,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn pass_right(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn start_repetition(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + ix.reps,
            reps: ix.reps + 1,
            rep_off: ix.rep_off + 1,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn end_repetition(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + ix.reps,
            reps: ix.reps - 1,
            rep_off: ix.rep_off - 1,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn pass_repetition(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off + ix.reps + 1,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn restart_repetition(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern - off,
            ..ix
        };
        Next { cost: 0, next, step: None }
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
        let pattern_len = conf.pattern.len();
        let num_nodes = conf.text.len() * pattern_len;
        State {
            nodes: vec![Node::Ready; num_nodes],
            pattern_len,
        }
    }

    fn get(&self, ix: Ix) -> Node<Ix> {
        let node_ix = self.node(ix);
        self.nodes[node_ix]
    }

    fn set(&mut self, ix: Ix, node: Node<Ix>) {
        let node_ix = self.node(ix);
        self.nodes[node_ix] = node;
    }
}

/// Indexes into [`State`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Ix {
    /// The index into [`Problem::pattern`](crate::Problem::pattern).
    pub text: usize,
    /// The index into [`Problem::text`](crate::Problem::text).
    pub pattern: usize,
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
    use crate::lattice_solution::tests;

    #[test]
    fn test_solve_match_empty() {
        tests::test_solve_match_empty::<TableSolution>();
    }

    #[test]
    fn test_solve_match_lit_1() {
        tests::test_solve_match_lit_1::<TableSolution>();
    }

    #[test]
    fn test_solve_match_lit_2() {
        tests::test_solve_match_lit_2::<TableSolution>();
    }

    #[test]
    fn test_solve_match_class_1() {
        tests::test_solve_match_class_1::<TableSolution>();
    }

    #[test]
    fn test_solve_match_class_2() {
        tests::test_solve_match_class_2::<TableSolution>();
    }

    #[test]
    fn test_solve_match_class_3() {
        tests::test_solve_match_class_3::<TableSolution>();
    }

    #[test]
    fn test_solve_match_alternative_1() {
        tests::test_solve_match_alternative_1::<TableSolution>();
    }

    #[test]
    fn test_solve_match_alternative_2() {
        tests::test_solve_match_alternative_2::<TableSolution>();
    }

    #[test]
    fn test_solve_match_alternative_3() {
        tests::test_solve_match_alternative_3::<TableSolution>();
    }

    #[test]
    fn test_solve_match_repetition_1() {
        tests::test_solve_match_repetition_1::<TableSolution>();
    }

    #[test]
    fn test_solve_match_repetition_2() {
        tests::test_solve_match_repetition_2::<TableSolution>();
    }

    #[test]
    fn test_solve_match_repetition_3() {
        tests::test_solve_match_repetition_3::<TableSolution>();
    }

    #[test]
    fn test_solve_fail_empty_1() {
        tests::test_solve_fail_empty_1::<TableSolution>();
    }

    #[test]
    fn test_solve_fail_empty_2() {
        tests::test_solve_fail_empty_2::<TableSolution>();
    }

    #[test]
    fn test_solve_fail_lit_1() {
        tests::test_solve_fail_lit_1::<TableSolution>();
    }

    #[test]
    fn test_solve_fail_lit_2() {
        tests::test_solve_fail_lit_2::<TableSolution>();
    }

    #[test]
    fn test_solve_fail_lit_3() {
        tests::test_solve_fail_lit_3::<TableSolution>();
    }

    #[test]
    fn test_solve_fail_class_1() {
        tests::test_solve_fail_class_1::<TableSolution>();
    }

    #[test]
    fn test_solve_fail_alternative_1() {
        tests::test_solve_fail_alternative_1::<TableSolution>();
    }

    #[test]
    fn test_solve_fail_repetition_1() {
        tests::test_solve_fail_repetition_1::<TableSolution>();
    }
}
