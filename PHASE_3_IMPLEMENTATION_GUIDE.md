# Phase 3 Implementation Guide for AI Coding Agents

## Quick Start Guide

This document provides AI coding agents with specific implementation examples, code templates, and testing patterns for Phase 3 tasks focusing on Plugins & AI integration.

## Code Templates and Examples

### Task 1: WASM Runtime Implementation

#### 1.1 Basic WASM Runtime Setup

```rust
// File: crates/plugins-host/src/lib.rs
//! QuantaTerm Plugin host and runtime
//!
//! Plugin host and runtime for WASM-based extensions.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod runtime;
pub mod loader;
pub mod capabilities;
pub mod actions;
pub mod host_context;
pub mod limits;

pub use runtime::{WasmRuntime, RuntimeError};
pub use loader::{PluginLoader, LoadedPlugin, PluginManifest};
pub use capabilities::{Capability, CapabilitySet, PermissionChecker};
pub use actions::{Action, ActionRegistry, ActionResult};

use anyhow::Result;
use std::collections::HashMap;

/// Main plugin host coordinator
pub struct PluginsHost {
    runtime: WasmRuntime,
    loader: PluginLoader,
    actions: ActionRegistry,
    loaded_plugins: HashMap<String, LoadedPlugin>,
}

impl PluginsHost {
    /// Create a new plugin host
    pub fn new() -> Result<Self> {
        let runtime = WasmRuntime::new()?;
        let loader = PluginLoader::new()?;
        let actions = ActionRegistry::new();
        
        Ok(Self {
            runtime,
            loader,
            actions,
            loaded_plugins: HashMap::new(),
        })
    }
    
    /// Load a plugin from the filesystem
    pub fn load_plugin(&mut self, path: &std::path::Path) -> Result<String, RuntimeError> {
        let manifest = self.loader.load_manifest(path)?;
        let plugin = self.runtime.load_plugin_module(path, &manifest)?;
        let plugin_id = manifest.name.clone();
        
        // Register plugin actions
        let actions = plugin.get_actions()?;
        for action in actions {
            self.actions.register_action(action)?;
        }
        
        self.loaded_plugins.insert(plugin_id.clone(), plugin);
        Ok(plugin_id)
    }
    
    /// Execute a plugin action
    pub fn execute_action(&self, action_id: &str, args: &[serde_json::Value]) -> Result<ActionResult, RuntimeError> {
        self.actions.execute_action(action_id, args)
    }
    
    /// Unload a plugin and clean up resources
    pub fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), RuntimeError> {
        if let Some(plugin) = self.loaded_plugins.remove(plugin_id) {
            // Unregister plugin actions
            let actions = plugin.get_actions()?;
            for action in actions {
                self.actions.unregister_action(&action.id)?;
            }
            
            // Clean up WASM instance
            self.runtime.unload_plugin(plugin_id)?;
        }
        Ok(())
    }
}

impl Default for PluginsHost {
    fn default() -> Self {
        Self::new().expect("Failed to create PluginsHost")
    }
}
```

#### 1.2 WASM Runtime with Wasmtime

