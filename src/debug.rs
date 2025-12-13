use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

use crate::action::Action;
use crate::app::AppState;
use crate::ui::UiState;

pub enum DebugTableRow {
    Section(String),
    Entry { key: String, value: String },
}

/// Visual representation of a cell for the preview box.
#[derive(Clone)]
pub struct CellPreview {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
}

pub struct DebugTableOverlay {
    pub title: String,
    pub rows: Vec<DebugTableRow>,
    /// Optional cell preview for inspect overlays.
    pub cell_preview: Option<CellPreview>,
}

pub enum DebugOverlay {
    Inspect(DebugTableOverlay),
    State(DebugTableOverlay),
}

impl DebugOverlay {
    pub fn table(&self) -> &DebugTableOverlay {
        match self {
            DebugOverlay::Inspect(table) | DebugOverlay::State(table) => table,
        }
    }
}

#[derive(Default)]
pub struct DebugFreeze {
    pub enabled: bool,
    pub pending_capture: bool,
    pub snapshot: Option<Buffer>,
    pub snapshot_text: String,
    pub queued_actions: Vec<Action>,
    pub message: Option<String>,
    pub overlay: Option<DebugOverlay>,
    pub mouse_capture_enabled: bool,
}

struct DebugTableBuilder {
    rows: Vec<DebugTableRow>,
    cell_preview: Option<CellPreview>,
}

impl DebugTableBuilder {
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            cell_preview: None,
        }
    }

    fn section(&mut self, title: impl Into<String>) {
        self.rows.push(DebugTableRow::Section(title.into()));
    }

    fn entry(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.rows.push(DebugTableRow::Entry {
            key: key.into(),
            value: value.into(),
        });
    }

    fn set_cell_preview(&mut self, preview: CellPreview) {
        self.cell_preview = Some(preview);
    }

    fn finish(self, title: impl Into<String>) -> DebugTableOverlay {
        DebugTableOverlay {
            title: title.into(),
            rows: self.rows,
            cell_preview: self.cell_preview,
        }
    }
}

pub fn build_inspect_overlay(
    column: u16,
    row: u16,
    snapshot: &Buffer,
    app_state: &AppState,
    ui_state: &UiState,
) -> DebugTableOverlay {
    let mut b = DebugTableBuilder::new();

    b.section("Location");
    b.entry("pos", format!("({column}, {row})"));

    // Capture cell for visual preview (no table entries - shown in preview box)
    if point_in_rect(snapshot.area, column, row) {
        let cell = &snapshot[(column, row)];
        b.set_cell_preview(CellPreview {
            symbol: cell.symbol().to_string(),
            fg: cell.fg,
            bg: cell.bg,
            modifier: cell.modifier,
        });
    }

    b.section("UI");
    b.entry("active_panel", format!("{:?}", ui_state.active_panel));
    b.entry(
        "connection.active_id",
        format!("{:?}", app_state.connection_manager.get_active_id()),
    );

    if ui_state.show_connection_palette {
        b.section("Region");
        b.entry("region", "connection_palette");
        if let Some(area) = ui_state.connection_palette_area {
            b.entry("connection_palette.area", format!("{area:?}"));
            let total = app_state.connection_manager.get_configs().len();
            if total > 0 && point_in_rect(area, column, row) {
                if let Some(idx) = ui_state
                    .connection_list
                    .index_at_position(area, column, row, total)
                {
                    b.entry("connection_palette.index", idx.to_string());
                    if let Some(cfg) = app_state.connection_manager.get_configs().get(idx) {
                        b.entry("connection_palette.id", cfg.id.clone());
                        b.entry("connection_palette.name", cfg.name.clone());
                    }
                }
            }
        }
        return b.finish(format!("Inspect ({column},{row})"));
    }

    if app_state.connection_manager.get_active_id().is_none() {
        b.section("Region");
        b.entry("region", "welcome");
        if let Some(area) = ui_state.welcome_screen.last_list_area {
            b.entry("welcome.list_area", format!("{area:?}"));
        }
        return b.finish(format!("Inspect ({column},{row})"));
    }

    if let Some(region) = ui_state
        .tab_regions
        .iter()
        .find(|r| point_in_rect(r.area, column, row))
    {
        b.section("Region");
        b.entry("region", "tab");
        b.entry("tab.id", region.id.clone());
        b.entry("tab.area", format!("{:?}", region.area));
        return b.finish(format!("Inspect ({column},{row})"));
    }

    if let Some(body_area) = ui_state.last_body_area {
        if ui_state.pane_split.is_on_handle(body_area, column)
            && row >= body_area.y
            && row < body_area.y.saturating_add(body_area.height)
        {
            b.section("Region");
            b.entry("region", "resize_handle");
            b.entry(
                "pane_split.ratio",
                format!("{:.3}", ui_state.pane_split.ratio),
            );
            return b.finish(format!("Inspect ({column},{row})"));
        }
    }

    if let Some(area) = ui_state.last_key_area {
        if point_in_rect(area, column, row) {
            b.section("Region");
            b.entry("region", "key_list");
            b.entry("key_list.area", format!("{area:?}"));
            b.entry("search.query", format!("{:?}", app_state.search_query));

            if !app_state.search_query.is_empty() {
                if let Some(result_idx) =
                    search_result_index_from_position(area, column, row, app_state)
                {
                    b.entry("search.result_index", result_idx.to_string());
                    if let Some(&key_idx) = app_state.search_results_local.get(result_idx) {
                        b.entry("key.index", key_idx.to_string());
                        if let Some(key) = app_state.keys.get(key_idx).and_then(|k| k.as_ref()) {
                            b.entry("key.name", key.name.clone());
                        }
                    }
                }
            } else {
                let total_count = app_state
                    .total_key_count
                    .map(|t| t as usize)
                    .unwrap_or_else(|| app_state.keys.len());
                if let Some(key_idx) =
                    ui_state
                        .key_list
                        .index_at_position(area, column, row, total_count)
                {
                    b.entry("key.index", key_idx.to_string());
                    if let Some(key) = app_state.keys.get(key_idx).and_then(|k| k.as_ref()) {
                        b.entry("key.name", key.name.clone());
                    }
                    b.entry(
                        "key.selected",
                        (app_state.selected_key_index == Some(key_idx)).to_string(),
                    );
                }
            }

            return b.finish(format!("Inspect ({column},{row})"));
        }
    }

    if let Some(area) = ui_state.last_value_area {
        if point_in_rect(area, column, row) {
            b.section("Region");
            b.entry("region", "value_viewer");
            b.entry("value_viewer.area", format!("{area:?}"));
            b.entry(
                "value.scroll_offset",
                ui_state.value_viewer.scroll_offset.to_string(),
            );
            b.entry(
                "value.viewport_height",
                ui_state.value_viewer.viewport_height.to_string(),
            );
            b.entry(
                "value.total_rows",
                ui_state.value_viewer.total_rows.to_string(),
            );
            return b.finish(format!("Inspect ({column},{row})"));
        }
    }

    b.section("Region");
    b.entry("region", "<none>");
    b.finish(format!("Inspect ({column},{row})"))
}

