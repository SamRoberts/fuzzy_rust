//! A theoretically faster implementation of [`Solution`](crate::Solution).
//!
//! This implementation pre-allocates a [vector](State) storing state for all [nodes](Ix), so in
//! theory it should be relatively efficient, although we haven't done any benchmarks yet. We will
//! do these in the future.

use crate::{Atoms, ElementCore, Match, Pattern, Step};
use crate::error::Error;
use crate::flat_pattern::{Flat, FlatPattern};
use nonempty::{NonEmpty, nonempty};

#[derive(Eq, PartialEq, Debug)]
pub struct TableSolution {
    pub score: usize,
    pub trace: Vec<Step<Match, char>>,
}

impl TableSolution {
    pub fn solve(pattern: &Pattern<ElementCore>, text: &Atoms) -> Result<Self, Error> {
        let conf = Config::new(pattern, text);
        let mut state = State::new(&conf);

        let start_ix = conf.start();
        let end_ix = conf.end();

        let _ = Self::calculate_optimal_path(&conf, &mut state)?;

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

        Ok(Self { score, trace })
    }

    fn calculate_optimal_path(
        conf: &Config,
        state: &mut State,
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

/// Stores the text and pattern from the original [`Problem`](crate::Problem).
///
/// Our state stores an array of nodes. This array forms a table, with one dimension representing
/// the text, while the other dimension represents an expanded pattern, per [`FlatPattern::custom`].
pub struct Config {
    text: Vec<char>,
    pattern: FlatPattern,
}

impl Config {
    fn new(pattern: &Pattern<ElementCore>, text: &Atoms) -> Self {
        let pattern = FlatPattern::custom(pattern, 1);
        let text = text.atoms.clone();
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
    nodes: Vec<Node>,
    pattern_len: usize,
}

impl State {
    fn node(&self, ix: Ix) -> usize {
        ix.text * self.pattern_len + ix.pattern + ix.rep_off
    }

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

    fn get(&self, ix: Ix) -> &Node {
        let node_ix = self.node(ix);
        &self.nodes[node_ix]
    }

    fn get_mut(&mut self, ix: Ix) -> &mut Node {
        let node_ix = self.node(ix);
        &mut self.nodes[node_ix]
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

impl Ix {
    fn can_restart(&self) -> bool {
        self.rep_off == 0
    }
}

#[derive(Debug)]
enum LoopState {
    Down(Down),
    Back(Back),
}

impl LoopState {
    fn current(&self) -> Ix {
        match self {
            LoopState::Down(down) => down.current,
            LoopState::Back(back) => back.current,
        }
    }
}

#[derive(Debug)]
struct Down {
    parent: Ix,
    current: Ix,
}

#[derive(Debug)]
struct Back {
    current: Ix,
    child: Ix,
}

// TODO make a better Node type
//
// Calculate_optimal_path (originally called solve_ix) used to store a lot of state on the stack:
// the parent node, our progress through the possible step types, the optimal score, etc. The
// node was a simple enum which was either Ready, Working, or Done. Only the Done value had any
// fields, and it was never mutated.
//
// Once we began to run out of stack space for mid-sized use-cases, we transferred all of that
// state into the heap by adding it to this Node struct. Much of this information is mutated as we
// try out each possible step type.
//
// I had a lot of trouble implementing this expanded node. Solve loops over my table of node
// values, taking a mutable reference to a single node in each iteration. My code originally
// pattern matched on the Node enum, and called methods on inner types which could only be accessed
// when node had the right case. But I struggled to do this and satisfy rust's borrow checker.
//
// For now, I've abandonded pattern matching and type safety, and implemented rust as an abstract
// data type. The node still has three states: Ready, Working, and Done, but they aren't reflected
// in rust's type system. Instead, Node methods return errors if they are called when the node is
// in the wrong state.
//
// The three states are a bit implicit in the Node structure. They are driven by current. Current
// changes from 0..=step_types.len()+1 over the life of the Node:
//
// 1. A node is Ready if current == 0
// 2. A node is Working if 1 >= current >= step_types.len()
// 3. A node is Done if current == step_types.len() + 1
//
// When a node is working, the current step type being attempted is step_types[current-1].
//
// When a node has processed at least one node (current >= 2), score/step_type/next record the
// optimal choice among step_types[0..current-1]. This means those fields are optimal when a Node
// is Done.
//
// I'd like to return to this Node when I'm more comfortable working with rust, and do a better job
// implementing it.

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Node {
    current: usize,
    parent: Ix,
    score: usize,
    step_type: StepType,
    next: Ix,
    step_types: Vec<StepType>,
}

impl Node {
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
    fn get(opt_flat: Option<&Flat>, opt_text: Option<&char>, ix: &Ix) -> Option<Self> {
        // TODO this is surprisingly hard to follow for something conceptually simple. Can I make it nicer?
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

    pub fn test_solve(test_case: TestCase) {
        let desugared = test_case.pattern.desugar();
        let actual = TableSolution::solve(&desugared, &test_case.text).unwrap();
        assert_eq!(test_case.score, actual.score);
        assert_eq!(test_case.trace, actual.trace);
    }
}
#[cfg(test)]
mod tests {
    use super::test_logic;
    use crate::test_cases::TestCase;
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
        test_logic::test_solve(test);
    }
}
