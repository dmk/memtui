# Changelog

## [0.4.0] - 2024-12-28

### Changed
- Extract core TUI architecture to `tui-dispatch` framework crate
- Update to ratatui 0.29, crossterm 0.28
- All state changes now go through unified action dispatch system

### Added
- Derive macros for Action, ComponentId, BindingContext (via tui-dispatch)
- Binary value view mode support
- Actions logger for debugging
- Custom terminal input handling
- SearchInput component

### Fixed
- Connection deletion now works correctly
- Mouse scroll in various components
- Key list hardcoded escape key handling
- Search functionality improvements

### Removed
- Hardcoded keybindings (now fully configurable)
- Dispatch helpers (replaced by action system)

## [0.3.3] - 2024-12-15

### Added
- TTL display for keys
- UI debug functionality (toggle with debug keybinding)
- Integration test foundation

## [0.3.2] - 2024-12-14

### Fixed
- GitHub Actions release workflow
- Build configuration fixes

## [0.3.0] - 2024-12-13

### Changed
- Complete event-driven architecture overhaul
- Centralized action dispatch system
- Extract action handlers from main loop

### Added
- Event bus foundation for component communication
- Search improvements with local and server-side search

### Fixed
- Unified and cleaned up UI
- Various UX improvements