```rust
// File: crates/plugins-host/src/runtime.rs
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use wasmtime::*;

use crate::host_context::HostContext;
use crate::limits::ExecutionLimits;
use crate::capabilities::CapabilitySet;

/// WASM runtime error types
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Plugin loading failed: {0}")]
    LoadError(#[from] anyhow::Error),
    #[error("Execution timeout after {0:?}")]
    Timeout(Duration),
    #[error("Memory limit exceeded: {used} > {limit}")]
    MemoryLimit { used: u64, limit: u64 },
    #[error("Permission denied: {0}")]
    Permission(String),
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
}

/// Main WASM runtime using Wasmtime
pub struct WasmRuntime {
    engine: Engine,
    linker: Linker<HostContext>,
    instances: HashMap<String, Instance>,
    limits: ExecutionLimits,
}

impl WasmRuntime {
    /// Create a new WASM runtime with default configuration
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(false);
        config.async_support(false);
        config.consume_fuel(true);
        
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        
        // Add WASI support with restricted capabilities
        wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut HostContext| &mut ctx.wasi)?;
        
        // Add custom host functions
        Self::add_host_functions(&mut linker)?;
        
        let limits = ExecutionLimits::default();
        
        Ok(Self {
            engine,
            linker,
            instances: HashMap::new(),
            limits,
        })
    }
    
    /// Load a plugin module from file
    pub fn load_plugin_module(&mut self, path: &Path, manifest: &crate::PluginManifest) -> Result<LoadedPlugin, RuntimeError> {
        let module_bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read plugin file: {}", path.display()))?;
            
        let module = Module::new(&self.engine, module_bytes)
            .with_context(|| "Failed to compile WASM module")?;
            
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdio()
            .build();
            
        let capabilities = CapabilitySet::from_manifest(manifest);
        let context = HostContext::new(wasi, capabilities, self.limits.clone());
        let mut store = Store::new(&self.engine, context);
        
        // Set fuel limit for execution time control
        store.add_fuel(self.limits.max_fuel)?;
        
        let instance = self.linker.instantiate(&mut store, &module)
            .with_context(|| "Failed to instantiate plugin")?;
            
        let plugin = LoadedPlugin::new(instance, store, manifest.clone());
        Ok(plugin)
    }
    
    /// Add custom host functions that plugins can call
    fn add_host_functions(linker: &mut Linker<HostContext>) -> Result<()> {
        // Add terminal interaction functions
        linker.func_wrap("quantaterm", "get_current_command", |caller: Caller<'_, HostContext>| -> String {
            // Implementation to get current terminal command
            caller.data().get_current_command().unwrap_or_default()
        })?;
        
        linker.func_wrap("quantaterm", "add_palette_action", 
            |mut caller: Caller<'_, HostContext>, name_ptr: i32, name_len: i32, desc_ptr: i32, desc_len: i32| -> i32 {
                // Implementation to register palette action
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let name = Self::read_string_from_memory(&memory, &caller, name_ptr, name_len)?;
                let desc = Self::read_string_from_memory(&memory, &caller, desc_ptr, desc_len)?;
                
                caller.data_mut().register_palette_action(&name, &desc)
                    .map(|_| 0)
                    .unwrap_or(-1)
            }
        )?;
        
        linker.func_wrap("quantaterm", "log_message",
            |mut caller: Caller<'_, HostContext>, level: i32, msg_ptr: i32, msg_len: i32| {
                let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
                let message = Self::read_string_from_memory(&memory, &caller, msg_ptr, msg_len).unwrap();
                
                match level {
                    0 => tracing::trace!("[Plugin] {}", message),
                    1 => tracing::debug!("[Plugin] {}", message),
                    2 => tracing::info!("[Plugin] {}", message),
                    3 => tracing::warn!("[Plugin] {}", message),
                    4 => tracing::error!("[Plugin] {}", message),
                    _ => tracing::info!("[Plugin] {}", message),
                }
            }
        )?;
        
        Ok(())
    }
    
    /// Helper to read string from WASM memory
    fn read_string_from_memory(memory: &Memory, caller: &Caller<HostContext>, ptr: i32, len: i32) -> Result<String> {
        let data = memory.data(caller);
        let start = ptr as usize;
        let end = start + len as usize;
        
        if end > data.len() {
            return Err(anyhow::anyhow!("String read out of bounds"));
        }
        
        String::from_utf8(data[start..end].to_vec())
            .with_context(|| "Invalid UTF-8 in plugin string")
    }
    
    /// Execute plugin function with timeout
    pub fn execute_with_timeout<T>(&mut self, plugin_id: &str, func_name: &str, args: &[wasmtime::Val]) -> Result<T, RuntimeError> 
    where
        T: for<'a> TryFrom<&'a [wasmtime::Val], Error = RuntimeError>,
    {
        let start_time = Instant::now();
        
        // Implementation would execute the function and monitor timeout
        // This is a simplified version
        if start_time.elapsed() > self.limits.max_time {
            return Err(RuntimeError::Timeout(self.limits.max_time));
        }
        
        // Execute function logic here...
        todo!("Implement function execution with proper timeout handling")
    }
    
    /// Unload a plugin instance
    pub fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), RuntimeError> {
        self.instances.remove(plugin_id);
        Ok(())
    }
}

/// Represents a loaded plugin instance
pub struct LoadedPlugin {
    instance: Instance,
    store: Store<HostContext>,
    manifest: crate::PluginManifest,
}

impl LoadedPlugin {
    fn new(instance: Instance, store: Store<HostContext>, manifest: crate::PluginManifest) -> Self {
        Self { instance, store, manifest }
    }
    
    /// Get actions this plugin provides
    pub fn get_actions(&self) -> Result<Vec<crate::Action>, RuntimeError> {
        // Call plugin's get_actions function
        // This would invoke the WASM function and parse results
        Ok(vec![]) // Placeholder
    }
    
    /// Execute a specific action
    pub fn execute_action(&mut self, action_id: &str, args: &[serde_json::Value]) -> Result<crate::ActionResult, RuntimeError> {
        // Execute plugin action function
        // This would serialize args, call WASM, and deserialize result
        Ok(crate::ActionResult::success("Action executed".to_string()))
    }
}
```

#### 1.3 Execution Limits and Resource Management