pub fn build_state_overlay(
    app_state: &AppState,
    ui_state: &UiState,
    debug: &DebugFreeze,
) -> DebugTableOverlay {
    let mut b = DebugTableBuilder::new();

    push_debug_section(&mut b, debug);
    push_connection_section(&mut b, app_state);
    push_keys_section(&mut b, app_state, ui_state);
    push_value_section(&mut b, app_state);
    push_search_section(&mut b, app_state);
    push_ui_flags_section(&mut b, ui_state);
    push_ui_layout_section(&mut b, ui_state);

    b.finish("App State")
}

fn push_debug_section(b: &mut DebugTableBuilder, debug: &DebugFreeze) {
    b.section("Debug");
    b.entry("frozen", debug.enabled.to_string());
    b.entry("queued_actions", debug.queued_actions.len().to_string());
    b.entry("pending_capture", debug.pending_capture.to_string());
    b.entry("snapshot.captured", debug.snapshot.is_some().to_string());
    if let Some(snapshot) = &debug.snapshot {
        b.entry("snapshot.area", format!("{:?}", snapshot.area));
    }
    b.entry("snapshot_text.bytes", debug.snapshot_text.len().to_string());
    b.entry(
        "snapshot_text.lines",
        debug.snapshot_text.lines().count().to_string(),
    );
}

fn push_connection_section(b: &mut DebugTableBuilder, app_state: &AppState) {
    use crate::types::Auth;

    b.section("Connection");
    b.entry(
        "active_id",
        format!("{:?}", app_state.connection_manager.get_active_id()),
    );
    b.entry(
        "active_status",
        format!("{:?}", app_state.connection_manager.get_active_status()),
    );
    b.entry(
        "configs.len",
        app_state.connection_manager.get_configs().len().to_string(),
    );
    if let Some(active_id) = app_state.connection_manager.get_active_id() {
        if let Some(cfg) = app_state.connection_manager.get_config(active_id) {
            b.entry("config.name", cfg.name.clone());
            b.entry("config.backend", cfg.backend_type.to_string());
            b.entry("config.host", cfg.host.clone());
            b.entry("config.port", cfg.port.to_string());
            b.entry("config.database", format!("{:?}", cfg.database));
            b.entry("config.read_only", cfg.read_only.to_string());
            b.entry(
                "config.tls.enabled",
                cfg.tls
                    .as_ref()
                    .map(|t| t.enabled)
                    .unwrap_or(false)
                    .to_string(),
            );
            let auth = match cfg.auth.as_ref() {
                None => "none",
                Some(Auth::Token(_)) => "token",
                Some(Auth::UserPassword { .. }) => "user+pass",
                Some(Auth::Certificate { .. }) => "certificate",
            };
            b.entry("config.auth", auth);
        }
    }
}

