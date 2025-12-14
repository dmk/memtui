# Integration test notes

- Frameworks: `rstest` for parametrized tests/fixtures and `insta` for text snapshots. Snapshots live in `tests/snapshots/*.snap`.
- Rendering: `tests/support/render.rs` wraps `ratatui::TestBackend` to capture buffers and stringify either the full frame (`buffer_to_string`) or a region (`buffer_rect_to_string`).
- Harness: `tests/support/harness.rs` builds `AppState`/`UiState`/`Keybindings`, dispatches common actions, and can simulate async results via `apply_scan_result`/`apply_server_search`. Fixtures for connections/keys live in `tests/support/fixtures.rs`.
- Running tests: `cargo test --tests` runs integration suites; first-time snapshots produce `*.snap.new`. Use `cargo insta review` (or `cargo insta accept`) to accept updates and commit the resulting `.snap` files.
- Tips: keep snapshots focused (use regions to avoid noise), prefer shared fixtures for determinism, and extend `dispatch_action`/helpers as new scenarios land.