```rust
// File: crates/plugins-host/src/limits.rs
use std::time::Duration;

/// Resource limits for plugin execution
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    /// Maximum memory allocation (bytes)
    pub max_memory: u64,
    /// Maximum execution time per call
    pub max_time: Duration,
    /// Maximum fuel units (computational complexity)
    pub max_fuel: u64,
    /// Maximum file descriptor count
    pub max_file_handles: u32,
    /// Maximum network connections
    pub max_network_connections: u32,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_memory: 16 * 1024 * 1024,           // 16MB
            max_time: Duration::from_millis(100),    // 100ms
            max_fuel: 1_000_000,                     // 1M instructions
            max_file_handles: 10,
            max_network_connections: 5,
        }
    }
}

impl ExecutionLimits {
    /// Create limits for development/testing (more generous)
    pub fn development() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024,           // 64MB
            max_time: Duration::from_millis(1000),   // 1s
            max_fuel: 10_000_000,                    // 10M instructions
            max_file_handles: 50,
            max_network_connections: 20,
        }
    }
    
    /// Create strict limits for production
    pub fn production() -> Self {
        Self {
            max_memory: 8 * 1024 * 1024,            // 8MB
            max_time: Duration::from_millis(50),     // 50ms
            max_fuel: 500_000,                       // 500K instructions
            max_file_handles: 5,
            max_network_connections: 2,
        }
    }
}

/// Monitor resource usage during execution
pub struct ResourceMonitor {
    start_time: std::time::Instant,
    limits: ExecutionLimits,
    memory_usage: u64,
    file_handles: u32,
    network_connections: u32,
}

impl ResourceMonitor {
    pub fn new(limits: ExecutionLimits) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            limits,
            memory_usage: 0,
            file_handles: 0,
            network_connections: 0,
        }
    }
    
    /// Check if execution should continue
    pub fn check_limits(&self) -> Result<(), crate::RuntimeError> {
        if self.start_time.elapsed() > self.limits.max_time {
            return Err(crate::RuntimeError::Timeout(self.limits.max_time));
        }
        
        if self.memory_usage > self.limits.max_memory {
            return Err(crate::RuntimeError::MemoryLimit {
                used: self.memory_usage,
                limit: self.limits.max_memory,
            });
        }
        
        Ok(())
    }
    
    /// Record memory allocation
    pub fn allocate_memory(&mut self, size: u64) -> Result<(), crate::RuntimeError> {
        self.memory_usage += size;
        self.check_limits()
    }
    
    /// Record file handle usage
    pub fn open_file_handle(&mut self) -> Result<(), crate::RuntimeError> {
        self.file_handles += 1;
        if self.file_handles > self.limits.max_file_handles {
            return Err(crate::RuntimeError::Permission(
                format!("File handle limit exceeded: {}", self.limits.max_file_handles)
            ));
        }
        Ok(())
    }
}
```

### Task 2: Capability System Implementation

#### 2.1 Capability Framework

