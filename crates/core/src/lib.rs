//! QuantaTerm Core
//!
//! Core functionality and types for the QuantaTerm terminal emulator.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod error;
pub mod logging;

pub use error::QuantaTermError;

/// Core result type for QuantaTerm operations
pub type Result<T> = std::result::Result<T, QuantaTermError>;

/// Version information for QuantaTerm
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// CSI sequence actions for terminal control
#[derive(Debug, Clone)]
pub enum CsiAction {
    /// SGR (Select Graphic Rendition) - formatting attributes
    Sgr(Vec<u16>),
    /// Cursor movement and positioning commands
    CursorUp(u16),
    /// Cursor down
    CursorDown(u16),
    /// Cursor forward (right)
    CursorForward(u16),
    /// Cursor backward (left)
    CursorBackward(u16),
    /// Cursor next line
    CursorNextLine(u16),
    /// Cursor previous line
    CursorPreviousLine(u16),
    /// Cursor horizontal absolute positioning
    CursorHorizontalAbsolute(u16),
    /// Cursor position (row, col)
    CursorPosition(u16, u16),
    /// Other CSI commands
    Other {
        /// The final byte of the CSI sequence
        command: char,
        /// Parameters for the command
        params: Vec<u16>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert_eq!(VERSION, "0.1.0");
    }
}
