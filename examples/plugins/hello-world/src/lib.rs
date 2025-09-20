//! Hello World Plugin for QuantaTerm
//! 
//! This is an example plugin that demonstrates:
//! - Basic plugin structure and lifecycle
//! - Registering actions in the command palette  
//! - Handling action execution
//! - Plugin configuration
//! - WASM export functions

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Plugin configuration structure
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    pub default_greeting: String,
    pub show_timestamp: bool,
    pub max_greetings: u32,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            default_greeting: "Hello from QuantaTerm!".to_string(),
            show_timestamp: true,
            max_greetings: 10,
        }
    }
}

/// Action result returned to QuantaTerm
#[derive(Debug, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
}

impl ActionResult {
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message,
            data: None,
        }
    }
    
    pub fn success_with_data(message: String, data: Value) -> Self {
        Self {
            success: true,
            message,
            data: Some(data),
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message,
            data: None,
        }
    }
}

/// Action definition for registration
#[derive(Debug, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub shortcut: Option<String>,
}

/// Global plugin state (in a real plugin, this would be more sophisticated)
static mut GREETING_COUNT: u32 = 0;
static mut CONFIG: Option<PluginConfig> = None;

/// Plugin entry point - called when the plugin is loaded
#[no_mangle]
pub extern "C" fn plugin_main() -> i32 {
    // Initialize plugin
    unsafe {
        GREETING_COUNT = 0;
        CONFIG = Some(PluginConfig::default());
    }
    
    // Log initialization (in real implementation, this would use QuantaTerm's logging API)
    log_message(1, "Hello World plugin initialized");
    
    0 // Success
}

/// Get actions this plugin provides - called by QuantaTerm during registration
#[no_mangle]
pub extern "C" fn get_actions() -> *const u8 {
    let actions = vec![
        ActionDefinition {
            id: "hello_world.greet".to_string(),
            name: "Say Hello".to_string(),
            description: "Display a greeting message".to_string(),
            category: "Examples".to_string(),
            shortcut: Some("Ctrl+Alt+H".to_string()),
        },
        ActionDefinition {
            id: "hello_world.info".to_string(),
            name: "Plugin Info".to_string(),
            description: "Show information about this plugin".to_string(),
            category: "Examples".to_string(),
            shortcut: None,
        },
    ];
    
    let json = serde_json::to_string(&actions).unwrap_or_default();
    let bytes = json.into_bytes();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes); // Prevent deallocation
    ptr
}

/// Execute an action - called when user triggers a plugin action
#[no_mangle]
pub extern "C" fn execute_action(action_id_ptr: *const u8, action_id_len: usize, args_ptr: *const u8, args_len: usize) -> *const u8 {
    // Read action ID from memory
    let action_id = unsafe {
        let slice = std::slice::from_raw_parts(action_id_ptr, action_id_len);
        String::from_utf8_lossy(slice).to_string()
    };
    
    // Read arguments from memory  
    let args: Vec<Value> = if args_len > 0 {
        let args_str = unsafe {
            let slice = std::slice::from_raw_parts(args_ptr, args_len);
            String::from_utf8_lossy(slice).to_string()
        };
        serde_json::from_str(&args_str).unwrap_or_default()
    } else {
        vec![]
    };
    
    // Execute the action
    let result = match action_id.as_str() {
        "hello_world.greet" => execute_greet_action(&args),
        "hello_world.info" => execute_info_action(&args),
        _ => ActionResult::error(format!("Unknown action: {}", action_id)),
    };
    
    // Return result as JSON
    let json = serde_json::to_string(&result).unwrap_or_default();
    let bytes = json.into_bytes();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes); // Prevent deallocation
    ptr
}