fn push_keys_section(b: &mut DebugTableBuilder, app_state: &AppState, ui_state: &UiState) {
    b.section("Keys");
    b.entry("keys.loaded_len", app_state.keys.len().to_string());
    b.entry(
        "keys.total_count",
        format!("{:?}", app_state.total_key_count),
    );
    b.entry("keys.cursor", format!("{:?}", app_state.keys_cursor));
    b.entry("keys.has_more", app_state.has_more_keys.to_string());
    b.entry("keys.is_loading", app_state.is_loading_keys.to_string());
    b.entry("keys.per_chunk", app_state.keys_per_chunk.to_string());
    b.entry("viewport_height", app_state.viewport_height.to_string());
    b.entry(
        "selected_key_index",
        format!("{:?}", app_state.selected_key_index),
    );
    b.entry(
        "key_list.selected",
        format!("{:?}", ui_state.key_list.state.selected()),
    );
    if let Some(idx) = app_state.selected_key_index {
        if let Some(key) = app_state.keys.get(idx).and_then(|k| k.as_ref()) {
            b.entry("selected_key.name", key.name.clone());
        }
    }
}

fn push_value_section(b: &mut DebugTableBuilder, app_state: &AppState) {
    b.section("Value");
    b.entry(
        "selected_value.present",
        app_state.selected_value.is_some().to_string(),
    );
    if let Some(value) = app_state.selected_value.as_ref() {
        b.entry("selected_value.type", value.value_type.to_string());
        b.entry("selected_value.bytes", value.data.len().to_string());
        b.entry("selected_value.encoding", format!("{:?}", value.encoding));
    }
}

fn push_search_section(b: &mut DebugTableBuilder, app_state: &AppState) {
    b.section("Search");
    b.entry("is_searching", app_state.is_searching.to_string());
    b.entry("query", app_state.search_query.clone());
    b.entry(
        "results.local",
        app_state.search_results_local.len().to_string(),
    );
    b.entry(
        "results.server",
        app_state.search_results_server.len().to_string(),
    );
    b.entry(
        "server_searching",
        app_state.is_server_searching.to_string(),
    );
    b.entry(
        "selection_index",
        format!("{:?}", app_state.search_selection_index),
    );
}

fn push_ui_flags_section(b: &mut DebugTableBuilder, ui_state: &UiState) {
    b.section("UI Flags");
    b.entry("show_help", ui_state.show_help.to_string());
    b.entry(
        "show_connection_form",
        ui_state.show_connection_form.to_string(),
    );
    b.entry(
        "show_connection_palette",
        ui_state.show_connection_palette.to_string(),
    );
    b.entry(
        "show_quit_confirmation",
        ui_state.show_quit_confirmation.to_string(),
    );
}

fn push_ui_layout_section(b: &mut DebugTableBuilder, ui_state: &UiState) {
    b.section("UI Layout");
    b.entry(
        "pane_split.ratio",
        format!("{:.3}", ui_state.pane_split.ratio),
    );
    b.entry("is_resizing", ui_state.is_resizing.to_string());
    b.entry("last_body_area", format!("{:?}", ui_state.last_body_area));
    b.entry("last_key_area", format!("{:?}", ui_state.last_key_area));
    b.entry("last_value_area", format!("{:?}", ui_state.last_value_area));
    b.entry("tab_bar_area", format!("{:?}", ui_state.tab_bar_area));
}

fn search_result_index_from_position(
    area: Rect,
    column: u16,
    row: u16,
    app_state: &AppState,
) -> Option<usize> {
    if area.height <= 2 || area.width <= 2 {
        return None;
    }

    let inner_left = area.x.saturating_add(1);
    let inner_right = area.x.saturating_add(area.width.saturating_sub(1));
    if column < inner_left || column >= inner_right {
        return None;
    }

    let inner_top = area.y.saturating_add(1);
    let inner_bottom = area.y.saturating_add(area.height.saturating_sub(1));
    if row < inner_top || row >= inner_bottom {
        return None;
    }

    let result_count = app_state.search_results_local.len();
    if result_count == 0 {
        return None;
    }

    let rel = (row - inner_top) as usize;
    if rel >= result_count {
        return None;
    }

    Some(rel)
}

fn point_in_rect(area: Rect, column: u16, row: u16) -> bool {
    let within_x = column >= area.x && column < area.x.saturating_add(area.width);
    let within_y = row >= area.y && row < area.y.saturating_add(area.height);
    within_x && within_y
}
