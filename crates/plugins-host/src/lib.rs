//! QuantaTerm Plugin host and runtime
//!
//! Plugin host and runtime.

#![warn(missing_docs)]
#![deny(unsafe_code)]

/// Placeholder module for plugins-host
pub struct PluginsHost;

impl PluginsHost {
    /// Create a new instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for PluginsHost {
    fn default() -> Self {
        Self::new()
    }
}
