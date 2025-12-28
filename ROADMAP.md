# memtui Roadmap

## Architectural Pillars

- **Action Enum is Law**: every state change goes through `Action` ✅ (v0.4.0)
- **Backend stays generic**: use capabilities, not type checks ✅
- **Async / Sync split**: network I/O async, navigation/render sync ✅
- **Type-driven development**: define types first, then implement

---

## Milestone 0 — Foundation ✅ COMPLETE (v0.4.0)

Extracted to `tui-dispatch` framework:
- EventBus, keybindings, event types
- Derive macros for Action, ComponentId, BindingContext
- All state mutations go through actions
- Test harness with ActionDispatcher trait

---

## Milestone 1 — Command Palette (Next)

**Goal:** Unified input component for search, commands, goto.

**Deliverables**
- Generalize `SearchInput` → `InputLine` component
  - Modes: Search (`/`), Command (`:`), Goto (`g`)
  - Shared text editing (cursor, word movement, history)
- Command mode (`:`)
  - `:q` quit, `:help` help
  - `:get <key>` jump to key
  - `:del <key>` delete key (with confirmation)
  - `:raw <cmd>` execute raw backend command
- Tab completion for commands and keys
- Command history (in-memory, then persisted)

**New Types**
```rust
enum InputMode { Search, Command, Goto }
struct InputLineState { mode, buffer, cursor, history_idx }
enum CommandAction { Quit, Help, Get(String), Del(String), Raw(String) }
```

**Gating**
- `:raw` requires `supports_raw_commands` capability
- `:del` requires `supports_write` capability (new)

---

## Milestone 2 — Key Operations

**Goal:** CRUD operations on keys.

**Deliverables**
- Delete key (`d` or `:del`)
  - Confirmation dialog
  - Refresh key list after delete
- Copy key name (`yk`)
- Copy value (`yv`)
  - Full value or truncated preview option
- Rename key (`:rename <old> <new>`) - Redis only

**New Capabilities**
```rust
supports_write: bool,      // delete, set
supports_rename: bool,     // Redis RENAME
```

**Async Effects**
- Clipboard operations (arboard crate)
- Delete/rename network calls

---

## Milestone 3 — Debug Framework (tui-dispatch)

**Goal:** Extract memtui's debug overlay to tui-dispatch for reuse.

**Deliverables**
- Move debug overlay to `tui-dispatch-debug` crate
- Macro for easy integration: `#[derive(DebugState)]`
- Features:
  - State inspector (current)
  - Actions log with filtering
  - Action timeline / scrubber
  - Copy state as JSON
  - Breakpoint on action (pause until keypress)

**Integration API**
```rust
// In app
#[derive(DebugState)]
struct AppState { ... }

// Framework provides
DebugOverlay::new()
    .with_state(&app_state)
    .with_actions(&action_log)
    .render(frame, area);
```

---

## Milestone 4 — Capabilities 2.0

**Goal:** Finer-grained capability flags for feature gating.

**New Capabilities**
```rust
pub struct BackendCapabilities {
    // Existing
    pub supports_ttl: bool,
    pub supports_scan: bool,
    pub supports_raw_commands: bool,
    pub supports_batch_get: bool,
    pub supports_efficient_pattern_search: bool,

    // New
    pub supports_ttl_read: bool,   // Can read TTL (vs just set)
    pub supports_ttl_write: bool,  // Can set/update TTL
    pub supports_write: bool,      // Delete/set keys
    pub supports_rename: bool,     // Rename keys
    pub supports_value_range: bool, // GETRANGE for lazy loading
}
```

---

## Milestone 5 — Search Improvements

**Goal:** Formalize search state, improve UX.

**Deliverables**
- `SearchState` type consolidating scattered fields
- Stable merge of local + server results
- Jump to server-only results (load value directly)
- Search result count in status bar
- Highlight match positions in key names

---

## Milestone 6 — Value UX

**Deliverables**
- Format toggles: Text / Hex / JSON (partially done)
- External editor integration (`$EDITOR`)
- Simple inline edit for strings
- Syntax highlighting for JSON

---

## Milestone 7 — TTL Visualizer

**Deliverables**
- Live countdown for selected key
- Color-coded TTL warnings (done in key list)
- Set/update TTL command

---

## Milestone 8 — Virtual Scrolling

**Goal:** Handle 1M+ keys without loading all into memory.

**Deliverables**
- Windowed key model with LRU cache
- Stable selection during async loads

---

## Milestone 9 — Bulk Operations

**Deliverables**
- Multi-select mode (Shift+Up/Down, `v` visual mode)
- Bulk delete with confirmation
- Bulk export (keys + values to file)

---

## Priority Order

| Priority | Milestone | Why |
|----------|-----------|-----|
| 1 | Command Palette | Unlocks all command-based features |
| 2 | Key Operations | Core CRUD, high user value |
| 3 | Debug Framework | Enables better debugging, benefits tui-dispatch |
| 4 | Search Improvements | Polish existing feature |
| 5+ | Others | As needed |

---

## See Also

- `ARCH_UPGRADE.md` - Completed architecture work
- `REFACTOR.md` - Code hygiene backlog
- `INTEGRATION_TESTS.md` - Test coverage tracking
