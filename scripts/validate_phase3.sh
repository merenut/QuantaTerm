#!/bin/bash
# Phase 3 Validation Script
# Validates Phase 3 implementation progress and dependencies

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

# Helper functions
check_pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((PASS_COUNT++))
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    ((WARN_COUNT++))
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    ((FAIL_COUNT++))
}

echo -e "${BLUE}=== Phase 3 Validation: Plugins & AI ===${NC}"
echo

# Environment checks
echo "--- Environment Setup ---"

# Check for WASM tools
if command -v wasmtime &> /dev/null; then
    WASMTIME_VERSION=$(wasmtime --version | head -n1)
    check_pass "Wasmtime installed: $WASMTIME_VERSION"
else
    check_fail "Wasmtime not found - run: curl https://wasmtime.dev/install.sh -sSf | bash"
fi

# Check Rust WASM target
if rustup target list --installed | grep -q wasm32-wasi; then
    check_pass "Rust WASM target (wasm32-wasi) installed"
else
    check_fail "WASM target not found - run: rustup target add wasm32-wasi"
fi

# Check for optional WASM tools
if command -v wasm-pack &> /dev/null; then
    check_pass "wasm-pack installed"
else
    check_warn "wasm-pack not found - install with: cargo install wasm-pack"
fi

if command -v cargo-wasi &> /dev/null; then
    check_pass "cargo-wasi installed"
else
    check_warn "cargo-wasi not found - install with: cargo install cargo-wasi"
fi

echo

# Dependency validation
echo "--- Dependency Validation ---"

# Check for plugin host dependencies
if grep -q "wasmtime.*=" crates/plugins-host/Cargo.toml 2>/dev/null; then
    check_pass "Wasmtime dependency present in plugins-host"
else
    check_warn "Wasmtime dependency not found in plugins-host - needed for Task 1"
fi

if grep -q "wasmtime-wasi.*=" crates/plugins-host/Cargo.toml 2>/dev/null; then
    check_pass "Wasmtime WASI dependency present"
else
    check_warn "Wasmtime WASI dependency not found - needed for plugin sandboxing"
fi

if grep -q "regex.*=" crates/plugins-host/Cargo.toml 2>/dev/null; then
    check_pass "Regex dependency present for capability patterns"
else
    check_warn "Regex dependency not found - needed for Task 2"
fi

if grep -q "notify.*=" crates/plugins-host/Cargo.toml 2>/dev/null; then
    check_pass "Notify dependency present for plugin hot-reload"
else
    check_warn "Notify dependency not found - needed for plugin development"
fi

# Check for AI dependencies
if grep -q "async-trait.*=" crates/ai/Cargo.toml 2>/dev/null; then
    check_pass "Async-trait dependency present"
else
    check_warn "Async-trait dependency not found - needed for Task 4"
fi

if grep -q "reqwest.*=" crates/ai/Cargo.toml 2>/dev/null; then
    check_pass "Reqwest HTTP client dependency present"
else
    check_warn "Reqwest dependency not found - needed for AI provider integration"
fi

if grep -q "regex.*=" crates/ai/Cargo.toml 2>/dev/null; then
    check_pass "Regex dependency present for secret redaction"
else
    check_warn "Regex dependency not found in AI crate - needed for Task 5"
fi

echo

# Build validation
echo "--- Build Validation ---"

echo "Building workspace..."
if cargo build --workspace --quiet; then
    check_pass "Workspace builds successfully"
else
    check_fail "Build errors found"
fi

# Check for specific Phase 3 modules
echo

echo "--- Task 1: WASM Runtime Validation ---"

# Check for runtime module structure
if [ -f "crates/plugins-host/src/runtime.rs" ] || grep -q "mod runtime" crates/plugins-host/src/lib.rs 2>/dev/null; then
    check_pass "WASM runtime module structure exists"
else
    check_warn "WASM runtime module not implemented - Task 1.1"
fi

