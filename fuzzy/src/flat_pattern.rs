use crate::{Class, Element, Match, Pattern};

/// A flattened alternative to [`Pattern`], so we can index our position with a single number.
pub struct FlatPattern {
    elems: Vec<Flat>,
}

impl FlatPattern {
    // TODO I am sure there is a trait I can implement instead
    pub fn get(&self, i: usize) -> Option<&Flat> {
        self.elems.get(i)
    }

    pub fn len(&self) -> usize {
        self.elems.len()
    }
}

impl FlatPattern {
    pub fn new(pattern: &Pattern) -> Self {
        Self::custom(pattern, 0)
    }

    /// This constructor includes custom flags to change the pattern we generate.
    ///
    /// The `rep_incr logic` arguably belongs in [`table_solution`](crate::table_solution::Config),
    /// but was easy to implement here. This flag controls how we increase the number of times the
    /// flat pattern replicates individual pattern elements as we enter repetition groups. See the
    /// ["repetition depth"](crate::table_solution::Ix::rep_off) discussion for more detail.
    ///
    /// If `rep_incr == 0`, the flat pattern just copies each individual pattern element once.
    ///
    /// If `rep_incr == 1`, we increase the number of copies inside repetition groups. For example:
    ///
    /// ```ignore
    /// Original pattern: abc<d e f < g  h  i  >  j k l > mno, offsets: 12,  4,  4, and 12, length: 19
    /// Expanded pattern: abc<ddeeff<<ggghhhiii>>>jjkkll>>mno, offsets: 27, 11, 11, and 27, length: 35
    ///
    /// (In this example, < and > represent the start and end of repetitions.)
    /// ```
    pub fn custom(pattern: &Pattern, rep_incr: usize) -> Self {
        let mut elems = vec![];
        Self::pattern_patts(&mut elems, &pattern, 1, rep_incr);
        FlatPattern { elems }
    }

    fn pattern_patts(result: &mut Vec<Flat>, pattern: &Pattern, reps: usize, rep_incr: usize) {
        for elem in pattern.elems.iter() {
            Self::elem_patts(result, elem, reps, rep_incr)
        }
    }

    fn elem_patts(result: &mut Vec<Flat>, elem: &Element, reps: usize, rep_incr: usize) {
        match elem {
            Element::Match(Match::Lit(c)) =>
                Self::single_patt(result, Flat::Lit(*c), reps),
            Element::Match(Match::Class(class)) =>
                Self::single_patt(result, Flat::Class(class.clone()), reps),
            Element::Capture(inner) => {
                Self::single_patt(result, Flat::GroupStart, reps);
                Self::pattern_patts(result, inner, reps, rep_incr);
                Self::single_patt(result, Flat::GroupEnd, reps);
            }
            Element::Repetition(inner) => {
                let next_reps = reps + rep_incr;
                let start_ix = result.len();
                Self::single_patt(result, Flat::RepetitionStart(0), reps);
                Self::pattern_patts(result, inner, next_reps, rep_incr);
                let end_ix = result.len();
                Self::single_patt(result, Flat::RepetitionEnd(0), next_reps);

                let off = end_ix - start_ix;
                Self::update_patt(result, Flat::RepetitionStart(off), start_ix, reps);
                Self::update_patt(result, Flat::RepetitionEnd(off), end_ix, next_reps);
            }
            Element::Alternative(p1, p2) => {
                let left_ix = result.len();
                Self::single_patt(result, Flat::AlternativeLeft(0), reps);
                Self::pattern_patts(result, p1, reps, rep_incr);
                let right_ix = result.len();
                Self::single_patt(result, Flat::AlternativeRight(0), reps);
                Self::pattern_patts(result, p2, reps, rep_incr);
                let next_ix = result.len();

                let left_off = right_ix - left_ix;
                let right_off = next_ix - right_ix;
                Self::update_patt(result, Flat::AlternativeLeft(left_off), left_ix, reps);
                Self::update_patt(result, Flat::AlternativeRight(right_off), right_ix, reps);
            }
        }
    }

    fn single_patt(result: &mut Vec<Flat>, elem: Flat, reps: usize) {
        for _ in 0..reps {
            result.push(elem.clone());
        }
    }

    fn update_patt(result: &mut Vec<Flat>, elem: Flat, ix: usize, reps: usize) {
        for i in 0..reps {
            result[ix + i] = elem.clone();
        }
    }
}

/// An individual element in [`FlatPattern`].
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Flat {
    /// Matches a specific character.
    ///
    /// Although this API implies this crate operates on unicode characters, the current code
    /// sometimes naively converts bytes to characters, assuming ASCII.
    Lit(char),
    /// Matches a class of characters, e.g. `.` or `[a-z]`.
    Class(Class),
    GroupStart,
    GroupEnd,
    /// Starts the first branch of an alternation.
    ///
    /// This stores the offset between this item and the corresponding
    /// [`AlternativeRight`](Flat::AlternativeRight) branch.
    AlternativeLeft(usize),
    /// Starts the second branch of an alternation.
    ///
    /// This stores the offset between this item and the element immediately after the alternation.
    AlternativeRight(usize),
    /// Starts a repetition.
    ///
    /// This stores the offset between this item and the corresponding future
    /// [`RepetitionEnd`](Flat::RepetitionEnd) item.
    RepetitionStart(usize),
    /// Ends a repetition.
    ///
    /// This stores the offset between this item and the corresponding past
    /// [`RepetitionStart`](Flat::RepetitionStart) item.
    RepetitionEnd(usize),
}

