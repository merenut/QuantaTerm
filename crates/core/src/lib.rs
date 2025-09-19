//! QuantaTerm Core
//!
//! Core functionality and types for the QuantaTerm terminal emulator.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod error;

pub use error::QuantaTermError;

/// Core result type for QuantaTerm operations
pub type Result<T> = std::result::Result<T, QuantaTermError>;

/// Version information for QuantaTerm
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert_eq!(VERSION, "0.1.0");
    }
}
