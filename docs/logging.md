# QuantaTerm Logging Patterns and Conventions

This document outlines the structured logging patterns and conventions used in QuantaTerm. Our logging infrastructure is built on the `tracing` crate and provides rich structured logging with per-module level controls and runtime configuration.

## Overview

QuantaTerm uses structured logging to provide:
- **Structured metadata**: All logs include timestamps, severity levels, and subsystem identifiers
- **Per-module controls**: Each subsystem can have independent log level configuration
- **Runtime reconfiguration**: Log levels can be adjusted during application runtime
- **Performance optimization**: Non-blocking, efficient logging that doesn't degrade main event loop
- **Multiple output formats**: Human-readable and JSON formats for different environments

## Core Principles

### 1. Structured Information
Use structured fields instead of string interpolation:

```rust
// Good - structured fields
info!(
    subsystem = "renderer",
    width = size.width,
    height = size.height,
    "Initializing GPU renderer"
);

// Avoid - string interpolation  
info!("Initializing GPU renderer with size: {}x{}", size.width, size.height);
```

### 2. Consistent Subsystem Identification
Always include a `subsystem` field to identify the module:

```rust
debug!(subsystem = "pty", "Processing command");
```

Subsystem identifiers are defined in `quantaterm_core::logging::modules`:
- `core` - Foundation layer operations
- `renderer` - GPU rendering operations  
- `pty` - Shell communication and I/O
- `blocks` - Terminal grid and cell management
- `config` - Configuration management
- `plugins_api` / `plugins_host` - Plugin system
- `ai` - AI integration 
- `telemetry` - Metrics collection
- `cli` - Application lifecycle

### 3. Appropriate Log Levels

Use log levels appropriately:

- **`trace`**: Fine-grained debugging information, high-frequency events
- **`debug`**: Debugging information for development 
- **`info`**: General application flow and state changes
- **`warn`**: Potentially problematic conditions that don't prevent operation
- **`error`**: Error conditions that impact functionality

```rust
// Application lifecycle events
info!(subsystem = "cli", version = VERSION, "Starting QuantaTerm");

// Development debugging
debug!(subsystem = "renderer", "Created wgpu instance");

// High frequency tracing
trace!(subsystem = "pty", bytes_read = count, "Read data from shell");

// Warnings for recoverable issues
warn!(subsystem = "renderer", "Ignoring invalid resize request");

// Errors for failures
error!(subsystem = "pty", error = %e, "Failed to write to shell");
```

### 4. Error Context
When logging errors, include the error and relevant context:

```rust
error!(
    subsystem = "pty",
    error = %e,
    command = ?cmd,
    "Failed to spawn shell process"
);
```

### 5. Performance-Critical Paths
Use `#[instrument]` for function tracing in performance-critical areas:

```rust
#[instrument(name = "renderer_resize", skip(self))]
pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
    // implementation
}
```

## Configuration

### Environment-Based Configuration

Set the `QUANTATERM_LOG` environment variable for custom filtering:

```bash
# Enable debug for specific modules
export QUANTATERM_LOG="quantaterm_renderer=debug,quantaterm_pty=trace"

# Global debug with specific module override
export QUANTATERM_LOG="debug,quantaterm_blocks=info"
```

### Code-Based Configuration

Use predefined configurations for different environments:

```rust
use quantaterm_core::logging::{self, dev_config, prod_config, ci_config};

// Development with debug output
let config = dev_config();
logging::init_logging(&config)?;

// Production with JSON output
let config = prod_config();
logging::init_logging(&config)?;

// CI with structured output for analysis
let config = ci_config();
logging::init_logging(&config)?;
```

### Custom Configuration

Create custom configurations:

```rust
use quantaterm_core::logging::{LoggingConfig, LogLevel, modules};

let mut config = LoggingConfig::default();
config.global_level = LogLevel::Info;
config.json_format = true;
config.module_levels.insert(modules::PTY.to_string(), LogLevel::Debug);
config.module_levels.insert(modules::RENDERER.to_string(), LogLevel::Warn);

logging::init_logging(&config)?;
```

