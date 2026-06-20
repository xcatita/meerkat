//! Error types for the Meerkat library

use std::fmt;
use std::result;

/// Result type alias using our library-specific Error type
pub type Result<T> = result::Result<T, Error>;

/// Represents errors that can occur within the Meerkat library
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A generic error message
    Message(String),
    /// Error indicating a zero-trust boundary limit was exceeded
    LimitExceeded(String),
}

impl fmt::Display for Error {
    /// Format the error for user display
    ///
    /// Args:
    ///     `f` (`&mut fmt::Formatter<'_>`): The formatter target
    ///
    /// Returns:
    ///     `fmt::Result`: The result of the formatting operation
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Message(msg) => write!(f, "{}", msg),
            Error::LimitExceeded(msg) => {
                write!(f, "Limit exceeded: {}", msg)
            }
        }
    }
}

impl std::error::Error for Error {}
