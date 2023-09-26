use crate::{Patt, Problem, Step, StepKind, Text};
use crate::lattice_solution::{Done, LatticeConfig, LatticeIx, LatticeSolution, LatticeState, Next, Node};
use std::collections::hash_map::HashMap;

// Initial naive attempt
// Takes hashmap from simple scala implementation as well as recursive traversal
// but representation of nodes and edges more from loop

// It won't be syntactically possible to interleave kleene ranges with group ranges
// And the parser will ensure that all groups are balanced
// So our algorithm does not have to worry about having more "starts" than "ends"

#[derive(Eq, PartialEq, Debug)]
pub struct MapSolution {
    score: usize,
    trace: Vec<Step>,
}

impl LatticeSolution for MapSolution {
    type Conf = Config;
    type Ix = Ix;
    type State = State;

    fn new(score: usize, trace: Vec<Step>) -> Self {
        MapSolution { score, trace }
    }

    fn score_lattice(&self) -> &usize {
        &self.score
    }

    fn trace_lattice(&self) -> &Vec<Step> {
        &self.trace
    }
}

pub struct Config {
    problem: Problem,
}

impl LatticeConfig<Ix> for Config {
    fn new(problem: &Problem) -> Self {
        Config { problem: problem.clone() }
    }

    fn get(&self, ix: Ix) -> (Patt, Text) {
        (self.problem.pattern[ix.pix], self.problem.text[ix.tix])
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
    // TODO let's change these ix names later ...
    pub pix: usize,
    pub tix: usize,
    pub kix: usize,
}

impl LatticeIx<Config> for Ix {
    fn start() -> Self {
        Self { pix: 0, tix: 0, kix: 0 }
    }

    fn end(conf: &Config) -> Self {
        Self { pix: conf.problem.pattern.len() - 1, tix: conf.problem.text.len() - 1, kix: 0 }
    }

    fn skip_text(&self) -> Next<Self> {
        let next = Ix { tix: self.tix + 1, kix: 0, ..*self };
        Next { cost: 1, next, kind: StepKind::SkipText }
    }

    fn skip_patt(&self) -> Next<Self> {
        let next = Ix { pix: self.pix + 1, ..*self };
        Next { cost: 1, next, kind: StepKind::SkipPattern }
    }

    fn hit(&self) -> Next<Self> {
        let next = Ix { pix: self.pix + 1, tix: self.tix + 1, kix: 0, ..*self };
        Next { cost: 0, next, kind: StepKind::Hit }
    }

    fn start_group(&self) -> Next<Self> {
        let next = Ix { pix: self.pix + 1, ..*self };
        Next { cost: 0, next, kind: StepKind::StartCapture }
    }

    fn stop_group(&self) -> Next<Self> {
        let next = Ix { pix: self.pix + 1, ..*self };
        Next { cost: 0, next, kind: StepKind::StopCapture }
    }

    fn start_kleene(&self) -> Next<Self> {
        let next = Ix { pix: self.pix + 1, kix: self.kix + 1, ..*self };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn end_kleene(&self) -> Next<Self> {
        let next = Ix { pix: self.pix + 1, kix: self.kix - 1, ..*self };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn pass_kleene(&self, off: usize) -> Next<Self> {
        let next = Ix { pix: self.pix + off + 1, ..*self};
        Next { cost: 0, next, kind: StepKind::NoOp}
    }

    fn restart_kleene(&self, off: usize) -> Next<Self> {
        let next = Ix { pix: self.pix - off, ..*self };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn can_restart(&self) -> bool {
        self.kix == 0
    }

    fn to_step(_conf: &Config, from: &Self, done: &Done<Self>) -> Step {
        Step {
            from_patt: from.pix,
            from_text: from.tix,
            to_patt: done.next.pix,
            to_text: done.next.tix,
            score: done.score,
            kind: done.kind,
        }
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
    fn test_solve_match_kleene_1() {
        tests::test_solve_match_kleene_1::<MapSolution>();
    }

    #[test]
    fn test_solve_match_kleene_2() {
        tests::test_solve_match_kleene_2::<MapSolution>();
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
    fn test_solve_fail_kleene_1() {
        tests::test_solve_fail_kleene_1::<MapSolution>();
    }
}
