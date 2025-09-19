# Phase 0: Window and Input Event Implementation

This document describes the Phase 0 implementation of QuantaTerm, focusing on the foundational window management and input event handling.

## Overview

Phase 0 establishes the basic windowing and input infrastructure using:
- **winit** for cross-platform window creation and event handling
- **wgpu** for GPU context initialization and basic rendering
- **Event-driven architecture** for future extensibility

## Architecture

### Components

1. **Main Entry Point** (`crates/cli/src/main.rs`)
   - Initializes tracing/logging
   - Creates winit event loop
   - Runs the main application

2. **Application Handler** (`crates/cli/src/app.rs`)
   - Implements winit's `ApplicationHandler` trait
   - Manages window creation and lifecycle
   - Handles keyboard input events
   - Coordinates with renderer

3. **GPU Renderer** (`crates/renderer/src/lib.rs`)
   - Initializes wgpu device and surface
   - Provides basic GPU-accelerated rendering
   - Clears screen with dark blue color (placeholder for future content)

### Event Flow

```
winit EventLoop
    ↓
QuantaTermApp::window_event()
    ↓
match event {
    CloseRequested → exit(),
    KeyboardInput → handle_keyboard_input(),
    RedrawRequested → renderer.render(),
    Resized → renderer.resize(),
    ...
}
```

### Keyboard Input Handling

- **Escape Key**: Cleanly exits the application
- **Other Keys**: Logged for debugging (future terminal input processing)

## Testing

### Manual Testing
```bash
# On systems with display environment
cargo run --bin quantaterm

# Expected behavior:
# 1. Window opens (800x600, titled "QuantaTerm")
# 2. Window shows dark blue background
# 3. Pressing ESC closes the application
# 4. Window close button also exits cleanly
```

### Automated Testing
```bash
# Unit tests
cargo test

# CI-friendly test (no display required)
./scripts/demo_phase0.sh
```

### CI Behavior
In CI environments without a display:
- Application correctly fails to create event loop
- Error message clearly indicates missing display environment
- This is expected and correct behavior

## Platform Support

- **Linux**: X11 and Wayland support via winit
- **macOS**: Native Cocoa support via winit  
- **Windows**: Native Win32 support via winit

All platforms use the same wgpu backend for GPU acceleration:
- Linux: Vulkan or OpenGL
- macOS: Metal
- Windows: DirectX 12 or Vulkan

## Future Extensions

The current architecture is designed for easy extension:

1. **Terminal State**: Can be integrated into the event loop
2. **PTY Integration**: Can send input events to shell process
3. **Advanced Rendering**: Renderer can be extended for text/glyph rendering
4. **Plugin System**: Event handling can route to plugin handlers

## Dependencies

- `winit = "0.30"` - Cross-platform windowing
- `wgpu = "26.0"` - Cross-platform GPU abstraction
- `pollster = "0.3"` - Simple async executor for wgpu initialization
- `tracing = "0.1"` - Structured logging
- `anyhow = "1.0"` - Error handling

## Code Quality

- All code passes `cargo clippy` with `-D warnings`
- Formatted with `cargo fmt`
- No `unsafe` code (enforced by `#![deny(unsafe_code)]`)
- Comprehensive error handling with proper context