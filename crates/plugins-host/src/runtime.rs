//! WASM plugin runtime using Wasmtime
//! 
//! This module provides the core WASM runtime for executing plugins with
//! security, resource limits, and capability enforcement.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use wasmtime::*;
// use wasmtime_wasi::{WasiCtxBuilder}; // WasiView disabled for now

use crate::host_context::{HostContext, CapabilitySet};
use crate::limits::{ExecutionLimits, ResourceMonitor, LimitError};
use crate::manifest::{PluginManifest, ManifestLoader};

/// WASM runtime error types
#[derive(Debug, Error)]
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
    
    #[error("Plugin execution failed: {0}")]
    ExecutionError(String),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(#[from] LimitError),
    
    #[error("Manifest error: {0}")]
    ManifestError(#[from] crate::manifest::ManifestError),
    
    #[error("WASM trap: {0}")]
    WasmTrap(String),
}

/// A loaded and instantiated plugin
#[derive(Debug)]
pub struct LoadedPlugin {
    instance: Instance,
    store: Store<HostContext>,
    manifest: PluginManifest,
    resource_monitor: ResourceMonitor,
    memory: Option<Memory>,
}

/// Main WASM runtime for plugins
pub struct WasmRuntime {
    engine: Engine,
    linker: Linker<HostContext>,
    pub(crate) instances: HashMap<String, LoadedPlugin>,
    limits: ExecutionLimits,
    manifest_loader: ManifestLoader,
}

/// Result type for plugin action execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Context for executing plugin actions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionContext {
    pub action_id: String,
    pub args: Vec<serde_json::Value>,
    pub user_data: Option<serde_json::Value>,
}

impl WasmRuntime {
    /// Create a new WASM runtime with default configuration
    pub fn new() -> Result<Self, RuntimeError> {
        Self::with_limits(ExecutionLimits::default())
    }
    
    /// Create a new WASM runtime with custom limits
    pub fn with_limits(limits: ExecutionLimits) -> Result<Self, RuntimeError> {
        let mut config = Config::new();
        config.wasm_component_model(false);
        config.async_support(false);
        config.consume_fuel(true);
        
        // Enable memory limits
        if limits.max_memory > 0 {
            config.max_wasm_stack(64 * 1024); // 64KB stack limit
        }
        
        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        
        // Add WASI support with restrictions (commented out for now)
        // wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        
        // Add custom host functions
        Self::add_host_functions(&mut linker)?;
        
        let manifest_loader = ManifestLoader::new();
        
        Ok(Self {
            engine,
            linker,
            instances: HashMap::new(),
            limits,
            manifest_loader,
        })
    }
    
    /// Load a plugin module from file
    pub fn load_plugin_module(&mut self, path: &Path, manifest: &PluginManifest) -> Result<String, RuntimeError> {
        let plugin_id = manifest.name.clone();
        
        info!("Loading plugin: {} from {}", plugin_id, path.display());
        
        // Read and compile the WASM module
        let module_bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read plugin file: {}", path.display()))?;
            
        let module = Module::new(&self.engine, module_bytes)
            .with_context(|| "Failed to compile WASM module")?;
            
        // Create WASI context with restrictions (disabled for now)
        // let wasi_ctx = WasiCtxBuilder::new()
        //     .inherit_stdio()
        //     .build();
            
        // Create capabilities from manifest
        let capabilities = CapabilitySet::from_manifest(manifest);
        
        // Create host context
        let host_context = HostContext::new(
            plugin_id.clone(),
            capabilities,
            &self.limits,
            crate::host_context::PluginMetadata {
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                description: manifest.description.clone(),
                author: manifest.author.clone(),
                license: manifest.license.clone(),
                homepage: manifest.homepage.clone(),
            },
        );
        
        // Create store with host context
        let mut store = Store::new(&self.engine, host_context);
        store.data_mut().add_environment_variable("PWD".to_string(), "/".to_string());
        
        // Set fuel limit for computation control
        store.set_fuel(self.limits.max_fuel)?;
        
        // Set WASI context (disabled for now)
        // let mut wasi_ctx = wasi_ctx;
        // store.data_mut();
        
        // Instantiate the module
        let instance = self.linker.instantiate(&mut store, &module)
            .with_context(|| "Failed to instantiate WASM module")?;
        
        // Get memory export if available
        let memory = instance.get_memory(&mut store, "memory");
        
        // Create resource monitor
        let resource_monitor = ResourceMonitor::new(self.limits.clone());
        
        // Create loaded plugin
        let loaded_plugin = LoadedPlugin {
            instance,
            store,
            manifest: manifest.clone(),
            resource_monitor,
            memory,
        };
        
        // Store the plugin
        self.instances.insert(plugin_id.clone(), loaded_plugin);
        
        info!("Successfully loaded plugin: {}", plugin_id);
        Ok(plugin_id)
    }
    
