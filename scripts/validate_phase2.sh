#!/bin/bash
# Phase 2 Task Validation Script
# This script helps AI coding agents validate their implementation against acceptance criteria

set -e

echo "=== QuantaTerm Phase 2 Task Validation ==="
echo "This script validates implementation against Phase 2 acceptance criteria"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track validation results
PASSED=0
FAILED=0
WARNINGS=0

check_pass() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
    ((PASSED++))
}

check_fail() {
    echo -e "${RED}✗ FAIL${NC}: $1"
    ((FAILED++))
}

check_warn() {
    echo -e "${YELLOW}⚠ WARN${NC}: $1"
    ((WARNINGS++))
}

echo "--- Build System Validation ---"

# Check if workspace builds
if cargo check --workspace --quiet; then
    check_pass "Workspace builds without errors"
else
    check_fail "Workspace build failed"
fi

# Check for clippy warnings
if cargo clippy --workspace --all-targets --all-features --quiet -- -D warnings; then
    check_pass "No clippy warnings"
else
    check_fail "Clippy warnings detected"
fi

# Check formatting
if cargo fmt --all -- --check; then
    check_pass "Code is properly formatted"
else
    check_fail "Code formatting issues detected"
fi

echo
echo "--- Dependency Validation ---"

# Check for required dependencies in renderer crate
if grep -q "ab-glyph" crates/renderer/Cargo.toml 2>/dev/null; then
    check_pass "Font loading dependency (ab-glyph) present"
else
    check_warn "Font loading dependency (ab-glyph) not found - needed for Task 1"
fi

if grep -q "harfbuzz" crates/renderer/Cargo.toml 2>/dev/null; then
    check_pass "Text shaping dependency (harfbuzz) present"
else
    check_warn "Text shaping dependency (harfbuzz) not found - needed for Task 1"
fi

if grep -q "bit-vec" crates/renderer/Cargo.toml 2>/dev/null; then
    check_pass "Dirty tracking dependency (bit-vec) present"
else
    check_warn "Dirty tracking dependency (bit-vec) not found - needed for Task 2"
fi

if grep -q "notify" crates/config/Cargo.toml 2>/dev/null; then
    check_pass "File watching dependency (notify) present"
else
    check_warn "File watching dependency (notify) not found - needed for Task 3"
fi

echo
echo "--- Test Coverage Validation ---"

# Run tests and check results
echo "Running test suite..."
if cargo test --workspace --quiet; then
    check_pass "All tests pass"
else
    check_fail "Some tests failing"
fi

# Check for specific test modules
if find . -name "*.rs" -exec grep -l "mod.*tests" {} \; | wc -l | grep -q "[1-9]"; then
    check_pass "Test modules present"
else
    check_warn "Limited test coverage detected"
fi

echo
echo "--- Task 1: Font System Validation ---"

# Check for font module structure
if [ -f "crates/renderer/src/font/mod.rs" ] || grep -q "font::" crates/renderer/src/lib.rs; then
    check_pass "Font module structure exists"
else
    check_warn "Font module not implemented - Task 1.1"
fi

# Check for font loading functionality
if grep -rq "FontLoader\|load_font" crates/renderer/src/ 2>/dev/null; then
    check_pass "Font loading traits/functions present"
else
    check_warn "Font loading not implemented - Task 1.1"
fi

# Check for glyph shaping
if grep -rq "GlyphShaper\|harfbuzz" crates/renderer/src/ 2>/dev/null; then
    check_pass "Text shaping implementation present"
else
    check_warn "Text shaping not implemented - Task 1.2"
fi

# Check for atlas implementation
if grep -rq "GlyphAtlas\|atlas" crates/renderer/src/ 2>/dev/null; then
    check_pass "Glyph atlas implementation present"
else
    check_warn "Glyph atlas not implemented - Task 1.3"
fi

echo
echo "--- Task 2: Dirty Rendering Validation ---"

# Check for dirty tracking
if grep -rq "DirtyTracker\|DirtyRegion" crates/renderer/src/ 2>/dev/null; then
    check_pass "Dirty tracking implementation present"
else
    check_warn "Dirty tracking not implemented - Task 2.1"
fi

# Check for partial rendering
if grep -rq "render_partial\|partial.*render" crates/renderer/src/ 2>/dev/null; then
    check_pass "Partial rendering implementation present"
