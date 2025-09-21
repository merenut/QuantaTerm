//! QuantaTerm Plugin host and runtime
//!
//! This crate provides the WASM-based plugin system for QuantaTerm, including:
//! - WASM runtime with Wasmtime
//! - Resource limits and security
//! - Capability-based permissions
//! - Plugin lifecycle management
//! - Action registry for command palette integration

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod actions;
pub mod host_context;
pub mod limits;
pub mod manifest;
pub mod runtime;

// Re-export the main types for convenience
pub use actions::{Action, ActionRegistry, ActionError};
pub use host_context::{HostContext, CapabilitySet, Capability, PathPattern, UrlPattern};
pub use limits::{ExecutionLimits, ResourceMonitor, LimitError};
pub use manifest::{PluginManifest, ManifestLoader, ManifestError};
pub use runtime::{WasmRuntime, RuntimeError, ActionResult, ActionContext, LoadedPlugin};

use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Main plugin host that manages all plugin operations
pub struct PluginsHost {
    runtime: WasmRuntime,
    action_registry: ActionRegistry,
    plugin_directories: Vec<PathBuf>,
}

impl PluginsHost {
    /// Create a new plugins host with default configuration
    pub fn new() -> Result<Self> {
        let runtime = WasmRuntime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create WASM runtime: {}", e))?;
        let action_registry = ActionRegistry::new();
        
        // Default plugin directories
        let mut plugin_directories = Vec::new();
        
        // Add user plugin directory
        if let Some(config_dir) = dirs::config_dir() {
            plugin_directories.push(config_dir.join("quantaterm").join("plugins"));
        }
        
        // Add system plugin directory (for development)
        plugin_directories.push(PathBuf::from("./plugins"));
        
        Ok(Self {
            runtime,
            action_registry,
            plugin_directories,
        })
    }
    
    /// Create a plugins host with custom configuration
    pub fn with_limits(limits: ExecutionLimits) -> Result<Self> {
        let runtime = WasmRuntime::with_limits(limits)
            .map_err(|e| anyhow::anyhow!("Failed to create WASM runtime: {}", e))?;
        let action_registry = ActionRegistry::new();
        let plugin_directories = vec![
            PathBuf::from("./plugins"),
        ];
        
        Ok(Self {
            runtime,
            action_registry,
            plugin_directories,
        })
    }
    
    /// Load a plugin from the given directory
    pub fn load_plugin(&mut self, plugin_dir: &Path) -> Result<String, RuntimeError> {
        info!("Loading plugin from: {}", plugin_dir.display());
        
        // Load the plugin into the runtime
        let plugin_id = self.runtime.load_plugin(plugin_dir)?;
        
        // Get the plugin's actions and register them
        if let Some(plugin) = self.runtime.instances.get(&plugin_id) {
            match plugin.get_actions() {
                Ok(actions) => {
                    for action in actions {
                        if let Err(e) = self.action_registry.register_action(action) {
                            warn!("Failed to register action from plugin {}: {}", plugin_id, e);
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed to get actions from plugin {}: {}", plugin_id, e);
                }
            }
        }
        
        info!("Successfully loaded plugin: {}", plugin_id);
        Ok(plugin_id)
    }
    
    /// Discover and load all plugins from configured directories
    pub fn discover_and_load_plugins(&mut self) -> Result<Vec<String>> {
        let mut loaded_plugins = Vec::new();
        
        for plugin_dir in &self.plugin_directories.clone() {
            if plugin_dir.exists() && plugin_dir.is_dir() {
                match self.discover_plugins_in_directory(plugin_dir) {
                    Ok(plugins) => {
                        for plugin_path in plugins {
                            match self.load_plugin(&plugin_path) {
                                Ok(plugin_id) => loaded_plugins.push(plugin_id),
                                Err(e) => warn!("Failed to load plugin from {}: {}", plugin_path.display(), e),
                            }
                        }
                    },
                    Err(e) => warn!("Failed to discover plugins in {}: {}", plugin_dir.display(), e),
                }
            }
        }
        
        info!("Loaded {} plugins", loaded_plugins.len());
        Ok(loaded_plugins)
    }
    
    /// Discover plugin directories in the given path
    fn discover_plugins_in_directory(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut plugins = Vec::new();
        
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        // Check if this directory contains a plugin.toml
                        let manifest_path = path.join("plugin.toml");
                        if manifest_path.exists() {
                            plugins.push(path);
                        }
                    }
                }
            }
        }
        
        Ok(plugins)
    }
    
    /// Execute a plugin action
    pub fn execute_action(&mut self, action_id: &str, args: &[serde_json::Value]) -> Result<ActionResult, RuntimeError> {
        // Find the action to get the plugin ID
        let action = self.action_registry.get_action(action_id)
            .ok_or_else(|| RuntimeError::ExecutionError(format!("Action not found: {}", action_id)))?;
        
        let plugin_id = action.plugin_id.clone();
        
        // Create action context
        let context = ActionContext {
            action_id: action_id.to_string(),
            args: args.to_vec(),
            user_data: None,
        };
        
        // Execute in the runtime
        self.runtime.execute_action(&plugin_id, &context)
    }
    
    /// Unload a plugin and clean up its resources
    pub fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), RuntimeError> {
        // Unregister all actions from this plugin
        if let Err(e) = self.action_registry.unregister_plugin_actions(plugin_id) {
            warn!("Failed to unregister actions for plugin {}: {}", plugin_id, e);
        }
        
        // Unload from runtime
        self.runtime.unload_plugin(plugin_id)?;
        
        info!("Unloaded plugin: {}", plugin_id);
        Ok(())
    }
    
    /// Get all available actions
    pub fn get_actions(&self) -> Vec<Action> {
        self.action_registry.list_actions()
    }
    
    /// Search actions by query
    pub fn search_actions(&self, query: &str) -> Vec<Action> {
        self.action_registry.search_actions(query)
    }
    
    /// Get actions for a specific plugin
    pub fn get_plugin_actions(&self, plugin_id: &str) -> Result<Vec<Action>, RuntimeError> {
        let actions = self.action_registry.get_plugin_actions(plugin_id);
        Ok(actions)
    }
    
    /// Get list of loaded plugins
    pub fn loaded_plugins(&self) -> Vec<String> {
        self.runtime.loaded_plugins()
    }
    
    /// Get plugin manifest by ID
    pub fn get_plugin_manifest(&self, plugin_id: &str) -> Option<&PluginManifest> {
        self.runtime.get_plugin_manifest(plugin_id)
    }
    
    /// Add a plugin directory to search
    pub fn add_plugin_directory(&mut self, dir: PathBuf) {
        if !self.plugin_directories.contains(&dir) {
            self.plugin_directories.push(dir);
        }
    }
    
    /// Get statistics about loaded plugins and actions
    pub fn get_statistics(&self) -> PluginStatistics {
        PluginStatistics {
            loaded_plugins: self.runtime.loaded_plugins().len(),
            registered_actions: self.action_registry.action_count(),
            plugin_directories: self.plugin_directories.len(),
        }
    }
}

