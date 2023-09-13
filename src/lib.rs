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

fn score_impl(problem: &Problem, state: &mut State, ix: Ix) -> Option<usize> {
    match state.scores.get(&ix) {
        Some(Progress::Working) => {
            // Loop: we are guaranteed it is never beneficial to loop,
            // because extra traversal never reduces cost, so stop here.
            println!("Infinite loop detected at {:?}", ix);
            None
        }
        Some(Progress::Done(score, _)) =>
            // We've already done this ix: stop here.
            Some(*score),
        None => {
            // TODO ok, so I have a bug:
            //
            // When I detect a loop at ix X, I ignore the possibility of that traversal when assigning
            // scores to all the nodes Yi leading from X to X
            // But ignoring Yi -> X is only valid when we were calculating Y for the purpose of
            // calculating X. Yi -> X is still valid when arriving at Yi from other places!
            //
            // Compared to my simple scala implementation, this mutable one has more sharing ...
            //
            // Is it possible to resolve this without making large assumptions about the sort of
            // loops we have in our traversal? (Assumptions which are valid and CAN be made ... but
            // I would like this to be less fragile)
 
            state.scores.insert(ix, Progress::Working);
            let steps = problem.succ(ix);

            let best_outcome = steps
                .iter()
                .filter_map(|step| {
                    let target_score = score_impl(problem, state, step.ix());
                    target_score.map(|ts| (step.cost + ts, *step))
                })
                .min();

            let (score, step) = match best_outcome {
                Some(outcome) =>
                    outcome,
                None => {
                    assert!(ix == problem.end_ix(), "No legal moves at {:?}", ix);
                    (0, Step::forward(0, ix, 0, 0))
                }
            };

            state.scores.insert(ix, Progress::Done(score, step));
            Some(score)
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
enum Progress {
    Working,
    Done(usize, Step),
}

struct State {
  scores: HashMap<Ix, Progress>,
}

impl State {
    fn new() -> State {
        State { scores: HashMap::new() }
    }

    fn score(&self, problem: &Problem) -> Option<usize> {
        match self.scores.get(&problem.start_ix()) {
            Some(Progress::Done(score, _)) => Some(*score),
            _ => None,
        }
    }

    fn trace(&self, problem: &Problem) -> Option<Vec<Step>> {
        let mut optimal = vec![];
        let mut ix = problem.start_ix();
        while let Some(Progress::Done(_, step)) = self.scores.get(&ix) {
            if ix == problem.end_ix() {
                return Some(optimal);
            }
            optimal.push(*step);
            ix = step.ix();
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

    fn p_empty() -> Problem {
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

    fn p_match_kleene_1() -> Problem {
        Problem {
            pattern: vec![Patt::KleeneStart(2), Patt::Lit('a'), Patt::KleeneEnd(2), Patt::End],
            text:    vec![Text::Lit('a'), Text::Lit('a'), Text::End],
        }
    }

    fn p_all() -> Vec<Problem> {
        vec![p_empty(), p_match_lit_1(), p_match_kleene_1()]
    }

    #[test]
    fn score_empty() {
        let p = p_empty();
        let state = score(&p);
        let expected = Some(vec![]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_lit_match() {
        let p = p_match_lit_1();
        let state = score(&p);
        let expected = Some(vec![
            Step { cost: 0, pix: 1, tix: 1 },
        ]);
        let actual = state.trace(&p);
        assert_eq!(expected, actual);
    }

    #[test]
    fn score_kleene_match() {
        let p = p_match_kleene_1();
        let state = score(&p);
        let expected = Some(vec![
            Step { cost: 0, pix: 1, tix: 0 },
            Step { cost: 0, pix: 2, tix: 1 },
            Step { cost: 0, pix: 0, tix: 1 },
            Step { cost: 0, pix: 1, tix: 1 },
            Step { cost: 0, pix: 2, tix: 2 },
            Step { cost: 0, pix: 3, tix: 2 },
        ]);
        let actual = state.trace(&p);
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
        assert_eq!(p_empty().end_ix(), Ix { pix: 0, tix: 0 });
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
