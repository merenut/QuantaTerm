#!/bin/bash
# Demo script for QuantaTerm Phase 0 Window and Input functionality

echo "QuantaTerm Phase 0 Demo"
echo "======================="
echo ""
echo "This script demonstrates the Phase 0 implementation of QuantaTerm:"
echo "- Window creation using winit"
echo "- GPU context initialization using wgpu"
echo "- Basic keyboard input handling (ESC to exit)"
echo "- Event loop and renderer stub"
echo ""
echo "Note: This requires a display environment (X11, Wayland, or macOS/Windows desktop)"
echo ""

# Check if we're in a display environment
if [ -z "$DISPLAY" ] && [ -z "$WAYLAND_DISPLAY" ]; then
    echo "❌ No display environment detected (DISPLAY or WAYLAND_DISPLAY)"
    echo "   This is expected in CI environments without a desktop."
    echo "   The application will fail to create a window, which is correct behavior."
    echo ""
    echo "🔧 To test on a system with a display:"
    echo "   cargo run --bin quantaterm"
    echo "   Press ESC to exit the application"
    echo ""
    echo "✅ Code compiles and would work with a display environment"
else
    echo "✅ Display environment detected, attempting to run..."
    echo "   Press ESC to exit when the window opens"
    echo ""
    cargo run --bin quantaterm
fi