```rust
// File: crates/plugins-host/src/capabilities.rs
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Represents a specific capability that can be granted to plugins
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Read files matching the given pattern
    FileSystemRead(PathPattern),
    /// Write files matching the given pattern
    FileSystemWrite(PathPattern),
    /// Make HTTP requests matching the given pattern
    NetworkFetch(UrlPattern),
    /// Read terminal command blocks
    BlockRead,
    /// Modify terminal command blocks
    BlockWrite,
    /// Add actions to the command palette
    PaletteAddAction,
    /// Read configuration values
    ConfigRead,
    /// Modify configuration values
    ConfigWrite,
    /// Access environment variables
    EnvironmentRead,
    /// Execute system commands
    SystemExecute(CommandPattern),
}

/// Pattern for file system access
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathPattern {
    /// Base directory (e.g., "/tmp", "~/.config/quantaterm")
    pub base: String,
    /// Whether subdirectories are allowed
    pub recursive: bool,
    /// File extensions allowed (empty = all)
    pub extensions: Vec<String>,
}

/// Pattern for network access
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UrlPattern {
    /// Allowed schemes (http, https)
    pub schemes: Vec<String>,
    /// Allowed domains (example.com, *.api.github.com)
    pub domains: Vec<String>,
    /// Allowed ports (empty = any)
    pub ports: Vec<u16>,
}

/// Pattern for command execution
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandPattern {
    /// Allowed executables
    pub executables: Vec<String>,
    /// Allowed argument patterns
    pub args: Vec<String>,
}

/// Set of capabilities granted to a plugin
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    capabilities: HashSet<Capability>,
    plugin_id: String,
}

impl CapabilitySet {
    /// Create a new capability set for a plugin
    pub fn new(plugin_id: String) -> Self {
        Self {
            capabilities: HashSet::new(),
            plugin_id,
        }
    }
    
    /// Create capability set from plugin manifest
    pub fn from_manifest(manifest: &crate::PluginManifest) -> Self {
        let mut caps = Self::new(manifest.name.clone());
        
        for cap_str in &manifest.capabilities {
            if let Ok(capability) = Self::parse_capability(cap_str) {
                caps.capabilities.insert(capability);
            } else {
                tracing::warn!("Invalid capability in manifest: {}", cap_str);
            }
        }
        
        caps
    }
    
    /// Parse capability string from manifest
    fn parse_capability(cap_str: &str) -> Result<Capability, String> {
        let parts: Vec<&str> = cap_str.split(':').collect();
        
        match parts.as_slice() {
            ["fs", "read", pattern] => {
                let path_pattern = PathPattern {
                    base: pattern.to_string(),
                    recursive: pattern.ends_with("/**"),
                    extensions: vec![],
                };
                Ok(Capability::FileSystemRead(path_pattern))
            },
            ["fs", "write", pattern] => {
                let path_pattern = PathPattern {
                    base: pattern.to_string(),
                    recursive: pattern.ends_with("/**"),
                    extensions: vec![],
                };
                Ok(Capability::FileSystemWrite(path_pattern))
            },
            ["net", "fetch", pattern] => {
                let url_pattern = UrlPattern {
                    schemes: vec!["https".to_string()],
                    domains: vec![pattern.to_string()],
                    ports: vec![],
                };
                Ok(Capability::NetworkFetch(url_pattern))
            },
            ["block", "read"] => Ok(Capability::BlockRead),
            ["block", "write"] => Ok(Capability::BlockWrite),
            ["palette", "add"] => Ok(Capability::PaletteAddAction),
            ["config", "read"] => Ok(Capability::ConfigRead),
            ["config", "write"] => Ok(Capability::ConfigWrite),
            ["env", "read"] => Ok(Capability::EnvironmentRead),
            _ => Err(format!("Unknown capability: {}", cap_str)),
        }
    }
    
    /// Grant a capability to this set
    pub fn grant(&mut self, capability: Capability) {
        self.capabilities.insert(capability);
    }
    
    /// Check if a capability is granted
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }
    
    /// Check if file access is allowed
    pub fn check_file_access(&self, path: &Path, write: bool) -> Result<(), PermissionError> {
        let required_cap = if write {
            Capability::FileSystemWrite
        } else {
            Capability::FileSystemRead
        };
        
        for cap in &self.capabilities {
            if let Capability::FileSystemRead(pattern) | Capability::FileSystemWrite(pattern) = cap {
                if self.path_matches(path, pattern)? {
                    return Ok(());
                }
            }
        }
        
        Err(PermissionError::FileAccess {
            path: path.to_path_buf(),
            write,
            plugin_id: self.plugin_id.clone(),
        })
    }
    
    /// Check if path matches the given pattern
    fn path_matches(&self, path: &Path, pattern: &PathPattern) -> Result<bool, PermissionError> {
        let canonical_path = path.canonicalize()
            .map_err(|e| PermissionError::InvalidPath {
                path: path.to_path_buf(),
                error: e.to_string(),
            })?;
            
        let base_path = PathBuf::from(&pattern.base);
        
        // Check if path is under base directory
        if !canonical_path.starts_with(&base_path) {
            return Ok(false);
        }
        
        // Check recursive permission
        if !pattern.recursive {
            if canonical_path.parent() != Some(&base_path) {
                return Ok(false);
            }
        }
        
        // Check file extension
        if !pattern.extensions.is_empty() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if !pattern.extensions.contains(&ext.to_string()) {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

/// Permission checking trait for runtime enforcement
pub trait PermissionChecker {
    /// Check if a plugin has permission for a capability
    fn check_permission(&self, plugin_id: &str, capability: &Capability) -> Result<(), PermissionError>;
    
    /// Grant a capability to a plugin
    fn grant_capability(&mut self, plugin_id: &str, capability: Capability) -> Result<(), PermissionError>;
    
    /// Revoke a capability from a plugin
    fn revoke_capability(&mut self, plugin_id: &str, capability: &Capability) -> Result<(), PermissionError>;
    
    /// List all capabilities for a plugin
    fn list_capabilities(&self, plugin_id: &str) -> Vec<Capability>;
}

/// Permission errors
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("File access denied for plugin {plugin_id}: {path} (write: {write})")]
    FileAccess {
        path: PathBuf,
        write: bool,
        plugin_id: String,
    },
    #[error("Network access denied for plugin {plugin_id}: {url}")]
    NetworkAccess {
        url: String,
        plugin_id: String,
    },
    #[error("Invalid path: {path} - {error}")]
    InvalidPath {
        path: PathBuf,
        error: String,
    },
    #[error("Plugin not found: {plugin_id}")]
    PluginNotFound {
        plugin_id: String,
    },
    #[error("Capability not granted: {capability:?}")]
    CapabilityNotGranted {
        capability: Capability,
    },
}

/// Default permission checker implementation
pub struct DefaultPermissionChecker {
    plugin_capabilities: std::collections::HashMap<String, CapabilitySet>,
}

impl DefaultPermissionChecker {
    pub fn new() -> Self {
        Self {
            plugin_capabilities: std::collections::HashMap::new(),
        }
    }
    
    /// Register a plugin with its capabilities
    pub fn register_plugin(&mut self, plugin_id: String, capabilities: CapabilitySet) {
        self.plugin_capabilities.insert(plugin_id, capabilities);
    }
    
    /// Unregister a plugin
    pub fn unregister_plugin(&mut self, plugin_id: &str) {
        self.plugin_capabilities.remove(plugin_id);
    }
}

impl PermissionChecker for DefaultPermissionChecker {
    fn check_permission(&self, plugin_id: &str, capability: &Capability) -> Result<(), PermissionError> {
        let caps = self.plugin_capabilities.get(plugin_id)
            .ok_or_else(|| PermissionError::PluginNotFound {
                plugin_id: plugin_id.to_string(),
            })?;
            
        if caps.has_capability(capability) {
            Ok(())
        } else {
            Err(PermissionError::CapabilityNotGranted {
                capability: capability.clone(),
            })
        }
    }
    
    fn grant_capability(&mut self, plugin_id: &str, capability: Capability) -> Result<(), PermissionError> {
        let caps = self.plugin_capabilities.get_mut(plugin_id)
            .ok_or_else(|| PermissionError::PluginNotFound {
                plugin_id: plugin_id.to_string(),
            })?;
            
        caps.grant(capability);
        Ok(())
    }
    
    fn revoke_capability(&mut self, plugin_id: &str, capability: &Capability) -> Result<(), PermissionError> {
        let caps = self.plugin_capabilities.get_mut(plugin_id)
            .ok_or_else(|| PermissionError::PluginNotFound {
                plugin_id: plugin_id.to_string(),
            })?;
            
        caps.capabilities.remove(capability);
        Ok(())
    }
    
    fn list_capabilities(&self, plugin_id: &str) -> Vec<Capability> {
        self.plugin_capabilities
            .get(plugin_id)
            .map(|caps| caps.capabilities.iter().cloned().collect())
            .unwrap_or_default()
    }
}
```

