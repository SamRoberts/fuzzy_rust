use std::collections::hash_map::HashMap;

// Initial naive attempt
// Takes hashmap from simple scala implementation as well as recursive traversal
// but representation of nodes and edges more from loop

// It won't be syntactically possible to interleave kleene ranges with group ranges
// And the parser will ensure that all groups are balanced
// So our algorithm does not have to worry about having more "starts" than "ends"


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

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Debug)]
struct Ix {
    // There is a separate score associated with each combination of:
    //   1. the place we are up to in the pattern
    //   2. the place we are up to in the text
    //   3. how many kleene patterns we have passed into since we last made
    //      progress in the text. It is never beneficial to backtrack
    //      to the start of a kleene group if we haven't progressed through
    //      the text since starting that kleene.

    // TODO let's change these ix names later ...
    pix: usize,
    tix: usize,
    kix: usize,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Debug)]
struct Step {
    cost: usize,
    next: Ix,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Debug)]
struct Score {
    score: usize,
    next: Ix,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum Node {
    Working,
    Done(Score),
}

struct State {
  nodes: HashMap<Ix, Node>,
}

struct Problem {
  pattern: Vec<Patt>,
  text: Vec<Text>,
}

fn score(problem: &Problem) -> State {
    let mut state = State::new();
    let _ = score_impl(problem, &mut state, problem.start_ix());
    state
}

fn score_impl(problem: &Problem, state: &mut State, ix: Ix) -> Score {
    match state.nodes.get(&ix) {
        Some(Node::Working) =>
            panic!("About to enter an infinite loop at {:?}", ix),
        Some(Node::Done(score)) =>
            Score { score: score.score, next: ix },
        None => {
            state.nodes.insert(ix, Node::Working);
            let steps = problem.succ(ix);

            let maybe_score = steps.iter()
                .map(|step| score_impl(problem, state, step.next).add_cost(step.cost))
                .reduce(Score::combine);

            let score = maybe_score.unwrap_or_else(|| {
                assert!(ix == problem.end_ix(), "No legal moves at {:?}", ix);
                Score { score: 0, next: problem.end_ix() }
            });

            state.nodes.insert(ix, Node::Done(score));
            Score { score: score.score, next: ix }
        }
    }
}

impl Step {
    fn new(cost: usize, pix: usize, tix: usize, kix: usize) -> Self {
        Step { cost, next: Ix { pix, tix, kix } }
    }

    fn skip_text(ix: Ix) -> Self {
        Step { cost: 1, next: Ix { tix: ix.tix + 1, kix: 0, ..ix } }
    }

    fn skip_patt(ix: Ix) -> Self {
        Step { cost: 1, next: Ix { pix: ix.pix + 1, ..ix } }
    }

    fn hit(ix: Ix) -> Self {
        Step { cost: 0, next: Ix { pix: ix.pix + 1, tix: ix.tix + 1, kix: 0 } }
    }

    fn pass_group(ix: Ix) -> Self {
        Step { cost: 0, next: Ix { pix: ix.pix + 1, ..ix } }
    }

    fn start_kleene(ix: Ix) -> Self {
        Step { cost: 0, next: Ix { pix: ix.pix + 1, kix: ix.kix + 1, ..ix } }
    }

    fn end_kleene(ix: Ix) -> Self {
        Step { cost: 0, next: Ix { pix: ix.pix + 1, kix: ix.kix - 1, ..ix } }
    }

    fn pass_kleene(ix: Ix, off: usize) -> Self {
        Step { cost: 0, next: Ix { pix: ix.pix + 1 + off, ..ix } }
    }

    fn restart_kleene(ix: Ix, off: usize) -> Self {
        Step { cost: 0, next: Ix { pix: ix.pix - off, ..ix } }
    }
}

impl Score {
    fn add_cost(&self, cost: usize) -> Score {
        Score { score: self.score + cost, ..*self }
    }

    fn combine(left: Self, right: Self) -> Self {
        if left.score <= right.score { left } else { right }
    }
}

impl State {
    fn new() -> State {
        State {
            nodes: HashMap::new(),
        }
    }

    fn score(&self, problem: &Problem) -> Option<usize> {
        match self.nodes.get(&problem.start_ix()) {
            Some(Node::Done(Score { score, .. })) => Some(*score),
            _ => None,
        }
    }

    fn trace(&self, problem: &Problem) -> Option<Vec<Ix>> {
        let mut optimal = vec![];
        let mut ix = problem.start_ix();
        while let Some(Node::Done(Score { next, .. })) = self.nodes.get(&ix) {
            if ix == problem.end_ix() {
                return Some(optimal);
            }
            ix = *next;
            optimal.push(ix);
        }
        return None;
    }
}

impl Problem {
    fn start_ix(&self) -> Ix {
        Ix { pix: 0, tix: 0, kix: 0 }
    }

    fn end_ix(&self) -> Ix {
        Ix { pix: self.pattern.len() - 1, tix: self.text.len() - 1, kix: 0 }
    }

