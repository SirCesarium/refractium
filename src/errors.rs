use std::{io, result};
use thiserror::Error;

/// Main error type for the Refractium library.
///
/// This enum covers all possible error conditions that can occur during
/// configuration, initialization, and runtime execution of the proxy.
#[derive(Error, Debug)]
pub enum RefractiumError {
    /// Returned when the server fails to bind to the requested network address.
    ///
    /// This usually happens if the port is already in use or the process lacks
    /// sufficient permissions.
    #[error("Failed to bind to {0}: {1}")]
    BindError(String, io::Error),

    /// Returned when there is an issue with the configuration data.
    ///
    /// This includes invalid TOML, missing required fields, or logical errors
    /// in the routing table (e.g., duplicate protocol names).
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// A wrapper for standard [`std::io::Error`]s encountered during runtime.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Returned when a backend hostname or address cannot be resolved.
    #[error("Address resolution failed: {0}")]
    AddrResolution(String),

    /// A generic error variant for custom error messages.
    #[error("Generic error: {0}")]
    Generic(String),

    /// An unexpected internal error. If you encounter this, it may be a bug.
    #[error("Unknown error occurred")]
    Unknown,
}

/// Convenience alias for `Result<T, RefractiumError>`.
pub type Result<T> = result::Result<T, RefractiumError>;
