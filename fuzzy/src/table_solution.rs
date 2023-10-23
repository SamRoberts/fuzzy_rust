//! A theoretically faster implementation of [`Solution`](crate::Solution).
//!
//! This implementation pre-allocates a [vector](State) storing state for all [nodes](Ix), so in
//! theory it should be relatively efficient, although we haven't done any benchmarks yet. We will
//! do these in the future.

use crate::{ProblemV2, Step};
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
/// The pattern needs to be expanded because of the ["repetition depth"](Ix::repetition_depth_this_text)
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
    /// This array extends [`pattern`] elements with larger offsets for the expanded pattern.
    expanded_offsets: Vec<usize>,
    /// We also need to store the total length of the expanded pattern.
    expanded_pattern_len: usize,
}

impl LatticeConfig<Ix> for Config {
    fn new(problem: &ProblemV2) -> Self {
        let pattern = Patt::extract(problem);
        let text = Text::extract(problem);
        let (expanded_pattern_len, expanded_offsets) = Self::expand(&pattern);
        Config {
            text: text,
            pattern: pattern,
            expanded_offsets,
            expanded_pattern_len,
        }
    }

    fn get(&self, ix: Ix) -> (&Patt, &Text) {
        (&self.pattern[ix.pattern], &self.text[ix.text])
    }

    fn start(&self) -> Ix {
        Ix { text: 0, pattern: 0, node: 0, repetition_depth: 0, repetition_depth_this_text: 0 }
    }

    fn end(&self) -> Ix {
        Ix {
            text: self.text.len() - 1,
            pattern: self.pattern.len() - 1,
            node: self.num_nodes() - 1,
            repetition_depth: 0,
            repetition_depth_this_text: 0,
        }
    }

    fn skip_text(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            text: ix.text + 1,
            node: ix.node + self.expanded_pattern_len - ix.repetition_depth_this_text,
            repetition_depth_this_text: 0,
            ..ix
        };
        Next { cost: 1, next, step: Some(Step::SkipText(())) }
    }

    fn skip_patt(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.repetition_depth + 1,
            ..ix
        };
        Next { cost: 1, next, step: Some(Step::SkipPattern(())) }
    }

    fn hit(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            text: ix.text + 1,
            pattern: ix.pattern + 1,
            node: ix.node + self.expanded_pattern_len + ix.repetition_depth + 1 - ix.repetition_depth_this_text,
            repetition_depth_this_text: 0,
            ..ix
        };
        Next { cost: 0, next, step: Some(Step::Hit((),())) }
    }

    fn start_group(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.repetition_depth + 1,
            ..ix
        };
        Next { cost: 0, next, step: Some(Step::StartCapture) }
    }

    fn stop_group(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.repetition_depth + 1,
            ..ix
        };
        Next { cost: 0, next, step: Some(Step::StopCapture) }
    }

    fn start_left(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.repetition_depth + 1,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn start_right(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off + 1,
            node: ix.node + self.expanded_offsets[ix.pattern] + ix.repetition_depth + 1,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn pass_right(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off,
            node: ix.node + self.expanded_offsets[ix.pattern],
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn start_repetition(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.repetition_depth + 2,
            repetition_depth: ix.repetition_depth + 1,
            repetition_depth_this_text: ix.repetition_depth_this_text + 1,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn end_repetition(&self, ix: Ix) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + 1,
            node: ix.node + ix.repetition_depth,
            repetition_depth: ix.repetition_depth - 1,
            repetition_depth_this_text: ix.repetition_depth_this_text - 1,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn pass_repetition(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern + off + 1,
            node: ix.node + self.expanded_offsets[ix.pattern] + ix.repetition_depth + 2,
            ..ix
        };
        Next { cost: 0, next, step: None }
    }

    fn restart_repetition(&self, ix: Ix, off: usize) -> Next<Ix> {
        let next = Ix {
            pattern: ix.pattern - off,
            node: ix.node - self.expanded_offsets[ix.pattern],
            repetition_depth: ix.repetition_depth - 1,
            ..ix
        };
        Next { cost: 0, next, step: None }
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
        let mut repetition_starts = vec![];
        let mut alternative_lefts = vec![];
        let mut alternative_rights = vec![];
        let mut offsets_expand = vec![];

        for (patt_ix, patt) in original.iter().enumerate() {
            while let Some((right_patt, right_expand, end_patt)) = alternative_rights.last() {
                if patt_ix == *end_patt {
                    let offset = len_expand - *right_expand;
                    offsets_expand[*right_patt] = offset;
                    alternative_rights.pop();
                } else {
                    break;
                }
            }

            match patt {
                Patt::Lit(_) | Patt::Class(_) | Patt::GroupStart | Patt::GroupEnd | Patt::End => {
                    len_expand += repetition_starts.len() + 1;
                    offsets_expand.push(0);
                },
                Patt::AlternativeLeft(_) => {
                    alternative_lefts.push((patt_ix, len_expand));
                    len_expand += repetition_starts.len() + 1;
                    offsets_expand.push(0); // will modify once we know where right branch is
                },
                Patt::AlternativeRight(off) => {
                    alternative_rights.push((patt_ix, len_expand, patt_ix + off));
                    let (left_patt, left_expand) = alternative_lefts.pop().unwrap();
                    let offset = len_expand - left_expand;
                    len_expand += repetition_starts.len() + 1;
                    offsets_expand[left_patt] = offset;
                    offsets_expand.push(0); // will modify once we know where end is
                },
                Patt::RepetitionStart(_) => {
                    repetition_starts.push((patt_ix, len_expand));
                    len_expand += repetition_starts.len();
                    offsets_expand.push(0); // will modify once we know where end is
                },
                Patt::RepetitionEnd(_) => {
                    let (start_patt, start_expand) = repetition_starts.pop().unwrap();
                    let offset = len_expand - start_expand;
                    len_expand += repetition_starts.len() + 2;
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
    /// This field tracks how many repetition groups the current pattern element is inside.
    ///
    /// When we move from one pattern element to the next we increment [`Ix::pattern`] by this amount + 1.
    pub repetition_depth: usize,
    /// This field represents our "repetition depth since we last changed text index".
    ///
    /// To avoid infinite loops, we have to avoid repeating a repetition group if that would take us
    /// back to the same index we started at. We keep track of how many repetition groups we entered
    /// since we last matched or skipped a text character, and avoid looping back unless this is 0.
    /// This ix the "repetition depth". Because the "repetition depth" affects future jumps, it also
    /// affects the future score, and so we have a separate score and a separate index for each
    /// repetition depth value.
    pub repetition_depth_this_text: usize,
}

impl LatticeIx<Config> for Ix {
    fn can_restart(&self) -> bool {
        self.repetition_depth_this_text == 0
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
