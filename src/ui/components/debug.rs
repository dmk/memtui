use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Clear, Paragraph, Row, Table},
    Frame,
};

use crate::{
    debug::{CellPreview, DebugFreeze, DebugTableOverlay, DebugTableRow},
    keybindings::{format_key_for_display, BindingContext, KeybindingsConfig},
    ui::{
        self,
        theme::raw,
        widgets::{Modal, ModalStyle},
    },
};

pub fn render_debug_freeze(
    f: &mut Frame,
    app_state: &mut crate::app::AppState,
    ui_state: &mut crate::ui::UiState,
    keybindings: &KeybindingsConfig,
    debug: &mut DebugFreeze,
) {
    if debug.pending_capture || debug.snapshot.is_none() {
        ui::render(f, app_state, ui_state, keybindings);

        let snapshot = f.buffer_mut().clone();
        debug.snapshot_text = buffer_to_text(&snapshot);
        debug.snapshot = Some(snapshot);
        debug.pending_capture = false;
    } else if let Some(snapshot) = &debug.snapshot {
        paint_snapshot(f, snapshot);
    }

    render_debug_overlay(f, keybindings, debug);
}

fn paint_snapshot(f: &mut Frame, snapshot: &Buffer) {
    let screen = f.area();
    f.render_widget(Clear, screen);

    let snap_area = snapshot.area;
    let x_end = screen
        .x
        .saturating_add(screen.width)
        .min(snap_area.x.saturating_add(snap_area.width));
    let y_end = screen
        .y
        .saturating_add(screen.height)
        .min(snap_area.y.saturating_add(snap_area.height));

    for y in screen.y..y_end {
        for x in screen.x..x_end {
            let cell = snapshot[(x, y)].clone();
            f.buffer_mut()[(x, y)] = cell;
        }
    }
}

fn buffer_to_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();

    for y in area.y..area.y.saturating_add(area.height) {
        let mut line = String::new();
        for x in area.x..area.x.saturating_add(area.width) {
            line.push_str(buffer[(x, y)].symbol());
        }
        out.push_str(line.trim_end_matches(' '));
        if y + 1 < area.y.saturating_add(area.height) {
            out.push('\n');
        }
    }

    out
}

fn render_debug_overlay(f: &mut Frame, keybindings: &KeybindingsConfig, debug: &DebugFreeze) {
    if debug.overlay.is_some() {
        crate::ui::theme::dim_buffer_in_place(f.buffer_mut(), 0.7);
    }
    render_debug_banner(f, keybindings, debug);
    if let Some(overlay) = &debug.overlay {
        render_debug_table_overlay(f, overlay.table());
    }
}

fn render_debug_banner(f: &mut Frame, keybindings: &KeybindingsConfig, debug: &DebugFreeze) {
    let screen = f.area();
    if screen.height == 0 {
        return;
    }

    let banner_area = Rect {
        x: screen.x,
        y: screen.y.saturating_add(screen.height.saturating_sub(1)),
        width: screen.width,
        height: 1,
    };

    let toggle_key = keybindings
        .get_context_bindings(BindingContext::Debug)
        .get("debug.toggle")
        .and_then(|keys| {
            keys.iter()
                .find(|k| k.eq_ignore_ascii_case("f12"))
                .or_else(|| keys.first())
        })
        .map(|k| format_key_for_display(k))
        .unwrap_or_else(|| "F12".to_string());
    let copy_key = keybindings
        .get_first_keybinding("debug.copy_frame", BindingContext::Debug)
        .map(|k| format_key_for_display(&k))
        .unwrap_or_else(|| "Y".to_string());
    let state_key = keybindings
        .get_first_keybinding("debug.state.toggle", BindingContext::Debug)
        .map(|k| format_key_for_display(&k))
        .unwrap_or_else(|| "S".to_string());
    let mouse_key = keybindings
        .get_first_keybinding("debug.mouse.toggle", BindingContext::Debug)
        .map(|k| format_key_for_display(&k))
        .unwrap_or_else(|| "I".to_string());

    f.render_widget(
        Block::default().style(Style::default().bg(raw::BG_DEEP())),
        banner_area,
    );

    let key_style = |bg| {
        Style::default()
            .fg(raw::BG_DEEP())
            .bg(bg)
            .add_modifier(Modifier::BOLD)
    };
    let text_style = Style::default().fg(raw::TEXT_SECONDARY());

    let spans = vec![
        Span::styled(" DEBUG ", key_style(raw::NEON_PURPLE())),
        Span::raw(" "),
        Span::styled(format!(" {toggle_key} "), key_style(raw::NEON_PINK())),
        Span::styled(" resume ", text_style),
        Span::styled(format!(" {copy_key} "), key_style(raw::NEON_AMBER())),
        Span::styled(" copy ", text_style),
        Span::styled(format!(" {state_key} "), key_style(raw::NEON_CYAN())),
        Span::styled(" state ", text_style),
        Span::styled(format!(" {mouse_key} "), key_style(raw::ELECTRIC_BLUE())),
        Span::styled(
            if debug.mouse_capture_enabled {
                " mouse:capture "
            } else {
                " mouse:select "
            },
            text_style,
        ),
    ];

    let line = Paragraph::new(Line::from(spans)).style(Style::default().bg(raw::BG_DEEP()));
    f.render_widget(line, banner_area);
}

