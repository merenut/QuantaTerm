//! QuantaTerm Plugin API definitions and traits
//!
//! Plugin API definitions and traits.

#![warn(missing_docs)]
#![deny(unsafe_code)]

/// Placeholder module for plugins-api
pub struct PluginsApi;

impl PluginsApi {
    /// Create a new instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for PluginsApi {
    fn default() -> Self {
        Self::new()
    }
}