# Check for plugin loading functionality
if grep -rq "WasmRuntime\|load_plugin" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Plugin loading infrastructure present"
else
    check_warn "Plugin loading not implemented - Task 1.2"
fi

# Check for execution limits
if grep -rq "ExecutionLimits\|ResourceMonitor" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Execution limits and monitoring present"
else
    check_warn "Execution limits not implemented - Task 1.3"
fi

echo

echo "--- Task 2: Capability System Validation ---"

# Check for capability system
if grep -rq "Capability.*enum\|CapabilitySet" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Capability system framework present"
else
    check_warn "Capability system not implemented - Task 2.1"
fi

# Check for permission checking
if grep -rq "PermissionChecker\|check_permission" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Permission checking system present"
else
    check_warn "Permission checking not implemented - Task 2.1"
fi

# Check for manifest support
if grep -rq "PluginManifest\|plugin\.toml" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Plugin manifest support present"
else
    check_warn "Plugin manifest support not implemented - Task 2.1"
fi

echo

echo "--- Task 3: Palette Extension API Validation ---"

# Check for action system
if grep -rq "Action.*struct\|ActionRegistry" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Action system present"
else
    check_warn "Action system not implemented - Task 3.1"
fi

# Check for palette integration
if grep -rq "palette.*action\|register_action" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Palette integration present"
else
    check_warn "Palette integration not implemented - Task 3.2"
fi

echo

echo "--- Task 4: AI Provider Validation ---"

# Check for AI provider trait
if grep -rq "trait.*AiProvider\|async.*fn.*explain_command" crates/ai/src/ 2>/dev/null; then
    check_pass "AI provider trait present"
else
    check_warn "AI provider trait not implemented - Task 4.1"
fi

# Check for OpenAI implementation
if grep -rq "OpenAiProvider\|openai" crates/ai/src/ 2>/dev/null; then
    check_pass "OpenAI provider implementation present"
else
    check_warn "OpenAI provider not implemented - Task 4.2"
fi

# Check for AI response types
if grep -rq "AiResponse\|CommandContext" crates/ai/src/ 2>/dev/null; then
    check_pass "AI response and context types present"
else
    check_warn "AI response types not implemented - Task 4.1"
fi

echo

echo "--- Task 5: Secret Redaction Validation ---"

# Check for secret redaction system
if grep -rq "SecretRedactor\|RedactionPattern" crates/ai/src/ 2>/dev/null; then
    check_pass "Secret redaction system present"
else
    check_warn "Secret redaction not implemented - Task 5.1"
fi

# Check for heuristics
if grep -rq "Heuristic\|AwsKey\|GithubToken" crates/ai/src/ 2>/dev/null; then
    check_pass "Secret detection heuristics present"
else
    check_warn "Secret detection heuristics not implemented - Task 5.1"
fi

echo

echo "--- Task 6: Theming System Validation ---"

# Check for theme system in config
if grep -rq "Theme.*struct\|ColorScheme" crates/config/src/ 2>/dev/null; then
    check_pass "Theme system structure present"
else
    check_warn "Theme system not implemented - Task 6.1"
fi

# Check for theme loading
if grep -rq "load_theme\|import_theme" crates/config/src/ 2>/dev/null; then
    check_pass "Theme loading functionality present"
else
    check_warn "Theme loading not implemented - Task 6.2"
fi

echo

# Test execution
echo "--- Test Execution ---"

echo "Running unit tests..."
if cargo test --workspace --lib --quiet; then
    check_pass "Unit tests pass"
else
    check_warn "Some unit tests failing"
fi

# Check for plugin-specific tests
if find . -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | grep -q plugins; then
    check_pass "Plugin-specific tests present"
else
    check_warn "Plugin-specific tests not found"
fi

# Check for AI-specific tests
if find . -name "*.rs" -exec grep -l "#\[tokio::test\]" {} \; | grep -q ai; then
    check_pass "AI async tests present"
else
    check_warn "AI async tests not found"