fn render_debug_table_overlay(f: &mut Frame, table: &DebugTableOverlay) {
    let screen = f.area();
    if screen.height < 2 {
        return;
    }

    let screen_wo_banner = Rect {
        x: screen.x,
        y: screen.y,
        width: screen.width,
        height: screen.height.saturating_sub(1),
    };
    if screen_wo_banner.height < 2 || screen_wo_banner.width < 10 {
        return;
    }

    let can_modal = screen_wo_banner.width >= 45 && screen_wo_banner.height >= 10;
    let inner = if can_modal {
        Modal::new(&table.title)
            .style(
                ModalStyle::new()
                    .size(80, 60)
                    .min_size(30, 8)
                    .max_size(120, 40),
            )
            .render(f, screen_wo_banner)
    } else {
        let height = (table.rows.len() as u16)
            .saturating_add(4)
            .clamp(4, screen_wo_banner.height);
        let area = Rect {
            x: screen_wo_banner.x,
            y: screen_wo_banner
                .y
                .saturating_add(screen_wo_banner.height.saturating_sub(height)),
            width: screen_wo_banner.width,
            height,
        };

        f.render_widget(Clear, area);
        let block = raw::modal_block(&table.title);
        let inner = block.inner(area);
        f.render_widget(block, area);
        inner
    };

    // Cell preview on top (if present), table below
    if let Some(preview) = &table.cell_preview {
        if inner.height > 3 {
            let preview_height = 2u16; // 1 line + 1 spacing
            let preview_area = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            };
            let table_area = Rect {
                x: inner.x,
                y: inner.y.saturating_add(preview_height),
                width: inner.width,
                height: inner.height.saturating_sub(preview_height),
            };
            render_cell_preview(f, preview_area, preview);
            render_debug_table(f, table_area, &table.rows);
            return;
        }
    }

    render_debug_table(f, inner, &table.rows);
}

/// Render a compact cell preview with the character and its colors inline.
fn render_cell_preview(f: &mut Frame, area: Rect, preview: &CellPreview) {
    if area.width < 20 || area.height < 1 {
        return;
    }

    // Render the character with its actual style
    let char_style = Style::default()
        .fg(preview.fg)
        .bg(preview.bg)
        .add_modifier(preview.modifier);

    // Format RGB values compactly
    let fg_rgb = format_color_compact(preview.fg);
    let bg_rgb = format_color_compact(preview.bg);
    let mod_str = if preview.modifier.is_empty() {
        String::new()
    } else {
        format!(" {:?}", preview.modifier)
    };

    // Single line: [char] fg: RGB bg: RGB mod
    let line = Line::from(vec![
        Span::styled(" ", Style::default().bg(raw::BG_SURFACE())),
        Span::styled(preview.symbol.clone(), char_style),
        Span::styled(" ", Style::default().bg(raw::BG_SURFACE())),
        Span::styled("  fg ", Style::default().fg(raw::TEXT_DIM())),
        Span::styled("█", Style::default().fg(preview.fg)),
        Span::styled(
            format!(" {fg_rgb}"),
            Style::default().fg(raw::TEXT_SECONDARY()),
        ),
        Span::styled("  bg ", Style::default().fg(raw::TEXT_DIM())),
        Span::styled("█", Style::default().fg(preview.bg)),
        Span::styled(
            format!(" {bg_rgb}"),
            Style::default().fg(raw::TEXT_SECONDARY()),
        ),
        Span::styled(mod_str, Style::default().fg(raw::NEON_PURPLE())),
    ]);

    let preview_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    f.render_widget(Paragraph::new(line), preview_area);
}

/// Format a Color as a compact RGB string.
fn format_color_compact(color: ratatui::style::Color) -> String {
    match color {
        ratatui::style::Color::Rgb(r, g, b) => format!("({r},{g},{b})"),
        other => format!("{other:?}"),
    }
}

fn render_debug_table(f: &mut Frame, area: Rect, rows: &[DebugTableRow]) {
    if area.height < 2 || area.width < 10 {
        return;
    }

    let max_key_len = rows
        .iter()
        .filter_map(|row| match row {
            DebugTableRow::Entry { key, .. } => Some(key.chars().count()),
            DebugTableRow::Section(_) => None,
        })
        .max()
        .unwrap_or(0) as u16;

    let max_label = area.width.saturating_sub(8).max(10);
    let label_width = max_key_len.saturating_add(2).clamp(12, 30).min(max_label);
    let constraints = [Constraint::Length(label_width), Constraint::Min(0)];

    let header_style = Style::default()
        .fg(raw::NEON_CYAN())
        .bg(raw::BG_PANEL())
        .add_modifier(Modifier::BOLD);
    let header = Row::new(vec![
        Cell::from("Field").style(header_style),
        Cell::from("Value").style(header_style),
    ]);

    let table_rows: Vec<Row> = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| match row {
            DebugTableRow::Section(title) => Row::new(vec![
                Cell::from(format!(" {title} ")).style(
                    Style::default()
                        .fg(raw::NEON_PURPLE())
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(""),
            ])
            .style(Style::default().bg(raw::BG_DEEP())),
            DebugTableRow::Entry { key, value } => {
                let key_style = Style::default()
                    .fg(raw::NEON_AMBER())
                    .add_modifier(Modifier::BOLD);
                let value_style = Style::default().fg(raw::TEXT_PRIMARY());
                let row_style = if idx % 2 == 0 {
                    Style::default().bg(raw::BG_PANEL())
                } else {
                    Style::default().bg(raw::BG_SURFACE())
                };
                Row::new(vec![
                    Cell::from(key.clone()).style(key_style),
                    Cell::from(value.clone()).style(value_style),
                ])
                .style(row_style)
            }
        })
        .collect();

    let table_widget = Table::new(table_rows, constraints)
        .header(header)
        .column_spacing(2);
    f.render_widget(table_widget, area);
}
