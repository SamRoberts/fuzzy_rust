//! Provides the [`enum@Error`] type currently used by all fuzzy code.

use thiserror::Error;
use std::fmt::Debug;
use std::io;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not read file: {0}")]
    CouldNotReadFile(#[from] io::Error),
    #[error("PATTERN not a valid regex: {0}")]
    PatternNotRegex(#[from] regex_syntax::Error),
    #[error("PATTERN has unsupported regex: {0}")]
    PatternUnsupported(String),
    #[error("PATTERN sets a regex bound that is too large for this architecture")]
    RegexBoundTooLarge,
    #[error("Gave up matching PATTERN against TEXT after {0} steps")]
    ExceededMaxSteps(usize),
    #[error("Internal error: node {0} is neiher working nor done after being processed")]
    NoNodeProgress(String),
    #[error("Internal error: could not find NodeType for non-end Ix {0}")]
    NoNodeType(String),
    #[error("Internal error: can only initialise node {0} once")]
    CannotInitialiseNode(String),
    #[error("Internal error: can only update node {0} if it is initialised and not done")]
    CannotUpdateNode(String),
    #[error("Internal error: cannot retrieve node field(s) '{0}' unless node is {1}")]
    CannotGetNodeField(&'static str, &'static str),
    #[error("Internal error: final state does not contain all output information")]
    IncompleteFinalState,
}