/// Handle the "greet" action
fn execute_greet_action(_args: &[Value]) -> ActionResult {
    unsafe {
        let config = CONFIG.as_ref().unwrap();
        
        // Check greeting limit
        if GREETING_COUNT >= config.max_greetings {
            return ActionResult::error(format!("Maximum greetings ({}) reached", config.max_greetings));
        }
        
        GREETING_COUNT += 1;
        
        let mut message = config.default_greeting.clone();
        
        if config.show_timestamp {
            // In a real implementation, this would use proper time APIs
            message.push_str(&format!(" (Greeting #{}/{})", GREETING_COUNT, config.max_greetings));
        }
        
        log_message(2, &format!("Greeting action executed: {}", message));
        
        let data = serde_json::json!({
            "greeting_count": GREETING_COUNT,
            "max_greetings": config.max_greetings,
            "timestamp": "2024-01-01T00:00:00Z" // Mock timestamp
        });
        
        ActionResult::success_with_data(message, data)
    }
}

/// Handle the "info" action
fn execute_info_action(_args: &[Value]) -> ActionResult {
    let info = serde_json::json!({
        "name": "Hello World Plugin",
        "version": "1.0.0",
        "description": "Example QuantaTerm plugin demonstrating basic functionality",
        "author": "QuantaTerm Team",
        "capabilities": ["palette.add_action"],
        "greeting_count": unsafe { GREETING_COUNT },
        "status": "active"
    });
    
    log_message(2, "Plugin info requested");
    
    ActionResult::success_with_data(
        "Plugin information retrieved".to_string(),
        info
    )
}

/// Plugin cleanup - called when plugin is unloaded
#[no_mangle]
pub extern "C" fn plugin_cleanup() -> i32 {
    log_message(1, "Hello World plugin cleanup");
    
    unsafe {
        GREETING_COUNT = 0;
        CONFIG = None;
    }
    
    0 // Success
}

/// Update plugin configuration - called when config changes
#[no_mangle]
pub extern "C" fn update_config(config_ptr: *const u8, config_len: usize) -> i32 {
    let config_str = unsafe {
        let slice = std::slice::from_raw_parts(config_ptr, config_len);
        String::from_utf8_lossy(slice).to_string()
    };
    
    match serde_json::from_str::<PluginConfig>(&config_str) {
        Ok(new_config) => {
            unsafe {
                CONFIG = Some(new_config);
            }
            log_message(2, "Plugin configuration updated");
            0 // Success
        },
        Err(e) => {
            log_message(4, &format!("Failed to parse config: {}", e));
            -1 // Error
        }
    }
}

/// Log a message (in real implementation, this would call QuantaTerm's logging API)
fn log_message(level: i32, message: &str) {
    // This is a mock implementation
    // In a real plugin, this would call the host's logging function
    let level_str = match level {
        0 => "TRACE",
        1 => "DEBUG", 
        2 => "INFO",
        3 => "WARN",
        4 => "ERROR",
        _ => "INFO",
    };
    
    // For now, we can't actually log from WASM, but this shows the structure
    // println!("[{}] [HelloWorld] {}", level_str, message);
}

/// Memory allocation function for the host
#[no_mangle]
pub extern "C" fn allocate(size: usize) -> *mut u8 {
    let mut vec = Vec::with_capacity(size);
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec);
    ptr
}

/// Memory deallocation function for the host
#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, 0, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_result_creation() {
        let success = ActionResult::success("Test".to_string());
        assert!(success.success);
        assert_eq!(success.message, "Test");
        
        let error = ActionResult::error("Error".to_string());
        assert!(!error.success);
        assert_eq!(error.message, "Error");
    }
    
    #[test] 
    fn test_config_serialization() {
        let config = PluginConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: PluginConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(config.default_greeting, parsed.default_greeting);
        assert_eq!(config.show_timestamp, parsed.show_timestamp);
        assert_eq!(config.max_greetings, parsed.max_greetings);
    }
    
    #[test]
    fn test_action_definition() {
        let action = ActionDefinition {
            id: "test.action".to_string(),
            name: "Test Action".to_string(),
            description: "Test description".to_string(),
            category: "Test".to_string(),
            shortcut: Some("Ctrl+T".to_string()),
        };
        
        assert_eq!(action.id, "test.action");
        assert_eq!(action.shortcut, Some("Ctrl+T".to_string()));
    }
}