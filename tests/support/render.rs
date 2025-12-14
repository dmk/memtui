use std::sync::Once;

use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

use memtui::ui::{init_theme, UiState};
use memtui::{app::AppState, keybindings::KeybindingsConfig};

static INIT: Once = Once::new();

/// Ensure the global theme is initialized before rendering.
pub fn ensure_theme() {
    INIT.call_once(|| init_theme(memtui::ui::theme::ThemeConfig::default()));
}

/// Render the application into a ratatui `Buffer` using the test backend.
pub fn render_app(
    size: (u16, u16),
    app_state: &mut AppState,
    ui_state: &mut UiState,
    keybindings: &KeybindingsConfig,
) -> Buffer {
    ensure_theme();

    let backend = TestBackend::new(size.0, size.1);
    let mut terminal = Terminal::new(backend).expect("failed to build test terminal");

    terminal
        .draw(|f| memtui::ui::render(f, app_state, ui_state, keybindings))
        .expect("render failed");

    terminal.backend().buffer().clone()
}

/// Convert a full buffer into a string for snapshotting.
#[allow(dead_code)]
pub fn buffer_to_string(buffer: &Buffer) -> String {
    buffer_rect_to_string(buffer, buffer.area)
}

/// Convert a rectangular region of the buffer into a trimmed, human-friendly string.
#[allow(dead_code)]
pub fn buffer_rect_to_string(buffer: &Buffer, area: Rect) -> String {
    let mut lines = Vec::with_capacity(area.height as usize);

    for y in area.y..area.y.saturating_add(area.height) {
        let mut line = String::with_capacity(area.width as usize);
        for x in area.x..area.x.saturating_add(area.width) {
            line.push_str(buffer[(x, y)].symbol());
        }
        lines.push(line.trim_end().to_string());
    }

    lines.join("\n")
}
