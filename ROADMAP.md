# memtui Roadmap

This roadmap is milestone-based and aligned to the architectural pillars:

- **Action Enum is Law**: every state change goes through `Action`.
- **Backend stays generic**: use **capabilities** so features gracefully disable per backend.
- **Async / Sync split**:
  - Sync: navigation, selection, rendering, local fuzzy search, virtual scrolling decisions
  - Async: network I/O, clipboard, external editor spawning, server-side search, TTL polling
- **Type-driven development**: define types first, then implement.

> Note: This roadmap intentionally avoids claiming features that aren’t implemented today.

---

## Architecture Notes (Contributors)

- Pillars: `Action` is the only way state changes; backend stays generic + capability-driven; sync work stays sync (nav/render/local search/windowing); async work stays async (I/O/clipboard/editor/server search/TTL polling); define types first.
- Action flow: UI emits `Action` -> reducer updates sync state -> async dispatcher runs side effects -> dispatcher emits follow-up result `Action`s -> reducer updates state -> render.

---

## Milestone 0 — “Action Is Law” Enforcement (Foundation)

**Goal:** make the current architecture consistent and testable without changing behavior.

**Deliverables**
- Centralize state transitions in a reducer-like layer:
  - UI emits `Action`
  - reducer updates sync state
  - async dispatcher runs side effects and emits result `Action`s
- Remove remaining direct state mutations in input handlers (scrolling, resizing, palette navigation) by introducing explicit actions for them.
- Make keybinding “commands” map to real `Action`s (or remove/disable dead commands).

**Proposed code-level changes**
- Add `src/effects.rs`:
  - `enum Effect { Connect{...}, ScanKeys{...}, LoadValue{...}, SearchServer{...}, ... }`
- Add `src/reducer.rs` (or `src/actions/reducer.rs`):
  - `fn reduce(app: &mut AppState, ui: &mut UiState, action: Action) -> Vec<Effect>`
- Convert `src/actions/async_handlers.rs` into an `Effect` dispatcher:
  - `fn dispatch(effect: Effect, tx: ActionTx, ...)`

**Tests**
- Unit tests for reducer transitions (no Tokio required).
- Keep existing backend unit tests; add a few “reducer-only” tests for navigation/search.

---

## Milestone 1 — Capabilities 2.0 + Feature Gating (Foundation)

**Goal:** unblock upcoming UI features without hardcoding backend type checks.

**Why:** current `BackendCapabilities` conflates “can set TTL” with “can read TTL” (Memcached/etcd set TTL but don’t populate `KeyMetadata.ttl` today).

**Proposed capabilities shape**
- Evolve `BackendCapabilities` (keep it plain-data, copyable):
  - `supports_key_scan`
  - `supports_server_search`
  - `supports_raw_commands`
  - `supports_batch_get`
  - `supports_ttl_read`
  - `supports_ttl_write`
  - `supports_expire` (set TTL on existing keys)
  - `supports_value_range_get` (lazy value loading)
  - `supports_write` (delete/set)
- Store a **capabilities snapshot per connected connection** (so UI can gate synchronously without awaiting an RwLock).

**Tests**
- Unit tests for capability gating logic (“feature disabled” UI states).
- Integration sanity checks (docker compose) can be added later.

---

## Milestone 2 — Search State Model + “Optimistic Hybrid Search” (Feature)

**Goal:** formalize search and make merging behavior deterministic.

**Deliverables**
- Introduce a type-driven search model:
  - `SearchState { mode, query, token, local, server, merged, selection }`
  - `SearchResultItem` that can represent:
    - “loaded key by index”
    - “server-only key (name + metadata)”
- Merge local + server results with stable ordering and dedupe.
- Allow selecting server-only results and loading their value directly (even if not in the current key window), gated by capabilities.

**Proposed files**
- `src/app/search_state.rs` (or `src/model/search.rs`)
- Add/adjust `Action`s:
  - `Action::SearchOpened`, `Action::SearchQueryChanged`, `Action::SearchLocalReady`, `Action::SearchServerReady`, `Action::SearchSelectionMoved`, …

**Tests**
- Unit tests for merge + dedupe + stable ordering.
- Mock-backend tests for “server-only selection loads value”.

---

## Milestone 3 — Namespace “Rainbow” + Custom Highlight Rules (Feature)

**Goal:** improve scanability of dense keyspaces.

**Deliverables**
- **Rainbow namespaces**:
  - Deterministic hash from namespace/prefix -> style color
  - Configurable separator and depth
- **GRC-style regex highlighting**:
  - User-defined rules from config (keys/value/both)
  - Priority + style merge rules

**Proposed files**
- `src/style/namespace.rs`: `NamespaceStyle`, `NamespaceHasher`
- `src/style/highlight.rs`: `HighlightRule`, compiled regex cache
- Extend `Config`:
  - `config.toml` gains `[namespaces]` and `[[highlight.rules]]`

**Dependencies**
- Regex highlighting requires adding a regex engine (likely `regex` crate).

