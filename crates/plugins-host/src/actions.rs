//! Plugin action system for command palette integration
//! 
//! This module defines the action system that allows plugins to register
//! commands that appear in the QuantaTerm command palette.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Represents a plugin action that can be executed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    /// Unique identifier for this action
    pub id: String,
    /// Display name shown in command palette
    pub name: String,
    /// Description shown in command palette
    pub description: String,
    /// Category for grouping actions
    pub category: String,
    /// Optional keyboard shortcut
    pub shortcut: Option<String>,
    /// Optional icon identifier
    pub icon: Option<String>,
    /// ID of the plugin that registered this action
    pub plugin_id: String,
}

/// Registry for managing plugin actions
#[derive(Debug)]
pub struct ActionRegistry {
    actions: HashMap<String, Action>,
    plugin_actions: HashMap<String, Vec<String>>, // plugin_id -> action_ids
}

/// Errors that can occur with action operations
#[derive(Debug, Error)]
pub enum ActionError {
    #[error("Action already exists: {0}")]
    ActionExists(String),
    
    #[error("Action not found: {0}")]
    ActionNotFound(String),
    
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    
    #[error("Invalid action ID: {0}")]
    InvalidActionId(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

impl ActionRegistry {
    /// Create a new action registry
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
            plugin_actions: HashMap::new(),
        }
    }
    
    /// Register a new action
    pub fn register_action(&mut self, action: Action) -> Result<(), ActionError> {
        // Validate action ID
        if action.id.is_empty() || !Self::is_valid_action_id(&action.id) {
            return Err(ActionError::InvalidActionId(action.id.clone()));
        }
        
        // Check if action already exists
        if self.actions.contains_key(&action.id) {
            return Err(ActionError::ActionExists(action.id.clone()));
        }
        
        let plugin_id = action.plugin_id.clone();
        let action_id = action.id.clone();
        
        // Register the action
        self.actions.insert(action_id.clone(), action);
        
        // Track which plugin owns this action
        self.plugin_actions
            .entry(plugin_id)
            .or_insert_with(Vec::new)
            .push(action_id);
        
        Ok(())
    }
    
    /// Unregister an action
    pub fn unregister_action(&mut self, action_id: &str) -> Result<(), ActionError> {
        let action = self.actions.remove(action_id)
            .ok_or_else(|| ActionError::ActionNotFound(action_id.to_string()))?;
        
        // Remove from plugin tracking
        if let Some(plugin_actions) = self.plugin_actions.get_mut(&action.plugin_id) {
            plugin_actions.retain(|id| id != action_id);
            
            // Remove plugin entry if no actions left
            if plugin_actions.is_empty() {
                self.plugin_actions.remove(&action.plugin_id);
            }
        }
        
        Ok(())
    }
    
    /// Get all registered actions
    pub fn list_actions(&self) -> Vec<Action> {
        self.actions.values().cloned().collect()
    }
    