impl Default for PluginsHost {
    fn default() -> Self {
        Self::new().expect("Failed to create PluginsHost")
    }
}

/// Statistics about the plugin system
#[derive(Debug, Clone)]
pub struct PluginStatistics {
    /// Number of loaded plugins
    pub loaded_plugins: usize,
    /// Number of registered actions
    pub registered_actions: usize,
    /// Number of configured plugin directories
    pub plugin_directories: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_plugins_host_creation() {
        let host = PluginsHost::new().unwrap();
        assert!(host.loaded_plugins().is_empty());
        assert!(host.get_actions().is_empty());
    }

    #[test]
    fn test_plugins_host_with_limits() {
        let limits = ExecutionLimits::development();
        let host = PluginsHost::with_limits(limits).unwrap();
        assert!(host.loaded_plugins().is_empty());
    }

    #[test]
    fn test_plugin_statistics() {
        let host = PluginsHost::new().unwrap();
        let stats = host.get_statistics();
        
        assert_eq!(stats.loaded_plugins, 0);
        assert_eq!(stats.registered_actions, 0);
        assert!(stats.plugin_directories > 0);
    }

    #[test]
    fn test_add_plugin_directory() {
        let mut host = PluginsHost::new().unwrap();
        let initial_count = host.plugin_directories.len();
        
        let new_dir = PathBuf::from("/test/plugins");
        host.add_plugin_directory(new_dir.clone());
        
        assert_eq!(host.plugin_directories.len(), initial_count + 1);
        assert!(host.plugin_directories.contains(&new_dir));
        
        // Adding the same directory again should not duplicate it
        host.add_plugin_directory(new_dir);
        assert_eq!(host.plugin_directories.len(), initial_count + 1);
    }

    #[test]
    fn test_discover_empty_directory() {
        let host = PluginsHost::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        
        let plugins = host.discover_plugins_in_directory(temp_dir.path()).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_unload_nonexistent_plugin() {
        let mut host = PluginsHost::new().unwrap();
        let result = host.unload_plugin("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_plugin_actions_empty() {
        let host = PluginsHost::new().unwrap();
        let actions = host.get_plugin_actions("nonexistent").unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_search_actions_empty() {
        let host = PluginsHost::new().unwrap();
        let results = host.search_actions("test");
        assert!(results.is_empty());
    }

    #[test]
    fn test_execute_nonexistent_action() {
        let mut host = PluginsHost::new().unwrap();
        let result = host.execute_action("nonexistent.action", &[]);
        assert!(result.is_err());
    }
}
