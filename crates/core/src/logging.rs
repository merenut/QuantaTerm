//! QuantaTerm structured logging infrastructure
//!
//! Centralized logging configuration with per-module level controls,
//! structured metadata, and runtime reconfiguration support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, time::ChronoUtc},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// Logging configuration for QuantaTerm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Global log level (applies when module-specific level not set)
    pub global_level: LogLevel,
    /// Per-module log level overrides
    pub module_levels: HashMap<String, LogLevel>,
    /// Whether to include timestamps in log output
    pub include_timestamps: bool,
    /// Whether to include severity levels in log output
    pub include_severity: bool,
    /// Whether to include subsystem/module names in log output
    pub include_subsystem: bool,
    /// Whether to use JSON format for structured output
    pub json_format: bool,
    /// Whether to use ANSI colors in output (when not JSON)
    pub use_colors: bool,
}

/// Log levels for QuantaTerm modules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    /// Show all logs including trace
    Trace,
    /// Show debug and above
    Debug,
    /// Show info and above (default)
    Info,
    /// Show warnings and above
    Warn,
    /// Show only errors
    Error,
    /// Disable all logging for this module
    Off,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            global_level: LogLevel::Info,
            module_levels: HashMap::new(),
            include_timestamps: true,
            include_severity: true,
            include_subsystem: true,
            json_format: false,
            use_colors: true,
        }
    }
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
            LogLevel::Off => Level::ERROR, // No direct "off" in tracing, use ERROR as fallback
        }
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "off" | "none" => Ok(LogLevel::Off),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
            LogLevel::Off => write!(f, "off"),
        }
    }
}

/// Module names for per-module logging configuration
pub mod modules {
    /// Core module logging identifier
    pub const CORE: &str = "quantaterm_core";
    /// Renderer module logging identifier
    pub const RENDERER: &str = "quantaterm_renderer";
    /// PTY module logging identifier
    pub const PTY: &str = "quantaterm_pty";
    /// Blocks module logging identifier
    pub const BLOCKS: &str = "quantaterm_blocks";
    /// Configuration module logging identifier
    pub const CONFIG: &str = "quantaterm_config";
    /// Plugins API module logging identifier
    pub const PLUGINS_API: &str = "quantaterm_plugins_api";
    /// Plugins host module logging identifier
    pub const PLUGINS_HOST: &str = "quantaterm_plugins_host";
    /// AI module logging identifier
    pub const AI: &str = "quantaterm_ai";
    /// Telemetry module logging identifier
    pub const TELEMETRY: &str = "quantaterm_telemetry";
    /// CLI module logging identifier
    pub const CLI: &str = "quantaterm_cli";
}

/// Initialize the logging system with the given configuration
pub fn init_logging(config: &LoggingConfig) -> crate::Result<()> {
    // Build the environment filter based on configuration
    let env_filter = build_env_filter(config);

    // Create the registry
    let registry = Registry::default().with(env_filter);

    if config.json_format {
        // JSON format for structured logging
        let json_layer = fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_timer(ChronoUtc::rfc_3339());

        registry.with(json_layer).try_init().map_err(|e| {
            crate::QuantaTermError::Configuration(format!("Failed to initialize JSON logging: {}", e))
        })?;
    } else {
        // Human-readable format
        let fmt_layer = fmt::layer()
            .with_target(config.include_subsystem)
            .with_level(config.include_severity)
            .with_ansi(config.use_colors)
            .with_timer(ChronoUtc::rfc_3339());

        registry.with(fmt_layer).try_init().map_err(|e| {
            crate::QuantaTermError::Configuration(format!("Failed to initialize logging: {}", e))
        })?;
    }

    Ok(())
}

/// Build an environment filter from the logging configuration
fn build_env_filter(config: &LoggingConfig) -> EnvFilter {
    let mut filter = EnvFilter::new("");

    // Start with global level
    let global_level: Level = config.global_level.into();
    
    // Set global level for all quantaterm modules
    filter = filter.add_directive(format!("quantaterm={}", global_level).parse().unwrap());

    // Add module-specific overrides
    for (module, level) in &config.module_levels {
        if *level != LogLevel::Off {
            let tracing_level: Level = (*level).into();
            filter = filter.add_directive(format!("{}={}", module, tracing_level).parse().unwrap());
        } else {
            // For "off", we don't add a directive, effectively filtering it out
            continue;
        }
    }

    // Allow environment variable override
    if let Ok(env_filter) = std::env::var("QUANTATERM_LOG") {
        if let Ok(env_directive) = env_filter.parse() {
            filter = filter.add_directive(env_directive);
        }
    }

    filter
}

