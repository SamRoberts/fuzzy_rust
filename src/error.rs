//! Provides the [`enum@Error`] type currently used by all fuzzy code.

use thiserror::Error;
use std::io;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not read file: {0}")]
    CouldNotReadFile(#[from] io::Error),
    #[error("PATTERN not a valid regex: {0}")]
    PatternNotRegex(#[from] regex_syntax::Error),
    #[error("PATTERN has unsupported regex: {0}")]
    PatternUnsupported(String),
    #[error("Internal error: entered an infinite loop at {0} when matching PATTERN against TEXT")]
    InfiniteLoop(String),
    #[error("Internal error: blocked at {0} when matching PATTERN against TEXT")]
    Blocked(String),
    #[error("Internal error: final state does not contain all output information")]
    IncompleteFinalState,
}
