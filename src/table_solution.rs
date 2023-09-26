use crate::{Patt, Problem, Step, StepKind, Text};
use crate::lattice_solution::{Done, LatticeConfig, LatticeIx, LatticeSolution, LatticeState, Next, Node};

#[derive(Eq, PartialEq, Debug)]
pub struct TableSolution {
    score: usize,
    trace: Vec<Step>,
}

impl LatticeSolution for TableSolution {
    type Conf = Config;
    type Ix = Ix;
    type State = State;

    fn new(score: usize, trace: Vec<Step>) -> Self {
        TableSolution { score, trace }
    }

    fn score_lattice(&self) -> &usize {
        &self.score
    }

    fn trace_lattice(&self) -> &Vec<Step> {
        &self.trace
    }
}

pub struct Config {
    // TODO reconsider need to store a copy of Patt for every index in expanded pattern space
    text: Vec<Text>,
    expanded_pattern: Vec<Patt>,
    original_pattern_ix: Vec<usize>, // the original ix for each element in expanded_pattern
}

impl LatticeConfig<Ix> for Config {
    fn new(problem: &Problem) -> Self {
        let (expanded_pattern, original_pattern_ix) = Self::expand_pattern(&problem.pattern);
        Config {
            text: problem.text.clone(),
            expanded_pattern,
            original_pattern_ix,
        }
    }

    fn get(&self, ix: Ix) -> (Patt, Text) {
        (self.expanded_pattern[ix.pattern], self.text[ix.text])
    }

}

impl Config {
    fn expand_pattern(original: &Vec<Patt>) -> (Vec<Patt>, Vec<usize>) {

        let mut expanded = vec![];
        let mut original_ix = vec![];
        let mut kleene_start_ixs = vec![];
        let mut kleene_depth = 0;

        for (orig_ix, patt) in original.iter().enumerate() {
            match patt {
                Patt::Lit(_) | Patt::Any | Patt::GroupStart | Patt::GroupEnd | Patt::End => {
                    for _ in 0..=kleene_depth {
                        expanded.push(*patt);
                        original_ix.push(orig_ix);
                    }
                },
                Patt::KleeneStart(_) => {
                    kleene_start_ixs.push(expanded.len());
                    for _ in 0..=kleene_depth {
                        expanded.push(Patt::KleeneStart(0)); // later will replace placeholder offset
                        original_ix.push(orig_ix);
                    }
                    kleene_depth += 1;
                },
                Patt::KleeneEnd(_) => {
                    let kleene_end_ix = expanded.len();
                    let kleene_start_ix = kleene_start_ixs.pop().expect("cannot have more ends then starts");
                    let offset = kleene_end_ix - kleene_start_ix;

                    for _ in 0..=kleene_depth {
                        expanded.push(Patt::KleeneEnd(offset));
                        original_ix.push(orig_ix);
                    }
                    kleene_depth -= 1;

                    for i in kleene_start_ix ..= kleene_start_ix + kleene_depth {
                        expanded[i] = Patt::KleeneStart(offset);
                    }
                }
            }
        }

        (expanded, original_ix)
    }
}

pub struct State {
    nodes: Vec<Node<Ix>>,
    expanded_pattern_len: usize
}

impl State {
    fn nodes_ix(&self, ix: Ix) -> usize {
        ix.text * self.expanded_pattern_len + ix.pattern + ix.kleene_depth_this_text
    }
}

impl LatticeState<Config, Ix> for State {
    fn new(conf: &Config) -> Self {
        let expanded_pattern_len = conf.expanded_pattern.len();
        let num_nodes = expanded_pattern_len * conf.text.len();
        State { nodes: vec![Node::Ready; num_nodes], expanded_pattern_len }
    }


    fn get(&self, ix: Ix) -> Node<Ix> {
        self.nodes[self.nodes_ix(ix)]
    }

    fn set(&mut self, ix: Ix, node: Node<Ix>) {
        let nodes_ix = self.nodes_ix(ix);
        self.nodes[nodes_ix] = node;
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Ix {
    pub text: usize,
    pub pattern: usize, // initial corresponding index in expanded pattern space
    pub kleene_depth: usize,
    pub kleene_depth_this_text: usize,
}

impl LatticeIx<Config> for Ix {
    fn start() -> Self {
        Self { text: 0, pattern: 0, kleene_depth: 0, kleene_depth_this_text: 0 }
    }

    fn end(conf: &Config) -> Self {
        Self {
            text: conf.text.len() - 1,
            pattern: conf.expanded_pattern.len() - 1, // kleene_depth == 0 at end
            kleene_depth: 0,
            kleene_depth_this_text: 0,
        }
    }

    fn skip_text(&self) -> Next<Self> {
        let next = Ix {
            text: self.text + 1,
            kleene_depth_this_text: 0,
            ..*self
        };
        Next { cost: 1, next, kind: StepKind::SkipText }
    }

    fn skip_patt(&self) -> Next<Self> {
        let next = Ix {
            pattern: self.pattern + self.kleene_depth + 1,
            ..*self
        };
        Next { cost: 1, next, kind: StepKind::SkipPattern }
    }

    fn hit(&self) -> Next<Self> {
        let next = Ix {
            text: self.text + 1,
            pattern: self.pattern + self.kleene_depth + 1,
            kleene_depth_this_text: 0,
            ..*self
        };
        Next { cost: 0, next, kind: StepKind::Hit }
    }

    fn start_group(&self) -> Next<Self> {
        let next = Ix {
            pattern: self.pattern + self.kleene_depth + 1,
            ..*self
        };
        Next { cost: 0, next, kind: StepKind::StartCapture }
    }

    fn stop_group(&self) -> Next<Self> {
        let next = Ix {
            pattern: self.pattern + self.kleene_depth + 1,
            ..*self
        };
        Next { cost: 0, next, kind: StepKind::StopCapture }
    }

    fn start_kleene(&self) -> Next<Self> {
        let next = Ix {
            pattern: self.pattern + self.kleene_depth + 1,
            kleene_depth: self.kleene_depth + 1,
            kleene_depth_this_text: self.kleene_depth_this_text + 1,
            ..*self
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn end_kleene(&self) -> Next<Self> {
        let next = Ix {
            pattern: self.pattern + self.kleene_depth + 1,
            kleene_depth: self.kleene_depth - 1,
            kleene_depth_this_text: self.kleene_depth_this_text - 1,
            ..*self
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn pass_kleene(&self, off: usize) -> Next<Self> {
        let next = Ix {
            pattern: self.pattern + off + self.kleene_depth + 2,
            ..*self
        };
        Next { cost: 0, next, kind: StepKind::NoOp}
    }

    fn restart_kleene(&self, off: usize) -> Next<Self> {
        let next = Ix {
            pattern: self.pattern - off,
            kleene_depth: self.kleene_depth - 1,
            ..*self
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }


    fn can_restart(&self) -> bool {
        self.kleene_depth_this_text == 0
    }

    fn to_step(conf: &Config, from: &Self, done: &Done<Self>) -> Step {
        Step {
            from_patt: conf.original_pattern_ix[from.pattern],
            to_patt: conf.original_pattern_ix[done.next.pattern],
            from_text: from.text,
            to_text: done.next.text,
            score: done.score,
            kind: done.kind,
        }
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
    fn test_solve_match_kleene_1() {
        tests::test_solve_match_kleene_1::<TableSolution>();
    }

    #[test]
    fn test_solve_match_kleene_2() {
        tests::test_solve_match_kleene_2::<TableSolution>();
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
    fn test_solve_fail_kleene_1() {
        tests::test_solve_fail_kleene_1::<TableSolution>();
    }
}
