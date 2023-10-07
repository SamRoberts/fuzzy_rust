//! A theoretically faster implementation of [`Solution`](crate::Solution).
//!
//! This implementation pre-allocates a [vector](State) storing state for all [nodes](Ix), so in
//! theory it should be relatively efficient, although we haven't done any benchmarks yet. We will
//! do these in the future.

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

/// Stores the text and pattern from the original [`Problem`](crate::Problem).
///
/// Our state stores an array of nodes. This array forms a table, with one dimension representing
/// the text, while the other dimension represents an expanded pattern.
///
/// The pattern needs to be expanded because of the ["kleene depth"](Ix::kleene_depth_this_text)
/// concept: we need extra nodes for pattern elements inside kleene groups. We don't actually need
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
    /// This array extends [`pattern`] elements with larger offsets for the expanded pattern.
    expanded_offsets: Vec<usize>,
    /// We also need to store the total length of the expanded pattern.
    expanded_pattern_len: usize,
}

impl LatticeConfig<Ix> for Config {
    fn new(problem: &Problem) -> Self {
        let (expanded_pattern_len, expanded_offsets) = Self::expand(&problem.pattern);
        Config {
            text: problem.text.clone(),
            pattern: problem.pattern.clone(),
            expanded_offsets,
            expanded_pattern_len,
        }
    }

    fn get(&self, ix: Ix) -> (&Patt, &Text) {
        (&self.pattern[ix.pattern], &self.text[ix.text])
    }

    fn start(&self) -> Ix {
        Ix { text: 0, pattern: 0, node: 0, kleene_depth: 0, kleene_depth_this_text: 0 }
    }

    fn end(&self) -> Ix {
        Ix {
            text: self.text.len() - 1,
            pattern: self.pattern.len() - 1,
            node: self.num_nodes() - 1,
            kleene_depth: 0,
            kleene_depth_this_text: 0,
        }
    }

