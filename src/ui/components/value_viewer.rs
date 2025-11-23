use super::Component;
use crate::action::Action;
use crate::formatter::{Formatter, JsonFormatter, TextFormatter};
use crate::types::{Value, ValueType};
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState, Wrap,
    },
};

use serde_json::Value as JsonValue;
use unicode_width::UnicodeWidthChar;

pub struct ValueViewerProps<'a> {
    pub selected_value: Option<&'a Value>,
    pub selected_key_type: Option<ValueType>,
    pub error_message: Option<&'a String>,
    pub json_formatter: &'a JsonFormatter,
    pub text_formatter: &'a TextFormatter,
    pub is_active: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TextContentKind {
    Plain,
    Json,
}

struct TextCache {
    source_ptr: usize,
    source_len: usize,
    wrap_width: u16,
    kind: TextContentKind,
    // Store UNWRAPPED lines - wrap on-demand only for visible portion
    unwrapped_lines: Vec<Line<'static>>,
    paragraph_style: Style,
    // Cached total wrapped line count (calculated once)
    total_wrapped_lines: usize,
    // Cache wrapped line count per unwrapped line for fast lookups
    wrapped_counts: Vec<usize>,
    // Cache wrapped lines per unwrapped line to avoid re-wrapping on every render
    wrapped_cache: Vec<Option<Vec<Line<'static>>>>,
}

impl TextCache {
    fn new(
        value: &Value,
        kind: TextContentKind,
        paragraph_style: Style,
        unwrapped_lines: Vec<Line<'static>>,
        wrap_width: u16,
    ) -> Self {
        let width = wrap_width.max(1) as usize;

        // Pre-calculate wrapped line count per unwrapped line (cheap - just math)
        // This allows fast lookups without wrapping everything
        let wrapped_counts: Vec<usize> = unwrapped_lines
            .iter()
            .map(|line| {
                let line_width = line.width();
                if line_width == 0 {
                    1
                } else {
                    line_width.div_ceil(width)
                }
            })
            .collect();

        let total_wrapped_lines: usize = wrapped_counts.iter().sum();

        // Pre-allocate cache but don't wrap yet (lazy)
        let wrapped_cache = vec![None; unwrapped_lines.len()];

        Self {
            source_ptr: value.data.as_ptr() as usize,
            source_len: value.data.len(),
            wrap_width: wrap_width.max(1),
            kind,
            unwrapped_lines,
            paragraph_style,
            total_wrapped_lines,
            wrapped_counts,
            wrapped_cache,
        }
    }

    /// Get wrapped lines for visible range only (lazy wrapping with caching)
    /// Uses cached wrapped_counts to quickly find which unwrapped lines to process
    /// Caches wrapped results to avoid re-wrapping on every render
    fn get_visible_wrapped_lines(
        &mut self,
        start_wrapped_line: usize,
        end_wrapped_line: usize,
    ) -> Vec<Line<'static>> {
        let mut result = Vec::new();
        let mut current_wrapped_index = 0usize;

        // Use cached counts to quickly find starting unwrapped line
        for (idx, unwrapped) in self.unwrapped_lines.iter().enumerate() {
            let wrapped_count = self.wrapped_counts.get(idx).copied().unwrap_or(1);

            // Skip unwrapped lines that are before our visible range
            if current_wrapped_index + wrapped_count <= start_wrapped_line {
                current_wrapped_index += wrapped_count;
                continue;
            }

            // Get wrapped lines from cache or wrap and cache
            let wrapped = if let Some(cached) = self.wrapped_cache.get_mut(idx).and_then(|c| c.as_ref()) {
                // Use cached wrapped lines
                cached.clone()
            } else {
                // Wrap and cache
                let wrapped = ValueViewer::wrap_single_line(unwrapped, self.wrap_width as usize);
                if let Some(cache_slot) = self.wrapped_cache.get_mut(idx) {
                    *cache_slot = Some(wrapped.clone());
                }
                wrapped
            };

            for wrapped_line in wrapped {
                if current_wrapped_index >= start_wrapped_line && current_wrapped_index < end_wrapped_line {
                    result.push(wrapped_line);
                }

                current_wrapped_index += 1;

                // Stop early if we've passed the end
                if current_wrapped_index >= end_wrapped_line {
                    return result;
                }
            }
        }

