//! Host context for WASM plugin execution
//! 
//! This module provides the execution context that plugins run within,
//! including their capabilities and resource constraints.

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::limits::ExecutionLimits;

/// Host context passed to each plugin instance
#[derive(Debug, Clone)]
pub struct HostContext {
    /// Plugin's granted capabilities
    pub capabilities: CapabilitySet,
    /// Memory limit for this plugin instance
    pub memory_limit: u64,
    /// Time limit for individual function calls
    pub time_limit: Duration,
    /// Plugin metadata
    pub plugin_metadata: PluginMetadata,
    /// Environment variables accessible to the plugin
    pub environment: HashMap<String, String>,
}

/// Set of capabilities granted to a plugin
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySet {
    capabilities: std::collections::HashSet<Capability>,
    plugin_id: String,
}

/// Individual capability types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Read access to specific file system paths
    FileSystemRead(PathPattern),
    /// Write access to specific file system paths
    FileSystemWrite(PathPattern),
    /// Network access to specific URLs or patterns
    NetworkFetch(UrlPattern),
    /// Access to read terminal block data
    BlockRead,
    /// Access to write terminal block data
    BlockWrite,
    /// Access to register actions in the command palette
    PaletteAddAction,
    /// Access to read configuration settings
    ConfigRead,
    /// Access to write configuration settings
    ConfigWrite,
    /// Access to AI provider services
    AiAccess,
}

/// File system path pattern for capability matching
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathPattern {
    /// Base path (e.g., "/tmp", "/home/user/documents")
    pub base: String,
    /// Whether to allow recursive access to subdirectories
    pub recursive: bool,
    /// Allowed file extensions (empty = all)
    pub extensions: Vec<String>,
}

/// URL pattern for network capability matching
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UrlPattern {
    /// Scheme (http, https, etc.)
    pub scheme: Option<String>,
    /// Host pattern (exact match or wildcard)
    pub host: String,
    /// Port restriction
    pub port: Option<u16>,
    /// Path prefix
    pub path_prefix: Option<String>,
}

/// Plugin metadata stored in the host context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
}

impl HostContext {
    /// Create a new host context for a plugin
    pub fn new(
        plugin_id: String,
        capabilities: CapabilitySet,
        limits: &ExecutionLimits,
        metadata: PluginMetadata,
    ) -> Self {
        let mut environment = HashMap::new();
        
        // Add safe environment variables
        environment.insert("PLUGIN_ID".to_string(), plugin_id.clone());
        environment.insert("PLUGIN_NAME".to_string(), metadata.name.clone());
        environment.insert("PLUGIN_VERSION".to_string(), metadata.version.clone());
        
        Self {
            capabilities,
            memory_limit: limits.max_memory,
            time_limit: limits.max_time,
            plugin_metadata: metadata,
            environment,
        }
    }
    
    /// Check if the plugin has a specific capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.has_capability(capability)
    }
    
    /// Add an environment variable
    pub fn add_environment_variable(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }
    
    /// Get environment variables as a slice for WASI
    pub fn environment_variables(&self) -> Vec<(String, String)> {
        self.environment.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl CapabilitySet {
    /// Create a new empty capability set
    pub fn new(plugin_id: String) -> Self {
        Self {
            capabilities: std::collections::HashSet::new(),
            plugin_id,
        }
    }
    
    /// Create a capability set from a plugin manifest
    pub fn from_manifest(manifest: &crate::manifest::PluginManifest) -> Self {
        let mut capabilities = std::collections::HashSet::new();
        
        for cap_str in &manifest.capabilities {
            if let Some(capability) = Self::parse_capability_string(cap_str) {
                capabilities.insert(capability);
            } else {
                tracing::warn!("Unknown capability in manifest: {}", cap_str);
            }
        }
        
        Self {
            capabilities,
            plugin_id: manifest.name.clone(),
        }
    }
    
    /// Parse a capability string from manifest
    fn parse_capability_string(cap_str: &str) -> Option<Capability> {
        match cap_str {
            "block.read" => Some(Capability::BlockRead),
            "block.write" => Some(Capability::BlockWrite),
            "palette.add_action" => Some(Capability::PaletteAddAction),
            "config.read" => Some(Capability::ConfigRead),
            "config.write" => Some(Capability::ConfigWrite),
            "ai.access" => Some(Capability::AiAccess),
            _ if cap_str.starts_with("fs.read:") => {
                let path = cap_str.strip_prefix("fs.read:")?;
                Some(Capability::FileSystemRead(PathPattern {
                    base: path.to_string(),
                    recursive: true,
                    extensions: vec![],
                }))
            },
            _ if cap_str.starts_with("fs.write:") => {
                let path = cap_str.strip_prefix("fs.write:")?;
                Some(Capability::FileSystemWrite(PathPattern {
                    base: path.to_string(),
                    recursive: true,
                    extensions: vec![],
                }))
            },
            _ if cap_str.starts_with("net.fetch:") => {
                let url = cap_str.strip_prefix("net.fetch:")?;
                Some(Capability::NetworkFetch(UrlPattern {
                    scheme: None,
                    host: url.to_string(),
                    port: None,
                    path_prefix: None,
                }))
            },
            _ => None,
        }
    }
    
    /// Check if this set contains a specific capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }
    
    /// Add a capability to the set
    pub fn add_capability(&mut self, capability: Capability) {
        self.capabilities.insert(capability);
    }
    
    /// Remove a capability from the set
    pub fn remove_capability(&mut self, capability: &Capability) {
        self.capabilities.remove(capability);
    }
    
    /// Get all capabilities as a vector
    pub fn capabilities(&self) -> Vec<Capability> {
        self.capabilities.iter().cloned().collect()
    }
    
    /// Check if plugin can access a specific file path
    pub fn check_file_access(&self, path: &std::path::Path, write: bool) -> Result<(), String> {
        let path_str = path.to_string_lossy();
        
        for capability in &self.capabilities {
            match capability {
                Capability::FileSystemRead(pattern) if !write => {
                    if Self::path_matches_pattern(&path_str, pattern) {
                        return Ok(());
                    }
                },
                Capability::FileSystemWrite(pattern) if write => {
                    if Self::path_matches_pattern(&path_str, pattern) {
                        return Ok(());
                    }
                },
                _ => continue,
            }
        }
        
        Err(format!(
            "Plugin {} does not have {} access to path: {}",
            self.plugin_id,
            if write { "write" } else { "read" },
            path_str
        ))
    }
    
    /// Check if a path matches a pattern
    fn path_matches_pattern(path: &str, pattern: &PathPattern) -> bool {
        if pattern.recursive {
            path.starts_with(&pattern.base)
        } else {
            // Check if it's in the exact directory
            let path_parent = std::path::Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy())
                .unwrap_or_default();
            path_parent == pattern.base
        }
    }
    
    /// Get the plugin ID this capability set belongs to
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}

