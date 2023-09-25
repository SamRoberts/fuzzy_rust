use crate::{Patt, Problem, Solution, Step, StepKind, Text};
use crate::error::Error;
use std::ops::{Index, IndexMut};

// Starting out with literal copy of MapSolution.
// We will try to modify this while keeping as much code the same as possible
// Then see how we can share that code later

#[derive(Eq, PartialEq, Debug)]
pub struct TableSolution {
    score: usize,
    trace: Vec<Step>,
}

impl Solution<Error> for TableSolution {
    fn solve(problem: &Problem) -> Result<TableSolution, Error> {
        let mut state = State::new(problem);
        let start_ix = Ix::start();
        let end_ix = Ix::end(problem, &state);

        let _ = Self::solve_impl(problem, &mut state, end_ix, start_ix, 0, StepKind::NoOp)?;

        let score = match state[start_ix] {
            Node::Done(Done { score, .. }) => Ok(score),
            _ => Err(Error::IncompleteFinalState),
        }?;

        let mut trace = vec![];
        let mut from = start_ix;
        while let Node::Done(done) = state[from] {
            if from == end_ix { break; }
            let step = Ix::to_step(problem, &state, &from, &done);
            trace.push(step);
            from = done.next;
        }
        if from != end_ix {
            return Err(Error::IncompleteFinalState);
        }

        Ok(TableSolution { score, trace })
    }

    fn score(&self) -> &usize {
        &self.score
    }

    fn trace(&self) -> &Vec<Step> {
        &self.trace
    }
}

impl TableSolution {
    fn solve_impl(problem: &Problem, state: &mut State, end_ix: Ix, ix: Ix, cost: usize, kind: StepKind) -> Result<Done, Error> {
        match state[ix] {
            Node::Working =>
                Err(Error::InfiniteLoop(format!("{:?}", ix))),
            Node::Done(done) =>
                Ok(Done { score: done.score + cost, next: ix, kind }),
            Node::Ready => {
                state[ix] = Node::Working;
                let steps = Self::succ(problem, state, ix);

                let maybe_score = steps.iter()
                    .map(|step| Self::solve_impl(problem, state, end_ix, step.next, step.cost, step.kind))
                    .reduce(Done::combine_result);

                let score = maybe_score.unwrap_or_else(|| {
                    if ix == end_ix {
                        Ok(Done { score: 0, next: end_ix, kind: StepKind::NoOp })
                    } else {
                        Err(Error::Blocked(format!("{:?}", ix)))
                    }
                })?;

                state[ix] = Node::Done(score);
                Ok(Done { score: score.score + cost, next: ix, kind })
            }
        }
    }

    fn succ(problem: &Problem, state: &State, ix: Ix) -> Vec<Next> {
        let patt = state.current_pattern(problem, ix);
        let text = state.current_text(problem, ix);

        let mut steps = vec![];

        match (patt, text) {
            (Patt::Any, Text::Lit(_)) =>
                steps.push(Next::hit(ix)),
            (Patt::Lit(a), Text::Lit(b)) if a == b =>
                steps.push(Next::hit(ix)),
            _ =>
                (),
        }

        match text {
            Text::Lit(_) =>
                steps.push(Next::skip_text(ix)),
            Text::End =>
                (),
        }

        match patt {
            Patt::Lit(_) | Patt::Any =>
                steps.push(Next::skip_patt(ix)),
            Patt::GroupStart =>
                steps.push(Next::start_group(ix)),
            Patt::GroupEnd =>
                steps.push(Next::stop_group(ix)),
            Patt::KleeneEnd(off) if ix.can_restart() =>
                steps.push(Next::restart_kleene(ix, off)),
            Patt::KleeneEnd(_) => // cannot restart
                steps.push(Next::end_kleene(ix)),
            Patt::KleeneStart(off) => {
                steps.push(Next::start_kleene(ix));
                steps.push(Next::pass_kleene(ix, off));
            }
            Patt::End =>
                (),
        }

        steps
    }
}