        result
    }

    fn matches(
        &self,
        value: &Value,
        kind: TextContentKind,
        width: u16,
        paragraph_style: Style,
    ) -> bool {
        self.source_ptr == value.data.as_ptr() as usize
            && self.source_len == value.data.len()
            && self.wrap_width == width.max(1)
            && self.kind == kind
            && self.paragraph_style == paragraph_style
    }
}

pub struct ValueViewer {
    pub table_state: TableState,
    pub scroll_offset: u16,
    pub total_rows: usize,
    pub viewport_height: u16,
    pub scrollbar_state: ScrollbarState,
    text_cache: Option<TextCache>,
}

impl ValueViewer {
    pub fn new() -> Self {
        Self {
            table_state: TableState::default(),
            scroll_offset: 0,
            total_rows: 0,
            viewport_height: 0,
            scrollbar_state: ScrollbarState::default(),
            text_cache: None,
        }
    }

    pub fn scroll_down(&mut self) {
        // Early return if there's nothing to scroll
        if self.total_rows == 0 || self.viewport_height == 0 {
            return;
        }

        if self.table_state.selected().is_some() {
            let i = self.table_state.selected().unwrap_or(0);
            if i < self.total_rows.saturating_sub(1) {
                self.table_state.select(Some(i.saturating_add(1)));
            }
        } else {
            let max_scroll = if self.total_rows > self.viewport_height as usize {
                (self.total_rows - self.viewport_height as usize) as u16
            } else {
                0
            };

            if self.scroll_offset < max_scroll {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
        }
    }

    pub fn scroll_up(&mut self) {
        // Early return if there's nothing to scroll
        if self.total_rows == 0 || self.viewport_height == 0 {
            return;
        }

        if self.table_state.selected().is_some() {
            let i = self.table_state.selected().unwrap_or(0);
            self.table_state.select(Some(i.saturating_sub(1)));
        } else {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        }
    }

    /// Scroll by a delta amount (positive = down, negative = up)
    /// More efficient than calling scroll_up/down in a loop
    pub fn scroll_by(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }

        // Early return if there's nothing to scroll
        if self.total_rows == 0 || self.viewport_height == 0 {
            return;
        }

        if self.table_state.selected().is_some() {
            let i = self.table_state.selected().unwrap_or(0);
            if delta > 0 {
                let new_i = i.saturating_add(delta as usize).min(self.total_rows.saturating_sub(1));
                self.table_state.select(Some(new_i));
            } else {
                let new_i = i.saturating_sub((-delta) as usize);
                self.table_state.select(Some(new_i));
            }
        } else {
            // Calculate max scroll - prevent scrolling beyond bounds
            let max_scroll = if self.total_rows > self.viewport_height as usize {
                (self.total_rows - self.viewport_height as usize) as u16
            } else {
                0 // No scrolling needed if content fits in viewport
            };

            if max_scroll == 0 {
                // Nothing to scroll, reset to 0
                self.scroll_offset = 0;
                return;
            }

            if delta > 0 {
                let new_offset = self.scroll_offset.saturating_add(delta as u16).min(max_scroll);
                self.scroll_offset = new_offset;
            } else {
                let new_offset = self.scroll_offset.saturating_sub((-delta) as u16);
                self.scroll_offset = new_offset; // saturating_sub already handles underflow
            }
        }
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
        self.table_state.select(None);
        self.scrollbar_state = ScrollbarState::default();
    }

    fn clear_text_cache(&mut self) {
        self.text_cache = None;
    }

