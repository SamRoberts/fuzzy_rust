use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("PATTERN not a valid regex: {0}")]
    PatternNotRegex(#[from] regex_syntax::Error),
    #[error("PATTERN has unsupported regex: {0}")]
    PatternUnsupported(String),
    #[error("Internal error: entered an infinite loop at {0} when matching PATTERN against TEXT")]
    InfiniteLoop(String),
    #[error("Internal error: blocked at {0} when matching PATTERN against TEXT")]
    Blocked(String),
    #[error("Internal error: could not interpret regex representation: {0}")]
    UnexpectedRegexRepr(String),
    #[error("Internal error: final state does not contain all output information")]
    IncompleteFinalState,
}
