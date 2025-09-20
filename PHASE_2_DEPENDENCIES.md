# Phase 2 Dependencies and Prerequisites

## Required Cargo Dependencies

Add these to your `Cargo.toml` files for Phase 2 implementation:

### Renderer Crate Dependencies
```toml
# crates/renderer/Cargo.toml
[dependencies]
# Existing dependencies (already in workspace)
wgpu = { workspace = true }
winit = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
bitflags = { workspace = true }

# New dependencies for Phase 2
ab-glyph = "0.2"           # Font loading and glyph metrics
harfbuzz_rs = "2.0"        # Text shaping
rusttype = "0.9"           # Alternative font loading
bit-vec = "0.6"            # Efficient dirty tracking
fontconfig = "0.8"         # Linux font discovery
core-text = "20.1"         # macOS font discovery (macOS only)
dwrote = "0.11"            # Windows font discovery (Windows only)

[target.'cfg(target_os = "linux")'.dependencies]
fontconfig = "0.8"

[target.'cfg(target_os = "macos")'.dependencies]
core-text = "20.1"

[target.'cfg(target_os = "windows")'.dependencies]
dwrote = "0.11"
```

### Config Crate Dependencies
```toml
# crates/config/Cargo.toml
[dependencies]
# Existing dependencies
serde = { workspace = true }
toml = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
dirs = { workspace = true }

# New dependencies for Phase 2
notify = "6.0"             # File system watching
tokio = { workspace = true, features = ["sync", "time"] }
```

### Core Crate Dependencies
```toml
# crates/core/Cargo.toml
[dependencies]
# Add fuzzy search for command palette
fuzzy-matcher = "0.3"
uuid = { version = "1.0", features = ["v4"] }
```

### PTY Crate Dependencies  
```toml
# crates/pty/Cargo.toml
[dependencies]
# Existing dependencies
portable-pty = { workspace = true }
vte = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

# New dependencies for Phase 2
regex = "1.10"             # Shell prompt detection
chrono = { workspace = true } # Timestamp tracking
```

### Dev Dependencies for Testing
```toml
# Add to workspace Cargo.toml [workspace.dependencies]
criterion = "0.5"          # Performance benchmarking  
tempfile = "3.8"           # Temporary files for tests
```

## System Dependencies

### Linux (Ubuntu/Debian)
```bash
sudo apt-get install -y \
    libfontconfig1-dev \
    libfreetype6-dev \
    libharfbuzz-dev \
    pkg-config
```

### Linux (Fedora/CentOS/RHEL)
```bash
sudo dnf install -y \
    fontconfig-devel \
    freetype-devel \
    harfbuzz-devel \
    pkgconfig
```

### macOS
```bash
# Using Homebrew
brew install harfbuzz freetype pkg-config

# Or using MacPorts
sudo port install harfbuzz freetype pkgconfig
```

### Windows
```powershell
# Using vcpkg
vcpkg install harfbuzz freetype

# Or download and install manually:
# - HarfBuzz: https://github.com/harfbuzz/harfbuzz/releases
# - FreeType: https://download.savannah.gnu.org/releases/freetype/
```

## Development Environment Setup

### VS Code Extensions (Recommended)
- `rust-analyzer` - Rust language support
- `crates` - Cargo.toml dependency management
- `Better TOML` - TOML syntax highlighting
- `Error Lens` - Inline error display

### Pre-commit Hooks Setup
```bash
# Install pre-commit
pip install pre-commit

# Create .pre-commit-config.yaml
cat > .pre-commit-config.yaml << EOF
repos:
  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt
        language: system
        args: [--all]
        pass_filenames: false
      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy
        language: system
        args: [--all-targets, --all-features, --, -D, warnings]
        pass_filenames: false
      - id: cargo-test
        name: cargo test
        entry: cargo test
        language: system
        args: [--workspace]
        pass_filenames: false
EOF

# Install hooks
pre-commit install
```

## Build Configuration

### Optimized Debug Builds
Add to workspace `Cargo.toml`:
```toml
[profile.dev]
opt-level = 1              # Basic optimizations for better performance
debug = true               # Keep debug info
incremental = true         # Faster rebuilds

[profile.dev.package."*"]
opt-level = 3              # Optimize dependencies fully

[profile.test]
opt-level = 1              # Optimize tests slightly for faster execution
```

