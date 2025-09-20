# Hello World Plugin

This is an example QuantaTerm plugin that demonstrates the basic plugin architecture and capabilities.

## Features

- **Action Registration**: Adds two actions to the command palette
- **Configuration Management**: Supports runtime configuration updates
- **Resource Management**: Demonstrates proper memory allocation/deallocation
- **Error Handling**: Shows how to handle and report errors
- **State Management**: Maintains simple plugin state

## Actions

### Say Hello (`hello_world.greet`)
- **Shortcut**: `Ctrl+Alt+H`
- **Description**: Displays a greeting message with optional timestamp
- **Configuration**: Respects `default_greeting`, `show_timestamp`, and `max_greetings` settings

### Plugin Info (`hello_world.info`)
- **Description**: Shows detailed information about the plugin
- **Returns**: Plugin metadata, current state, and statistics

## Configuration

The plugin supports the following configuration options in `plugin.toml`:

```toml
[config]
default_greeting = "Hello from QuantaTerm!"  # Default greeting message
show_timestamp = true                        # Include greeting count in message
max_greetings = 10                          # Maximum number of greetings before limit
```

## Building

### Prerequisites
```bash
# Install Rust WASM target
rustup target add wasm32-wasi

# Install WASM tools (optional)
cargo install wasm-pack
```

### Compile for WASM
```bash
# Build for WASM target
cargo build --target wasm32-wasi --release

# The output will be in target/wasm32-wasi/release/hello_world_plugin.wasm
```

### Using wasm-pack (alternative)
```bash
# Build with wasm-pack for web deployment
wasm-pack build --target web --out-dir pkg
```

## Installation

1. Copy the built `.wasm` file and `plugin.toml` to your QuantaTerm plugins directory:
   ```bash
   cp target/wasm32-wasi/release/hello_world_plugin.wasm ~/.config/quantaterm/plugins/hello-world/
   cp plugin.toml ~/.config/quantaterm/plugins/hello-world/
   ```

2. Restart QuantaTerm or reload plugins to load the new plugin

3. Open the command palette and look for "Say Hello" and "Plugin Info" actions

## Testing

```bash
# Run unit tests
cargo test

# Test WASM compilation
cargo check --target wasm32-wasi
```

## Plugin API Reference

This plugin demonstrates the core plugin API functions:

### Required Exports

- `plugin_main()` - Called when plugin is loaded
- `get_actions()` - Returns available actions for registration
- `execute_action(action_id, args)` - Handles action execution
- `plugin_cleanup()` - Called when plugin is unloaded

### Optional Exports

- `update_config(config)` - Handle configuration updates
- `allocate(size)` - Memory allocation for host communication
- `deallocate(ptr, size)` - Memory deallocation

### Host Imports

The plugin can call these host functions (when available):

- `quantaterm.get_current_command()` - Get the current terminal command
- `quantaterm.add_palette_action(name, desc)` - Register palette action
- `quantaterm.log_message(level, message)` - Send log message to host

## Capabilities

This plugin requests the following capabilities:

- `palette.add_action` - Add actions to the command palette

For more capabilities, see the [Plugin Development Guide](../../../docs/plugin_dev.md).

## Troubleshooting

### Plugin Not Loading
- Check that the `.wasm` file is in the correct plugins directory
- Verify `plugin.toml` is valid and in the same directory
- Check QuantaTerm logs for loading errors

### Actions Not Appearing
- Ensure the plugin has `palette.add_action` capability
- Verify `get_actions()` returns valid JSON
- Check that action IDs are unique

### Memory Issues
- Make sure `allocate()` and `deallocate()` are properly implemented
- Avoid memory leaks by properly managing allocated strings
- Use `std::mem::forget()` carefully to prevent premature deallocation

## Development Tips

1. **Keep it Simple**: Start with basic functionality and add features incrementally
2. **Error Handling**: Always return proper error messages for debugging
3. **Memory Management**: Be careful with raw pointers and memory allocation
4. **Testing**: Write unit tests for your plugin logic
5. **Logging**: Use the host logging API for debugging

## Next Steps

To create your own plugin:

1. Copy this template to a new directory
2. Update `plugin.toml` with your plugin details
3. Modify `src/lib.rs` to implement your functionality
4. Update the capability requirements as needed
5. Test thoroughly before deployment

For more advanced features, see the other example plugins and the [Plugin Development Guide](../../../docs/plugin_dev.md).