    fn succ(&self, ix: Ix) -> Vec<Step> {
        let patt = self.pattern[ix.pix];
        let text = self.text[ix.tix];

        let mut steps = vec![];

        match (patt, text) {
            (Patt::Any, Text::Lit(_)) =>
                steps.push(Step::hit(ix)),
            (Patt::Lit(a), Text::Lit(b)) if a == b =>
                steps.push(Step::hit(ix)),
            _ =>
                (),
        }

        match text {
            Text::Lit(_) =>
                steps.push(Step::skip_text(ix)),
            Text::End =>
                (),
        }

        match patt {
            Patt::Lit(_) | Patt::Any =>
                steps.push(Step::skip_patt(ix)),
            Patt::GroupStart | Patt::GroupEnd =>
                steps.push(Step::pass_group(ix)),
            Patt::KleeneEnd(_) if ix.kix > 0 =>
                steps.push(Step::end_kleene(ix)),
            Patt::KleeneEnd(off) => // ix.kix == 0
                steps.push(Step::restart_kleene(ix, off)),
            Patt::KleeneStart(off) => {
                steps.push(Step::start_kleene(ix));
                steps.push(Step::pass_kleene(ix, off));
            }
            Patt::End =>
                (),
        }

        steps
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
        vec![
            p_match_empty(),
            p_match_lit_1(),
            p_match_lit_2(),
            p_match_kleene_1(),
            p_match_kleene_2(),
            p_fail_empty_1(),
            p_fail_empty_2(),
            p_fail_lit_1(),
            p_fail_lit_2(),
            p_fail_kleene_1(),
        ]
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
            Ix { pix: 1, tix: 1, kix: 0 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_lit_2() {
        let p = p_match_lit_2();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 1, kix: 0 },
            Ix { pix: 2, tix: 2, kix: 0 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_kleene_1() {
        let p = p_match_kleene_1();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 0, kix: 1 },
            Ix { pix: 2, tix: 1, kix: 0 },
            Ix { pix: 0, tix: 1, kix: 0 },
            Ix { pix: 1, tix: 1, kix: 1 },
            Ix { pix: 2, tix: 2, kix: 0 },
            Ix { pix: 0, tix: 2, kix: 0 },
            Ix { pix: 3, tix: 2, kix: 0 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_match_kleene_2() {
        let p = p_match_kleene_2();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 0, kix: 1 },
            Ix { pix: 2, tix: 1, kix: 0 },
            Ix { pix: 5, tix: 1, kix: 0 },
            Ix { pix: 0, tix: 1, kix: 0 },
            Ix { pix: 1, tix: 1, kix: 1 },
            Ix { pix: 2, tix: 2, kix: 0 },
            Ix { pix: 3, tix: 2, kix: 1 },
            Ix { pix: 4, tix: 3, kix: 0 },
            Ix { pix: 2, tix: 3, kix: 0 },
            Ix { pix: 5, tix: 3, kix: 0 },
            Ix { pix: 0, tix: 3, kix: 0 },
            Ix { pix: 1, tix: 3, kix: 1 },
            Ix { pix: 2, tix: 4, kix: 0 },
            Ix { pix: 3, tix: 4, kix: 1 },
            Ix { pix: 4, tix: 5, kix: 0 },
            Ix { pix: 2, tix: 5, kix: 0 },
            Ix { pix: 3, tix: 5, kix: 1 },
            Ix { pix: 4, tix: 6, kix: 0 },
            Ix { pix: 2, tix: 6, kix: 0 },
            Ix { pix: 5, tix: 6, kix: 0 },
            Ix { pix: 0, tix: 6, kix: 0 },
            Ix { pix: 6, tix: 6, kix: 0 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_empty_1() {
        let p = p_fail_empty_1();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 0, tix: 1, kix: 0 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_empty_2() {
        let p = p_fail_empty_2();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 0, kix: 0 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_lit_1() {
        let p = p_fail_lit_1();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 1, kix: 0 },
            Ix { pix: 1, tix: 2, kix: 0 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_fail_lit_2() {
        let p = p_fail_lit_2();
        let state = score(&p);
        let expected = Some(vec![
            Ix { pix: 1, tix: 1, kix: 0 },
            Ix { pix: 2, tix: 1, kix: 0 },
            Ix { pix: 3, tix: 2, kix: 0 },
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
            assert_eq!(p.start_ix(), Ix { pix: 0, tix: 0, kix: 0 });
        }
    }

    #[test]
    fn problem_end_ix() {
        assert_eq!(p_match_empty().end_ix(), Ix { pix: 0, tix: 0, kix: 0 });
        assert_eq!(p_match_lit_1().end_ix(), Ix { pix: 1, tix: 1, kix: 0 });
        assert_eq!(p_match_kleene_1().end_ix(), Ix { pix: 3, tix: 2, kix: 0 });
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
            Step::new(0, 1, 1, 0),
            Step::new(1, 1, 0, 0),
            Step::new(1, 0, 1, 0),
        ]);
        let actual = HashSet::from_iter(p.succ(p.start_ix()));

        assert_eq!(expected, actual);
    }

    #[test]
    fn problem_succ_kleene_start() {
        let p = p_match_kleene_1();
        let expected = HashSet::from([
            Step::new(1, 0, 1, 0),
            Step::new(0, 1, 0, 1),
            Step::new(0, 3, 0, 0),
        ]);
        let actual = HashSet::from_iter(p.succ(p.start_ix()));

        assert_eq!(expected, actual);
    }

    #[test]
    fn problem_succ_kleene_end() {
        let p = p_match_kleene_1();
        let expected = HashSet::from([
            Step::new(1, 2, 1, 0),
            Step::new(0, 0, 0, 0),
        ]);
        let actual = HashSet::from_iter(p.succ(Ix { pix: 2, tix: 0, kix: 0}));

        assert_eq!(expected, actual);
    }
}
