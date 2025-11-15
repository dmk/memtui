# Contributing to memtui

Thanks for your interest in contributing! 🎉

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/memtui.git`
3. Create a branch: `git checkout -b feature/your-feature`

## Development

### Prerequisites

- Rust 1.70+ (edition 2024)
- Make (optional, for convenience)

### Building

```bash
make build          # Debug build
make release        # Release build
```

Or use cargo directly:

```bash
cargo build
cargo build --release
```

### Running

```bash
make run
# or
cargo run
```

### Testing

```bash
make test
# or
cargo test
```

## Code Quality

Before submitting a PR, please ensure:

1. **Format**: Code is formatted with `rustfmt`
   ```bash
   make fmt
   ```

2. **Lint**: No clippy warnings
   ```bash
   make clippy
   ```

3. **Tests**: All tests pass
   ```bash
   make test
   ```

4. **Full check**: Run the complete verification suite
   ```bash
   make verify
   ```

## Architecture

memtui uses a two-layer architecture:

- **Layer 1: Backend** (`src/backend/`) - Storage connection abstraction (Redis, Memcached, etcd)
- **Layer 2: UI** (`src/ui/`) - Terminal UI rendering, completely backend-agnostic

See the [architecture docs](./minor-code-notes.md) for details.

## Adding a New Backend

1. Implement the `Backend` trait in `src/backend/your_backend.rs`
2. Add integration tests
3. Update documentation

## Pull Request Process

1. Update the README if needed
2. Add tests for new functionality
3. Ensure `make verify` passes
4. Submit PR with clear description

## Questions?

Open an issue or discussion on GitHub!

