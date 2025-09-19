//! QuantaTerm Telemetry and metrics collection
//!
//! Telemetry and metrics collection.

#![warn(missing_docs)]
#![deny(unsafe_code)]

/// Placeholder module for telemetry
pub struct Telemetry;

impl Telemetry {
    /// Create a new instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}
