use std::io;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PrismaError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Identification timeout after {0:?}")]
    Timeout(Duration),

    #[error("Protocol identification failed")]
    IdentificationFailed,

    #[error("Core engine failure: {0}")]
    CoreError(String),
}