struct State {
    // TODO reconsider need to store a copy of Patt for every index in expanded pattern space
    nodes: Vec<Node>,
    expanded_pattern: Vec<Patt>,
    original_pattern_ix: Vec<usize>, // the original ix for each element in expanded_pattern
}

impl Index<Ix> for State {
    type Output = Node;

    fn index(&self, ix: Ix) -> &Self::Output {
        &self.nodes[self.nodes_ix(ix)]
    }
}

impl IndexMut<Ix> for State {
    fn index_mut(&mut self, ix: Ix) -> &mut Self::Output {
        self.nodes.index_mut(self.nodes_ix(ix))
    }
}

impl State {
    fn nodes_ix(&self, ix: Ix) -> usize {
        ix.text * self.expanded_pattern.len() + ix.pattern + ix.kleene_depth_this_text
    }
}


impl State {
    fn new(problem: &Problem) -> Self {
        let (expanded_pattern, original_pattern_ix) = Self::expand_pattern(&problem.pattern);
        let num_nodes = expanded_pattern.len() * problem.text.len();

        State { nodes: vec![Node::Ready; num_nodes], expanded_pattern, original_pattern_ix }
    }

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

    // TODO replace with config class which captures immutable stuff calculated from Problem
    fn current_pattern(&self, _problem: &Problem, ix: Ix) -> Patt {
        self.expanded_pattern[ix.pattern]
    }

