use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
pub use tui_dispatch::debug::{
    ActionLoggerConfig, DebugFreeze, DebugOverlay, DebugTableOverlay, DebugTableRow,
};
use tui_dispatch::debug::{CellPreview, DebugSection, DebugState, DebugTableBuilder};

use crate::action::Action;
use crate::app::AppState;
use crate::ui::UiState;

/// Helper to check if a point is within a rect.
fn point_in_rect(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

pub fn build_inspect_overlay(
    column: u16,
    row: u16,
    snapshot: &Buffer,
    app_state: &AppState,
    ui_state: &UiState,
) -> DebugTableOverlay {
    let mut b = DebugTableBuilder::new();

    b.push_section("Location");
    b.push_entry("pos", format!("({column}, {row})"));

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

    b.push_section("UI");
    b.push_entry("active_panel", format!("{:?}", ui_state.active_panel));
    b.push_entry(
        "connection.active_id",
        format!("{:?}", app_state.connection_manager.get_active_id()),
    );

    if ui_state.show_connection_palette {
        b.push_section("Region");
        b.push_entry("region", "connection_palette");
        if let Some(area) = ui_state.connection_palette_area {
            b.push_entry("connection_palette.area", format!("{area:?}"));
            let total = app_state.connection_manager.get_configs().len();
            if total > 0 && point_in_rect(area, column, row) {
                if let Some(idx) = ui_state
                    .connection_list
                    .index_at_position(area, column, row, total)
                {
                    b.push_entry("connection_palette.index", idx.to_string());
                    if let Some(cfg) = app_state.connection_manager.get_configs().get(idx) {
                        b.push_entry("connection_palette.id", cfg.id.clone());
                        b.push_entry("connection_palette.name", cfg.name.clone());
                    }
                }
            }
        }
        return b.finish(format!("Inspect ({column},{row})"));
    }

    if app_state.connection_manager.get_active_id().is_none() {
        b.push_section("Region");
        b.push_entry("region", "welcome");
        if let Some(area) = ui_state.welcome_screen.last_list_area {
            b.push_entry("welcome.list_area", format!("{area:?}"));
        }
        return b.finish(format!("Inspect ({column},{row})"));
    }

    if let Some(region) = ui_state
        .tab_regions
        .iter()
        .find(|r| point_in_rect(r.area, column, row))
    {
        b.push_section("Region");
        b.push_entry("region", "tab");
        b.push_entry("tab.id", region.id.clone());
        b.push_entry("tab.area", format!("{:?}", region.area));
        return b.finish(format!("Inspect ({column},{row})"));
    }

    if let Some(body_area) = ui_state.last_body_area {
        if ui_state.pane_split.is_on_handle(body_area, column)
            && row >= body_area.y
            && row < body_area.y.saturating_add(body_area.height)
        {
            b.push_section("Region");
            b.push_entry("region", "resize_handle");
            b.push_entry(
                "pane_split.ratio",
                format!("{:.3}", ui_state.pane_split.ratio),
            );
            return b.finish(format!("Inspect ({column},{row})"));
        }
    }

    if let Some(area) = ui_state.last_key_area {
        if point_in_rect(area, column, row) {
            b.push_section("Region");
            b.push_entry("region", "key_list");
            b.push_entry("key_list.area", format!("{area:?}"));
            b.push_entry("cmdline.buffer", format!("{:?}", app_state.cmdline_buffer));

            if !app_state.cmdline_buffer.is_empty() {
                if let Some(result_idx) =
                    search_result_index_from_position(area, column, row, app_state)
                {
                    b.push_entry("search.result_index", result_idx.to_string());
                    if let Some(&key_idx) = app_state.search_results_local.get(result_idx) {
                        b.push_entry("key.index", key_idx.to_string());
                        if let Some(key) = app_state.keys.get(key_idx).and_then(|k| k.as_ref()) {
                            b.push_entry("key.name", key.name.clone());
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
                    b.push_entry("key.index", key_idx.to_string());
                    if let Some(key) = app_state.keys.get(key_idx).and_then(|k| k.as_ref()) {
                        b.push_entry("key.name", key.name.clone());
                    }
                    b.push_entry(
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
            b.push_section("Region");
            b.push_entry("region", "value_viewer");
            b.push_entry("value_viewer.area", format!("{area:?}"));
            b.push_entry(
                "value.scroll_offset",
                ui_state.value_viewer.scroll_offset.to_string(),
            );
            b.push_entry(
                "value.viewport_height",
                ui_state.value_viewer.viewport_height.to_string(),
            );
            b.push_entry(
                "value.total_rows",
                ui_state.value_viewer.total_rows.to_string(),
            );
            return b.finish(format!("Inspect ({column},{row})"));
        }
    }

    b.push_section("Region");
    b.push_entry("region", "<none>");
    b.finish(format!("Inspect ({column},{row})"))
}

pub fn build_state_overlay(
    app_state: &AppState,
    ui_state: &UiState,
    debug: &DebugFreeze<Action>,
) -> DebugTableOverlay {
    // Combine sections from DebugState impls + debug freeze info
    let mut sections = vec![DebugSection::new("Debug")
        .entry("frozen", debug.enabled.to_string())
        .entry("queued_actions", debug.queued_actions.len().to_string())
        .entry("pending_capture", debug.pending_capture.to_string())
        .entry("snapshot.captured", debug.snapshot.is_some().to_string())];

    // Add sections from AppState and UiState
    sections.extend(app_state.debug_sections());
    sections.extend(ui_state.debug_sections());

    // Build the overlay
    let mut builder = DebugTableBuilder::new();
    for section in sections {
        builder.push_section(&section.title);
        for entry in section.entries {
            builder.push_entry(entry.key, entry.value);
        }
    }
    builder.finish("App State")
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
