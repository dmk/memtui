use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::modal::render_modal;
use crate::keybindings::{format_key_for_display, BindingContext, KeybindingsConfig};
use crate::ui::theme::{self, raw};

pub fn render_help(f: &mut Frame, keybindings: &KeybindingsConfig) {
    let area = theme::centered_rect(50, 60, f.area());
    let context = BindingContext::Default;

    // Helper to get keybinding
    let get_key = |action: &str, default: &str| -> String {
        keybindings
            .get_first_keybinding(action, context)
            .map(|k| format_key_for_display(&k))
            .unwrap_or_else(|| default.to_string())
    };

    // Get keybindings from config
    let next_panel = get_key("navigation.next_panel", "Tab");
    let prev_panel = get_key("navigation.prev_panel", "⇧Tab");
    let next_item = get_key("navigation.next_item", "↓");
    let prev_item = get_key("navigation.prev_item", "↑");
    let search = get_key("search.start", "/");
    let value_view_mode = get_key("value.view_mode.cycle", "V");
    let palette = get_key("connection.palette.toggle", "^O");
    let new_conn = get_key("connection.form.open", "^N");
    let tab_next = get_key("connection.tab.next", "^→");
    let tab_prev = get_key("connection.tab.prev", "^←");
    let resize_left = get_key("pane.resize.left", "^H");
    let resize_right = get_key("pane.resize.right", "^L");
    let help = get_key("help.toggle", "?");
    let quit = get_key("quit.show", "Q");
    let debug_toggle = get_key("debug.toggle", "F12");
    let debug_copy = keybindings
        .get_first_keybinding("debug.copy_frame", BindingContext::Debug)
        .map(|k| format_key_for_display(&k))
        .unwrap_or_else(|| "Y".to_string());
    let debug_state = keybindings
        .get_first_keybinding("debug.state.toggle", BindingContext::Debug)
        .map(|k| format_key_for_display(&k))
        .unwrap_or_else(|| "S".to_string());
    let debug_mouse = keybindings
        .get_first_keybinding("debug.mouse.toggle", BindingContext::Debug)
        .map(|k| format_key_for_display(&k))
        .unwrap_or_else(|| "I".to_string());

    let help_sections: Vec<(&str, Vec<(&str, String)>)> = vec![
        (
            "Navigation",
            vec![
                ("Switch panels", format!("{}  {}", next_panel, prev_panel)),
                ("Move up/down", format!("{}  {}", prev_item, next_item)),
                ("Search keys", search),
                ("Value view mode", value_view_mode),
            ],
        ),
        (
            "Connections",
            vec![
                ("Connection palette", palette),
                ("New connection", new_conn),
                ("Switch tabs", format!("{}  {}", tab_prev, tab_next)),
            ],
        ),
        (
            "Panes",
            vec![
                ("Resize panes", format!("{}  {}", resize_left, resize_right)),
                ("Mouse resize", "drag border".to_string()),
            ],
        ),
        (
            "Debug",
            vec![
                ("Freeze / resume", debug_toggle),
                ("Copy frozen frame", debug_copy),
                ("Show app state", debug_state),
                ("Inspect cell", format!("{debug_mouse} + Click")),
                ("Toggle mouse capture", debug_mouse),
            ],
        ),
        ("General", vec![("Toggle help", help), ("Quit", quit)]),
    ];

    // Render modal with dimmed background
    let block_inner = render_modal(f, area, "Help");

    // Calculate inner area with extra padding
    let inner = Rect {
        x: block_inner.x + 1,
        y: block_inner.y + 1,
        width: block_inner.width.saturating_sub(2),
        height: block_inner.height.saturating_sub(1),
    };

    // Render title centered (use raw colors for modal content)
    let title = Paragraph::new(Span::styled(
        "Keyboard Shortcuts",
        Style::default()
            .fg(raw::NEON_CYAN())
            .add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center);

    let title_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(title, title_area);

    // Render sections
    let mut y_offset = 2u16;

    for (section_name, bindings) in help_sections {
        // Section header
        let section_y = inner.y + y_offset;
        if section_y >= inner.y + inner.height {
            break;
        }

        let section_header = Paragraph::new(Span::styled(
            section_name,
            Style::default()
                .fg(raw::NEON_AMBER())
                .add_modifier(Modifier::BOLD),
        ));
        f.render_widget(
            section_header,
            Rect {
                x: inner.x + 2,
                y: section_y,
                width: inner.width.saturating_sub(4),
                height: 1,
            },
        );
        y_offset += 1;

        // Bindings
        for (desc, key) in bindings {
            let binding_y = inner.y + y_offset;
            if binding_y >= inner.y + inner.height.saturating_sub(1) {
                break;
            }

            // Calculate available width for the line
            let line_width = inner.width.saturating_sub(10) as usize;
            let desc_len = desc.chars().count();
            let key_len = key.chars().count();
            let fill_len = line_width.saturating_sub(desc_len + key_len);

            // Create dotted fill using ASCII dots for consistent spacing
            let fill = if fill_len > 2 {
                format!(" {} ", ".".repeat(fill_len - 2))
            } else {
                " ".repeat(fill_len)
            };

            // Create the full line: "Description ..... Key"
            let line = Line::from(vec![
                Span::styled(desc, Style::default().fg(raw::TEXT_DIM())),
                Span::styled(fill, Style::default().fg(Color::Rgb(55, 60, 70))),
                Span::styled(&key, Style::default().fg(raw::NEON_CYAN())),
            ]);

            let line_widget = Paragraph::new(line);
            f.render_widget(
                line_widget,
                Rect {
                    x: inner.x + 4,
                    y: binding_y,
                    width: inner.width.saturating_sub(6),
                    height: 1,
                },
            );

            y_offset += 1;
        }
        y_offset += 1; // Space between sections
    }

    // Footer
    let footer_y = inner.y + inner.height.saturating_sub(1);
    let footer = Paragraph::new(Span::styled(
        "Press any key to close",
        Style::default()
            .fg(raw::TEXT_DIM())
            .add_modifier(Modifier::ITALIC),
    ))
    .alignment(Alignment::Center);
    f.render_widget(
        footer,
        Rect {
            x: inner.x,
            y: footer_y,
            width: inner.width,
            height: 1,
        },
    );
}