    fn viewer_block(is_active: bool, title: impl Into<String>) -> Block<'static> {
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        Block::default()
            .title(Line::from(title.into()))
            .title_alignment(Alignment::Left)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
    }

    fn render_scrollbar_for_offset(&mut self, f: &mut Frame, area: Rect) {
        if self.total_rows <= self.viewport_height as usize {
            return;
        }

        self.scrollbar_state = self
            .scrollbar_state
            .content_length(self.total_rows)
            .viewport_content_length(self.viewport_height as usize)
            .position(self.scroll_offset as usize);

        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼")),
            area,
            &mut self.scrollbar_state,
        );
    }

    fn render_paragraph(
        &mut self,
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        title: &str,
        lines: Vec<Line<'static>>,
        style: Style,
    ) {
        let block = Self::viewer_block(is_active, title);
        let inner = block.inner(area);

        // Update dimensions for scroll handling
        self.viewport_height = inner.height;
        let width = inner.width.max(1) as usize;

        if width > 0 {
            self.total_rows = lines
                .iter()
                .map(|line| {
                    let line_width = line.width();
                    if line_width == 0 {
                        1
                    } else {
                        line_width.div_ceil(width)
                    }
                })
                .sum();
        } else {
            self.total_rows = lines.len();
        }

        let widget = Paragraph::new(lines)
            .style(style)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));

        f.render_widget(widget, area);

        self.render_scrollbar_for_offset(f, area);
    }

    fn render_plain_value(
        &mut self,
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        title: &str,
        value: &Value,
        props: &ValueViewerProps<'_>,
    ) {
        let (text, paragraph_style) = ValueViewer::format_plain_text(value, props);
        self.render_text_value(
            f,
            area,
            is_active,
            title,
            value,
            TextContentKind::Plain,
            paragraph_style,
            move || ValueViewer::plain_text_to_lines(&text),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_text_value<F>(
        &mut self,
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        title: &str,
        value: &Value,
        kind: TextContentKind,
        paragraph_style: Style,
        lines_builder: F,
    ) where
        F: FnOnce() -> Vec<Line<'static>>,
    {
        let block = Self::viewer_block(is_active, title);
        let inner = block.inner(area);
        let content_width = inner.width.max(1);

        self.viewport_height = inner.height;
        self.ensure_text_cache(value, kind, content_width, paragraph_style, lines_builder);

        let Some(cache_mut) = self.text_cache.as_mut() else {
            return;
        };

        self.total_rows = cache_mut.total_wrapped_lines.max(1);

        // Only wrap visible lines + small buffer (lazy wrapping)
        // This avoids cloning/wrapping thousands of lines
        let viewport_height = self.viewport_height as usize;
        let scroll_offset = self.scroll_offset as usize;

        // Calculate range: visible + buffer above/below for smooth scrolling
        let buffer = 10; // Small buffer for smooth scrolling
        let start_wrapped_line = scroll_offset.saturating_sub(buffer);
        let end_wrapped_line = (scroll_offset + viewport_height + buffer)
            .min(cache_mut.total_wrapped_lines);

        // Get only the visible wrapped lines (lazy wrapping with caching)
        let visible_wrapped = cache_mut.get_visible_wrapped_lines(start_wrapped_line, end_wrapped_line);

        // Adjust scroll offset for the visible slice
        let adjusted_scroll = scroll_offset.saturating_sub(start_wrapped_line) as u16;

        // Get style after mutable borrow is done
        let paragraph_style = cache_mut.paragraph_style;

        let widget = Paragraph::new(visible_wrapped)
            .style(paragraph_style)
            .block(block)
            .scroll((adjusted_scroll, 0));

        f.render_widget(widget, area);
        self.render_scrollbar_for_offset(f, area);
    }

    fn ensure_text_cache<F>(
        &mut self,
        value: &Value,
        kind: TextContentKind,
        content_width: u16,
        paragraph_style: Style,
        lines_builder: F,
    ) where
        F: FnOnce() -> Vec<Line<'static>>,
    {
        let needs_rebuild = self
            .text_cache
            .as_ref()
            .map(|cache| !cache.matches(value, kind, content_width, paragraph_style))
            .unwrap_or(true);

        if needs_rebuild {
            let lines = lines_builder();
            self.text_cache = Some(TextCache::new(
                value,
                kind,
                paragraph_style,
                lines,
                content_width,
            ));
        }
    }

    fn try_render_json_value(
        &mut self,
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        title: &str,
        value: &Value,
        props: &ValueViewerProps<'_>,
    ) -> bool {
        if !props.json_formatter.can_format(value) {
            return false;
        }

        match props.json_formatter.format_to_lines(value) {
            Ok(lines) => {
                self.render_text_value(
                    f,
                    area,
                    is_active,
                    title,
                    value,
                    TextContentKind::Json,
                    Style::default(),
                    move || lines,
                );
                true
            }
            Err(_) => {
                self.clear_text_cache();
                false
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_table(
        &mut self,
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        base_title: &str,
        headers: Vec<&str>,
        rows_data: Vec<Vec<String>>,
        json_formatter: &JsonFormatter,
    ) {
        // Update total rows and viewport for scroll handling
        self.total_rows = rows_data.len();
        self.viewport_height = area.height.saturating_sub(2);

        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let entry_count = rows_data.len();
        let table_title = if entry_count == 1 {
            format!("{base_title} | 1 item")
        } else {
            format!("{base_title} | {} items", entry_count)
        };

        // Calculate widths
        let mut constraints = Vec::new();
        if headers.len() == 2 {
            // Key-Value style (Hash, ZSet)
            let key_width = rows_data
                .iter()
                .map(|row| row.first().map(|s| s.chars().count()).unwrap_or(0))
                .max()
                .unwrap_or(6)
                .clamp(8, 40) as u16;
            constraints.push(Constraint::Length(key_width));
            constraints.push(Constraint::Min(10));
        } else {
            // List/Set style (Single value)
            constraints.push(Constraint::Percentage(100));
        }

        let rows: Vec<Row> = rows_data
            .into_iter()
            .enumerate()
            .map(|(idx, cols)| {
                // Pre-calculate cell content to determine height correctly
                let mut formatted_cells = Vec::with_capacity(cols.len());
                for (i, c) in cols.iter().enumerate() {
                    let is_value_column = if headers.is_empty() {
                        true
                    } else {
                        i == headers.len() - 1
                    };

                    if is_value_column
                        && let Ok(json) = serde_json::from_str::<JsonValue>(c)
                        && let Ok(pretty) = serde_json::to_string_pretty(&json)
                    {
                        let colored_lines = json_formatter.colorize_json(&pretty);
                        formatted_cells.push(colored_lines);
                        continue;
                    }
                    formatted_cells.push(vec![Line::from(c.clone())]);
                }

                let height = formatted_cells
                    .iter()
                    .map(|lines| lines.len())
                    .max()
                    .unwrap_or(1) as u16;

                let cells: Vec<Cell> = formatted_cells
                    .into_iter()
                    .enumerate()
                    .map(|(i, lines)| {
                        let style = if i == 0 && headers.len() == 2 {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        };
                        Cell::from(lines).style(style)
                    })
                    .collect();

                // Alternate row colors, but don't highlight unless active
                let row_style = if idx % 2 == 0 {
                    Style::default()
                } else {
                    Style::default().bg(Color::Rgb(20, 20, 20))
                };

                Row::new(cells).style(row_style).height(height)
            })
            .collect();

        // Ensure we have a selection if we have data
        if !rows.is_empty() && self.table_state.selected().is_none() {
            self.table_state.select(Some(0));
        }
        // Ensure selection is valid
        if let Some(selected) = self.table_state.selected()
            && selected >= rows.len()
        {
            self.table_state.select(Some(rows.len().saturating_sub(1)));
        }

        let highlight_style = if is_active {
            Style::default()
                .bg(Color::Rgb(60, 60, 60))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let mut table = Table::new(rows, constraints)
            .column_spacing(2)
            .highlight_style(highlight_style)
            .block(Self::viewer_block(is_active, table_title));

        if !headers.is_empty() {
            let header_cells: Vec<Cell> = headers
                .iter()
                .map(|h| Cell::from(*h).style(header_style))
                .collect();
            table = table.header(Row::new(header_cells).bottom_margin(0));
        }

        f.render_stateful_widget(table, area, &mut self.table_state);

        if self.total_rows > self.viewport_height as usize {
            if let Some(selected) = self.table_state.selected() {
                self.scrollbar_state = self
                    .scrollbar_state
                    .content_length(self.total_rows)
                    .viewport_content_length(self.viewport_height as usize)
                    .position(selected);
            } else {
                self.scrollbar_state = self
                    .scrollbar_state
                    .content_length(self.total_rows)
                    .viewport_content_length(self.viewport_height as usize)
                    .position(0);
            }

            f.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼")),
                area,
                &mut self.scrollbar_state,
            );
        }
    }

    fn format_plain_text(value: &Value, props: &ValueViewerProps<'_>) -> (String, Style) {
        match props.text_formatter.format(value) {
            Ok(text) => (text, Style::default()),
            Err(_) => (
                "<formatting error>".to_string(),
                Style::default().fg(Color::Red),
            ),
        }
    }

    fn plain_text_to_lines(text: &str) -> Vec<Line<'static>> {
        if text.is_empty() {
            return vec![Line::from(String::new())];
        }

        text.split('\n')
            .map(|segment| Line::from(segment.to_string()))
            .collect()
    }

    fn wrap_single_line(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
        if width == 0 {
            return vec![Line::from(String::new())];
        }

        let mut result = Vec::new();
        let mut current_spans: Vec<Span<'static>> = Vec::new();
        let mut current_width = 0usize;
        let mut had_content = false;

        for span in &line.spans {
            if span.content.is_empty() {
                continue;
            }

            had_content = true;
            let content = span.content.clone().into_owned();
            let mut start = 0usize;

            while start < content.len() {
                if current_width >= width {
                    result.push(Self::line_from_spans(
                        std::mem::take(&mut current_spans),
                        line.alignment,
                    ));
                    current_width = 0;
                }

                let available = width.saturating_sub(current_width).max(1);
                let (advance, consumed_width) = Self::split_at_width(&content[start..], available);
                if advance == 0 {
                    break;
                }

                let end = start + advance;
                let slice = content[start..end].to_string();
                if !slice.is_empty() {
                    current_spans.push(Span::styled(slice, span.style));
                    current_width += consumed_width;
                }
                start = end;
            }
        }

        if !current_spans.is_empty() {
            result.push(Self::line_from_spans(
                std::mem::take(&mut current_spans),
                line.alignment,
            ));
        } else if !had_content {
            result.push(Self::line_from_spans(Vec::new(), line.alignment));
        }

        result
    }

    fn line_from_spans(spans: Vec<Span<'static>>, alignment: Option<Alignment>) -> Line<'static> {
        Line {
            spans,
            style: Style::default(),
            alignment,
        }
    }

    fn split_at_width(text: &str, max_width: usize) -> (usize, usize) {
        if text.is_empty() {
            return (0, 0);
        }

        let mut consumed_width = 0usize;
        let mut last_index = 0usize;

        for (idx, ch) in text.char_indices() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if consumed_width + ch_width > max_width && consumed_width != 0 {
                break;
            }

            consumed_width += ch_width;
            last_index = idx + ch.len_utf8();

            if consumed_width >= max_width {
                break;
            }
        }

        if last_index == 0 {
            if let Some(ch) = text.chars().next() {
                (ch.len_utf8(), UnicodeWidthChar::width(ch).unwrap_or(0))
            } else {
                (0, 0)
            }
        } else {
            (last_index, consumed_width)
        }
    }

    fn parse_json_data(value: &Value) -> Result<JsonValue, String> {
        let json_str = String::from_utf8(value.data.clone())
            .map_err(|_| "Invalid UTF-8 in value".to_string())?;
        serde_json::from_str(&json_str).map_err(|e| format!("Invalid payload: {e}"))
    }

    fn parse_hash_entries(value: &Value) -> Result<Vec<Vec<String>>, String> {
        let parsed = Self::parse_json_data(value)?;
        let obj = parsed
            .as_object()
            .ok_or_else(|| "Hash value is not an object".to_string())?;

        let mut entries: Vec<Vec<String>> = obj
            .iter()
            .map(|(field, value)| vec![field.clone(), Self::json_value_to_string(value)])
            .collect();

        entries.sort_by(|a, b| a[0].cmp(&b[0]));
        Ok(entries)
    }

    fn parse_list_entries(value: &Value) -> Result<Vec<Vec<String>>, String> {
        let parsed = Self::parse_json_data(value)?;
        let arr = parsed
            .as_array()
            .ok_or_else(|| "List value is not an array".to_string())?;

        let entries: Vec<Vec<String>> = arr
            .iter()
            .enumerate()
            .map(|(i, value)| vec![format!("{}", i + 1), Self::json_value_to_string(value)])
            .collect();
        Ok(entries)
    }

    fn parse_set_entries(value: &Value) -> Result<Vec<Vec<String>>, String> {
        let parsed = Self::parse_json_data(value)?;
        let arr = parsed
            .as_array()
            .ok_or_else(|| "Set value is not an array".to_string())?;

        let mut entries: Vec<Vec<String>> = arr
            .iter()
            .map(|value| vec![Self::json_value_to_string(value)])
            .collect();
        entries.sort_by(|a, b| a[0].cmp(&b[0]));
        Ok(entries)
    }

    fn parse_zset_entries(value: &Value) -> Result<Vec<Vec<String>>, String> {
        let parsed = Self::parse_json_data(value)?;
        let arr = parsed
            .as_array()
            .ok_or_else(|| "Sorted Set value is not an array".to_string())?;

        // ZSET is array of [member, score] tuples in JSON from backend
        let entries: Vec<Vec<String>> = arr
            .iter()
            .map(|item| {
                if let Some(tuple) = item.as_array()
                    && tuple.len() >= 2
                {
                    let member = Self::json_value_to_string(&tuple[0]);
                    let score = Self::json_value_to_string(&tuple[1]);
                    return vec![score, member]; // Score first for ZSet display usually? Or Member? Let's do Score | Member
                }
                vec!["?".to_string(), Self::json_value_to_string(item)]
            })
            .collect();

        Ok(entries)
    }

    fn json_value_to_string(value: &JsonValue) -> String {
        match value {
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "null".to_string(),
            JsonValue::Array(_) | JsonValue::Object(_) => {
                // Keep complex objects as JSON string so we can pretty print them in the cell
                value.to_string()
            }
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
            self.clear_text_cache();
            self.render_paragraph(
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
            match props.selected_key_type {
                Some(ValueType::Hash) => {
                    self.clear_text_cache();
                    match ValueViewer::parse_hash_entries(value) {
                        Ok(rows) => {
                            self.render_table(
                                f,
                                area,
                                props.is_active,
                                &base_title,
                                vec!["Field", "Value"],
                                rows,
                                props.json_formatter,
                            );
                            return;
                        }
                        Err(e) => {
                            self.render_paragraph(
                                f,
                                area,
                                props.is_active,
                                &base_title,
                                vec![Line::from(format!("Parse error: {}", e))],
                                Style::default().fg(Color::Red),
                            );
                            return;
                        }
                    }
                }
                Some(ValueType::List) => {
                    self.clear_text_cache();
                    match ValueViewer::parse_list_entries(value) {
                        Ok(rows) => {
                            self.render_table(
                                f,
                                area,
                                props.is_active,
                                &base_title,
                                vec!["Index", "Value"],
                                rows,
                                props.json_formatter,
                            );
                            return;
                        }
                        Err(e) => {
                            self.render_paragraph(
                                f,
                                area,
                                props.is_active,
                                &base_title,
                                vec![Line::from(format!("Parse error: {}", e))],
                                Style::default().fg(Color::Red),
                            );
                            return;
                        }
                    }
                }
                Some(ValueType::Set) => {
                    self.clear_text_cache();
                    match ValueViewer::parse_set_entries(value) {
                        Ok(rows) => {
                            self.render_table(
                                f,
                                area,
                                props.is_active,
                                &base_title,
                                vec![],
                                rows,
                                props.json_formatter,
                            );
                            return;
                        }
                        Err(e) => {
                            self.render_paragraph(
                                f,
                                area,
                                props.is_active,
                                &base_title,
                                vec![Line::from(format!("Parse error: {}", e))],
                                Style::default().fg(Color::Red),
                            );
                            return;
                        }
                    }
                }
                Some(ValueType::SortedSet) => {
                    self.clear_text_cache();
                    match ValueViewer::parse_zset_entries(value) {
                        Ok(rows) => {
                            self.render_table(
                                f,
                                area,
                                props.is_active,
                                &base_title,
                                vec!["Score", "Member"],
                                rows,
                                props.json_formatter,
                            );
                            return;
                        }
                        Err(e) => {
                            self.render_paragraph(
                                f,
                                area,
                                props.is_active,
                                &base_title,
                                vec![Line::from(format!("Parse error: {}", e))],
                                Style::default().fg(Color::Red),
                            );
                            return;
                        }
                    }
                }
                _ => {
                    // Reset table state when viewing non-table data
                    self.table_state.select(None);

                    if self.try_render_json_value(
                        f,
                        area,
                        props.is_active,
                        &base_title,
                        value,
                        &props,
                    ) {
                        return;
                    }

                    self.render_plain_value(f, area, props.is_active, &base_title, value, &props);
                    return;
                }
            }
        }

        self.clear_text_cache();
        self.render_paragraph(
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
