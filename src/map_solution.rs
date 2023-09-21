use std::collections::hash_map::HashMap;
use crate::{Patt, Problem, Solution, Step, StepKind, Text};
use crate::error::Error;

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

impl Solution<Error> for MapSolution {
    fn solve(problem: &Problem) -> Result<MapSolution, Error> {
        let start_ix = Ix { pix: 0, tix: 0, kix: 0 };
        let end_ix = Ix { pix: problem.pattern.len() - 1, tix: problem.text.len() - 1, kix: 0 };

        let mut state = State { nodes: HashMap::new(), };
        let _ = Self::solve_impl(problem, &mut state, end_ix, start_ix, 0, StepKind::NoOp)?;

        let score = match state.nodes.get(&start_ix) {
            Some(Node::Done(Done { score, .. })) => Ok(*score),
            _ => Err(Error::IncompleteFinalState),
        }?;

        let mut trace = vec![];
        let mut from = &start_ix;
        while let Some(Node::Done(Done { score, kind, next })) = state.nodes.get(from) {
            if from == &end_ix { break; }
            let step = Step {
                from_patt: from.pix,
                from_text: from.tix,
                to_patt: next.pix,
                to_text: next.tix,
                score: *score,
                kind: *kind,
            };
            trace.push(step);
            from = next;
        }
        if from != &end_ix {
            return Err(Error::IncompleteFinalState);
        }

        Ok(MapSolution { score, trace })
    }

    fn score(&self) -> &usize {
        &self.score
    }

    fn trace(&self) -> &Vec<Step> {
        &self.trace
    }
}

impl MapSolution {
    fn solve_impl(problem: &Problem, state: &mut State, end_ix: Ix, ix: Ix, cost: usize, kind: StepKind) -> Result<Done, Error> {
        match state.nodes.get(&ix) {
            Some(Node::Working) =>
                Err(Error::InfiniteLoop(format!("{:?}", ix))),
            Some(Node::Done(done)) =>
                Ok(Done { score: done.score + cost, next: ix, kind }),
            None => {
                state.nodes.insert(ix, Node::Working);
                let steps = Self::succ(problem, ix);

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

                state.nodes.insert(ix, Node::Done(score));
                Ok(Done { score: score.score + cost, next: ix, kind })
            }
        }
    }

    fn succ(problem: &Problem, ix: Ix) -> Vec<Next> {
        let patt = problem.pattern[ix.pix];
        let text = problem.text[ix.tix];

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
            Patt::KleeneEnd(_) if ix.kix > 0 =>
                steps.push(Next::end_kleene(ix)),
            Patt::KleeneEnd(off) => // ix.kix == 0
                steps.push(Next::restart_kleene(ix, off)),
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

pub struct State {
  nodes: HashMap<Ix, Node>,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Ix {
    // There is a separate score associated with each combination of:
    //   1. the place we are up to in the pattern
    //   2. the place we are up to in the text
    //   3. how many kleene patterns we have passed into since we last made
    //      progress in the text. It is never beneficial to backtrack
    //      to the start of a kleene group if we haven't progressed through
    //      the text since starting that kleene.

    // TODO let's change these ix names later ...
    pub pix: usize,
    pub tix: usize,
    pub kix: usize,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Node {
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
        Next { cost: 1, next: Ix { tix: ix.tix + 1, kix: 0, ..ix }, kind: StepKind::SkipText }
    }

    fn skip_patt(ix: Ix) -> Self {
        Next { cost: 1, next: Ix { pix: ix.pix + 1, ..ix }, kind: StepKind::SkipPattern }
    }

    fn hit(ix: Ix) -> Self {
        Next { cost: 0, next: Ix { pix: ix.pix + 1, tix: ix.tix + 1, kix: 0 }, kind: StepKind::Hit }
    }

    fn start_group(ix: Ix) -> Self {
        Next { cost: 0, next: Ix { pix: ix.pix + 1, ..ix }, kind: StepKind::StartCapture }
    }

    fn stop_group(ix: Ix) -> Self {
        Next { cost: 0, next: Ix { pix: ix.pix + 1, ..ix }, kind: StepKind::StopCapture }
    }

    fn start_kleene(ix: Ix) -> Self {
        Next { cost: 0, next: Ix { pix: ix.pix + 1, kix: ix.kix + 1, ..ix }, kind: StepKind::NoOp }
    }

    fn end_kleene(ix: Ix) -> Self {
        Next { cost: 0, next: Ix { pix: ix.pix + 1, kix: ix.kix - 1, ..ix }, kind: StepKind::NoOp }
    }

    fn pass_kleene(ix: Ix, off: usize) -> Self {
        Next { cost: 0, next: Ix { pix: ix.pix + 1 + off, ..ix }, kind: StepKind::NoOp}
    }

    fn restart_kleene(ix: Ix, off: usize) -> Self {
        Next { cost: 0, next: Ix { pix: ix.pix - off, ..ix }, kind: StepKind::NoOp }
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
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(0, vec![]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_lit_1() {
        let p = p_match_lit_1();
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(0, vec![
            step(0, 0, 1, 1, 0, StepKind::Hit),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_lit_2() {
        let p = p_match_lit_2();
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(0, vec![
            step(0, 0, 1, 1, 0, StepKind::Hit),
            step(1, 1, 2, 2, 0, StepKind::Hit),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_kleene_1() {
        let p = p_match_kleene_1();
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(0, vec![
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
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(0, vec![
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
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(1, vec![
            step(0, 0, 0, 1, 1, StepKind::SkipText),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_empty_2() {
        let p = p_fail_empty_2();
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(1, vec![
            step(0, 0, 1, 0, 1, StepKind::SkipPattern),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_lit_1() {
        let p = p_fail_lit_1();
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(1, vec![
            step(0, 0, 1, 1, 1, StepKind::Hit),
            step(1, 1, 1, 2, 1, StepKind::SkipText),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_lit_2() {
        let p = p_fail_lit_2();
        let actual = MapSolution::solve(&p).unwrap();
        let expected = map_solution(1, vec![
            step(0, 0, 1, 1, 1, StepKind::Hit),
            step(1, 1, 2, 1, 1, StepKind::SkipPattern),
            step(2, 1, 3, 2, 0, StepKind::Hit),
        ]);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_kleene_1() {
        let p = p_fail_kleene_1();
        let actual = MapSolution::solve(&p).unwrap();

        // there are multiple possible traces
        let expected_score = 1;
        assert_eq!(&expected_score, actual.score());
    }

    fn map_solution(score: usize, trace: Vec<Step>) -> MapSolution {
        MapSolution { score, trace }
    }

    fn step(from_patt: usize, from_text: usize, to_patt: usize, to_text: usize, score: usize, kind: StepKind) -> Step {
        Step { from_patt, from_text, to_patt, to_text, score, kind }
    }
}