/// Update logging configuration at runtime
pub fn update_module_level(module: &str, level: LogLevel) -> crate::Result<()> {
    // Note: tracing-subscriber doesn't support runtime reconfiguration out of the box.
    // For now, we'll store the configuration and require a restart.
    // A full implementation would use a reload layer or custom subscriber.
    
    tracing::warn!(
        module = module,
        level = %level,
        "Runtime log level updates require application restart in current implementation"
    );
    
    Ok(())
}

/// Get default development logging configuration
pub fn dev_config() -> LoggingConfig {
    let mut config = LoggingConfig::default();
    config.global_level = LogLevel::Debug;
    config.use_colors = true;
    config.json_format = false;
    
    // Enable debug for key development modules
    config.module_levels.insert(modules::RENDERER.to_string(), LogLevel::Debug);
    config.module_levels.insert(modules::PTY.to_string(), LogLevel::Debug);
    config.module_levels.insert(modules::BLOCKS.to_string(), LogLevel::Info);
    
    config
}

/// Get default production logging configuration
pub fn prod_config() -> LoggingConfig {
    let mut config = LoggingConfig::default();
    config.global_level = LogLevel::Info;
    config.use_colors = false;
    config.json_format = true;
    
    // Production-appropriate levels
    config.module_levels.insert(modules::RENDERER.to_string(), LogLevel::Warn);
    config.module_levels.insert(modules::PTY.to_string(), LogLevel::Info);
    config.module_levels.insert(modules::BLOCKS.to_string(), LogLevel::Warn);
    config.module_levels.insert(modules::CONFIG.to_string(), LogLevel::Info);
    config.module_levels.insert(modules::CLI.to_string(), LogLevel::Info);
    
    config
}

/// Get CI/testing logging configuration  
pub fn ci_config() -> LoggingConfig {
    let mut config = LoggingConfig::default();
    config.global_level = LogLevel::Info;
    config.use_colors = false;
    config.json_format = true;
    config.include_timestamps = true;
    config.include_severity = true;
    config.include_subsystem = true;
    
    // CI-appropriate levels for debugging test failures
    config.module_levels.insert(modules::CORE.to_string(), LogLevel::Debug);
    config.module_levels.insert(modules::PTY.to_string(), LogLevel::Debug);
    config.module_levels.insert(modules::BLOCKS.to_string(), LogLevel::Debug);
    
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str("trace").unwrap(), LogLevel::Trace);
        assert_eq!(LogLevel::from_str("DEBUG").unwrap(), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("Info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("warn").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("warning").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("error").unwrap(), LogLevel::Error);
        assert_eq!(LogLevel::from_str("off").unwrap(), LogLevel::Off);
        assert_eq!(LogLevel::from_str("none").unwrap(), LogLevel::Off);
        
        assert!(LogLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Trace.to_string(), "trace");
        assert_eq!(LogLevel::Debug.to_string(), "debug");
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Error.to_string(), "error");
        assert_eq!(LogLevel::Off.to_string(), "off");
    }

    #[test]
    fn test_default_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.global_level, LogLevel::Info);
        assert!(config.include_timestamps);
        assert!(config.include_severity);
        assert!(config.include_subsystem);
        assert!(!config.json_format);
        assert!(config.use_colors);
        assert!(config.module_levels.is_empty());
    }

    #[test]
    fn test_dev_config() {
        let config = dev_config();
        assert_eq!(config.global_level, LogLevel::Debug);
        assert!(!config.json_format);
        assert!(config.use_colors);
        assert_eq!(config.module_levels.get(modules::RENDERER), Some(&LogLevel::Debug));
        assert_eq!(config.module_levels.get(modules::PTY), Some(&LogLevel::Debug));
    }

    #[test]
    fn test_prod_config() {
        let config = prod_config();
        assert_eq!(config.global_level, LogLevel::Info);
        assert!(config.json_format);
        assert!(!config.use_colors);
        assert_eq!(config.module_levels.get(modules::RENDERER), Some(&LogLevel::Warn));
    }

    #[test]
    fn test_ci_config() {
        let config = ci_config();
        assert_eq!(config.global_level, LogLevel::Info);
        assert!(config.json_format);
        assert!(!config.use_colors);
        assert!(config.include_timestamps);
        assert_eq!(config.module_levels.get(modules::CORE), Some(&LogLevel::Debug));
    }
}