**Tests**
- Unit tests for namespace hashing determinism.
- Unit tests for highlighting rule matching and precedence.

---

## Milestone 4 — TTL Visualizer (Feature)

**Goal:** show TTL and expiry progression live, without blocking UI.

**Deliverables**
- TTL display in key list and/or value header when available.
- Live countdown for selected key (and optionally visible window), driven by polling:
  - Async polling emits `Action::TtlUpdated { ... }`
- Graceful “TTL unsupported/unknown” UX via capabilities.

**Proposed types**
- `TtlState { selected: Option<TtlSnapshot>, polling: bool, last_update: Instant, ... }`
- `TtlSnapshot { ttl: Option<Duration>, fetched_at: Instant }`

**Dependencies**
- Requires `supports_ttl_read` and backend method(s) to refresh TTL cheaply (likely `key_info` or a dedicated TTL call).

**Tests**
- Unit tests for countdown math (time-based, but can be deterministic with injected clock).
- Integration tests against Redis container.

---

## Milestone 5 — Raw Command Mode (`:` prompt) (Feature)

**Goal:** direct backend access for advanced users, while staying safe by default.

**Deliverables**
- `:` opens command prompt modal
- Command history (in-memory initially)
- Execute against backend if `supports_raw_commands`
- Respect write safety: raw commands that mutate should be blocked unless write mode is enabled

**Proposed types**
- `CommandModeState { input, history, last_result, status }`
- `CommandType` (optional): parsed command classification for safety gating

**Tests**
- Reducer tests: open/close prompt, input edits, history navigation.
- Mock-backend tests for execute + result handling.

---

## Milestone 6 — Value UX: Format Toggles + Clipboard + Simple Editor (Feature)

**Deliverables**
- `ValueFormat` toggles: Text / Hex / JSON
- Clipboard integration:
  - yank key name
  - yank value (or a safe truncated preview)
- Simple built-in editor for primitive edits (string/int/json), gated by write mode + backend write support
- External editor integration (`$EDITOR`):
  - open value in editor
  - save -> apply `set` back to backend

**Async side effects**
- Clipboard operations
- Spawning editor processes
- Writing temp files

**Tests**
- Unit tests for format toggling reducer logic.
- Integration tests (where feasible) for write mode + Redis set.

---

## Milestone 7 — Lazy Value Loading (Feature)

**Goal:** handle very large values without loading everything at once.

**Deliverables**
- Backend capability: `supports_value_range_get`
- Backend API additions (example):
  - `get_range(key, offset, len)` or `get_stream(key)`-like interface
- UI streaming viewer:
  - renders partial data progressively
  - supports “jump to offset” / paging

**Dependencies**
- Requires backend support; Redis can support `GETRANGE` for string values.
- Other backends may degrade to “full get” or disable the feature.

**Tests**
- Mock backend that yields chunks.
- Reducer tests for streaming state machine.

---

## Milestone 8 — Virtual Scrolling for 1M+ Keys (Feature)

**Goal:** keep memory bounded and UI responsive for huge keyspaces.

**Deliverables**
- Replace `Vec<Option<KeyMetadata>>` preallocation strategy with a windowed model:
  - `KeyWindow { start, items, total_estimate, cursor_state }`
  - LRU cache of pages (optional)
- UI shows consistent scroll/selection without requiring all keys in memory.

**Dependencies**
- Must integrate with search state and selection semantics.
- Works best when backend can provide stable-ish pagination (or accept best-effort ordering).

**Tests**
- Property tests for window math (bounds, wrapping, selection).
- Integration tests that scan a large Redis dataset (docker).

---

## Milestone 9 — Bulk Actions (Feature)

**Deliverables**
- Bulk selection mode + multi-select (range, toggle)
- Bulk delete / bulk expire (where supported)
- Confirmation UX and safety checks

**Dependencies**
- Requires write mode
- Requires backend capabilities:
  - `supports_write`
  - `supports_expire` for TTL operations

**Tests**
- Reducer tests for selection and bulk action workflows.
- Integration tests for Redis.

---

## Integration Test Strategy (Across Milestones)

- Keep fast unit tests for reducers/state machines.
- Use dockerized backends (`docker-compose.yml`) for integration tests:
  - Redis, Memcached, etcd
- Prefer a “test harness” that can be run locally and in CI:
  - smoke tests for connect/scan/get
  - feature-gated tests per backend capability

---

## Next PRs (Smallest Shippable Slices)

- Add `Effect` + `reduce()` skeleton (no behavior change); adapt existing async handlers to dispatch effects.
- Store a capabilities snapshot per connected connection; expand TTL into read/write + add placeholders for range-get/write.
- Replace remaining direct state mutations (scrolling/resizing/palette navigation) with explicit `Action`s.
- Fix keybinding command coverage: wire `search.next_result` / `search.prev_result` into actions or remove dead defaults.
- Consolidate current search fields into a single `SearchState` type (no UX change).
- Add namespace rainbow hashing as a pure UI decoration (no config yet; deterministic by prefix).
- Add `HighlightRule` types + config parsing stub (no regex execution until wired).
