//! Plugin manifest parsing and validation
//! 
//! This module handles loading and validating plugin.toml manifest files
//! that describe plugin metadata and required capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Plugin manifest loaded from plugin.toml
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginManifest {
    /// Plugin name (must be unique)
    pub name: String,
    /// Plugin version (semver format)
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Main WASM entry point file
    pub entry_point: String,
    /// Required capabilities
    pub capabilities: Vec<String>,
    /// Minimum required QuantaTerm version
    pub quantaterm_version: String,
    /// Optional metadata
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    /// Keywords for discovery
    pub keywords: Option<Vec<String>>,
    /// Plugin configuration schema
    pub config_schema: Option<HashMap<String, ConfigValue>>,
}

/// Configuration value types for plugin settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
}

/// Errors that can occur during manifest loading and validation
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("Failed to read manifest file: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Failed to parse manifest TOML: {0}")]
    TomlError(#[from] toml::de::Error),
    
    #[error("Invalid plugin name: {0}")]
    InvalidName(String),
    
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Unknown capability: {0}")]
    UnknownCapability(String),
    
    #[error("Incompatible QuantaTerm version: requires {required}, current {current}")]
    IncompatibleVersion { required: String, current: String },
    
    #[error("Invalid entry point: {0}")]
    InvalidEntryPoint(String),
}

/// Manifest loader and validator
pub struct ManifestLoader {
    current_quantaterm_version: String,
    allowed_capabilities: Vec<String>,
}

impl ManifestLoader {
    /// Create a new manifest loader
    pub fn new() -> Self {
        Self {
            current_quantaterm_version: env!("CARGO_PKG_VERSION").to_string(),
            allowed_capabilities: Self::default_capabilities(),
        }
    }
    
    /// Load and validate a manifest from a file
    pub fn load_manifest(&self, manifest_path: &Path) -> Result<PluginManifest, ManifestError> {
        // Read the manifest file
        let content = std::fs::read_to_string(manifest_path)?;
        
        // Parse TOML
        let manifest: PluginManifest = toml::from_str(&content)?;
        
        // Validate the manifest
        self.validate_manifest(&manifest)?;
        
        Ok(manifest)
    }
    
    /// Load a manifest from a string (for testing)
    pub fn load_manifest_from_string(&self, content: &str) -> Result<PluginManifest, ManifestError> {
        let manifest: PluginManifest = toml::from_str(content)?;
        self.validate_manifest(&manifest)?;
        Ok(manifest)
    }
    
    /// Validate a loaded manifest
    pub fn validate_manifest(&self, manifest: &PluginManifest) -> Result<(), ManifestError> {
        // Validate name
        if manifest.name.is_empty() {
            return Err(ManifestError::MissingField("name".to_string()));
        }
        
        if !Self::is_valid_plugin_name(&manifest.name) {
            return Err(ManifestError::InvalidName(manifest.name.clone()));
        }
        
        // Validate version
        if manifest.version.is_empty() {
            return Err(ManifestError::MissingField("version".to_string()));
        }
        
        if !Self::is_valid_version(&manifest.version) {
            return Err(ManifestError::InvalidVersion(manifest.version.clone()));
        }
        
        // Validate entry point
        if manifest.entry_point.is_empty() {
            return Err(ManifestError::MissingField("entry_point".to_string()));
        }
        
        if !manifest.entry_point.ends_with(".wasm") {
            return Err(ManifestError::InvalidEntryPoint(manifest.entry_point.clone()));
        }
        
        // Validate capabilities
        for capability in &manifest.capabilities {
            if !self.is_capability_allowed(capability) {
                return Err(ManifestError::UnknownCapability(capability.clone()));
            }
        }
        
        // Check QuantaTerm version compatibility
        if !Self::is_version_compatible(&manifest.quantaterm_version, &self.current_quantaterm_version) {
            return Err(ManifestError::IncompatibleVersion {
                required: manifest.quantaterm_version.clone(),
                current: self.current_quantaterm_version.clone(),
            });
        }
        
        Ok(())
    }
    
    /// Check if a plugin name is valid
    fn is_valid_plugin_name(name: &str) -> bool {
        // Plugin names should be alphanumeric with hyphens and underscores
        !name.is_empty() 
            && name.len() <= 64 
            && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && !name.starts_with('-')
            && !name.ends_with('-')
    }
    
    /// Check if a version string is valid (basic semver check)
    fn is_valid_version(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        parts.len() >= 2 
            && parts.len() <= 3 
            && parts.iter().all(|part| part.parse::<u32>().is_ok())
    }
    
    /// Check if version is compatible (simple >= check)
    fn is_version_compatible(required: &str, current: &str) -> bool {
        // For now, just check that we can parse both versions
        // In production, this would do proper semver comparison
        Self::is_valid_version(required) && Self::is_valid_version(current)
    }
    
    /// Check if a capability is in the allowed list
    fn is_capability_allowed(&self, capability: &str) -> bool {
        self.allowed_capabilities.iter().any(|allowed| {
            allowed == capability || capability.starts_with(&format!("{}:", allowed))
        })
    }
    
    /// Get the default list of allowed capabilities
    fn default_capabilities() -> Vec<String> {
        vec![
            "block.read".to_string(),
            "block.write".to_string(),
            "palette.add_action".to_string(),
            "config.read".to_string(),
            "config.write".to_string(),
            "ai.access".to_string(),
            "fs.read".to_string(),
            "fs.write".to_string(),
            "net.fetch".to_string(),
        ]
    }
}