    fn skip_text(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            text: ix.text + 1,
            node: ix.node + self.expanded_pattern_len - ix.kleene_depth_this_text,
            kleene_depth_this_text: 0,
            ..ix
        };
        Next { cost: 1, next, kind: StepKind::SkipText }
    }

    fn skip_patt(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.kleene_depth + 1,
            ..ix
        };
        Next { cost: 1, next, kind: StepKind::SkipPattern }
    }

    fn hit(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            text: ix.text + 1,
            pattern: ix.pattern + 1,
            node: ix.node + self.expanded_pattern_len + ix.kleene_depth + 1 - ix.kleene_depth_this_text,
            kleene_depth_this_text: 0,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::Hit }
    }

    fn start_group(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.kleene_depth + 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::StartCapture }
    }

    fn stop_group(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.kleene_depth + 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::StopCapture }
    }

    fn start_left(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.kleene_depth + 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn start_right(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off + 1,
            node: ix.node + self.expanded_offsets[ix.pattern] + ix.kleene_depth + 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn pass_right(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off,
            node: ix.node + self.expanded_offsets[ix.pattern],
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn start_kleene(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.kleene_depth + 2,
            kleene_depth: ix.kleene_depth + 1,
            kleene_depth_this_text: ix.kleene_depth_this_text + 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn end_kleene(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.kleene_depth,
            kleene_depth: ix.kleene_depth - 1,
            kleene_depth_this_text: ix.kleene_depth_this_text - 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn pass_kleene(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off + 1,
            node: ix.node + self.expanded_offsets[ix.pattern] + ix.kleene_depth + 2,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp}
    }

    fn restart_kleene(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern - off,
            node: ix.node - self.expanded_offsets[ix.pattern],
            kleene_depth: ix.kleene_depth - 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }
}

impl Config {
    fn num_nodes(&self) -> usize {
        self.text.len() * self.expanded_pattern_len
    }

    fn expand(original: &Vec<Patt>) -> (usize, Vec<usize>) {
        // This is the most horrible code. It would be a lot easier with a hierarachical pattern
        // representation.
        let mut len_expand = 0;
        let mut kleene_starts = vec![];
        let mut alternative_lefts = vec![];
        let mut alternative_rights = vec![];
        let mut offsets_expand = vec![];

        for (patt_ix, patt) in original.iter().enumerate() {
            if let Some((right_patt, right_expand, end_patt)) = alternative_rights.last() {
                if patt_ix == *end_patt {
                    let offset = len_expand - *right_expand;
                    offsets_expand[*right_patt] = offset;
                    alternative_rights.pop();
                }
            }

            match patt {
                Patt::Lit(_) | Patt::Class(_) | Patt::GroupStart | Patt::GroupEnd | Patt::End => {
                    len_expand += kleene_starts.len() + 1;
                    offsets_expand.push(0);
                },
                Patt::AlternativeLeft(_) => {
                    alternative_lefts.push((patt_ix, len_expand));
                    len_expand += kleene_starts.len() + 1;
                    offsets_expand.push(0); // will modify once we know where right branch is
                },
                Patt::AlternativeRight(off) => {
                    alternative_rights.push((patt_ix, len_expand, patt_ix + off));
                    let (left_patt, left_expand) = alternative_lefts.pop().unwrap();
                    let offset = len_expand - left_expand;
                    len_expand += kleene_starts.len() + 1;
                    offsets_expand[left_patt] = offset;
                    offsets_expand.push(0); // will modify once we know where end is
                },
                Patt::KleeneStart(_) => {
                    kleene_starts.push((patt_ix, len_expand));
                    len_expand += kleene_starts.len();
                    offsets_expand.push(0); // will modify once we know where end is
                },
                Patt::KleeneEnd(_) => {
                    let (start_patt, start_expand) = kleene_starts.pop().unwrap();
                    let offset = len_expand - start_expand;
                    len_expand += kleene_starts.len() + 2;
                    offsets_expand[start_patt] = offset;
                    offsets_expand.push(offset);
                },
            }
        }

        (len_expand, offsets_expand)
    }
}

pub struct State {
    nodes: Vec<Node<Ix>>,
}

impl LatticeState<Config, Ix> for State {
    fn new(conf: &Config) -> Self {
        State { nodes: vec![Node::Ready; conf.num_nodes()] }
    }


    fn get(&self, ix: Ix) -> Node<Ix> {
        self.nodes[ix.node]
    }

    fn set(&mut self, ix: Ix, node: Node<Ix>) {
        self.nodes[ix.node] = node;
    }
}

/// Indexes into [`State`].
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Ix {
    /// The index into [`Problem::pattern`](crate::Problem::pattern).
    pub text: usize,
    /// The index into [`Problem::text`](crate::Problem::text).
    pub pattern: usize,
    /// The index into [`State`] nodes vector.
    pub node: usize,
    /// This field tracks how many kleene groups the current pattern element is inside.
    ///
    /// When we move from one pattern element to the next we increment [`Ix::pattern`] by this amount + 1.
    pub kleene_depth: usize,
    /// This field represents our "kleene depth since we last changed text index".
    ///
    /// To avoid infinite loops, we have to avoid repeating a kleene group if that would take us
    /// back to the same index we started at. We keep track of how many kleene groups we entered
    /// since we last matched or skipped a text character, and avoid looping back unless this is 0.
    /// This ix the "kleene depth". Because the "kleene depth" affects future jumps, it also
    /// affects the future score, and so we have a separate score and a separate index for each
    /// kleene depth value.
    pub kleene_depth_this_text: usize,
}

impl LatticeIx<Config> for Ix {
    fn can_restart(&self) -> bool {
        self.kleene_depth_this_text == 0
    }

    fn to_step(_conf: &Config, from: &Self, done: &Done<Self>) -> Step {
        Step {
            from_patt: from.pattern,
            from_text: from.text,
            to_patt: done.next.pattern,
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
    fn test_solve_match_kleene_1() {
        tests::test_solve_match_kleene_1::<TableSolution>();
    }

    #[test]
    fn test_solve_match_kleene_2() {
        tests::test_solve_match_kleene_2::<TableSolution>();
    }

    #[test]
    fn test_solve_match_kleene_3() {
        tests::test_solve_match_kleene_3::<TableSolution>();
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
    fn test_solve_fail_kleene_1() {
        tests::test_solve_fail_kleene_1::<TableSolution>();
    }
}
