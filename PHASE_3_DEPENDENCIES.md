# Phase 3 Dependencies and Prerequisites

## Required Cargo Dependencies

Add these to your `Cargo.toml` files for Phase 3 implementation:

### Plugins Host Crate Dependencies
```toml
# crates/plugins-host/Cargo.toml
[dependencies]
# Existing dependencies (already in workspace)
anyhow = { workspace = true }
tracing = { workspace = true }
serde = { workspace = true }
tokio = { workspace = true }

# New dependencies for Phase 3
wasmtime = "25.0"           # WASM runtime engine
wasmtime-wasi = "25.0"      # WASI support for sandboxing
thiserror = "1.0"           # Error handling
regex = "1.10"              # Pattern matching for capabilities
notify = "6.0"              # File system watching for plugin hot-reload
uuid = "1.10"               # Plugin instance identification

[target.'cfg(unix)'.dependencies]
nix = "0.27"                # Unix-specific resource monitoring

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["processthreadsapi", "memoryapi"] }
```

### Plugins API Crate Dependencies
```toml
# crates/plugins-api/Cargo.toml
[dependencies]
# Existing dependencies
serde = { workspace = true }
anyhow = { workspace = true }

# New dependencies for Phase 3
serde_json = "1.0"          # JSON serialization for plugin communication
chrono = { workspace = true, features = ["serde"] }  # Time handling
bitflags = "2.4"            # Capability flags
```

### AI Crate Dependencies
```toml
# crates/ai/Cargo.toml
[dependencies]
# Existing dependencies
anyhow = { workspace = true }
tracing = { workspace = true }
serde = { workspace = true }
tokio = { workspace = true }

# New dependencies for Phase 3
async-trait = "0.1"         # Async trait support
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }  # HTTP client for AI APIs
regex = "1.10"              # Secret detection patterns
once_cell = "1.19"          # Lazy static initialization
ratelimit = "0.9"           # Rate limiting for AI requests
base64 = "0.22"             # Base64 encoding for API keys
url = "2.5"                 # URL parsing and validation
futures = "0.3"             # Async utilities
```

### Config Crate Dependencies (Extensions)
```toml
# crates/config/Cargo.toml - Additional dependencies for Phase 3
[dependencies]
# Existing dependencies from Phase 2
# ... (keep existing)

# New dependencies for Phase 3
jsonschema = "0.17"         # JSON schema validation for themes
colorsys = "0.6"            # Color space conversions for theming
palette = "0.7"             # Advanced color handling
```

## System Dependencies

### Linux (Ubuntu/Debian)
```bash
# WASM compilation tools
sudo apt update
sudo apt install -y build-essential pkg-config

# For plugin development
curl https://wasmtime.dev/install.sh -sSf | bash

# Add to PATH in ~/.bashrc or ~/.zshrc
export PATH="$HOME/.wasmtime/bin:$PATH"
```

### macOS
```bash
# WASM tools via Homebrew
brew install wasmtime

# Xcode Command Line Tools (if not already installed)
xcode-select --install
```

### Windows
```powershell
# Install via winget
winget install Bytecodealliance.Wasmtime

# Or download from https://wasmtime.dev/
```

## Development Tools

### WASM Plugin Development
```bash
# Install Rust WASM target
rustup target add wasm32-wasi

# Install WASM tools
cargo install wasm-pack
cargo install cargo-wasi

# For plugin testing
cargo install wasmtime-cli
```

### AI Development Tools
```bash
# For testing AI integration
cargo install httpie  # HTTP testing tool

# Optional: Install jq for JSON processing in scripts
# Ubuntu/Debian: sudo apt install jq
# macOS: brew install jq
# Windows: winget install jqlang.jq
```

## Plugin Development Setup

### Example Plugin Workspace
```toml
# Cargo.toml for plugin development
[package]
name = "example-quantaterm-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
quantaterm-plugins-api = { path = "../quantaterm/crates/plugins-api" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
wasm-bindgen = "0.2"

[profile.release]
opt-level = "s"  # Optimize for size
lto = true       # Enable link-time optimization
```

### Plugin Manifest Template
```toml
# plugin.toml
[plugin]
name = "example-plugin"
version = "1.0.0"
description = "Example QuantaTerm plugin"
author = "Your Name <your.email@example.com>"
license = "MIT"
entry_point = "plugin_main"

[compatibility]
quantaterm_version = ">=0.3.0"
wasm_version = "1.0"

[capabilities]
# File system access
filesystem = [
    "read:/tmp/**",
    "write:/tmp/plugin-data/**"
]

# Network access
network = [
    "fetch:https://api.example.com/**"
]

# QuantaTerm integration
quantaterm = [
    "palette.add_action",
    "blocks.read",
    "config.read"
]
```

## Environment Configuration

### Development Environment
```bash
# Set development-specific environment variables
export QTERM_PLUGIN_DEV=1                    # Enable plugin development mode
export QTERM_PLUGIN_TIMEOUT=5000            # Longer timeouts for debugging
export QTERM_AI_MOCK=1                       # Use mock AI responses
export QTERM_LOG_LEVEL=debug                 # Verbose logging
export QTERM_PLUGIN_DIR=./dev-plugins        # Custom plugin directory
```

### AI Provider Configuration
```bash
# OpenAI configuration (optional - can be set in config file)
export OPENAI_API_KEY="your-api-key-here"
export OPENAI_BASE_URL="https://api.openai.com/v1"  # Default

# For testing with other providers
export QTERM_AI_PROVIDER="openai"           # Default provider
export QTERM_AI_TIMEOUT=10                  # Request timeout in seconds
```