else
    check_warn "Partial rendering not implemented - Task 2.2"
fi

echo
echo "--- Task 3: Configuration System Validation ---"

# Check for enhanced config structure
if grep -rq "FontConfig\|font.*config" crates/config/src/ 2>/dev/null; then
    check_pass "Enhanced font configuration present"
else
    check_warn "Enhanced configuration not implemented - Task 3.1"
fi

# Check for file watching
if grep -rq "ConfigWatcher\|notify::" crates/config/src/ 2>/dev/null; then
    check_pass "File watching implementation present"
else
    check_warn "File watching not implemented - Task 3.2"
fi

echo
echo "--- Task 4: Command Palette Validation ---"

# Check for action system
if grep -rq "Action.*trait\|ActionRegistry" crates/ 2>/dev/null; then
    check_pass "Action system present"
else
    check_warn "Action system not implemented - Task 4.1"
fi

# Check for command palette UI
if grep -rq "CommandPalette\|command.*palette" crates/ 2>/dev/null; then
    check_pass "Command palette implementation present"
else
    check_warn "Command palette not implemented - Task 4.2"
fi

echo
echo "--- Task 5: Shell Integration Validation ---"

# Check for shell hooks
if grep -rq "ShellHooks\|shell.*hook" crates/pty/src/ 2>/dev/null; then
    check_pass "Shell hooks implementation present"
else
    check_warn "Shell hooks not implemented - Task 5.1"
fi

# Check for boundary detection
if grep -rq "BoundaryDetector\|CommandBoundary" crates/pty/src/ 2>/dev/null; then
    check_pass "Boundary detection present"
else
    check_warn "Boundary detection not implemented - Task 5.1"
fi

# Check for shell detection
if grep -rq "ShellDetector\|detect.*shell" crates/pty/src/ 2>/dev/null; then
    check_pass "Shell detection present"
else
    check_warn "Shell detection not implemented - Task 5.2"
fi

echo
echo "--- Task 6: Command Blocks Validation ---"

# Check for block data model
if grep -rq "CommandBlock\|BlockManager" crates/blocks/src/ 2>/dev/null; then
    check_pass "Command block data model present"
else
    check_warn "Command blocks not implemented - Task 6.1"
fi

# Check for block UI
if grep -rq "BlockRenderer\|block.*ui" crates/ 2>/dev/null; then
    check_pass "Block UI implementation present"
else
    check_warn "Block UI not implemented - Task 6.2"
fi

echo
echo "--- Performance Validation ---"

# Check if benchmarks exist
if [ -d "benchmarks" ] && find benchmarks -name "*.rs" | grep -q .; then
    check_pass "Benchmark framework present"
else
    check_warn "Benchmarks not implemented - needed for performance validation"
fi

# Try to run a quick performance check if available
if command -v criterion >/dev/null 2>&1 && [ -f "Cargo.toml" ] && grep -q "criterion" Cargo.toml; then
    echo "Running quick performance check..."
    if timeout 30s cargo bench --quiet >/dev/null 2>&1; then
        check_pass "Performance benchmarks executable"
    else
        check_warn "Performance benchmarks failed or timed out"
    fi
else
    check_warn "Criterion not available for performance testing"
fi

echo
echo "--- Documentation Validation ---"

# Check for updated documentation
if [ -f "PHASE_2_TASKS.md" ]; then
    check_pass "Phase 2 task documentation present"
else
    check_warn "Phase 2 documentation missing"
fi

# Check for API documentation
if grep -rq "#!\[.*doc\]" crates/*/src/lib.rs; then
    check_pass "Crate-level documentation present"
else
    check_warn "Limited crate documentation"
fi

echo
echo "=== Validation Summary ==="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}" 
echo -e "Warnings: ${YELLOW}$WARNINGS${NC}"
echo

# Determine overall status
if [ $FAILED -eq 0 ]; then
    if [ $WARNINGS -eq 0 ]; then
        echo -e "${GREEN}🎉 All validation checks passed! Phase 2 implementation appears complete.${NC}"
        exit 0
    else
        echo -e "${YELLOW}⚠️  Basic validation passed but some features may be incomplete.${NC}"
        echo "Review warnings above and implement missing components."
        exit 0
    fi
else
    echo -e "${RED}❌ Validation failed. Address the failures above before proceeding.${NC}"
    exit 1
fi