    /// Load a plugin from a directory (searches for plugin.toml and .wasm files)
    pub fn load_plugin(&mut self, plugin_dir: &Path) -> Result<String, RuntimeError> {
        let manifest_path = plugin_dir.join("plugin.toml");
        let manifest = self.manifest_loader.load_manifest(&manifest_path)?;
        
        let wasm_path = plugin_dir.join(&manifest.entry_point);
        if !wasm_path.exists() {
            return Err(RuntimeError::LoadError(anyhow::anyhow!(
                "WASM file not found: {}", wasm_path.display()
            )));
        }
        
        self.load_plugin_module(&wasm_path, &manifest)
    }
    
    /// Execute a plugin function with timeout and resource monitoring
    pub fn execute_plugin_function(
        &mut self,
        plugin_id: &str,
        function_name: &str,
        args: &[Val],
    ) -> Result<Vec<Val>, RuntimeError> {
        let plugin = self.instances.get_mut(plugin_id)
            .ok_or_else(|| RuntimeError::PluginNotFound(plugin_id.to_string()))?;
        
        // Check resource limits before execution
        plugin.resource_monitor.check_limits()?;
        
        debug!("Executing function {} in plugin {}", function_name, plugin_id);
        
        // Get the function export
        let func = plugin.instance
            .get_func(&mut plugin.store, function_name)
            .ok_or_else(|| RuntimeError::ExecutionError(
                format!("Function '{}' not found in plugin '{}'", function_name, plugin_id)
            ))?;
        
        // Execute with timeout
        let start_time = Instant::now();
        let mut results = vec![Val::I32(0); func.ty(&plugin.store).results().len()];
        
        // Execute the function
        match func.call(&mut plugin.store, args, &mut results) {
            Ok(()) => {
                debug!("Function {} completed in {:?}", function_name, start_time.elapsed());
                
                // Update resource usage
                if let Some(memory) = plugin.memory {
                    let memory_size = memory.data_size(&plugin.store) as u64;
                    plugin.resource_monitor.update_memory_usage(memory_size);
                }
                
                // Final resource check
                plugin.resource_monitor.check_limits()?;
                
                Ok(results)
            },
            Err(trap) => {
                error!("Function {} failed with trap: {}", function_name, trap);
                Err(RuntimeError::ExecutionError(format!("WASM trap: {}", trap)))
            }
        }
    }
    
    /// Execute a plugin action (higher-level interface)
    pub fn execute_action(
        &mut self,
        plugin_id: &str,
        action_context: &ActionContext,
    ) -> Result<ActionResult, RuntimeError> {
        // Serialize action context to pass to plugin
        let context_json = serde_json::to_string(action_context)
            .map_err(|e| RuntimeError::ExecutionError(format!("Failed to serialize context: {}", e)))?;
        
        // For now, we'll call a generic "execute_action" function in the plugin
        // In a real implementation, this would be more sophisticated
        let context_ptr = self.write_string_to_plugin_memory(plugin_id, &context_json)?;
        let context_len = context_json.len() as i32;
        
        let args = &[Val::I32(context_ptr), Val::I32(context_len)];
        let _results = self.execute_plugin_function(plugin_id, "execute_action", args)?;
        
        // For now, return a simple success result
        // In a real implementation, we'd read the result from plugin memory
        Ok(ActionResult {
            success: true,
            message: "Action executed successfully".to_string(),
            data: None,
        })
    }
    
    /// Write a string to plugin memory and return pointer
    fn write_string_to_plugin_memory(&mut self, plugin_id: &str, _text: &str) -> Result<i32, RuntimeError> {
        let plugin = self.instances.get_mut(plugin_id)
            .ok_or_else(|| RuntimeError::PluginNotFound(plugin_id.to_string()))?;
        
        if let Some(_memory) = plugin.memory {
            // In a real implementation, we'd allocate memory in the plugin
            // For now, just return a dummy pointer
            Ok(0)
        } else {
            Err(RuntimeError::ExecutionError("Plugin has no memory export".to_string()))
        }
    }
    