impl Default for ManifestLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManifest {
    /// Create a minimal manifest for testing
    pub fn minimal(name: &str, entry_point: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: entry_point.to_string(),
            capabilities: vec![],
            quantaterm_version: "0.1.0".to_string(),
            author: None,
            license: None,
            homepage: None,
            repository: None,
            keywords: None,
            config_schema: None,
        }
    }
    
    /// Check if this plugin has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }
    
    /// Get the plugin's display name
    pub fn display_name(&self) -> String {
        format!("{} v{}", self.name, self.version)
    }
    
    /// Convert to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_manifest() -> PluginManifest {
        PluginManifest {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            entry_point: "plugin.wasm".to_string(),
            capabilities: vec!["block.read".to_string(), "fs.read:/tmp".to_string()],
            quantaterm_version: "0.1.0".to_string(),
            author: Some("Test Author".to_string()),
            license: Some("MIT".to_string()),
            homepage: None,
            repository: None,
            keywords: Some(vec!["test".to_string(), "example".to_string()]),
            config_schema: None,
        }
    }

    #[test]
    fn test_manifest_creation() {
        let manifest = create_test_manifest();
        assert_eq!(manifest.name, "test_plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.has_capability("block.read"));
        assert!(!manifest.has_capability("unknown"));
    }

    #[test]
    fn test_minimal_manifest() {
        let manifest = PluginManifest::minimal("test", "test.wasm");
        assert_eq!(manifest.name, "test");
        assert_eq!(manifest.entry_point, "test.wasm");
        assert!(manifest.capabilities.is_empty());
    }

    #[test]
    fn test_manifest_loader_creation() {
        let loader = ManifestLoader::new();
        assert!(!loader.current_quantaterm_version.is_empty());
        assert!(!loader.allowed_capabilities.is_empty());
    }

    #[test]
    fn test_manifest_validation_success() {
        let loader = ManifestLoader::new();
        let manifest = create_test_manifest();
        
        assert!(loader.validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn test_invalid_plugin_name() {
        let loader = ManifestLoader::new();
        let mut manifest = create_test_manifest();
        
        manifest.name = "".to_string();
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::MissingField(_))));
        
        manifest.name = "-invalid".to_string();
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::InvalidName(_))));
        
        manifest.name = "invalid-".to_string();
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::InvalidName(_))));
    }

    #[test]
    fn test_invalid_version() {
        let loader = ManifestLoader::new();
        let mut manifest = create_test_manifest();
        
        manifest.version = "".to_string();
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::MissingField(_))));
        
        manifest.version = "invalid".to_string();
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::InvalidVersion(_))));
        
        manifest.version = "1.2.3.4".to_string();
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::InvalidVersion(_))));
    }

    #[test]
    fn test_invalid_entry_point() {
        let loader = ManifestLoader::new();
        let mut manifest = create_test_manifest();
        
        manifest.entry_point = "".to_string();
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::MissingField(_))));
        
        manifest.entry_point = "plugin.exe".to_string();
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::InvalidEntryPoint(_))));
    }

    #[test]
    fn test_unknown_capability() {
        let loader = ManifestLoader::new();
        let mut manifest = create_test_manifest();
        
        manifest.capabilities = vec!["unknown.capability".to_string()];
        assert!(matches!(loader.validate_manifest(&manifest), Err(ManifestError::UnknownCapability(_))));
    }

    #[test]
    fn test_plugin_name_validation() {
        assert!(ManifestLoader::is_valid_plugin_name("valid_name"));
        assert!(ManifestLoader::is_valid_plugin_name("valid-name"));
        assert!(ManifestLoader::is_valid_plugin_name("ValidName123"));
        
        assert!(!ManifestLoader::is_valid_plugin_name(""));
        assert!(!ManifestLoader::is_valid_plugin_name("-invalid"));
        assert!(!ManifestLoader::is_valid_plugin_name("invalid-"));
        assert!(!ManifestLoader::is_valid_plugin_name("invalid name"));
    }

    #[test]
    fn test_version_validation() {
        assert!(ManifestLoader::is_valid_version("1.0"));
        assert!(ManifestLoader::is_valid_version("1.0.0"));
        assert!(ManifestLoader::is_valid_version("2.1.5"));
        
        assert!(!ManifestLoader::is_valid_version(""));
        assert!(!ManifestLoader::is_valid_version("1"));
        assert!(!ManifestLoader::is_valid_version("1.2.3.4"));
        assert!(!ManifestLoader::is_valid_version("v1.0.0"));
        assert!(!ManifestLoader::is_valid_version("1.a.0"));
    }

    #[test]
    fn test_manifest_toml_serialization() {
        let manifest = create_test_manifest();
        let toml_str = manifest.to_toml().unwrap();
        
        // Should be able to parse it back
        let loader = ManifestLoader::new();
        let parsed = loader.load_manifest_from_string(&toml_str).unwrap();
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn test_display_name() {
        let manifest = create_test_manifest();
        assert_eq!(manifest.display_name(), "test_plugin v1.0.0");
    }

    #[test]
    fn test_capability_patterns() {
        let loader = ManifestLoader::new();
        
        // Test that pattern-based capabilities are allowed
        assert!(loader.is_capability_allowed("fs.read:/tmp"));
        assert!(loader.is_capability_allowed("fs.write:/home/user"));
        assert!(loader.is_capability_allowed("net.fetch:example.com"));
        
        // Test that unknown base capabilities are not allowed
        assert!(!loader.is_capability_allowed("unknown.capability"));
        assert!(!loader.is_capability_allowed("unknown:pattern"));
    }
}