### Security Configuration
```bash
# Plugin security settings
export QTERM_PLUGIN_SANDBOX=strict          # Sandbox level: strict|normal|permissive
export QTERM_SECRET_SCAN=1                  # Enable secret scanning
export QTERM_PLUGIN_MEMORY_LIMIT=16         # Memory limit in MB
export QTERM_PLUGIN_CPU_LIMIT=100           # CPU limit in milliseconds
```

## CI/CD Dependencies

### GitHub Actions Workflow Dependencies
```yaml
# .github/workflows/phase3.yml
name: Phase 3 Tests

on: [push, pull_request]

jobs:
  test-plugins:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-wasi
          
      - name: Install system dependencies
        run: |
          sudo apt update
          sudo apt install -y pkg-config
          
      - name: Install WASM tools
        run: |
          curl https://wasmtime.dev/install.sh -sSf | bash
          echo "$HOME/.wasmtime/bin" >> $GITHUB_PATH
          
      - name: Run plugin tests
        run: |
          cargo test --workspace
          cargo test -p quantaterm-plugins-host
          cargo test -p quantaterm-ai
          
      - name: Build example plugins
        run: |
          cd examples/plugins/hello-world
          cargo build --target wasm32-wasi --release
          
      - name: Test plugin loading
        run: |
          cargo run --bin quantaterm -- --test-plugin examples/plugins/hello-world/target/wasm32-wasi/release/hello_world.wasm
```

## Docker Development Environment

### Dockerfile for Plugin Development
```dockerfile
# Dockerfile.plugin-dev
FROM rust:1.80

# Install WASM tools
RUN curl https://wasmtime.dev/install.sh -sSf | bash
ENV PATH="/root/.wasmtime/bin:$PATH"

# Install WASM target
RUN rustup target add wasm32-wasi

# Install development tools
RUN cargo install wasm-pack cargo-wasi

# Set up workspace
WORKDIR /workspace
COPY . .

# Build the project
RUN cargo build --workspace

EXPOSE 3000
CMD ["cargo", "run", "--bin", "quantaterm"]
```

### Docker Compose for Development
```yaml
# docker-compose.dev.yml
version: '3.8'

services:
  quantaterm-dev:
    build:
      context: .
      dockerfile: Dockerfile.plugin-dev
    volumes:
      - .:/workspace
      - ./dev-plugins:/workspace/plugins
    environment:
      - QTERM_PLUGIN_DEV=1
      - QTERM_LOG_LEVEL=debug
      - QTERM_PLUGIN_DIR=/workspace/plugins
    ports:
      - "3000:3000"
    
  # Mock AI service for testing
  mock-ai:
    image: nginx:alpine
    volumes:
      - ./dev/mock-ai-responses:/usr/share/nginx/html
    ports:
      - "8080:80"
```

## Testing Dependencies

### Additional Test Dependencies
```toml
# Add to [dev-dependencies] in relevant crates

[dev-dependencies]
# Existing test dependencies
tokio-test = "0.4"           # Async testing utilities
tempfile = "3.8"             # Temporary files for tests
mockall = "0.12"             # Mocking framework
wiremock = "0.6"             # HTTP mocking for AI tests
criterion = "0.5"            # Benchmarking

# Phase 3 specific test dependencies
wasmtime-test = "25.0"       # WASM testing utilities
test-log = "0.2"             # Test logging
serial_test = "3.0"          # Sequential test execution
```

### Performance Testing Setup
```bash
# Install benchmarking tools
cargo install cargo-criterion

# Install flamegraph for profiling
cargo install flamegraph

# System monitoring tools (Linux)
sudo apt install -y htop iotop nethogs

# Memory debugging tools
cargo install cargo-valgrind  # Linux only
# macOS: brew install valgrind
```

## Validation Scripts

### Phase 3 Validation Script
```bash
#!/bin/bash
# scripts/validate_phase3.sh

echo "=== Phase 3 Validation ==="

# Check WASM tools
if command -v wasmtime &> /dev/null; then
    echo "✓ Wasmtime installed"
else
    echo "✗ Wasmtime not found - run: curl https://wasmtime.dev/install.sh -sSf | bash"
fi

# Check Rust WASM target
if rustup target list --installed | grep -q wasm32-wasi; then
    echo "✓ WASM target installed"
else
    echo "✗ WASM target not found - run: rustup target add wasm32-wasi"
fi

# Check dependencies
echo ""
echo "--- Dependency Check ---"

# Check for plugin dependencies
if grep -q "wasmtime" crates/plugins-host/Cargo.toml; then
    echo "✓ Wasmtime dependency present"
else
    echo "✗ Wasmtime dependency missing"
fi

if grep -q "async-trait" crates/ai/Cargo.toml; then
    echo "✓ AI dependencies present"
else
    echo "✗ AI dependencies missing"
fi

# Build check
echo ""
echo "--- Build Check ---"
if cargo check --workspace; then
    echo "✓ Workspace builds successfully"
else
    echo "✗ Build errors found"
fi

# Test check
echo ""
echo "--- Test Check ---"
if cargo test --workspace --lib; then
    echo "✓ All tests pass"
else
    echo "✗ Test failures found"
fi

echo ""
echo "Phase 3 validation complete!"
```

## IDE Configuration

### VS Code Extensions for Plugin Development
```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "ms-vscode.wasm-wasi-core",
    "ms-vscode.hexeditor",
    "bradlc.vscode-tailwindcss"
  ],
  "settings": {
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.checkOnSave.command": "check",
    "rust-analyzer.cargo.allTargets": true,
    "files.associations": {
      "*.wasm": "wasm"
    }
  }
}
```

This dependencies document provides all the necessary tools, libraries, and configuration needed for AI coding agents to successfully implement Phase 3 features.