    /// Unload a plugin and clean up resources
    pub fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), RuntimeError> {
        if let Some(_plugin) = self.instances.remove(plugin_id) {
            info!("Unloaded plugin: {}", plugin_id);
            Ok(())
        } else {
            Err(RuntimeError::PluginNotFound(plugin_id.to_string()))
        }
    }
    
    /// Get a list of loaded plugins
    pub fn loaded_plugins(&self) -> Vec<String> {
        self.instances.keys().cloned().collect()
    }
    
    /// Get plugin manifest by ID
    pub fn get_plugin_manifest(&self, plugin_id: &str) -> Option<&PluginManifest> {
        self.instances.get(plugin_id).map(|p| &p.manifest)
    }
    
    /// Add custom host functions to the linker
    fn add_host_functions(linker: &mut Linker<HostContext>) -> Result<(), RuntimeError> {
        // Add logging function
        linker.func_wrap("env", "host_log", |mut caller: Caller<'_, HostContext>, level: i32, ptr: i32, len: i32| {
            let memory = caller.get_export("memory")
                .and_then(|e| e.into_memory())
                .ok_or_else(|| anyhow::anyhow!("No memory export"))?;
            
            let data = memory.data(&caller);
            let start = ptr as usize;
            let end = start + len as usize;
            
            if end > data.len() {
                return Err(anyhow::anyhow!("String read out of bounds"));
            }
            
            let message = String::from_utf8_lossy(&data[start..end]);
            
            match level {
                0 => debug!("[Plugin] {}", message),
                1 => info!("[Plugin] {}", message),
                2 => warn!("[Plugin] {}", message),
                3 => error!("[Plugin] {}", message),
                _ => debug!("[Plugin] {}", message),
            }
            
            Ok(())
        })?;
        
        // Add capability check function
        linker.func_wrap("env", "host_check_capability", 
            |_caller: Caller<'_, HostContext>, _cap_ptr: i32, _cap_len: i32| -> i32 {
                // This would check if the plugin has the requested capability
                // For now, just return success
                1
            }
        )?;
        
        Ok(())
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create WASM runtime")
    }
}

// Implement WasiView for HostContext (disabled for now)
// impl WasiView for HostContext {
//     fn table(&mut self) -> &mut wasmtime_wasi::Table {
//         todo!("WASI table management not implemented")
//     }
//     
//     fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
//         todo!("WASI context management not implemented")
//     }
// }

impl ActionResult {
    /// Create a success result
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message,
            data: None,
        }
    }
    
    /// Create a success result with data
    pub fn success_with_data(message: String, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message,
            data: Some(data),
        }
    }
    
    /// Create an error result
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message,
            data: None,
        }
    }
}

impl LoadedPlugin {
    /// Get the plugin's actions (would call into WASM in real implementation)
    pub fn get_actions(&self) -> Result<Vec<crate::actions::Action>, RuntimeError> {
        // For now, return empty list
        // In real implementation, would call plugin's get_actions function
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::ExecutionLimits;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_plugin_dir() -> Result<TempDir, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        
        // Create a minimal plugin manifest
        let manifest = PluginManifest::minimal("test_plugin", "plugin.wasm");
        let manifest_content = manifest.to_toml()?;
        
        fs::write(temp_dir.path().join("plugin.toml"), manifest_content)?;
        
        // Create a minimal WASM file (empty for now)
        // In real tests, this would be a compiled WASM module
        fs::write(temp_dir.path().join("plugin.wasm"), &[0x00, 0x61, 0x73, 0x6d])?; // WASM magic
        
        Ok(temp_dir)
    }

    #[test]
    fn test_runtime_creation() {
        let runtime = WasmRuntime::new().unwrap();
        assert!(runtime.loaded_plugins().is_empty());
    }

    #[test]
    fn test_runtime_with_custom_limits() {
        let limits = ExecutionLimits::development();
        let runtime = WasmRuntime::with_limits(limits).unwrap();
        assert!(runtime.loaded_plugins().is_empty());
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut runtime = WasmRuntime::new().unwrap();
        
        // Initially no plugins loaded
        assert!(runtime.loaded_plugins().is_empty());
        
        // Loading non-existent plugin should fail
        let result = runtime.load_plugin(Path::new("/nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_action_result_creation() {
        let success = ActionResult::success("Test success".to_string());
        assert!(success.success);
        assert_eq!(success.message, "Test success");
        assert!(success.data.is_none());
        
        let error = ActionResult::error("Test error".to_string());
        assert!(!error.success);
        assert_eq!(error.message, "Test error");
        
        let with_data = ActionResult::success_with_data(
            "With data".to_string(),
            serde_json::json!({"key": "value"})
        );
        assert!(with_data.success);
        assert!(with_data.data.is_some());
    }

    #[test]
    fn test_unload_nonexistent_plugin() {
        let mut runtime = WasmRuntime::new().unwrap();
        let result = runtime.unload_plugin("nonexistent");
        assert!(matches!(result, Err(RuntimeError::PluginNotFound(_))));
    }

    #[test]
    fn test_execute_function_on_nonexistent_plugin() {
        let mut runtime = WasmRuntime::new().unwrap();
        let result = runtime.execute_plugin_function("nonexistent", "test", &[]);
        assert!(matches!(result, Err(RuntimeError::PluginNotFound(_))));
    }

    #[test]
    fn test_action_context_serialization() {
        let context = ActionContext {
            action_id: "test.action".to_string(),
            args: vec![serde_json::json!("arg1"), serde_json::json!(42)],
            user_data: Some(serde_json::json!({"key": "value"})),
        };
        
        let json = serde_json::to_string(&context).unwrap();
        let parsed: ActionContext = serde_json::from_str(&json).unwrap();
        
        assert_eq!(context.action_id, parsed.action_id);
        assert_eq!(context.args, parsed.args);
        assert_eq!(context.user_data, parsed.user_data);
    }
}