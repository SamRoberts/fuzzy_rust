//! An implementation of [`Solution`](crate::Solution) that should be relatively easy to develop new features for.
//!
//! This implementation uses a [map](State) to store state for each [node](Ix), so it should be
//! easy to change node representation and expand the state space over time.

use crate::{ProblemV2, Step};
use crate::lattice_solution::{LatticeConfig, LatticeIx, LatticeSolution, LatticeState, Next, Node, Patt, Text};
use std::collections::hash_map::HashMap;

#[derive(Eq, PartialEq, Debug)]
pub struct MapSolution {
    score: usize,
    trace: Vec<Step<Patt, Text>>,
}

impl LatticeSolution for MapSolution {
    type Conf = Config;
    type Ix = Ix;
    type State = State;

    fn new(score: usize, trace: Vec<Step<Patt, Text>>) -> Self {
        MapSolution { score, trace }
    }

    fn score_lattice(&self) -> &usize {
        &self.score
    }

    fn trace_lattice(&self) -> &Vec<Step<Patt, Text>> {
        &self.trace
    }
}

pub struct Config {
    pattern: Vec<Patt>,
    text: Vec<Text>,
}

impl LatticeConfig<Ix> for Config {
    fn new(problem: &ProblemV2) -> Self {
        let pattern = Patt::extract(problem);
        let text = Text::extract(problem);
        Config { pattern, text }
    }

    fn get(&self, ix: Ix) -> (&Patt, &Text) {
        (&self.pattern[ix.pix], &self.text[ix.tix])
    }

    fn start(&self) -> Ix {
        Ix { pix: 0, tix: 0, kix: 0 }
    }

    fn end(&self) -> Ix {
        Ix { pix: self.pattern.len() - 1, tix: self.text.len() - 1, kix: 0 }
    }

    fn skip_text(&self, ix: Ix) -> Next<Ix> {
        let next = Ix { tix: ix.tix + 1, kix: 0, ..ix };
        Next { cost: 1, next, step: Some(Step::SkipText(())) }
    }

    fn skip_patt(&self, ix: Ix) -> Next<Ix> {
        let next = Ix { pix: ix.pix + 1, ..ix };
        Next { cost: 1, next, step: Some(Step::SkipPattern(())) }
    }

    fn hit(&self, ix: Ix) -> Next<Ix> {
        let next = Ix { pix: ix.pix + 1, tix: ix.tix + 1, kix: 0, ..ix };
        Next { cost: 0, next, step: Some(Step::Hit((), ())) }
    }

    fn start_group(&self, ix: Ix) -> Next<Ix> {
        let next = Ix { pix: ix.pix + 1, ..ix };
        Next { cost: 0, next, step: Some(Step::StartCapture) }
    }

    fn stop_group(&self, ix: Ix) -> Next<Ix> {
        let next = Ix { pix: ix.pix + 1, ..ix };
        Next { cost: 0, next, step: Some(Step::StopCapture) }
    }

    fn start_left(&self, ix: Ix) -> Next<Ix> {
        let next = Ix { pix: ix.pix + 1, ..ix };
        Next { cost: 0, next, step: None }
    }

    fn start_right(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix { pix: ix.pix + off + 1, ..ix };
        Next { cost: 0, next, step: None }
    }

    fn pass_right(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix { pix: ix.pix + off, ..ix };
        Next { cost: 0, next, step: None }
    }

    fn start_repetition(&self, ix: Ix) -> Next<Ix> {
        let next = Ix { pix: ix.pix + 1, kix: ix.kix + 1, ..ix };
        Next { cost: 0, next, step: None }
    }

    fn end_repetition(&self, ix: Ix) -> Next<Ix> {
        let next = Ix { pix: ix.pix + 1, kix: ix.kix - 1, ..ix };
        Next { cost: 0, next, step: None }
    }

    fn pass_repetition(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix { pix: ix.pix + off + 1, ..ix};
        Next { cost: 0, next, step: None}
    }

    fn restart_repetition(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix { pix: ix.pix - off, ..ix };
        Next { cost: 0, next, step: None }
    }

}

