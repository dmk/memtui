# memtui Makefile
# Convenience targets for build, test, lint, and development

.PHONY: all build check test fmt clippy run clean help verify dev release lint

# Default target
all: build

# Build the project (debug)
build:
	cargo build

# Build release version
release:
	cargo build --release

lint: fmt-check check clippy

# Check compilation without building
check:
	cargo check --all --all-features

# Run all tests
test:
	cargo test --all --all-features

# Format code
fmt:
	cargo fmt --all

# Check formatting (CI-friendly)
fmt-check:
	cargo fmt --all -- --check

# Run clippy linter
clippy:
	cargo clippy --all-targets --all-features -- -D warnings

# Run the TUI application
run:
	cargo run

# Run in release mode
run-release:
	cargo run --release

# Development mode - watch and rebuild
dev:
	cargo watch -x check -x test -x run

# Full verification (for CI/pre-commit)
verify: fmt-check check clippy test

# Clean build artifacts
clean:
	cargo clean

# Show help
help:
	@echo "memtui - Makefile targets:"
	@echo ""
	@echo "  make build       - Build debug binary"
	@echo "  make release     - Build release binary"
	@echo "  make check       - Check compilation"
	@echo "  make test        - Run tests"
	@echo "  make fmt         - Format code"
	@echo "  make fmt-check   - Check code formatting"
	@echo "  make clippy      - Run linter"
	@echo "  make run         - Run the TUI"
	@echo "  make verify      - Run all checks (CI)"
	@echo "  make clean       - Remove build artifacts"
	@echo "  make help        - Show this help"

