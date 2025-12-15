# memtui

[![CI](https://github.com/dmk/memtui/actions/workflows/ci.yml/badge.svg)](https://github.com/dmk/memtui/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/dmk/memtui?label=latest%20release)](https://github.com/dmk/memtui/releases/latest)

`memtui` is an interactive TUI (Terminal User Interface) for browsing and inspecting key-value stores from a single interface. It’s written in Rust with an async event loop (Tokio) and a component-based UI (ratatui).

See `ROADMAP.md` for planned features and milestones.

## Features (Available Today)

- **Backends**: Redis, Memcached, etcd
- **Connection management**
  - Saved connection profiles (local persistence)
  - One-shot connection via CLI connection string (not saved; shown with a warning)
  - Multiple open connections shown as tabs; cycle with keybindings
  - Connection palette + connection form modals
- **Key browsing**
  - Chunked scanning (`keys_per_chunk`)
  - Sparse key list with “loading…” placeholders that fill in as you navigate
  - Mouse support (click/select, wheel scroll) and resizable pane split (drag handle)
- **Search**
  - Local fuzzy search (nucleo matcher) with match highlighting
  - Background server search (`search_keys("*query*")`) merged into results
- **Value inspection**
  - Handles large values with efficient wrapping/caching and scrolling
  - JSON pretty-print + syntax coloring (configurable)
- **Customization**
  - `config.toml` for UI/performance/data/json formatting
  - `theme.json` for UI colors (JSON-with-`//` comments)
  - `keybindings.json` for keybindings (JSON-with-`//` comments)

## Install / Run

Homebrew:
```bash
brew tap dmk/memtui
brew install memtui
```

From this repo:
```bash
cargo install --path .
memtui
```

Or run directly:
```bash
cargo run
```

Dev shortcuts:
```bash
make run
make test
make verify
```

## CLI Usage

```bash
memtui [CONNECTION_STRING]
memtui --connect "Saved Connection Name"
memtui --config /path/to/config.toml
memtui --log-file ./memtui.log --log-level debug
```

Connection strings currently supported:
- `redis://[user:pass@]host:port[/db]`
- `memcached://host:port`
- `etcd://host:port`

## Keybindings (Defaults)

Keybindings are configurable in `~/.config/memtui/keybindings.json`.

- **Global**
  - `q` / `Esc`: quit confirmation
  - `?`: help
  - `Ctrl+P`: connection palette
  - `Ctrl+N`: new connection form
- **Navigation**
  - `Tab` / `Shift+Tab`: switch panel (Keys <-> Value)
  - `Down/j` and `Up/k`: move selection (Keys panel)
  - `Ctrl+Right` / `Ctrl+Left`: cycle open connection tabs
  - Mouse: click to select; scroll wheel; drag the splitter handle
- **Search**
  - `/`: start search
  - Type to filter; local results update immediately; server results arrive shortly after
  - `Esc`: clear search

## Supported Backends + Capability Matrix (Today)

Capabilities come from `Backend::capabilities()` and are used to describe backend behavior.

| Capability | Redis | Memcached | etcd |
|---|---|---|---|
| Connect / ping / info | Yes | Yes | Yes |
| Key listing (`scan_keys`) | Yes | Best-effort* | Yes |
| Server-side pattern search (`supports_efficient_pattern_search`) | Yes | No | Yes |
| Batch get (`get_many`) | Yes | Yes | Yes |
| Raw commands (`execute_raw`) | Yes | No | No |
| TTL populated in `KeyMetadata` today | Yes | No | No |
| TLS knobs in config | Yes (`rediss://`) | No | Yes (`https://`) |

\* Memcached key listing is best-effort; the UI warns that results may not be consistent.

## Config + Persistence

Paths are currently XDG-like:

- Config: `~/.config/memtui/config.toml`
- Theme: `~/.config/memtui/theme.json`
- Keybindings: `~/.config/memtui/keybindings.json`
- Saved connections: `~/.local/share/memtui/connections.json`
- Recent connections: `~/.local/share/memtui/recent_connections.json`

### `config.toml` (Supported Today)

`config.toml` is created on first run with comments. Key sections:

- `[ui]`: viewport height, recents count
- `[performance]`: tick interval, poll timeout, loop sleep
- `[data]`: keys-per-chunk, value-load debounce
- `[json]`: indentation + syntax colors

Example (subset):
```toml
[ui]
viewport_height = 20
max_recent_connections = 8

[data]
keys_per_chunk = 200
value_load_debounce = 120

[json]
indent = 2
key_color = "cyan"
string_color = "green"
```

## Safety Model (Today)

- The current UI is effectively **read-only** (browse/search/inspect).
- The backend layer contains write methods (`set`, `delete`) and a per-connection `read_only` flag, but write actions are not currently exposed in the UI.

## Architecture (Current Code)

High-level modules:

- `src/main.rs`: owns `AppState` + `UiState`, runs the Tokio-driven event loop, dispatches `Action`s.
- `src/action.rs`: `Action` enum (intents + async results).
- `src/actions/`
  - `sync_handlers.rs`: synchronous state updates (UI toggles, navigation, search input edits)
  - `async_handlers.rs`: spawns async tasks (connect, scan keys, load values, hybrid search) and emits result `Action`s
- `src/app/`: `ConnectionManager`, `EventRunner`
- `src/backend/`: `Backend` trait + `RedisBackend` / `MemcachedBackend` / `EtcdBackend`
- `src/ui/`: renderer + components (`KeyList`, `ValueViewer`, modals, status bar)
- `src/search.rs`: fuzzy search + match positions
- `src/userdata.rs`: config/theme/keybindings persistence

## Contributing

See `CONTRIBUTING.md`. Planned milestones and architecture direction live in `ROADMAP.md`.

## License

Apache-2.0 (see `LICENSE`).
