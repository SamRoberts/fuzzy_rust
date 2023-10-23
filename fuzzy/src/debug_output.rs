//! Provides an implementation of [`Output`] suitable for development.

use crate::{Match, Output, ProblemV2, Step};
use std::fmt;

pub struct DebugOutput {
    output: String,
}

impl Output for DebugOutput {
    fn new(_problem: &ProblemV2, score: &usize, trace: &Vec<Step<Match, char>>) -> Self {
        Self { output: format!("score: {}\ntrace: {:#?}", *score, *trace) }
    }
}

impl fmt::Display for DebugOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.output)
    }
}