## Output Formats

### Human-Readable Format (Development)
```
2024-01-15T10:30:45.123Z  INFO quantaterm_cli: Starting QuantaTerm version="0.1.0"
2024-01-15T10:30:45.124Z DEBUG quantaterm_renderer: Created wgpu instance subsystem="renderer"
2024-01-15T10:30:45.125Z  INFO quantaterm_pty: Shell session started successfully subsystem="pty" width=80 height=24
```

### JSON Format (Production/CI)
```json
{"timestamp":"2024-01-15T10:30:45.123Z","level":"INFO","target":"quantaterm_cli","fields":{"message":"Starting QuantaTerm","version":"0.1.0"}}
{"timestamp":"2024-01-15T10:30:45.124Z","level":"DEBUG","target":"quantaterm_renderer","fields":{"message":"Created wgpu instance","subsystem":"renderer"}}
{"timestamp":"2024-01-15T10:30:45.125Z","level":"INFO","target":"quantaterm_pty","fields":{"message":"Shell session started successfully","subsystem":"pty","width":80,"height":24}}
```

## Module-Specific Patterns

### Renderer Module
Focus on GPU operations, performance metrics, and resource management:

```rust
info!(
    subsystem = "renderer",
    adapter_name = ?adapter.get_info().name,
    surface_format = ?surface_format,
    "GPU adapter selected"
);

trace!(
    subsystem = "renderer", 
    frame_time_ms = frame_duration.as_millis(),
    "Frame rendered"
);
```

### PTY Module  
Log shell interactions, I/O operations, and process lifecycle:

```rust
info!(
    subsystem = "pty",
    shell_command = ?cmd,
    "Spawning shell process"
);

debug!(
    subsystem = "pty",
    byte_count = data.len(),
    "Writing data to shell"
);
```

### Configuration Module
Log configuration changes and validation:

```rust
info!(
    subsystem = "config",
    config_file = ?path,
    "Loading configuration"
);

warn!(
    subsystem = "config",
    key = "log_level",
    value = ?invalid_value,
    "Invalid configuration value, using default"
);
```

## Testing and CI Integration

### Testing Log Output
Tests can verify logging behavior:

```rust
#[test]
fn test_logging_configuration() {
    let config = dev_config();
    assert_eq!(config.global_level, LogLevel::Debug);
    assert!(!config.json_format);
    assert!(config.use_colors);
}
```

### CI Verification
CI pipelines can verify log format and required fields:

```bash
# Run tests with JSON logging and verify format
QUANTATERM_LOG="info" cargo test 2>&1 | jq '.timestamp, .level, .target, .fields'
```

## Performance Considerations

### Non-Blocking Operations
- All logging operations are non-blocking
- Uses efficient structured data representation
- Minimal allocation in hot paths

### Conditional Logging
Use tracing's built-in level checking for expensive operations:

```rust
// Only construct expensive debug information if debug level is enabled
if tracing::enabled!(tracing::Level::DEBUG) {
    let expensive_debug_info = compute_debug_info();
    debug!(subsystem = "module", debug_info = ?expensive_debug_info, "Debug information");
}
```

### Instrumentation Guidelines
- Use `#[instrument]` sparingly on hot paths
- Skip large data structures with `skip` parameter
- Use `name` parameter for cleaner span names

## Best Practices Summary

1. **Always use structured fields** instead of string formatting
2. **Include subsystem identifier** in all log messages
3. **Use appropriate log levels** based on message importance
4. **Include error context** when logging failures
5. **Use `#[instrument]` judiciously** for function tracing
6. **Test logging configuration** in different environments
7. **Monitor performance impact** of logging in hot paths
8. **Document module-specific logging patterns** as they evolve

## Future Enhancements

- **Runtime log level updates**: Full support for changing levels without restart
- **Distributed tracing**: Integration with OpenTelemetry for distributed systems
- **Log aggregation**: Integration with centralized logging systems
- **Performance monitoring**: Built-in metrics for logging overhead