impl PathPattern {
    /// Create a new path pattern
    pub fn new(base: String, recursive: bool, extensions: Vec<String>) -> Self {
        Self {
            base,
            recursive,
            extensions,
        }
    }
    
    /// Check if a file extension is allowed
    pub fn allows_extension(&self, extension: &str) -> bool {
        self.extensions.is_empty() || self.extensions.contains(&extension.to_string())
    }
}

impl UrlPattern {
    /// Create a new URL pattern
    pub fn new(host: String) -> Self {
        Self {
            scheme: None,
            host,
            port: None,
            path_prefix: None,
        }
    }
    
    /// Check if a URL matches this pattern
    pub fn matches(&self, url: &str) -> bool {
        // Simple implementation - in production this would be more sophisticated
        url.contains(&self.host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::ExecutionLimits;

    fn create_test_metadata() -> PluginMetadata {
        PluginMetadata {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: Some("Test Author".to_string()),
            license: Some("MIT".to_string()),
            homepage: None,
        }
    }

    #[test]
    fn test_host_context_creation() {
        let capabilities = CapabilitySet::new("test".to_string());
        let limits = ExecutionLimits::default();
        let metadata = create_test_metadata();
        
        let context = HostContext::new("test".to_string(), capabilities, &limits, metadata);
        
        assert_eq!(context.memory_limit, limits.max_memory);
        assert_eq!(context.time_limit, limits.max_time);
        assert_eq!(context.plugin_metadata.name, "test_plugin");
    }

    #[test]
    fn test_capability_set_creation() {
        let caps = CapabilitySet::new("test".to_string());
        assert_eq!(caps.plugin_id(), "test");
        assert!(caps.capabilities().is_empty());
    }

    #[test]
    fn test_capability_parsing() {
        let capability = CapabilitySet::parse_capability_string("block.read");
        assert!(matches!(capability, Some(Capability::BlockRead)));
        
        let capability = CapabilitySet::parse_capability_string("fs.read:/tmp");
        assert!(matches!(capability, Some(Capability::FileSystemRead(_))));
        
        let capability = CapabilitySet::parse_capability_string("unknown");
        assert!(capability.is_none());
    }

    #[test]
    fn test_file_access_check() {
        let mut caps = CapabilitySet::new("test".to_string());
        caps.add_capability(Capability::FileSystemRead(PathPattern {
            base: "/tmp".to_string(),
            recursive: true,
            extensions: vec![],
        }));
        
        // Should allow access to /tmp files
        let tmp_file = std::path::Path::new("/tmp/test.txt");
        assert!(caps.check_file_access(tmp_file, false).is_ok());
        
        // Should deny access to other directories
        let home_file = std::path::Path::new("/home/user/test.txt");
        assert!(caps.check_file_access(home_file, false).is_err());
        
        // Should deny write access (only read granted)
        assert!(caps.check_file_access(tmp_file, true).is_err());
    }

    #[test]
    fn test_path_pattern_matching() {
        let pattern = PathPattern {
            base: "/tmp".to_string(),
            recursive: true,
            extensions: vec!["txt".to_string()],
        };
        
        assert!(CapabilitySet::path_matches_pattern("/tmp/test.txt", &pattern));
        assert!(CapabilitySet::path_matches_pattern("/tmp/subdir/test.txt", &pattern));
        assert!(!CapabilitySet::path_matches_pattern("/home/test.txt", &pattern));
        
        assert!(pattern.allows_extension("txt"));
        assert!(!pattern.allows_extension("exe"));
    }

    #[test]
    fn test_environment_variables() {
        let capabilities = CapabilitySet::new("test".to_string());
        let limits = ExecutionLimits::default();
        let metadata = create_test_metadata();
        
        let mut context = HostContext::new("test".to_string(), capabilities, &limits, metadata);
        context.add_environment_variable("TEST_VAR".to_string(), "test_value".to_string());
        
        let env_vars = context.environment_variables();
        assert!(env_vars.iter().any(|(k, v)| k == "TEST_VAR" && v == "test_value"));
        assert!(env_vars.iter().any(|(k, v)| k == "PLUGIN_ID" && v == "test"));
    }
}