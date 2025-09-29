# CacheKill Makefile
# Provides convenient commands for development and release management

.PHONY: help build test clean install release check-version bump-version

# Default target
help:
	@echo "CacheKill Development Commands"
	@echo "=============================="
	@echo ""
	@echo "Development:"
	@echo "  build          - Build the project in release mode"
	@echo "  test           - Run all tests"
	@echo "  clean          - Clean build artifacts"
	@echo "  install        - Install locally with cargo"
	@echo ""
	@echo "Release Management:"
	@echo "  check-version  - Check current version"
	@echo "  bump-version   - Bump version (usage: make bump-version VERSION=0.1.3)"
	@echo "  release        - Create a new release (usage: make release VERSION=0.1.3)"
	@echo ""
	@echo "Cross-compilation:"
	@echo "  build-linux    - Build for Linux (x86_64)"
	@echo "  build-windows  - Build for Windows (x86_64)"
	@echo "  build-macos    - Build for macOS (x86_64 and aarch64)"
	@echo "  build-all      - Build for all supported platforms"

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean

# Install locally
install:
	cargo install --path .

# Check current version
check-version:
	@echo "Current version: $(shell grep '^version = ' Cargo.toml | cut -d'"' -f2)"

# Bump version in Cargo.toml
bump-version:
ifndef VERSION
	@echo "Error: VERSION is required. Usage: make bump-version VERSION=0.1.3"
	@exit 1
endif
	@echo "Bumping version to $(VERSION)..."
	@sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@sed -i.bak 's/version = "$(VERSION)"/version = "$(VERSION)"/' src/main.rs
	@rm -f Cargo.toml.bak src/main.rs.bak
	@echo "Version bumped to $(VERSION)"

# Create a new release
release:
ifndef VERSION
	@echo "Error: VERSION is required. Usage: make release VERSION=0.1.3"
	@exit 1
endif
	@echo "Creating release $(VERSION)..."
	@make bump-version VERSION=$(VERSION)
	@git add Cargo.toml src/main.rs
	@git commit -m "Bump version to $(VERSION)"
	@git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	@git push origin main
	@git push origin "v$(VERSION)"
	@echo "Release $(VERSION) created and pushed!"

# Cross-compilation targets
build-linux:
	cargo build --release --target x86_64-unknown-linux-gnu

build-windows:
	cargo build --release --target x86_64-pc-windows-gnu

build-macos:
	cargo build --release --target x86_64-apple-darwin
	cargo build --release --target aarch64-apple-darwin

build-all: build-linux build-windows build-macos
	cargo build --release --target aarch64-unknown-linux-gnu

# Development helpers
dev-setup:
	@echo "Setting up development environment..."
	@rustup target add x86_64-unknown-linux-gnu
	@rustup target add x86_64-pc-windows-gnu
	@rustup target add x86_64-apple-darwin
	@rustup target add aarch64-apple-darwin
	@rustup target add aarch64-unknown-linux-gnu
	@echo "Development environment setup complete!"

# Check if all required tools are installed
check-tools:
	@echo "Checking required tools..."
	@command -v cargo >/dev/null || (echo "Error: cargo not found" && exit 1)
	@command -v rustup >/dev/null || (echo "Error: rustup not found" && exit 1)
	@command -v git >/dev/null || (echo "Error: git not found" && exit 1)
	@echo "All required tools are available!"