### Task 3: AI Provider Implementation

#### 3.1 AI Provider Trait and Abstractions

```rust
// File: crates/ai/src/lib.rs
//! QuantaTerm AI integration and assistance
//!
//! Provides AI-powered terminal assistance with pluggable providers.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod provider;
pub mod providers;
pub mod redaction;
pub mod context;
pub mod error;

pub use provider::{AiProvider, AiResponse, CommandContext, OutputAnalysis, Completion};
pub use providers::openai::OpenAiProvider;
pub use redaction::{SecretRedactor, RedactionConfig};
pub use context::{RequestContext, ContextBuilder};
pub use error::{AiError, RedactionError};

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main AI integration coordinator
pub struct AiIntegration {
    providers: HashMap<String, Arc<dyn AiProvider + Send + Sync>>,
    active_provider: Option<String>,
    redactor: SecretRedactor,
    config: AiConfig,
}

/// Configuration for AI integration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AiConfig {
    /// Whether AI features are enabled
    pub enabled: bool,
    /// Default provider to use
    pub default_provider: String,
    /// Timeout for AI requests
    pub request_timeout_ms: u64,
    /// Whether to log AI requests (for debugging)
    pub log_requests: bool,
    /// Secret redaction configuration
    pub redaction: RedactionConfig,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in only
            default_provider: "openai".to_string(),
            request_timeout_ms: 10000, // 10 seconds
            log_requests: false,
            redaction: RedactionConfig::default(),
        }
    }
}

impl AiIntegration {
    /// Create new AI integration
    pub fn new(config: AiConfig) -> Result<Self> {
        let redactor = SecretRedactor::new(config.redaction.clone())?;
        
        Ok(Self {
            providers: HashMap::new(),
            active_provider: None,
            redactor,
            config,
        })
    }
    
    /// Register an AI provider
    pub fn register_provider(&mut self, name: String, provider: Arc<dyn AiProvider + Send + Sync>) {
        self.providers.insert(name.clone(), provider);
        
        if self.active_provider.is_none() {
            self.active_provider = Some(name);
        }
    }
    
    /// Set the active provider
    pub fn set_active_provider(&mut self, name: &str) -> Result<(), AiError> {
        if self.providers.contains_key(name) {
            self.active_provider = Some(name.to_string());
            Ok(())
        } else {
            Err(AiError::ProviderNotFound(name.to_string()))
        }
    }
    
    /// Explain a command using AI
    pub async fn explain_command(&self, command: &str, output: &str, error: &str) -> Result<AiResponse, AiError> {
        if !self.config.enabled {
            return Err(AiError::Disabled);
        }
        
        let provider = self.get_active_provider()?;
        
        // Redact secrets from input
        let safe_command = self.redactor.redact(command)?;
        let safe_output = self.redactor.redact(output)?;
        let safe_error = self.redactor.redact(error)?;
        
        if self.config.log_requests {
            tracing::info!("AI request: explain_command({}, output_len={}, error_len={})", 
                safe_command, safe_output.len(), safe_error.len());
        }
        
        provider.explain_command(&safe_command, &safe_output, &safe_error).await
    }
    
    /// Get suggestions for fixing an error
    pub async fn suggest_fix(&self, error_output: &str) -> Result<AiResponse, AiError> {
        if !self.config.enabled {
            return Err(AiError::Disabled);
        }
        
        let provider = self.get_active_provider()?;
        let safe_error = self.redactor.redact(error_output)?;
        
        provider.suggest_fix(&safe_error).await
    }
    
    /// Get command completions
    pub async fn complete_command(&self, partial: &str, context: &CommandContext) -> Result<Vec<Completion>, AiError> {
        if !self.config.enabled {
            return Err(AiError::Disabled);
        }
        
        let provider = self.get_active_provider()?;
        
        // Create safe context without sensitive information
        let safe_context = CommandContext {
            shell: context.shell.clone(),
            working_dir: context.working_dir.clone(),
            env_vars: HashMap::new(), // Never send env vars to AI
            recent_commands: context.recent_commands.iter()
                .map(|cmd| self.redactor.redact(cmd).unwrap_or_else(|_| "[REDACTED]".to_string()))
                .collect(),
        };
        
        provider.complete_command(partial, &safe_context).await
    }
    
    fn get_active_provider(&self) -> Result<&Arc<dyn AiProvider + Send + Sync>, AiError> {
        let provider_name = self.active_provider.as_ref()
            .ok_or(AiError::NoProviderConfigured)?;
            
        self.providers.get(provider_name)
            .ok_or_else(|| AiError::ProviderNotFound(provider_name.clone()))
    }
}
```

