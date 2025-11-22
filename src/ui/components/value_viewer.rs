use super::Component;
use crate::action::Action;
use crate::formatter::{Formatter, JsonFormatter, TextFormatter};
use crate::types::{Value, ValueType};
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};
use serde_json::Value as JsonValue;

pub struct ValueViewerProps<'a> {
    pub selected_value: Option<&'a Value>,
    pub selected_key_type: Option<ValueType>,
    pub error_message: Option<&'a String>,
    pub json_formatter: &'a JsonFormatter,
    pub text_formatter: &'a TextFormatter,
    pub is_active: bool,
}

pub struct ValueViewer;

impl ValueViewer {
    pub fn new() -> Self {
        Self
    }

    fn viewer_block(is_active: bool, title: impl Into<String>) -> Block<'static> {
        Block::default()
            .title(Line::from(title.into()))
            .borders(Borders::ALL)
            .border_style(if is_active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            })
    }

    fn render_paragraph(
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        title: &str,
        lines: Vec<Line<'static>>,
        style: Style,
    ) {
        let widget = Paragraph::new(lines)
            .style(style)
            .block(Self::viewer_block(is_active, title))
            .wrap(Wrap { trim: false });

        f.render_widget(widget, area);
    }

    fn render_hash_table(
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        base_title: &str,
        entries: Vec<(String, String)>,
    ) {
        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let entry_count = entries.len();
        let table_title = if entry_count == 1 {
            format!("{base_title} | 1 field")
        } else {
            format!("{base_title} | {} fields", entry_count)
        };

        let field_column_width = entries
            .iter()
            .map(|(field, _)| field.chars().count())
            .max()
            .unwrap_or(6)
            .clamp(8, 32) as u16;

        let mut rows: Vec<Row> = entries
            .into_iter()
            .enumerate()
            .map(|(idx, (field, value))| {
                let field_cell = Cell::from(field).style(Style::default().fg(Color::Yellow));
                let value_cell = Cell::from(Self::clean_table_value(&value));
                let row_style = if idx % 2 == 0 {
                    Style::default()
                } else {
                    Style::default().bg(Color::Rgb(20, 20, 20))
                };
                Row::new(vec![field_cell, value_cell]).style(row_style)
            })
            .collect();

        if rows.is_empty() {
            rows.push(Row::new(vec![Cell::from("<empty>"), Cell::from("")]));
        }

        let table = Table::new(
            rows,
            [Constraint::Length(field_column_width), Constraint::Min(10)],
        )
        .header(Row::new(vec![
            Cell::from("Field").style(header_style),
            Cell::from("Value").style(header_style),
        ]))
        .column_spacing(2)
        .block(Self::viewer_block(is_active, table_title));

        f.render_widget(table, area);
    }

    fn format_value_lines(
        value: &Value,
        props: &ValueViewerProps<'_>,
    ) -> (Vec<Line<'static>>, Style) {
        if props.json_formatter.can_format(value) {
            match props.json_formatter.format_to_lines(value) {
                Ok(json_lines) => (json_lines, Style::default()),
                Err(_) => Self::format_with_text(value, props),
            }
        } else {
            Self::format_with_text(value, props)
        }
    }

    fn format_with_text(
        value: &Value,
        props: &ValueViewerProps<'_>,
    ) -> (Vec<Line<'static>>, Style) {
        match props.text_formatter.format(value) {
            Ok(text) => (
                text.lines().map(|l| Line::from(l.to_string())).collect(),
                Style::default(),
            ),
            Err(_) => (
                vec![Line::from("<formatting error>")],
                Style::default().fg(Color::Red),
            ),
        }
    }

    fn parse_hash_entries(value: &Value) -> Result<Vec<(String, String)>, String> {
        let json_str = String::from_utf8(value.data.clone())
            .map_err(|_| "Invalid UTF-8 in hash value".to_string())?;
        let parsed: JsonValue =
            serde_json::from_str(&json_str).map_err(|e| format!("Invalid hash payload: {e}"))?;
        let obj = parsed
            .as_object()
            .ok_or_else(|| "Hash value is not an object".to_string())?;

        let mut entries: Vec<(String, String)> = obj
            .iter()
            .map(|(field, value)| (field.clone(), Self::json_value_to_string(value)))
            .collect();

        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }

    fn json_value_to_string(value: &JsonValue) -> String {
        match value {
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "null".to_string(),
            JsonValue::Array(items) => format!("[{} items]", items.len()),
            JsonValue::Object(_) => "{...}".to_string(),
        }
    }

    fn clean_table_value(value: &str) -> String {
        const MAX_CHARS: usize = 120;
        let single_line = value.replace('\n', " <nl> ");
        if single_line.chars().count() > MAX_CHARS {
            let mut truncated: String = single_line
                .chars()
                .take(MAX_CHARS.saturating_sub(3))
                .collect();
            truncated.push_str("...");
            truncated
        } else {
            single_line
        }
    }

    fn title_for(value_type: Option<ValueType>) -> String {
        match value_type {
            Some(t) => format!("Value Viewer | {}", t),
            None => "Value Viewer".to_string(),
        }
    }
}

impl Default for ValueViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ValueViewer {
    type Props<'a> = ValueViewerProps<'a>;
    type Msg = Action;

    fn render(&mut self, f: &mut Frame, area: Rect, props: Self::Props<'_>) {
        let base_title = ValueViewer::title_for(props.selected_key_type);

        if let Some(err) = props.error_message {
            ValueViewer::render_paragraph(
                f,
                area,
                props.is_active,
                &base_title,
                vec![Line::from(format!("Error: {}", err))],
                Style::default().fg(Color::Red),
            );
            return;
        }

        if let Some(value) = props.selected_value {
            if matches!(props.selected_key_type, Some(ValueType::Hash)) {
                match ValueViewer::parse_hash_entries(value) {
                    Ok(entries) => {
                        ValueViewer::render_hash_table(
                            f,
                            area,
                            props.is_active,
                            &base_title,
                            entries,
                        );
                        return;
                    }
                    Err(parse_err) => {
                        ValueViewer::render_paragraph(
                            f,
                            area,
                            props.is_active,
                            &base_title,
                            vec![Line::from(format!("Hash parse error: {}", parse_err))],
                            Style::default().fg(Color::Red),
                        );
                        return;
                    }
                }
            }

            let (lines, style) = ValueViewer::format_value_lines(value, &props);
            ValueViewer::render_paragraph(f, area, props.is_active, &base_title, lines, style);
            return;
        }

        ValueViewer::render_paragraph(
            f,
            area,
            props.is_active,
            &base_title,
            vec![Line::from("Select a key to view its value")],
            Style::default().fg(Color::DarkGray),
        );
    }

    fn handle_input(&mut self, _key: KeyEvent, _props: Self::Props<'_>) -> Option<Self::Msg> {
        None
    }
}