    fn current_text(&self, problem: &Problem, ix: Ix) -> Text {
        problem.text[ix.text]
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Ix {
    pub text: usize,
    pub pattern: usize, // initial corresponding index in expanded pattern space
    pub kleene_depth: usize,
    pub kleene_depth_this_text: usize,
}

impl Ix {
    fn start() -> Self {
        Self { text: 0, pattern: 0, kleene_depth: 0, kleene_depth_this_text: 0 }
    }

    // TODO replace these uses of (&Problem, &State) which only look at immutable part of state
    //      instead, both solutions use their own Config for immutable stuff calculated from
    //      Problem/State
    fn end(problem: &Problem, state: &State) -> Self {
        Self {
            text: problem.text.len() - 1,
            pattern: state.expanded_pattern.len() - 1, // kleene_depth == 0 at end
            kleene_depth: 0,
            kleene_depth_this_text: 0,
        }
    }

    fn to_step(_problem: &Problem, state: &State, from: &Ix, done: &Done) -> Step {
        Step {
            from_patt: state.original_pattern_ix[from.pattern],
            to_patt: state.original_pattern_ix[done.next.pattern],
            from_text: from.text,
            to_text: done.next.text,
            score: done.score,
            kind: done.kind,
        }
    }

    fn can_restart(&self) -> bool {
        self.kleene_depth_this_text == 0
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Node {
    Ready,
    Working,
    Done(Done),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct Done {
    score: usize,
    next: Ix,
    kind: StepKind,
}

impl Done {
    fn combine_result<E>(left: Result<Self, E>, right: Result<Self, E>) -> Result<Self, E> {
        match (left, right) {
            (Ok(l), Ok(r)) => Ok(Self::combine(l, r)),
            (Err(l), _)    => Err(l),
            (_, Err(r))    => Err(r),
        }
    }

    fn combine(left: Self, right: Self) -> Self {
        if left.score <= right.score { left } else { right }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct Next {
    cost: usize,
    next: Ix,
    kind: StepKind,
}

impl Next {
    fn skip_text(ix: Ix) -> Self {
        let next = Ix {
            text: ix.text + 1,
            kleene_depth_this_text: 0,
            ..ix
        };
        Next { cost: 1, next, kind: StepKind::SkipText }
    }

    fn skip_patt(ix: Ix) -> Self {
        let next = Ix {
            pattern: ix.pattern + ix.kleene_depth + 1,
            ..ix
        };
        Next { cost: 1, next, kind: StepKind::SkipPattern }
    }

    fn hit(ix: Ix) -> Self {
        let next = Ix {
            text: ix.text + 1,
            pattern: ix.pattern + ix.kleene_depth + 1,
            kleene_depth_this_text: 0,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::Hit }
    }

    fn start_group(ix: Ix) -> Self {
        let next = Ix {
            pattern: ix.pattern + ix.kleene_depth + 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::StartCapture }
    }

    fn stop_group(ix: Ix) -> Self {
        let next = Ix {
            pattern: ix.pattern + ix.kleene_depth + 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::StopCapture }
    }

    fn start_kleene(ix: Ix) -> Self {
        let next = Ix {
            pattern: ix.pattern + ix.kleene_depth + 1,
            kleene_depth: ix.kleene_depth + 1,
            kleene_depth_this_text: ix.kleene_depth_this_text + 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn end_kleene(ix: Ix) -> Self {
        let next = Ix {
            pattern: ix.pattern + ix.kleene_depth + 1,
            kleene_depth: ix.kleene_depth - 1,
            kleene_depth_this_text: ix.kleene_depth_this_text - 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }

    fn pass_kleene(ix: Ix, off: usize) -> Self {
        let next = Ix {
            pattern: ix.pattern + off + ix.kleene_depth + 2,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp}
    }

    fn restart_kleene(ix: Ix, off: usize) -> Self {
        let next = Ix {
            pattern: ix.pattern - off,
            kleene_depth: ix.kleene_depth - 1,
            ..ix
        };
        Next { cost: 0, next, kind: StepKind::NoOp }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p_match_empty() -> Problem {
        Problem {
            pattern: vec![Patt::End],
            text:    vec![Text::End],
        }
    }

    fn p_match_lit_1() -> Problem {
        Problem {
            pattern: vec![Patt::Lit('a'), Patt::End],
            text:    vec![Text::Lit('a'), Text::End],
        }
    }

    fn p_match_lit_2() -> Problem {
        Problem {
            pattern: vec![Patt::Lit('a'), Patt::Lit('b'), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('b'), Text::End],
        }
    }

    fn p_match_kleene_1() -> Problem {
        Problem {
            pattern: vec![Patt::KleeneStart(2), Patt::Lit('a'), Patt::KleeneEnd(2), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
        }
    }

    fn p_match_kleene_2() -> Problem {
        Problem {
            pattern: vec![
                Patt::KleeneStart(5),
                Patt::Lit('a'),
                Patt::KleeneStart(2),
                Patt::Lit('b'),
                Patt::KleeneEnd(2),
                Patt::KleeneEnd(5),
                Patt::End
            ],
            text: vec![
                Text::Lit('a'),
                Text::Lit('a'),
                Text::Lit('b'),
                Text::Lit('a'),
                Text::Lit('b'),
                Text::Lit('b'),
                Text::End
            ],
        }
    }

    fn p_fail_empty_1() -> Problem {
        Problem {
            pattern: vec![Patt::End],
            text:    vec![Text::Lit('a'), Text::End],
        }
    }

    fn p_fail_empty_2() -> Problem {
        Problem {
            pattern: vec![Patt::Lit('a'), Patt::End],
            text:    vec![Text::End],
        }
    }

    fn p_fail_lit_1() -> Problem {
        Problem {
            pattern: vec![Patt::Lit('a'), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
        }
    }

    fn p_fail_lit_2() -> Problem {
        Problem {
            pattern: vec![Patt::Lit('a'), Patt::Lit('b'), Patt::Lit('a'), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
        }
    }

    fn p_fail_kleene_1() -> Problem {
        Problem {
            pattern: vec![Patt::KleeneStart(2), Patt::Lit('a'), Patt::KleeneEnd(2), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('b'), Text::Lit('a'), Text::End],
        }
    }

    #[test]
    fn score_match_empty() {
        let p = p_match_empty();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(0, vec![]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_lit_1() {
        let p = p_match_lit_1();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(0, vec![
            step(0, 0, 1, 1, 0, StepKind::Hit),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_lit_2() {
        let p = p_match_lit_2();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(0, vec![
            step(0, 0, 1, 1, 0, StepKind::Hit),
            step(1, 1, 2, 2, 0, StepKind::Hit),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_kleene_1() {
        let p = p_match_kleene_1();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(0, vec![
            step(0, 0, 1, 0, 0, StepKind::NoOp),
            step(1, 0, 2, 1, 0, StepKind::Hit),
            step(2, 1, 0, 1, 0, StepKind::NoOp),
            step(0, 1, 1, 1, 0, StepKind::NoOp),
            step(1, 1, 2, 2, 0, StepKind::Hit),
            step(2, 2, 0, 2, 0, StepKind::NoOp),
            step(0, 2, 3, 2, 0, StepKind::NoOp),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_kleene_2() {
        let p = p_match_kleene_2();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(0, vec![
            step(0, 0, 1, 0, 0, StepKind::NoOp),
            step(1, 0, 2, 1, 0, StepKind::Hit),
            step(2, 1, 5, 1, 0, StepKind::NoOp),
            step(5, 1, 0, 1, 0, StepKind::NoOp),
            step(0, 1, 1, 1, 0, StepKind::NoOp),
            step(1, 1, 2, 2, 0, StepKind::Hit),
            step(2, 2, 3, 2, 0, StepKind::NoOp),
            step(3, 2, 4, 3, 0, StepKind::Hit),
            step(4, 3, 2, 3, 0, StepKind::NoOp),
            step(2, 3, 5, 3, 0, StepKind::NoOp),
            step(5, 3, 0, 3, 0, StepKind::NoOp),
            step(0, 3, 1, 3, 0, StepKind::NoOp),
            step(1, 3, 2, 4, 0, StepKind::Hit),
            step(2, 4, 3, 4, 0, StepKind::NoOp),
            step(3, 4, 4, 5, 0, StepKind::Hit),
            step(4, 5, 2, 5, 0, StepKind::NoOp),
            step(2, 5, 3, 5, 0, StepKind::NoOp),
            step(3, 5, 4, 6, 0, StepKind::Hit),
            step(4, 6, 2, 6, 0, StepKind::NoOp),
            step(2, 6, 5, 6, 0, StepKind::NoOp),
            step(5, 6, 0, 6, 0, StepKind::NoOp),
            step(0, 6, 6, 6, 0, StepKind::NoOp),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_empty_1() {
        let p = p_fail_empty_1();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(1, vec![
            step(0, 0, 0, 1, 1, StepKind::SkipText),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_empty_2() {
        let p = p_fail_empty_2();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(1, vec![
            step(0, 0, 1, 0, 1, StepKind::SkipPattern),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_lit_1() {
        let p = p_fail_lit_1();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(1, vec![
            step(0, 0, 1, 1, 1, StepKind::Hit),
            step(1, 1, 1, 2, 1, StepKind::SkipText),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_lit_2() {
        let p = p_fail_lit_2();
        let actual = TableSolution::solve(&p).unwrap();
        let expected = table_solution(1, vec![
            step(0, 0, 1, 1, 1, StepKind::Hit),
            step(1, 1, 2, 1, 1, StepKind::SkipPattern),
            step(2, 1, 3, 2, 0, StepKind::Hit),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_kleene_1() {
        let p = p_fail_kleene_1();
        let actual = TableSolution::solve(&p).unwrap();

        // there are multiple possible traces
        let expected_score = 1;
        assert_eq!(&expected_score, actual.score());
    }

    fn table_solution(score: usize, trace: Vec<Step>) -> TableSolution {
        TableSolution { score, trace }
    }

    fn step(from_patt: usize, from_text: usize, to_patt: usize, to_text: usize, score: usize, kind: StepKind) -> Step {
        Step { from_patt, from_text, to_patt, to_text, score, kind }
    }
}