#### 3.2 AI Provider Trait Definition

```rust
// File: crates/ai/src/provider.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::AiError;

/// Trait for AI service providers
#[async_trait]
pub trait AiProvider {
    /// Explain what a command does and why it might have failed
    async fn explain_command(&self, command: &str, output: &str, error: &str) -> Result<AiResponse, AiError>;
    
    /// Suggest how to fix an error
    async fn suggest_fix(&self, error_output: &str) -> Result<AiResponse, AiError>;
    
    /// Provide command completion suggestions
    async fn complete_command(&self, partial: &str, context: &CommandContext) -> Result<Vec<Completion>, AiError>;
    
    /// Analyze command output for insights
    async fn analyze_output(&self, output: &str) -> Result<OutputAnalysis, AiError>;
    
    /// Get provider name and version
    fn provider_info(&self) -> ProviderInfo;
}

/// Response from AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    /// Main response content
    pub content: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Sources or references used
    pub sources: Vec<String>,
    /// Model used for the response
    pub model: String,
    /// Number of tokens consumed
    pub tokens_used: u32,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Context about the current terminal session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContext {
    /// Current shell (bash, zsh, fish, etc.)
    pub shell: String,
    /// Current working directory
    pub working_dir: PathBuf,
    /// Environment variables (filtered for security)
    pub env_vars: HashMap<String, String>,
    /// Recent commands in history
    pub recent_commands: Vec<String>,
}

/// Command completion suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    /// The completion text
    pub text: String,
    /// Description of what this completion does
    pub description: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Type of completion (command, option, argument, etc.)
    pub completion_type: CompletionType,
}

/// Type of command completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionType {
    /// A command or executable
    Command,
    /// A command-line option/flag
    Option,
    /// A file or directory path
    Path,
    /// An argument value
    Argument,
    /// A variable name
    Variable,
}

/// Analysis of command output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputAnalysis {
    /// Summary of the output
    pub summary: String,
    /// Detected errors or warnings
    pub issues: Vec<Issue>,
    /// Suggested next actions
    pub suggestions: Vec<String>,
    /// Confidence in the analysis
    pub confidence: f32,
}

/// An issue detected in command output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Issue type
    pub issue_type: IssueType,
    /// Description of the issue
    pub description: String,
    /// Suggested fix
    pub suggested_fix: Option<String>,
    /// Severity level
    pub severity: IssueSeverity,
}

/// Type of issue detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueType {
    /// Command failed with error
    Error,
    /// Warning or potential issue
    Warning,
    /// Performance concern
    Performance,
    /// Security concern
    Security,
}

/// Severity of an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// Information about an AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider name
    pub name: String,
    /// Provider version
    pub version: String,
    /// Supported features
    pub features: Vec<String>,
    /// API endpoint or service information
    pub endpoint: Option<String>,
}
```

#### 3.3 OpenAI Provider Implementation

