# CacheKill Makefile
# Development workflow for the cachekill CLI tool

.PHONY: help build test fmt clippy clean run install uninstall

# Default target
help:
	@echo "CacheKill Development Commands:"
	@echo "  build     - Build the project in release mode"
	@echo "  test      - Run all tests"
	@echo "  fmt       - Format code with rustfmt"
	@echo "  clippy    - Run clippy linter"
	@echo "  clean     - Clean build artifacts"
	@echo "  run       - Run the binary with default args"
	@echo "  install   - Install the binary to ~/.cargo/bin"
	@echo "  uninstall - Remove the binary from ~/.cargo/bin"
	@echo "  check     - Run fmt, clippy, and test"
	@echo "  dev       - Run in development mode with --dry-run"

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test

# Format code
fmt:
	cargo fmt

# Run clippy
clippy:
	cargo clippy -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Run the binary
run:
	cargo run

# Run with dry-run for development
dev:
	cargo run -- --dry-run

# Run with list mode
list:
	cargo run -- --list

# Run with JSON output
json:
	cargo run -- --list --json

# Install the binary
install: build
	cargo install --path .

# Uninstall the binary
uninstall:
	cargo uninstall cachekill

# Run all checks
check: fmt clippy test
	@echo "✅ All checks passed!"

# Development workflow
dev-setup: fmt clippy test
	@echo "✅ Development environment ready!"

# Build and test
ci: fmt clippy test build
	@echo "✅ CI pipeline completed!"

# Show help
help-all:
	@echo "Available make targets:"
	@make -qp | awk -F':' '/^[a-zA-Z0-9][^$#\/\t=]*:([^=]|$$)/ {split($$1,A,/ /);for(i in A)print A[i]}' | sort -u