    /// Search actions by name or description
    pub fn search_actions(&self, query: &str) -> Vec<Action> {
        let query_lower = query.to_lowercase();
        
        self.actions
            .values()
            .filter(|action| {
                action.name.to_lowercase().contains(&query_lower) ||
                action.description.to_lowercase().contains(&query_lower) ||
                action.category.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }
    
    /// Get actions for a specific plugin
    pub fn get_plugin_actions(&self, plugin_id: &str) -> Vec<Action> {
        self.plugin_actions
            .get(plugin_id)
            .map(|action_ids| {
                action_ids
                    .iter()
                    .filter_map(|id| self.actions.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get an action by ID
    pub fn get_action(&self, action_id: &str) -> Option<&Action> {
        self.actions.get(action_id)
    }
    
    /// Unregister all actions for a plugin
    pub fn unregister_plugin_actions(&mut self, plugin_id: &str) -> Result<(), ActionError> {
        let action_ids = self.plugin_actions
            .remove(plugin_id)
            .unwrap_or_default();
        
        for action_id in action_ids {
            self.actions.remove(&action_id);
        }
        
        Ok(())
    }
    
    /// Execute an action (placeholder for now)
    pub fn execute_action(&self, action_id: &str, _args: &[serde_json::Value]) -> Result<crate::runtime::ActionResult, ActionError> {
        let _action = self.actions.get(action_id)
            .ok_or_else(|| ActionError::ActionNotFound(action_id.to_string()))?;
        
        // In real implementation, this would delegate to the plugin runtime
        Ok(crate::runtime::ActionResult::success(
            format!("Action {} executed", action_id)
        ))
    }
    
    /// Check if an action ID is valid
    fn is_valid_action_id(action_id: &str) -> bool {
        // Action IDs should be in format "plugin.action_name"
        let parts: Vec<&str> = action_id.split('.').collect();
        parts.len() == 2 
            && !parts[0].is_empty() 
            && !parts[1].is_empty()
            && parts.iter().all(|part| {
                part.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            })
    }
    
    /// Get actions grouped by category
    pub fn actions_by_category(&self) -> HashMap<String, Vec<Action>> {
        let mut result = HashMap::new();
        
        for action in self.actions.values() {
            result
                .entry(action.category.clone())
                .or_insert_with(Vec::new)
                .push(action.clone());
        }
        
        result
    }
    
    /// Get the total number of registered actions
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }
    
    /// Get the number of plugins with registered actions
    pub fn plugin_count(&self) -> usize {
        self.plugin_actions.len()
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Action {
    /// Create a new action
    pub fn new(
        id: String,
        name: String,
        description: String,
        category: String,
        plugin_id: String,
    ) -> Self {
        Self {
            id,
            name,
            description,
            category,
            shortcut: None,
            icon: None,
            plugin_id,
        }
    }
    
    /// Set a keyboard shortcut for this action
    pub fn with_shortcut(mut self, shortcut: String) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
    
    /// Set an icon for this action
    pub fn with_icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }
    
    /// Get a display string for this action
    pub fn display_string(&self) -> String {
        match &self.shortcut {
            Some(shortcut) => format!("{} ({})", self.name, shortcut),
            None => self.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_action(id: &str, plugin_id: &str) -> Action {
        Action::new(
            id.to_string(),
            format!("Test Action {}", id),
            format!("Description for {}", id),
            "test".to_string(),
            plugin_id.to_string(),
        )
    }

    #[test]
    fn test_action_registry_creation() {
        let registry = ActionRegistry::new();
        assert_eq!(registry.action_count(), 0);
        assert_eq!(registry.plugin_count(), 0);
        assert!(registry.list_actions().is_empty());
    }

    #[test]
    fn test_action_registration() {
        let mut registry = ActionRegistry::new();
        let action = create_test_action("test.hello", "test_plugin");
        
        let result = registry.register_action(action.clone());
        assert!(result.is_ok());
        
        assert_eq!(registry.action_count(), 1);
        assert_eq!(registry.plugin_count(), 1);
        
        let retrieved = registry.get_action("test.hello");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, action.id);
    }

    #[test]
    fn test_duplicate_action_registration() {
        let mut registry = ActionRegistry::new();
        let action = create_test_action("test.hello", "test_plugin");
        
        registry.register_action(action.clone()).unwrap();
        let result = registry.register_action(action);
        
        assert!(matches!(result, Err(ActionError::ActionExists(_))));
    }

    #[test]
    fn test_action_unregistration() {
        let mut registry = ActionRegistry::new();
        let action = create_test_action("test.hello", "test_plugin");
        
        registry.register_action(action).unwrap();
        assert_eq!(registry.action_count(), 1);
        
        let result = registry.unregister_action("test.hello");
        assert!(result.is_ok());
        assert_eq!(registry.action_count(), 0);
        
        let retrieved = registry.get_action("test.hello");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_action_search() {
        let mut registry = ActionRegistry::new();
        
        registry.register_action(create_test_action("test.hello", "test_plugin")).unwrap();
        registry.register_action(create_test_action("test.world", "test_plugin")).unwrap();
        registry.register_action(create_test_action("other.action", "other_plugin")).unwrap();
        
        let results = registry.search_actions("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test.hello");
        
        let results = registry.search_actions("test");
        assert_eq!(results.len(), 3); // All have "test" in category or name
        
        let results = registry.search_actions("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_plugin_actions() {
        let mut registry = ActionRegistry::new();
        
        registry.register_action(create_test_action("test.hello", "test_plugin")).unwrap();
        registry.register_action(create_test_action("test.world", "test_plugin")).unwrap();
        registry.register_action(create_test_action("other.action", "other_plugin")).unwrap();
        
        let test_actions = registry.get_plugin_actions("test_plugin");
        assert_eq!(test_actions.len(), 2);
        
        let other_actions = registry.get_plugin_actions("other_plugin");
        assert_eq!(other_actions.len(), 1);
        
        let nonexistent_actions = registry.get_plugin_actions("nonexistent");
        assert!(nonexistent_actions.is_empty());
    }

    #[test]
    fn test_unregister_plugin_actions() {
        let mut registry = ActionRegistry::new();
        
        registry.register_action(create_test_action("test.hello", "test_plugin")).unwrap();
        registry.register_action(create_test_action("test.world", "test_plugin")).unwrap();
        registry.register_action(create_test_action("other.action", "other_plugin")).unwrap();
        
        assert_eq!(registry.action_count(), 3);
        assert_eq!(registry.plugin_count(), 2);
        
        let result = registry.unregister_plugin_actions("test_plugin");
        assert!(result.is_ok());
        
        assert_eq!(registry.action_count(), 1);
        assert_eq!(registry.plugin_count(), 1);
        
        let remaining_actions = registry.list_actions();
        assert_eq!(remaining_actions.len(), 1);
        assert_eq!(remaining_actions[0].id, "other.action");
    }

    #[test]
    fn test_action_id_validation() {
        assert!(ActionRegistry::is_valid_action_id("plugin.action"));
        assert!(ActionRegistry::is_valid_action_id("my_plugin.my_action"));
        assert!(ActionRegistry::is_valid_action_id("test-plugin.test-action"));
        
        assert!(!ActionRegistry::is_valid_action_id(""));
        assert!(!ActionRegistry::is_valid_action_id("invalid"));
        assert!(!ActionRegistry::is_valid_action_id("plugin."));
        assert!(!ActionRegistry::is_valid_action_id(".action"));
        assert!(!ActionRegistry::is_valid_action_id("plugin.action.extra"));
        assert!(!ActionRegistry::is_valid_action_id("plugin with spaces.action"));
    }

    #[test]
    fn test_action_with_shortcut_and_icon() {
        let action = Action::new(
            "test.action".to_string(),
            "Test Action".to_string(),
            "A test action".to_string(),
            "test".to_string(),
            "test_plugin".to_string(),
        )
        .with_shortcut("Ctrl+T".to_string())
        .with_icon("test-icon".to_string());
        
        assert_eq!(action.shortcut, Some("Ctrl+T".to_string()));
        assert_eq!(action.icon, Some("test-icon".to_string()));
        assert_eq!(action.display_string(), "Test Action (Ctrl+T)");
    }

    #[test]
    fn test_actions_by_category() {
        let mut registry = ActionRegistry::new();
        
        let mut action1 = create_test_action("test.hello", "test_plugin");
        action1.category = "category1".to_string();
        
        let mut action2 = create_test_action("test.world", "test_plugin");
        action2.category = "category1".to_string();
        
        let mut action3 = create_test_action("other.action", "other_plugin");
        action3.category = "category2".to_string();
        
        registry.register_action(action1).unwrap();
        registry.register_action(action2).unwrap();
        registry.register_action(action3).unwrap();
        
        let by_category = registry.actions_by_category();
        assert_eq!(by_category.len(), 2);
        assert_eq!(by_category.get("category1").unwrap().len(), 2);
        assert_eq!(by_category.get("category2").unwrap().len(), 1);
    }

    #[test]
    fn test_action_execution() {
        let mut registry = ActionRegistry::new();
        let action = create_test_action("test.hello", "test_plugin");
        
        registry.register_action(action).unwrap();
        
        let result = registry.execute_action("test.hello", &[]);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
        
        let result = registry.execute_action("nonexistent", &[]);
        assert!(matches!(result, Err(ActionError::ActionNotFound(_))));
    }
}