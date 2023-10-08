//! Provides an implementation of [`Output`] suitable for development.

use crate::{Output, Problem, Step};
use std::fmt;

pub struct DebugOutput {
    output: String,
}

impl Output for DebugOutput {
    fn new(_problem: &Problem, score: &usize, trace: &Vec<Step>) -> Self {
        Self { output: format!("score: {}\ntrace: {:#?}", *score, *trace) }
    }
}

impl fmt::Display for DebugOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.output)
    }
}