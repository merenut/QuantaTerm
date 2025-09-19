//! Error types for QuantaTerm

use thiserror::Error;

/// Main error type for QuantaTerm operations
#[derive(Error, Debug)]
pub enum QuantaTermError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Rendering error
    #[error("Rendering error: {0}")]
    Render(String),

    /// PTY error
    #[error("PTY error: {0}")]
    Pty(String),

    /// Plugin error
    #[error("Plugin error: {0}")]
    Plugin(String),

    /// Generic error
    #[error("Error: {0}")]
    Generic(String),
}
