use std::collections::hash_map::HashMap;

// Initial naive attempt
// Takes hashmap from simple scala implementation as well as recursive traversal
// but representation of nodes and edges more from loop

// It won't be syntactically possible to interleave kleene ranges with group ranges
// And the parser will ensure that all groups are balanced
// So our algorithm does not have to worry about having more "starts" than "ends"

fn score(problem: &Problem) -> State {
    let mut state = State::new();
    let _ = score_impl(problem, &mut state, problem.start_ix());
    state
}

fn score_impl(problem: &Problem, state: &mut State, ix: Ix) -> Edge {
    match state.nodes.get(&ix) {
        Some(Node::Working) =>
            // We will enter an infinite loop if we go here.
            // However, this path may still be valid in the end: we
            // just don't know what it costs yet. So return an eddge that
            // depends on this node.
            Edge::Depends { cost: 0, node: ix },
        Some(Node::Done { score, next }) =>
            // We've already done this ix: stop here.
            Edge::Score { score: *score, next: ix },
        None => {
            state.nodes.insert(ix, Node::Working);
            let steps = problem.succ(ix);

            let best_edge = steps.iter()
                .map(|step| score_impl(problem, state, step.ix()).add_cost(step.cost))
                .reduce(Edge::combine)
                .and_then(|combined| combined.remove_loop(ix));

            match best_edge {
                None => {
                    assert!(ix == problem.end_ix(), "No legal moves at {:?}", ix);
                    state.nodes.insert(ix, Node::Done { score: 0, next: ix });
                    Edge::Score { score: 0, next: ix }
                },
                Some(Edge::Score { score , next }) => {
                    state.nodes.insert(ix, Node::Done { score, next });
                    Edge::Score { score , next: ix }
                },
                Some(Edge::Depends { cost, node }) => {
                    state.nodes.remove(&ix);
                    Edge::Depends { cost, node }
                },
                Some(Edge::DependsOrScore { cost, node, alt_score, alt_next }) => {
                    state.nodes.remove(&ix);
                    Edge::DependsOrScore { cost, node, alt_score, alt_next: ix }
                },
            }
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum Patt {
    Lit(char),
    Any,
    GroupStart,
    GroupEnd,
    KleeneStart(usize), // the offset of the end
    KleeneEnd(usize),   // the offset of the start
    End,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum Text {
    Lit(char),
    End
}

#[derive(Eq, PartialEq, Copy, Clone, Hash, Debug)]
struct Ix {
    pix: usize,
    tix: usize,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Debug)]
struct Step {
    cost: usize,
    pix: usize,
    tix: usize,
}

impl Step {
    fn forward(cost: usize, ix: Ix, poff: usize, toff: usize) -> Step {
        Step { cost: cost, pix: ix.pix + poff, tix: ix.tix + toff }
    }

    fn back(cost: usize, ix: Ix, poff: usize, toff: usize) -> Step {
        Step { cost: cost, pix: ix.pix - poff, tix: ix.tix - toff }
    }

    fn ix(&self) -> Ix {
        Ix { pix: self.pix, tix: self.tix }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum Node {
    Working,
    Done { score: usize, next: Ix },
}

#[derive(Copy, Clone, Debug)]
enum Edge {
    Score { score: usize , next: Ix },
    Depends { cost: usize, node: Ix },
    // Either depends path or path with known score might be viable
    // alt_score should be > cost
    DependsOrScore { cost: usize, node: Ix, alt_score: usize, alt_next: Ix },
}

impl Edge {
    fn combine(left: Self, right: Self) -> Self {
        match (left, right) {
            (Edge::Score { score: score_l , .. }, Edge::Score { score: score_r, .. }) if score_l <= score_r =>
                left,
            (Edge::Score { score: score_l , .. }, Edge::Score { score: score_r, .. }) if score_l > score_r =>
                right,
            (Edge::Score { score, .. }, Edge::Depends { cost, .. }) if score <= cost =>
                left,
            (Edge::Score { score, next }, Edge::Depends { cost, node }) if score > cost =>
                Edge::DependsOrScore { cost: cost, node: node, alt_score: score, alt_next: next },
            (Edge::Score { score, ..}, Edge::DependsOrScore { cost, .. }) if score <= cost =>
                left,
            (Edge::Score { score, .. }, Edge::DependsOrScore { alt_score, .. }) if score > alt_score =>
                right,
            (Edge::Score { score, next }, Edge::DependsOrScore { cost, node, .. }) => 
                Edge::DependsOrScore { cost: cost, node: node, alt_score: score, alt_next: next },
            (Edge::Depends { .. }, Edge::Score { .. }) =>
                Self::combine(right, left),
            (Edge::DependsOrScore { .. }, Edge::Score { .. }) =>
                Self::combine(right, left),
            _ =>
                // Ok, so we do hit this condition with nested kleenes:
                // need to understand how, as I thought this would only happen if kleene groups were interleaved
                panic!("Cannot handle multiple depends edges {:?} and {:?}", left, right),
        }
    }

    fn remove_loop(&self, current: Ix) -> Option<Edge> {
        match self {
            Edge::Depends { node, .. } if *node == current =>
                None,
            Edge::DependsOrScore { node, alt_score, alt_next, .. } if *node == current =>
                Some(Edge::Score { score: *alt_score, next: *alt_next }),
            _ =>
                Some(*self)
        }
    }

    fn add_cost(&self, extra: usize) -> Edge {
        match self {
            Edge::Score { score, next } => Edge::Score { score: *score + extra, next: *next },
            Edge::Depends { cost, node } => Edge::Depends { cost: *cost + extra, node: *node },
            Edge::DependsOrScore { cost, node, alt_score, alt_next } => Edge::DependsOrScore {
                cost: *cost + extra,
                node: *node,
                alt_score: *alt_score + extra,
                alt_next: *alt_next
            }
        }
    }
}

struct State {
  nodes: HashMap<Ix, Node>,
}

impl State {
    fn new() -> State {
        State {
            nodes: HashMap::new(),
        }
    }

    fn score(&self, problem: &Problem) -> Option<usize> {
        match self.nodes.get(&problem.start_ix()) {
            Some(Node::Done { score, .. }) => Some(*score),
            _ => None,
        }
    }

    fn trace(&self, problem: &Problem) -> Option<Vec<Ix>> {
        let mut optimal = vec![];
        let mut ix = problem.start_ix();
        while let Some(Node::Done { score, next }) = self.nodes.get(&ix) {
            if ix == problem.end_ix() {
                return Some(optimal);
            }
            ix = *next;
            optimal.push(ix);
        }
        return None;
    }
}

struct Problem {
  pattern: Vec<Patt>,
  text: Vec<Text>,
}

impl Problem {
    fn start_ix(&self) -> Ix {
        Ix { pix: 0, tix: 0}
    }

    fn end_ix(&self) -> Ix {
        Ix { pix: self.pattern.len() - 1, tix: self.text.len() - 1 }
    }

    fn succ(&self, ix: Ix) -> Vec<Step> {
        let patt = self.pattern[ix.pix];
        let text = self.text[ix.tix];

        let mut scores = vec![];

        match (patt, text) {
            (Patt::Any, Text::Lit(_)) =>
                scores.push(Step::forward(0, ix, 1, 1 )),
            (Patt::Lit(a), Text::Lit(b)) if a == b =>
                scores.push(Step::forward(0, ix, 1, 1 )),
            _ =>
                (),
        }

        match text {
            Text::Lit(_) =>
                scores.push(Step::forward(1, ix, 0, 1)),
            Text::End =>
                (),
        }

        match patt {
            Patt::Lit(_) | Patt::Any =>
                scores.push(Step::forward(1, ix, 1, 0)),
            Patt::GroupStart | Patt::GroupEnd =>
                scores.push(Step::forward(0, ix, 1, 0)),
            Patt::KleeneEnd(off) => {
                scores.push(Step::forward(0, ix, 1, 0));
                scores.push(Step::back(0, ix, off, 0));
            }
            Patt::KleeneStart(off) => {
                scores.push(Step::forward(0, ix, 1, 0));
                scores.push(Step::forward(0, ix, off + 1, 0));
            }
            Patt::End =>
                (),
        }

        scores
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
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

    fn p_all() -> Vec<Problem> {
        vec![p_match_empty(), p_match_lit_1(), p_match_kleene_1()]
    }

    #[test]
    fn score_match_empty() {
        let p = p_match_empty();
        let state = score(&p);
        let expected = Some(vec![]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_lit_1() {
        let p = p_match_lit_1();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 1 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_lit_2() {
        let p = p_match_lit_2();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 1 },
            Ix { pix: 2, tix: 2 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_kleene_1() {
        let p = p_match_kleene_1();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 0 },
            Ix { pix: 2, tix: 1 },
            Ix { pix: 0, tix: 1 },
            Ix { pix: 1, tix: 1 },
            Ix { pix: 2, tix: 2 },
            Ix { pix: 3, tix: 2 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_kleene_2() {
        let p = p_match_kleene_2();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 0 },
            Ix { pix: 2, tix: 1 },
            Ix { pix: 0, tix: 1 },
            Ix { pix: 1, tix: 1 },
            Ix { pix: 2, tix: 2 },
            Ix { pix: 3, tix: 2 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_empty_1() {
        let p = p_fail_empty_1();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 0, tix: 1 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_empty_2() {
        let p = p_fail_empty_2();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 0 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_lit_1() {
        let p = p_fail_lit_1();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 1 },
            Ix { pix: 1, tix: 2 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_lit_2() {
        let p = p_fail_lit_2();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 1 },
            Ix { pix: 2, tix: 1 },
            Ix { pix: 3, tix: 2 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_kleene_1() {
        let p = p_fail_kleene_1();
        let state = score(&p);
        let expected = Some(1);
        let actual = state.score(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn problem_start_ix() {
        for p in p_all() {
            assert_eq!(p.start_ix(), Ix { pix: 0, tix: 0 });
        }
    }

    #[test]
    fn problem_end_ix() {
        assert_eq!(p_match_empty().end_ix(), Ix { pix: 0, tix: 0 });
        assert_eq!(p_match_lit_1().end_ix(), Ix { pix: 1, tix: 1 });
        assert_eq!(p_match_kleene_1().end_ix(), Ix { pix: 3, tix: 2 });
    }

    #[test]
    fn problem_succ_nothing_at_end() {
        for p in p_all() {
            assert_eq!(p.succ(p.end_ix()), vec![]);
        }
    }

    #[test]
    fn problem_succ_lit_match() {
        let p = p_match_lit_1();
        let expected = HashSet::from([
            Step { cost: 0, pix: 1, tix: 1 },
            Step { cost: 1, pix: 1, tix: 0 },
            Step { cost: 1, pix: 0, tix: 1 },
        ]);
        let actual = HashSet::from_iter(p.succ(p.start_ix()));

        assert_eq!(expected, actual);
    }

    #[test]
    fn problem_succ_kleene_start() {
        let p = p_match_kleene_1();
        let expected = HashSet::from([
            Step { cost: 1, pix: 0, tix: 1 },
            Step { cost: 0, pix: 1, tix: 0 },
            Step { cost: 0, pix: 3, tix: 0 },
        ]);
        let actual = HashSet::from_iter(p.succ(p.start_ix()));

        assert_eq!(expected, actual);
    }

    #[test]
    fn problem_succ_kleene_end() {
        let p = p_match_kleene_1();
        let expected = HashSet::from([
            Step { cost: 1, pix: 2, tix: 1 },
            Step { cost: 0, pix: 0, tix: 0 },
            Step { cost: 0, pix: 3, tix: 0 },
        ]);
        let actual = HashSet::from_iter(p.succ(Ix { pix: 2, tix: 0}));

        assert_eq!(expected, actual);
    }
}
