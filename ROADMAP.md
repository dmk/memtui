# Feature List

## MVP (Completed Features)

### Core Backend Features

- [x] Redis connection & basic read ops (get, scan, info)
- [x] Memcached connection & basic read ops
- [ ] etcd connection & basic ops (not implemented - shows error message)
- [x] Connection profiles (save/load connections)
- [x] Authentication (password)

### Key Browsing & Navigation

- [x] List all keys (with pagination)
- [x] Pattern-based key filtering (glob/regex)
- [ ] Fuzzy search for keys
- [ ] Key sorting (name, size, TTL, type)
- [x] Key type indicators (string, list, hash, etc.)
- [x] Key size display (human-readable)
- [ ] TTL display with countdown (TTL is stored but not displayed in UI)

### Value Display

- [x] View key values (read-only)
- [x] Large value truncation/pagination

### Monitoring & Stats

- [x] Server info display (version, uptime, memory)
- [ ] Key count by pattern

### UI & UX

- [x] Two-panel layout (keys | value) with connection list
- [x] Mouse support (click to select, scroll)
- [x] Help modal (show all keybindings)

### Configuration & Persistence

- [x] Connection history (recent connections)

---

## Backlog

### Core Backend Features

- [x] Multi-connection support (connect to multiple DBs simultaneously)
- [x] TLS/SSL support
- [ ] Authentication (token, cert)
- [x] Connection health check / ping
- [ ] Auto-reconnect on disconnect

### Key Browsing & Navigation

- [ ] Tree view for hierarchical keys (e.g., user:123:session)
- [x] Key metadata view (last accessed, encoding, etc. - stored in KeyMetadata but not displayed in UI)
- [ ] Bookmark/favorite keys

### Value Display

- [x] Auto-detect value format (JSON, text, binary)
- [x] JSON syntax highlighting & pretty-print
- [ ] Binary hex dump display
- [ ] Copy value to clipboard
- [ ] Export value to file
- [ ] Diff two values side-by-side
- [ ] Search within value content

### Write Operations (Opt-in, disabled by default)

- [ ] Set/update key value (set method exists in Backend trait, but UI not implemented)
- [ ] Delete single key (delete method exists in Backend trait, but UI not implemented)
- [ ] Bulk delete keys (with confirmation)
- [ ] Set TTL on key (set method supports TTL parameter, but UI not implemented)
- [ ] Rename key
- [ ] Copy key to another connection
- [~] Write protection (read_only flag in ConnectionConfig) - no write mode at all yet

### Raw Command Mode

- [~] Execute raw backend commands (execute_raw method exists in Backend trait)
- [ ] Command history (navigate with up/down)
- [ ] Command auto-completion
- [ ] Syntax highlighting for commands
- [ ] Save command as snippet
- [ ] Multi-line command input
- [ ] UI for raw command mode

### Monitoring & Stats

- [ ] Real-time stats (ops/sec, hit rate, memory usage)
- [ ] Connection latency display
- [ ] Memory usage per key
- [ ] Slow query log (if backend supports)
- [ ] Live key watch/monitoring (etcd-style)

### UI & UX

- [~] Vim-style keybindings (j/k for navigation)
- [ ] Emacs-style keybindings
- [ ] Color themes (dark, light, custom)
- [ ] Customizable keybindings
- [ ] Search modal with preview (pattern filtering exists but no UI modal)
- [x] Status bar (show current mode, connection status)
- [x] Tab/workspace switching (multiple connection tabs)
- [ ] Split panes (view multiple keys simultaneously)

### Configuration & Persistence

- [ ] Config file support (~/.config/memtui/config.toml)
- [ ] Save window layout
- [ ] Recently accessed keys
- [ ] Command history persistence
- [ ] Import/export configuration

### Advanced Features

- [~] Bulk operations (batch get via get_many)
- [ ] Transaction support (Redis MULTI/EXEC)
- [ ] Pub/Sub monitoring (Redis)
- [ ] Cluster mode support (Redis cluster, etcd cluster)
- [ ] Replica/slave info display
- [ ] Key migration between servers
- [ ] Backup/restore keys
- [ ] Stream support (Redis streams)
- [ ] Namespace/prefix isolation
- [ ] Custom formatters via plugin
- [ ] Scripting support (Lua for Redis)

### Developer Experience

- [ ] Logging/debug mode
- [ ] Error reporting with context
- [ ] Performance metrics (render time, query time)
- [ ] Crash reports
- [ ] Update checker
- [ ] Built-in tutorial/demo mode

### Additional Backends (Post-MVP)

- [ ] Valkey support
- [ ] DragonflyDB support
- [ ] KeyDB support
- [ ] Consul support
- [ ] ZooKeeper support
- [ ] RocksDB support (embedded)
