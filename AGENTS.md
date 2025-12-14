# Agent Notes

- Core loop: `src/main.rs` runs the Tokio event loop, owns `AppState`/`UiState`, and routes `Action`s (defined in `src/action.rs`) to sync/async handlers in `src/actions/`.
- State + app layer: `src/app/` holds the connection manager and event runner that spawn tasks and push results back into the action pipeline.
- Backends: `src/backend/` defines the `Backend` trait and the Redis/Memcached/etcd implementations with capability flags used by the UI.
- UI: `src/ui/` is ratatui-based with components like `KeyList`/`ValueViewer`, modals, and the status bar; JSON formatting lives in `src/formatter/json.rs`.
- Persistence + config: `src/userdata.rs` manages config/theme/keybindings/saved connections; `src/config.rs` handles config parsing; search utilities are in `src/search.rs`.

## After Meaningful Changes
Run the sanity suite before sending anything up:
```bash
make fmt test lint
```
