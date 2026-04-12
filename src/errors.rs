use std::{io, result};
use thiserror::Error;

/// Main error type for the Refractium library.
#[derive(Error, Debug)]
pub enum RefractiumError {
    /// Error returned when binding to a socket fails.
    #[error("Failed to bind to {0}: {1}")]
    BindError(String, io::Error),

    /// Error returned when configuration loading or parsing fails.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Wrapper for standard IO errors.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Error returned when an address cannot be resolved.
    #[error("Address resolution failed: {0}")]
    AddrResolution(String),

    /// Generic catch-all error with a custom message.
    #[error("Generic error: {0}")]
    Generic(String),

    /// Error returned when an unknown or unexpected internal error occurs.
    #[error("Unknown error occurred")]
    Unknown,
}

/// Convenience alias for `std::result::Result<T, RefractiumError>`.
pub type Result<T> = result::Result<T, RefractiumError>;
