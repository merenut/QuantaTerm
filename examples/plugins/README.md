# QuantaTerm Plugin Examples

This directory contains example plugins that demonstrate various aspects of the QuantaTerm plugin system.

## Available Examples

### [Hello World](hello-world/)
A basic example plugin that demonstrates:
- Plugin lifecycle management
- Action registration and execution
- Configuration handling
- Memory management
- Error handling

**Capabilities Used**: `palette.add_action`

## Building Examples

To build all example plugins:

```bash
# Install WASM target if not already installed
rustup target add wasm32-wasi

# Build all examples
for plugin in */; do
    if [ -f "$plugin/Cargo.toml" ]; then
        echo "Building $plugin..."
        cd "$plugin"
        cargo build --target wasm32-wasi --release
        cd ..
    fi
done
```

## Installing Examples

Copy the built plugins to your QuantaTerm plugins directory:

```bash
# Create plugins directory if it doesn't exist
mkdir -p ~/.config/quantaterm/plugins

# Copy each plugin
for plugin in */; do
    if [ -f "$plugin/target/wasm32-wasi/release/"*.wasm ]; then
        plugin_name=$(basename "$plugin")
        echo "Installing $plugin_name..."
        
        # Create plugin directory
        mkdir -p ~/.config/quantaterm/plugins/"$plugin_name"
        
        # Copy WASM file and manifest
        cp "$plugin"/target/wasm32-wasi/release/*.wasm ~/.config/quantaterm/plugins/"$plugin_name"/
        cp "$plugin"/plugin.toml ~/.config/quantaterm/plugins/"$plugin_name"/
    fi
done
```

## Plugin Development Workflow

1. **Start with an Example**: Copy one of these examples as a starting point
2. **Modify the Manifest**: Update `plugin.toml` with your plugin details
3. **Implement Your Logic**: Modify `src/lib.rs` to add your functionality
4. **Test Locally**: Build and test your plugin
5. **Deploy**: Copy to the plugins directory and restart QuantaTerm

## Plugin Template Structure

Each plugin should have this structure:

```
my-plugin/
├── Cargo.toml          # Rust package configuration
├── plugin.toml         # Plugin manifest and metadata
├── README.md           # Plugin documentation
└── src/
    └── lib.rs          # Plugin implementation
```

## Testing Plugins

### Unit Testing
Each plugin includes unit tests that can be run with:

```bash
cd plugin-directory
cargo test
```

### Integration Testing
Test plugins with QuantaTerm:

```bash
# Load plugin in development mode
quantaterm --plugin-dev path/to/plugin.wasm

# Or install and restart QuantaTerm
```

### WASM Validation
Validate WASM output:

```bash
# Check WASM file validity
wasmtime validate target/wasm32-wasi/release/plugin.wasm

# Inspect WASM exports
wasmtime inspect target/wasm32-wasi/release/plugin.wasm
```

## Common Plugin Patterns

### Action-Based Plugins
Plugins that add commands to the palette:
- Implement `get_actions()` to register actions
- Handle action execution in `execute_action()`
- Request `palette.add_action` capability

### Data Processing Plugins
Plugins that process terminal data:
- Request `blocks.read` or `blocks.write` capabilities
- Implement data transformation logic
- Handle streaming data efficiently

### Network-Enabled Plugins
Plugins that make external API calls:
- Request appropriate `network.fetch` capabilities
- Implement proper error handling for network failures
- Consider rate limiting and caching

### Configuration-Driven Plugins
Plugins with extensive configuration:
- Define configuration schema in `plugin.toml`
- Implement `update_config()` for runtime updates
- Validate configuration on load

## Security Considerations

1. **Minimal Capabilities**: Only request capabilities your plugin actually needs
2. **Input Validation**: Validate all input from the host and user
3. **Error Handling**: Don't expose sensitive information in error messages
4. **Resource Limits**: Be mindful of memory and CPU usage
5. **Safe Defaults**: Use secure defaults for configuration options

## Performance Tips

1. **Optimize for Size**: Use `opt-level = "s"` and `lto = true` in release builds
2. **Minimize Allocations**: Reuse buffers and avoid unnecessary allocations
3. **Lazy Loading**: Load resources only when needed
4. **Efficient Serialization**: Use efficient formats for data exchange
5. **Profile Regularly**: Use profiling tools to identify bottlenecks

## Debugging Plugins

### Logging
Use the host logging API:
```rust
// Log at different levels
log_message(0, "Trace message");
log_message(1, "Debug message");  
log_message(2, "Info message");
log_message(3, "Warning message");
log_message(4, "Error message");
```

### Error Reporting
Return detailed error information:
```rust
ActionResult::error(format!("Failed to process input: {}", error))
```

### WASM Debugging
- Use `wasmtime` CLI for testing WASM modules
- Enable debug symbols during development
- Use WASM debugging tools in browsers

## Contributing

To contribute a new example plugin:

1. Create a new directory with a descriptive name
2. Follow the established structure and conventions
3. Include comprehensive documentation
4. Add unit tests
5. Update this README with your example

## Resources

- [Plugin Development Guide](../../docs/plugin_dev.md)
- [QuantaTerm Plugin API Reference](../../docs/api/)
- [WASM Documentation](https://wasmtime.dev/)
- [Rust WASM Book](https://rustwasm.github.io/docs/book/)

## Support

For questions about plugin development:
- Check the [Plugin Development Guide](../../docs/plugin_dev.md)
- Review existing examples for patterns
- Open an issue in the QuantaTerm repository
- Join the QuantaTerm community discussions