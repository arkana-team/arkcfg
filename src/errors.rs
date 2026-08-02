use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArkcfgError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("No displays found. Is a display server running?")]
    NoDisplaysFound,

    #[error("Invalid resolution format '{0}'. Expected WIDTHxHEIGHT (e.g. 1920x1080)")]
    InvalidResolution(String),

    #[error("Resolution '{0}' is not supported by the current display")]
    UnsupportedResolution(String),

    #[error("Display '{0}' not found")]
    DisplayNotFound(String),
}

pub type Result<T> = std::result::Result<T, ArkcfgError>;
