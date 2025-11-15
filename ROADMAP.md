# Feature List

## MVP (Completed Features)

### Core Backend Features

- [ ] Redis connection & basic ops (get, scan, info)
- [ ] Memcached connection & basic ops
- [ ] etcd connection & basic ops
- [ ] Connection profiles (save/load connections)
- [ ] Authentication (password)

### Key Browsing & Navigation

- [ ] List all keys (with pagination)
- [ ] Pattern-based key filtering (glob/regex)
- [ ] Fuzzy search for keys
- [ ] Key sorting (name, size, TTL, type)
- [ ] Key type indicators (string, list, hash, etc.)
- [ ] Key size display (human-readable)
- [ ] TTL display with countdown

### Value Display

- [ ] View key values (read-only)
- [ ] Large value truncation/pagination

### Monitoring & Stats

- [ ] Server info display (version, uptime, memory)
- [ ] Key count by pattern

### UI & UX

- [ ] Three-panel layout (connections | keys | value)
- [ ] Mouse support (click to select)
- [ ] Help modal (show all keybindings)

### Configuration & Persistence

- [ ] Connection history

---

## Backlog

### Core Backend Features

- [ ] Multi-connection support (connect to multiple DBs simultaneously)
- [ ] TLS/SSL support
- [ ] Authentication (token, cert)
- [ ] Connection health check / ping
- [ ] Auto-reconnect on disconnect

### Key Browsing & Navigation

- [ ] Tree view for hierarchical keys (e.g., user:123:session)
- [ ] Key metadata view (last accessed, encoding, etc.)
- [ ] Bookmark/favorite keys

### Value Display

- [ ] Auto-detect value format (JSON, msgpack, text, binary)
- [ ] JSON syntax highlighting & pretty-print
- [ ] Binary hex dump display
- [ ] Copy value to clipboard
- [ ] Export value to file
- [ ] Diff two values side-by-side
- [ ] Search within value content

### Write Operations (Opt-in, disabled by default)

- [ ] Set/update key value
- [ ] Delete single key
- [ ] Bulk delete keys (with confirmation)
- [ ] Set TTL on key
- [ ] Rename key
- [ ] Copy key to another connection
- [ ] Write protection (require confirmation for dangerous ops)

### Raw Command Mode

- [ ] Execute raw backend commands (Redis: INFO, etcd: etcdctl equivalents)
- [ ] Command history (navigate with up/down)
- [ ] Command auto-completion
- [ ] Syntax highlighting for commands
- [ ] Save command as snippet
- [ ] Multi-line command input

### Monitoring & Stats

- [ ] Real-time stats (ops/sec, hit rate, memory usage)
- [ ] Connection latency display
- [ ] Memory usage per key
- [ ] Slow query log (if backend supports)
- [ ] Live key watch/monitoring (etcd-style)

### UI & UX

- [ ] Vim-style keybindings
- [ ] Emacs-style keybindings
- [ ] Color themes (dark, light, custom)
- [ ] Customizable keybindings
- [ ] Search modal with preview
- [ ] Status bar (show current mode, connection status)
- [ ] Tab/workspace switching (multiple views)
- [ ] Split panes (view multiple keys simultaneously)

### Configuration & Persistence

- [ ] Config file support (~/.config/memtui/config.toml)
- [ ] Save window layout
- [ ] Recently accessed keys
- [ ] Command history persistence
- [ ] Import/export configuration

### Advanced Features

- [ ] Bulk operations (batch get/delete)
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
