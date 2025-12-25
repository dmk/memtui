# Agent Notes

- Core loop: `src/main.rs` runs the Tokio event loop, owns `AppState`/`UiState`, and routes `Action`s (defined in `src/action.rs`) to sync/async handlers in `src/actions/`.
- State + app layer: `src/app/` holds the connection manager and event runner that spawn tasks and push results back into the action pipeline.
- Backends: `src/backend/` defines the `Backend` trait and the Redis/Memcached/etcd implementations with capability flags used by the UI.
- UI: `src/ui/` is ratatui-based with components like `KeyList`/`ValueViewer`, modals, and the status bar; JSON formatting lives in `src/formatter/json.rs`.
- Persistence + config: `src/userdata.rs` manages config/theme/keybindings/saved connections; `src/config.rs` handles config parsing; search utilities are in `src/search.rs`.

## Action Architecture Rules

Components MUST follow these rules:

1. **No direct state mutation** - `handle_event()` returns `Vec<Action>`, never mutates `self` state
2. **No `Action::Tick` after mutation** - if you mutate then return `Tick`, that's a bypass (bad)
3. **Actions are the only way to change state** - handlers in `src/actions/sync_handlers.rs` do the actual mutations
4. **New UI behavior = new Action** - add to `action.rs`, add handler, have component return it

Flow: `Event → Component::handle_event() → Action → sync_handlers → state mutation`

## After Meaningful Changes
Run the sanity suite before sending anything up:
```bash
make fmt test lint
```

