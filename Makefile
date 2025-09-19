.PHONY: help dev build test lint fmt clean doc install-hooks

# Default target
help: ## Show this help message
	@echo "QuantaTerm Development Commands"
	@echo "=============================="
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

dev: ## Start development mode with watch and incremental build
	cargo watch -x "check --all" -x "test --all" -x "clippy --all -- -D warnings"

build: ## Build all crates
	cargo build --all

build-release: ## Build all crates in release mode
	cargo build --all --release

test: ## Run all tests
	cargo test --all

test-doc: ## Run documentation tests
	cargo test --all --doc

lint: ## Run clippy linter
	cargo clippy --all -- -D warnings

fmt: ## Format code using rustfmt
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

clean: ## Clean build artifacts
	cargo clean

doc: ## Generate documentation
	cargo doc --all --no-deps --open

doc-private: ## Generate documentation including private items
	cargo doc --all --no-deps --document-private-items --open

check-all: fmt-check lint test ## Run all checks (formatting, linting, tests)

install-hooks: ## Install pre-commit hooks
	@echo "Installing pre-commit hooks..."
	@cp scripts/pre-commit .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hooks installed successfully!"

# CI targets
ci-check: ## Run CI checks
	cargo check --all --locked
	cargo fmt --all -- --check
	cargo clippy --all --locked -- -D warnings
	cargo test --all --locked

# Security and licensing
audit: ## Run security audit
	cargo audit

deny: ## Check dependencies with cargo-deny
	cargo deny check

# Benchmarks (placeholder)
bench: ## Run benchmarks
	@echo "Benchmarks not yet implemented"

# Fuzzing (placeholder)  
fuzz: ## Run fuzzing tests
	@echo "Fuzzing not yet implemented"