```rust
// File: crates/ai/src/providers/openai.rs
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::provider::*;
use crate::error::AiError;

/// OpenAI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// API key for OpenAI
    pub api_key: String,
    /// Model to use (gpt-3.5-turbo, gpt-4, etc.)
    pub model: Option<String>,
    /// Base URL for API (for OpenAI-compatible services)
    pub base_url: Option<String>,
    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Maximum tokens per request
    pub max_tokens: Option<u32>,
    /// Temperature for response generation
    pub temperature: Option<f32>,
}

/// OpenAI API provider
pub struct OpenAiProvider {
    client: Client,
    config: OpenAiConfig,
    base_url: String,
    model: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(config: OpenAiConfig) -> Result<Self, AiError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds.unwrap_or(30)))
            .build()
            .map_err(|e| AiError::ProviderInitError(format!("Failed to create HTTP client: {}", e)))?;
            
        let base_url = config.base_url.clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            
        let model = config.model.clone()
            .unwrap_or_else(|| "gpt-3.5-turbo".to_string());
            
        Ok(Self {
            client,
            config,
            base_url,
            model,
        })
    }
    
    /// Make a chat completion request
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, AiError> {
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature.unwrap_or(0.7),
        };
        
        let response = self.client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(format!("HTTP request failed: {}", e)))?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiError(format!("API request failed with status {}: {}", status, error_text)));
        }
        
        let chat_response: ChatResponse = response.json().await
            .map_err(|e| AiError::ResponseParseError(format!("Failed to parse response: {}", e)))?;
            
        Ok(chat_response)
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn explain_command(&self, command: &str, output: &str, error: &str) -> Result<AiResponse, AiError> {
        let start_time = Instant::now();
        
        let system_prompt = "You are a helpful terminal assistant. Explain what commands do and why they might fail. Be concise and practical.";
        
        let user_prompt = if !error.is_empty() {
            format!(
                "Explain this command and why it failed:\nCommand: {}\nError: {}\nOutput: {}",
                command, error, output
            )
        } else {
            format!(
                "Explain what this command does:\nCommand: {}\nOutput: {}",
                command, output
            )
        };
        
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        let response_time = start_time.elapsed().as_millis() as u64;
        
        let content = response.choices.first()
            .ok_or_else(|| AiError::EmptyResponse)?
            .message
            .content
            .clone();
            
        Ok(AiResponse {
            content,
            confidence: 0.85, // Static confidence for now
            sources: vec!["OpenAI".to_string()],
            model: self.model.clone(),
            tokens_used: response.usage.total_tokens,
            response_time_ms: response_time,
        })
    }
    
    async fn suggest_fix(&self, error_output: &str) -> Result<AiResponse, AiError> {
        let start_time = Instant::now();
        
        let system_prompt = "You are a helpful terminal assistant. Suggest specific fixes for command errors. Provide actionable solutions.";
        
        let user_prompt = format!(
            "Suggest how to fix this error:\n{}",
            error_output
        );
        
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        let response_time = start_time.elapsed().as_millis() as u64;
        
        let content = response.choices.first()
            .ok_or_else(|| AiError::EmptyResponse)?
            .message
            .content
            .clone();
            
        Ok(AiResponse {
            content,
            confidence: 0.80,
            sources: vec!["OpenAI".to_string()],
            model: self.model.clone(),
            tokens_used: response.usage.total_tokens,
            response_time_ms: response_time,
        })
    }
    
    async fn complete_command(&self, partial: &str, context: &CommandContext) -> Result<Vec<Completion>, AiError> {
        let start_time = Instant::now();
        
        let system_prompt = "You are a terminal command completion assistant. Suggest command completions based on the partial input and context. Return up to 5 suggestions.";
        
        let user_prompt = format!(
            "Complete this command:\nPartial: {}\nShell: {}\nWorking directory: {}\nRecent commands: {}",
            partial,
            context.shell,
            context.working_dir.display(),
            context.recent_commands.join(", ")
        );
        
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        
        // Parse completions from response
        // This is a simplified implementation - in practice you'd want more structured output
        let content = response.choices.first()
            .ok_or_else(|| AiError::EmptyResponse)?
            .message
            .content
            .clone();
            
        let completions = content
            .lines()
            .take(5)
            .enumerate()
            .map(|(i, line)| Completion {
                text: line.trim().to_string(),
                description: format!("AI suggestion {}", i + 1),
                confidence: 0.75 - (i as f32 * 0.1),
                completion_type: CompletionType::Command,
            })
            .collect();
            
        Ok(completions)
    }
    
    async fn analyze_output(&self, output: &str) -> Result<OutputAnalysis, AiError> {
        let start_time = Instant::now();
        
        let system_prompt = "You are a terminal output analyzer. Analyze command output and identify any issues, warnings, or important information. Provide a summary and suggestions.";
        
        let user_prompt = format!("Analyze this command output:\n{}", output);
        
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];
        
        let response = self.chat_completion(messages).await?;
        
        let content = response.choices.first()
            .ok_or_else(|| AiError::EmptyResponse)?
            .message
            .content
            .clone();
            
        // This is a simplified analysis - in practice you'd parse structured output
        Ok(OutputAnalysis {
            summary: content,
            issues: vec![], // Would be parsed from structured response
            suggestions: vec![], // Would be parsed from structured response
            confidence: 0.80,
        })
    }
    
    fn provider_info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "OpenAI".to_string(),
            version: "1.0.0".to_string(),
            features: vec![
                "command_explanation".to_string(),
                "error_analysis".to_string(),
                "command_completion".to_string(),
                "output_analysis".to_string(),
            ],
            endpoint: Some(self.base_url.clone()),
        }
    }
}

// OpenAI API types
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: u32,
}
```

## Testing Framework Templates

### Unit Testing Template for Plugin System

```rust
// File: crates/plugins-host/src/tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    
    fn create_test_plugin_manifest() -> PluginManifest {
        PluginManifest {
            name: "test_plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: "main".to_string(),
            capabilities: vec![
                "fs:read:/tmp/**".to_string(),
                "palette:add".to_string(),
            ],
            quantaterm_version: "0.1.0".to_string(),
        }
    }
    
    #[test]
    fn test_capability_parsing() {
        let manifest = create_test_plugin_manifest();
        let caps = CapabilitySet::from_manifest(&manifest);
        
        assert!(caps.has_capability(&Capability::PaletteAddAction));
        
        let fs_read = Capability::FileSystemRead(PathPattern {
            base: "/tmp".to_string(),
            recursive: true,
            extensions: vec![],
        });
        assert!(caps.has_capability(&fs_read));
    }
    
    #[test]
    fn test_file_access_permission() {
        let manifest = create_test_plugin_manifest();
        let caps = CapabilitySet::from_manifest(&manifest);
        
        // Should allow access to /tmp files
        let tmp_file = PathBuf::from("/tmp/test.txt");
        assert!(caps.check_file_access(&tmp_file, false).is_ok());
        
        // Should deny access to other directories
        let home_file = PathBuf::from("/home/user/test.txt");
        assert!(caps.check_file_access(&home_file, false).is_err());
    }
    
    #[tokio::test]
    async fn test_plugin_loading() {
        let mut host = PluginsHost::new().unwrap();
        
        // This would require actual WASM file for full test
        // For now, test the loading infrastructure
        assert!(host.loaded_plugins.is_empty());
    }
    
    #[test]
    fn test_execution_limits() {
        let limits = ExecutionLimits::default();
        let monitor = ResourceMonitor::new(limits);
        
        // Should start within limits
        assert!(monitor.check_limits().is_ok());
        
        // Would test timeout in real implementation with actual execution
    }
    
    #[test]
    fn test_action_registration() {
        let mut registry = ActionRegistry::new();
        
        let action = Action {
            id: "test.hello".to_string(),
            name: "Say Hello".to_string(),
            description: "Shows a hello message".to_string(),
            category: "test".to_string(),
            shortcut: Some("Ctrl+H".to_string()),
            icon: None,
            plugin_id: "test_plugin".to_string(),
        };
        
        assert!(registry.register_action(action.clone()).is_ok());
        assert_eq!(registry.list_actions().len(), 1);
        
        // Test duplicate registration
        assert!(registry.register_action(action).is_err());
    }
}
```