pub struct State {
  nodes: HashMap<Ix, Node<Ix>>,
}

impl LatticeState<Config, Ix> for State {
    fn new(_conf: &Config) -> Self {
        State { nodes: HashMap::new() }
    }

    fn get(&self, ix: Ix) -> Node<Ix> {
        match self.nodes.get(&ix) {
            Some(node) => *node,
            None => Node::Ready,
        }
    }

    fn set(&mut self, ix: Ix, node: Node<Ix>) {
        let _ = self.nodes.insert(ix, node);
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Ix {
    /// The index into [`Problem::pattern`](crate::Problem::pattern).
    ///
    /// We will change these field names in the future!
    pub pix: usize,
    /// The index into [`Problem::text`](crate::Problem::text).
    pub tix: usize,
    /// This field represents our "repetition depth since we last changed text index".
    ///
    /// To avoid infinite loops, we have to avoid repeating a repetition group if that would take us
    /// back to the same index we started at. We keep track of how many repetition groups we entered
    /// since we last matched or skipped a text character, and avoid looping back unless this is 0.
    /// This ix the "repetition depth". Because the "repetition depth" affects future jumps, it also
    /// affects the future score, and so we have a separate score and a separate index for each
    /// repetition depth value.
    pub kix: usize,
}

impl LatticeIx<Config> for Ix {
    fn can_restart(&self) -> bool {
        self.kix == 0
    }
}

#[cfg(test)]
mod tests {
    use super::MapSolution;
    use crate::lattice_solution::tests;

    #[test]
    fn test_solve_match_empty() {
        tests::test_solve_match_empty::<MapSolution>();
    }

    #[test]
    fn test_solve_match_lit_1() {
        tests::test_solve_match_lit_1::<MapSolution>();
    }

    #[test]
    fn test_solve_match_lit_2() {
        tests::test_solve_match_lit_2::<MapSolution>();
    }

    #[test]
    fn test_solve_match_class_1() {
        tests::test_solve_match_class_1::<MapSolution>();
    }

    #[test]
    fn test_solve_match_class_2() {
        tests::test_solve_match_class_2::<MapSolution>();
    }

    #[test]
    fn test_solve_match_class_3() {
        tests::test_solve_match_class_3::<MapSolution>();
    }

    #[test]
    fn test_solve_match_alternative_1() {
        tests::test_solve_match_alternative_1::<MapSolution>();
    }

    #[test]
    fn test_solve_match_alternative_2() {
        tests::test_solve_match_alternative_2::<MapSolution>();
    }

    #[test]
    fn test_solve_match_alternative_3() {
        tests::test_solve_match_alternative_3::<MapSolution>();
    }

    #[test]
    fn test_solve_match_repetition_1() {
        tests::test_solve_match_repetition_1::<MapSolution>();
    }

    #[test]
    fn test_solve_match_repetition_2() {
        tests::test_solve_match_repetition_2::<MapSolution>();
    }

    #[test]
    fn test_solve_match_repetition_3() {
        tests::test_solve_match_repetition_3::<MapSolution>();
    }

    #[test]
    fn test_solve_fail_empty_1() {
        tests::test_solve_fail_empty_1::<MapSolution>();
    }

    #[test]
    fn test_solve_fail_empty_2() {
        tests::test_solve_fail_empty_2::<MapSolution>();
    }

    #[test]
    fn test_solve_fail_lit_1() {
        tests::test_solve_fail_lit_1::<MapSolution>();
    }

    #[test]
    fn test_solve_fail_lit_2() {
        tests::test_solve_fail_lit_2::<MapSolution>();
    }

    #[test]
    fn test_solve_fail_lit_3() {
        tests::test_solve_fail_lit_3::<MapSolution>();
    }

    #[test]
    fn test_solve_fail_class_1() {
        tests::test_solve_fail_class_1::<MapSolution>();
    }

    #[test]
    fn test_solve_fail_alternative_1() {
        tests::test_solve_fail_alternative_1::<MapSolution>();
    }

    #[test]
    fn test_solve_fail_repetition_1() {
        tests::test_solve_fail_repetition_1::<MapSolution>();
    }
}