### Release Profile Tuning
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true

# For benchmarking
[profile.bench]
inherits = "release"
debug = true               # Keep debug symbols for profiling
```

## CI/CD Pipeline Requirements

### GitHub Actions Matrix
Test on these platforms for Phase 2:
- Ubuntu 20.04, 22.04 (x86_64)
- macOS 12, 13 (x86_64, aarch64)  
- Windows Server 2019, 2022 (x86_64)

### Required CI Steps
1. **Setup**: Install system dependencies
2. **Cache**: Cargo registry and target directory
3. **Format**: `cargo fmt --check`
4. **Lint**: `cargo clippy --all-targets --all-features -- -D warnings`
5. **Test**: `cargo test --workspace`
6. **Benchmark**: `cargo bench` (on main branch only)
7. **Integration**: Run integration tests
8. **Audit**: `cargo audit` for security vulnerabilities

## Performance Monitoring Setup

### Criterion Benchmarks
```bash
# Run benchmarks
cargo bench

# Generate HTML reports
cargo bench -- --output-format html

# Compare against baseline
cargo bench -- --save-baseline main
cargo bench -- --baseline main
```

### Memory Profiling
```bash
# Install valgrind (Linux)
sudo apt-get install valgrind

# Run with memory checking
cargo build --release
valgrind --tool=memcheck --leak-check=full ./target/release/quantaterm

# Or use heaptrack (alternative)
heaptrack ./target/release/quantaterm
```

### CPU Profiling
```bash
# Install perf (Linux)
sudo apt-get install linux-perf

# Profile application
perf record -g ./target/release/quantaterm
perf report

# Or use flamegraph
cargo install flamegraph
sudo cargo flamegraph --bin quantaterm
```

## Development Workflow

### Recommended Development Loop
```bash
# 1. Make changes
# 2. Fast iteration cycle
cargo check                    # Quick syntax check
cargo test --lib               # Unit tests only
cargo test --workspace         # Full test suite

# 3. Before commit
cargo fmt                      # Format code
cargo clippy -- -D warnings   # Lint
cargo test --workspace         # All tests
cargo bench                    # Performance regression check

# 4. Integration testing
cargo run                      # Manual testing
```

### Debug Build Performance
For development, use these optimizations in `.cargo/config.toml`:
```toml
[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]  # Faster linking

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

## Platform-Specific Notes

### Linux Wayland Support
Ensure wgpu/winit Wayland support:
```bash
export WAYLAND_DISPLAY=wayland-0
export XDG_SESSION_TYPE=wayland
```

### macOS Metal Validation
For GPU debugging on macOS:
```bash
export METAL_DEVICE_WRAPPER_TYPE=1
export METAL_DEBUG_ERROR_MODE=0
```

### Windows DirectX Debug
For GPU debugging on Windows:
```powershell
$env:WGPU_BACKEND="dx12"
$env:RUST_LOG="wgpu=debug"
```

## Task Implementation Order for AI Agents

Based on dependencies, implement in this exact order:

### Week 1-2: Foundation (Tasks 1.1-1.3)
1. Set up font loading infrastructure
2. Integrate HarfBuzz text shaping  
3. Implement GPU glyph atlas
4. **Checkpoint**: Basic text rendering works

### Week 3: Configuration (Tasks 3.1-3.2)
1. Enhance config structure for fonts
2. Implement file watching and live reload
3. **Checkpoint**: Font changes apply without restart

### Week 4: Performance (Tasks 2.1-2.2)  
1. Implement dirty region tracking
2. Add partial rendering pipeline
3. **Checkpoint**: 30% performance improvement measured

### Week 5: Shell Integration (Tasks 5.1-5.2)
1. Add shell hook system
2. Implement boundary detection
3. **Checkpoint**: 95% command detection accuracy

### Week 6: Blocks and Palette (Tasks 4.1-4.2, 6.1-6.2)
1. Implement command blocks data model
2. Add block UI and interactions
3. Build command palette system
4. **Checkpoint**: All Phase 2 acceptance criteria met

Each checkpoint should include:
- All tests passing
- Performance targets met
- Cross-platform validation
- Documentation updated

This dependency guide ensures AI coding agents have all necessary context and prerequisites for successful Phase 2 implementation.