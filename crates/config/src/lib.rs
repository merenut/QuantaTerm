//! QuantaTerm Configuration management and parsing
//!
//! Configuration management and parsing with structured logging support.

#![warn(missing_docs)]
#![deny(unsafe_code)]

use quantaterm_core::logging::{LoggingConfig, LogLevel};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn, error, instrument};

/// Main configuration structure for QuantaTerm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Terminal configuration
    pub terminal: TerminalConfig,
    /// Rendering configuration
    pub renderer: RendererConfig,
}

/// Terminal-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Default number of columns
    pub default_cols: u16,
    /// Default number of rows
    pub default_rows: u16,
    /// Maximum scrollback lines
    pub max_scrollback: usize,
    /// Shell command override
    pub shell_command: Option<String>,
}

/// Renderer-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererConfig {
    /// Enable VSync
    pub vsync: bool,
    /// Target FPS (when VSync disabled)
    pub target_fps: u32,
    /// Font size
    pub font_size: f32,
    /// Font family
    pub font_family: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            logging: LoggingConfig::default(),
            terminal: TerminalConfig::default(),
            renderer: RendererConfig::default(),
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            default_cols: 80,
            default_rows: 24,
            max_scrollback: 10000,
            shell_command: None,
        }
    }
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            vsync: true,
            target_fps: 60,
            font_size: 14.0,
            font_family: "monospace".to_string(),
        }
    }
}

impl Config {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from file
    #[instrument(name = "config_load", skip(path))]
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> quantaterm_core::Result<Self> {
        let path = path.as_ref();
        info!(
            subsystem = "config", 
            config_file = ?path,
            "Loading configuration from file"
        );

        let content = std::fs::read_to_string(path)
            .map_err(|e| {
                error!(
                    subsystem = "config",
                    config_file = ?path,
                    error = %e,
                    "Failed to read configuration file"
                );
                quantaterm_core::QuantaTermError::Configuration(
                    format!("Failed to read config file '{}': {}", path.display(), e)
                )
            })?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| {
                error!(
                    subsystem = "config",
                    config_file = ?path,
                    error = %e,
                    "Failed to parse configuration file"
                );
                quantaterm_core::QuantaTermError::Configuration(
                    format!("Failed to parse config file '{}': {}", path.display(), e)
                )
            })?;

        debug!(
            subsystem = "config",
            config_file = ?path,
            logging_level = ?config.logging.global_level,
            terminal_cols = config.terminal.default_cols,
            terminal_rows = config.terminal.default_rows,
            "Configuration loaded successfully"
        );

        Ok(config)
    }

    /// Save configuration to file
    #[instrument(name = "config_save", skip(self, path))]
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> quantaterm_core::Result<()> {
        let path = path.as_ref();
        info!(
            subsystem = "config",
            config_file = ?path,
            "Saving configuration to file"
        );

        let content = toml::to_string_pretty(self)
            .map_err(|e| {
                error!(
                    subsystem = "config",
                    error = %e,
                    "Failed to serialize configuration"
                );
                quantaterm_core::QuantaTermError::Configuration(
                    format!("Failed to serialize config: {}", e)
                )
            })?;

        std::fs::write(path, content)
            .map_err(|e| {
                error!(
                    subsystem = "config",
                    config_file = ?path,
                    error = %e,
                    "Failed to write configuration file"
                );
                quantaterm_core::QuantaTermError::Configuration(
                    format!("Failed to write config file '{}': {}", path.display(), e)
                )
            })?;

        debug!(
            subsystem = "config",
            config_file = ?path,
            "Configuration saved successfully"
        );

        Ok(())
    }

    /// Get default configuration file path
    pub fn default_config_path() -> quantaterm_core::Result<PathBuf> {
        let config_dir = if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("quantaterm")
        } else {
            warn!(
                subsystem = "config",
                "No standard config directory found, using current directory"
            );
            PathBuf::from(".")
        };

        Ok(config_dir.join("config.toml"))
    }

    /// Load configuration with fallback to defaults
    #[instrument(name = "config_load_or_default")]
    pub fn load_or_default() -> Self {
        match Self::default_config_path() {
            Ok(path) => {
                if path.exists() {
                    match Self::load_from_file(&path) {
                        Ok(config) => {
                            info!(
                                subsystem = "config",
                                config_file = ?path,
                                "Configuration loaded from file"
                            );
                            config
                        }
                        Err(e) => {
                            warn!(
                                subsystem = "config",
                                config_file = ?path,
                                error = %e,
                                "Failed to load config file, using defaults"
                            );
                            Self::default()
                        }
                    }
                } else {
                    debug!(
                        subsystem = "config",
                        config_file = ?path,
                        "Configuration file does not exist, using defaults"
                    );
                    Self::default()
                }
            }
            Err(e) => {
                warn!(
                    subsystem = "config",
                    error = %e,
                    "Failed to determine config path, using defaults"
                );
                Self::default()
            }
        }
    }

    /// Update logging level for a specific module
    #[instrument(name = "config_update_log_level", skip(self))]
    pub fn update_log_level(&mut self, module: &str, level: LogLevel) {
        debug!(
            subsystem = "config",
            module = module,
            level = ?level,
            "Updating log level for module"
        );
        self.logging.module_levels.insert(module.to_string(), level);
    }

    /// Validate configuration values
    #[instrument(name = "config_validate", skip(self))]
    pub fn validate(&self) -> quantaterm_core::Result<()> {
        debug!(subsystem = "config", "Validating configuration");

        // Validate terminal dimensions
        if self.terminal.default_cols == 0 || self.terminal.default_rows == 0 {
            error!(
                subsystem = "config",
                cols = self.terminal.default_cols,
                rows = self.terminal.default_rows,
                "Invalid terminal dimensions"
            );
            return Err(quantaterm_core::QuantaTermError::Configuration(
                "Terminal dimensions must be greater than 0".to_string()
            ));
        }

        // Validate renderer settings
        if self.renderer.font_size <= 0.0 {
            error!(
                subsystem = "config",
                font_size = self.renderer.font_size,
                "Invalid font size"
            );
            return Err(quantaterm_core::QuantaTermError::Configuration(
                "Font size must be greater than 0".to_string()
            ));
        }

        if self.renderer.target_fps == 0 {
            error!(
                subsystem = "config",
                target_fps = self.renderer.target_fps,
                "Invalid target FPS"
            );
            return Err(quantaterm_core::QuantaTermError::Configuration(
                "Target FPS must be greater than 0".to_string()
            ));
        }

        debug!(subsystem = "config", "Configuration validation passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.terminal.default_cols, 80);
        assert_eq!(config.terminal.default_rows, 24);
        assert_eq!(config.renderer.font_size, 14.0);
        assert!(config.renderer.vsync);
    }

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = Config::default();
        invalid_config.terminal.default_cols = 0;
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = Config::default();
        invalid_config.renderer.font_size = -1.0;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_log_level_update() {
        let mut config = Config::default();
        config.update_log_level("test_module", LogLevel::Debug);
        assert_eq!(
            config.logging.module_levels.get("test_module"),
            Some(&LogLevel::Debug)
        );
    }
}
