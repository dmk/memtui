# memtui

`memtui` is an interactive TUI (Terminal User Interface) that lets you browse, search, and inspect key-value stores from a single interface. Built in Rust for speed and reliability.

## Features

### MVP (v0.1)

- **Multi-backend support**: Redis, Memcached, etcd
- **Connection management**: Save and load connection profiles
- **Key browsing**: List, search (fuzzy + pattern matching), sort keys
- **Value inspection**: View values with pagination for large data
- **Server info**: Display version, uptime, memory usage
- **Clean UI**: Three-panel layout with mouse support and help modal

### Read-only by default

All operations are read-only to prevent accidents. Write operations will be opt-in via flag.

## Architecture

```
┌───────────────────────────────────────────────────────┐
│                     memtui                            │
├───────────────────────────────────────────────────────┤
│                                                       │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Connections │  │  Key Browser │  │ Value Viewer │  │
│  │             │  │              │  │              │  │
│  │  • Redis    │  │  Filter: *   │  │  {           │  │
│  │  • Memcache │  │              │  │    "user":.. │  │
│  │  • etcd     │  │  user:123    │  │    "email".. │  │
│  │             │  │  user:456    │  │  }           │  │
│  │             │  │  session:*   │  │              │  │
│  └─────────────┘  └──────────────┘  └──────────────┘  │
│                                                       │
│  Status: Connected | Keys: 1,234 | [?] Help           │
└───────────────────────────────────────────────────────┘
```

### Two-layer design

**Layer 1: Backend abstraction**

- Trait-based backend system
- Each store (Redis, Memcached, etcd) implements `Backend` trait
- Unified API: `connect()`, `scan_keys()`, `get()`, `info()`, etc.
- Capability flags: backends declare what features they support

**Layer 2: UI layer**

- Backend-agnostic display code
- Unified keybindings across all backends
- Pluggable formatters for different value types
- Consistent UX regardless of backend

## Roadmap

### Near future

- Format auto-detection (JSON, msgpack, binary)
- Syntax highlighting for JSON
- Raw command mode
- Status bar with real-time connection info

### Later

- Multi-connection support (connect to multiple DBs at once)
- TLS/SSL support
- Tree view for hierarchical keys
- Write operations (set, delete) with confirmations
- Export/import functionality
- Config file support
- Custom themes and keybindings

### Way later

- Additional backends: Valkey, DragonflyDB, Consul, ZooKeeper
- Cluster mode support
- Pub/Sub monitoring
- Real-time key watching (etcd-style)
- Transaction support
- Plugin system for custom formatters

## Contributing

This is early stage. If you want to add a backend, implement the `Backend` trait in `src/backend/`.

## License

TBD