fi

echo

# Security validation
echo "--- Security Validation ---"

# Check for capability enforcement
if grep -rq "check_permission\|PermissionError" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Permission enforcement mechanisms present"
else
    check_warn "Permission enforcement not implemented"
fi

# Check for secret redaction in AI context
if grep -rq "redact.*before\|sanitize.*context" crates/ai/src/ 2>/dev/null; then
    check_pass "AI context sanitization present"
else
    check_warn "AI context sanitization not implemented"
fi

echo

# Performance validation
echo "--- Performance Validation ---"

# Check for resource monitoring
if grep -rq "ResourceMonitor\|memory_limit\|time_limit" crates/plugins-host/src/ 2>/dev/null; then
    check_pass "Resource monitoring infrastructure present"
else
    check_warn "Resource monitoring not implemented"
fi

# Check for rate limiting
if grep -rq "RateLimiter\|rate_limit" crates/ai/src/ 2>/dev/null; then
    check_pass "AI rate limiting present"
else
    check_warn "AI rate limiting not implemented"
fi

echo

# Integration validation
echo "--- Integration Validation ---"

# Check for plugin host integration
if grep -rq "PluginsHost\|plugin.*host" crates/cli/src/ 2>/dev/null; then
    check_pass "Plugin host integration in CLI present"
else
    check_warn "Plugin host not integrated into main application"
fi

# Check for AI integration
if grep -rq "AiIntegration\|ai.*provider" crates/cli/src/ 2>/dev/null; then
    check_pass "AI integration in CLI present"
else
    check_warn "AI not integrated into main application"
fi

echo

# Example validation
echo "--- Example Plugin Validation ---"

# Check for example plugins
if [ -d "examples/plugins" ] || [ -d "plugins/examples" ]; then
    check_pass "Example plugins directory exists"
    
    # Check for hello world plugin
    if find . -name "hello*world*" -o -name "*example*plugin*" | grep -q .; then
        check_pass "Example plugin found"
    else
        check_warn "No example plugins found"
    fi
else
    check_warn "Example plugins directory not found"
fi

# Check for plugin templates
if find . -name "plugin.toml" -o -name "*template*" | grep -q .; then
    check_pass "Plugin templates or manifests found"
else
    check_warn "Plugin templates not found"
fi

echo

# Documentation validation
echo "--- Documentation Validation ---"

# Check for plugin development documentation
if [ -f "docs/plugin_dev.md" ]; then
    if grep -q "WASM\|plugin.*api" docs/plugin_dev.md; then
        check_pass "Plugin development documentation updated"
    else
        check_warn "Plugin development documentation needs Phase 3 updates"
    fi
else
    check_warn "Plugin development documentation not found"
fi

# Check for AI integration documentation
if grep -rq "AI.*integration\|provider.*trait" docs/ 2>/dev/null; then
    check_pass "AI integration documentation present"
else
    check_warn "AI integration documentation not found"
fi

echo

# Summary
echo -e "${BLUE}=== Validation Summary ===${NC}"
echo -e "✓ Passed: ${GREEN}$PASS_COUNT${NC}"
echo -e "⚠ Warnings: ${YELLOW}$WARN_COUNT${NC}"
echo -e "✗ Failed: ${RED}$FAIL_COUNT${NC}"

if [ $FAIL_COUNT -gt 0 ]; then
    echo -e "\n${RED}Critical issues found. Phase 3 implementation incomplete.${NC}"
    exit 1
elif [ $WARN_COUNT -gt 10 ]; then
    echo -e "\n${YELLOW}Many components missing. Phase 3 implementation in early stages.${NC}"
    exit 0
elif [ $WARN_COUNT -gt 5 ]; then
    echo -e "\n${YELLOW}Some components missing. Phase 3 implementation in progress.${NC}"
    exit 0
else
    echo -e "\n${GREEN}Phase 3 implementation looks good!${NC}"
    exit 0
fi