### Integration Testing Template for AI

```rust
// File: crates/ai/src/tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    
    fn create_test_config() -> OpenAiConfig {
        OpenAiConfig {
            api_key: "test-key".to_string(),
            model: Some("gpt-3.5-turbo".to_string()),
            base_url: None,
            timeout_seconds: Some(30),
            max_tokens: Some(100),
            temperature: Some(0.7),
        }
    }
    
    #[tokio::test]
    async fn test_ai_provider_creation() {
        let config = create_test_config();
        let provider = OpenAiProvider::new(config);
        assert!(provider.is_ok());
    }
    
    #[test]
    fn test_secret_redaction() {
        let redactor = SecretRedactor::new(RedactionConfig::default()).unwrap();
        
        let input = "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let output = redactor.redact(input).unwrap();
        
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("AKIAIOSFODNN7EXAMPLE"));
    }
    
    #[test]
    fn test_context_sanitization() {
        let mut context = CommandContext {
            shell: "bash".to_string(),
            working_dir: PathBuf::from("/home/user"),
            env_vars: [
                ("PATH".to_string(), "/usr/bin".to_string()),
                ("AWS_SECRET_ACCESS_KEY".to_string(), "secret123".to_string()),
            ].into_iter().collect(),
            recent_commands: vec!["ls -la".to_string()],
        };
        
        // In real implementation, would sanitize env_vars
        context.env_vars.clear();
        
        assert!(context.env_vars.is_empty());
    }
    
    #[tokio::test]
    async fn test_ai_integration_disabled() {
        let config = AiConfig {
            enabled: false,
            ..Default::default()
        };
        
        let ai = AiIntegration::new(config).unwrap();
        let result = ai.explain_command("ls", "", "").await;
        
        assert!(matches!(result, Err(AiError::Disabled)));
    }
}
```

## Common Pitfalls and Solutions

### 1. WASM Memory Management
**Problem**: Plugins consuming excessive memory or leaking resources
**Solution**: Implement strict resource monitoring and automatic cleanup
```rust
// Monitor memory usage continuously
impl ResourceMonitor {
    pub fn track_allocation(&mut self, size: u64) -> Result<(), RuntimeError> {
        self.memory_usage += size;
        if self.memory_usage > self.limits.max_memory {
            return Err(RuntimeError::MemoryLimit {
                used: self.memory_usage,
                limit: self.limits.max_memory,
            });
        }
        Ok(())
    }
}
```

### 2. AI Request Security
**Problem**: Sensitive data leaked to AI providers
**Solution**: Comprehensive redaction and context sanitization
```rust
// Always redact before sending to AI
let safe_command = self.redactor.redact(command)?;
let safe_context = CommandContext {
    env_vars: HashMap::new(), // Never send env vars
    // ... other sanitized fields
};
```

### 3. Plugin Capability Violations
**Problem**: Runtime capability checks are too slow
**Solution**: Use efficient capability caching and pre-validation
```rust
// Cache capability check results
pub struct CapabilityCache {
    cache: HashMap<(String, Capability), bool>,
}
```

### 4. WASM Module Compatibility
**Problem**: Plugins compiled with different WASM versions fail to load
**Solution**: Strict version checking and compatibility matrix
```rust
// Check WASM compatibility before loading
fn check_wasm_compatibility(module: &Module) -> Result<(), RuntimeError> {
    // Validate WASM version, imports, exports
    Ok(())
}
```

### 5. AI Provider Rate Limiting
**Problem**: API rate limits cause request failures
**Solution**: Implement rate limiting and request queuing
```rust
pub struct RateLimiter {
    requests_per_minute: u32,
    request_times: Vec<Instant>,
}

impl RateLimiter {
    pub async fn wait_if_needed(&mut self) -> Result<(), AiError> {
        // Implement rate limiting logic
        Ok(())
    }
}
```

This implementation guide provides AI coding agents with concrete starting points, patterns, and solutions for successful Phase 3 implementation.
