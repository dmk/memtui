use super::Component;
use crate::action::Action;
use crate::formatter::{Formatter, JsonFormatter, TextFormatter};
use crate::types::{Value, ValueType};
use crate::ui::theme::{self, AnimationState};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, ScrollbarState, Table, TableState, Wrap},
    Frame,
};

/// Get current dim factor as a cache key component
fn dim_cache_key() -> u8 {
    // Convert dim to a u8 to use as part of cache key
    (theme::get_dim() * 255.0) as u8
}

use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::trace;
use unicode_width::UnicodeWidthChar;

pub struct ValueViewerProps<'a> {
    pub selected_value: Option<&'a Value>,
    pub selected_key_type: Option<ValueType>,
    pub selected_key_name: Option<&'a str>,
    pub error_message: Option<&'a String>,
    pub json_formatter: &'a JsonFormatter,
    pub text_formatter: &'a TextFormatter,
    pub is_active: bool,
    pub backend_type: Option<crate::types::BackendType>,
    pub animation: &'a AnimationState,
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
    // Dim factor when cache was created (invalidate if changed)
    dim_key: u8,
    // Store UNWRAPPED lines - wrap on-demand only for visible portion
    unwrapped_lines: Vec<Line<'static>>,
    paragraph_style: Style,
    // Cached total wrapped line count (calculated once)
    total_wrapped_lines: usize,
    // Cache wrapped line count per unwrapped line for fast lookups
    wrapped_counts: Vec<usize>,
    // Cache start index of wrapped lines for each unwrapped line (prefix sums)
    line_offsets: Vec<usize>,
    // Cache wrapped lines per unwrapped line to avoid re-wrapping on every render
    // Use Arc to avoid expensive cloning on every frame
    wrapped_cache: Vec<Option<Arc<Vec<Line<'static>>>>>,
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

        // Pre-calculate wrapped line count per unwrapped line using EXACT wrapping logic
        // This ensures scrollbar and render loop are perfectly synced
        let wrapped_counts: Vec<usize> = unwrapped_lines
            .iter()
            .map(|line| ValueViewer::count_wrapped_lines(line, width))
            .collect();

        let total_wrapped_lines: usize = wrapped_counts.iter().sum();

        let mut line_offsets = Vec::with_capacity(wrapped_counts.len());
        let mut current = 0;
        for &count in &wrapped_counts {
            line_offsets.push(current);
            current += count;
        }

        // Pre-allocate cache but don't wrap yet (lazy)
        let wrapped_cache = vec![None; unwrapped_lines.len()];

        Self {
            source_ptr: value.data.as_ptr() as usize,
            source_len: value.data.len(),
            wrap_width: wrap_width.max(1),
            kind,
            dim_key: dim_cache_key(),
            unwrapped_lines,
            paragraph_style,
            total_wrapped_lines,
            wrapped_counts,
            line_offsets,
            wrapped_cache,
        }
    }

    /// Render lines directly from cache without intermediate Vec (zero-copy!)
    fn render_visible_lines(
        &mut self,
        buf: &mut ratatui::buffer::Buffer,
        start_wrapped_line: usize,
        end_wrapped_line: usize,
        area: Rect,
        paragraph_style: Style,
    ) {
        // Bounds check
        if start_wrapped_line >= end_wrapped_line || start_wrapped_line >= self.total_wrapped_lines
        {
            return;
        }

        let end_wrapped_line = end_wrapped_line.min(self.total_wrapped_lines);
        let mut screen_y = 0u16;

        // Find start index using binary search to avoid iterating all lines
        let start_idx = match self.line_offsets.binary_search(&start_wrapped_line) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };

        let mut current_wrapped_index = self.line_offsets.get(start_idx).copied().unwrap_or(0);

        // Find and render visible lines directly from cache
        for (idx, unwrapped) in self.unwrapped_lines.iter().enumerate().skip(start_idx) {
            let wrapped_count = self.wrapped_counts.get(idx).copied().unwrap_or(1);

            // Skip lines before visible range (only happens if binary search landed us early)
            if current_wrapped_index + wrapped_count <= start_wrapped_line {
                current_wrapped_index += wrapped_count;
                continue;
            }

            // Get wrapped lines from cache (or wrap and cache)
            let wrapped = if let Some(cached) = self.wrapped_cache.get(idx).and_then(|c| c.as_ref())
            {
                Arc::clone(cached)
            } else {
                let wrapped = Arc::new(ValueViewer::wrap_single_line(
                    unwrapped,
                    self.wrap_width as usize,
                ));
                if let Some(cache_slot) = self.wrapped_cache.get_mut(idx) {
                    *cache_slot = Some(Arc::clone(&wrapped));
                }
                wrapped
            };

            // Render each wrapped line
            for line in wrapped.iter() {
                if current_wrapped_index >= start_wrapped_line
                    && current_wrapped_index < end_wrapped_line
                {
                    let y = area.y.saturating_add(screen_y);

                    // Bounds check for buffer access
                    if y >= area.y + area.height || y >= buf.area().height {
                        return;
                    }

                    let mut x = area.x;
                    for span in &line.spans {
                        for ch in span.content.chars() {
                            // Bounds check before buffer access
                            if x >= area.x + area.width || x >= buf.area().width {
                                break;
                            }

                            // Safe buffer access with bounds check
                            if let Some(cell) = buf.cell_mut((x, y)) {
                                cell.set_char(ch);
                                cell.set_style(paragraph_style.patch(span.style));
                            }
                            x = x.saturating_add(1);
                        }
                    }

                    screen_y = screen_y.saturating_add(1);
                    if screen_y >= area.height {
                        return;
                    }
                }

                current_wrapped_index += 1;
                if current_wrapped_index >= end_wrapped_line {
                    return;
                }
            }
        }
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
            && self.dim_key == dim_cache_key()
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
            // Safe calculation with proper bounds checking to prevent overflow
            let max_scroll = self
                .total_rows
                .saturating_sub(self.viewport_height as usize)
                .min(u16::MAX as usize) as u16;

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
    /// Optimized for fast scrolling - minimal arithmetic
    #[tracing::instrument(skip(self), level = "trace")]
    pub fn scroll_by(&mut self, delta: isize) {
        if delta == 0 {
            trace!("No-op scroll request");
            return;
        }

        // Early return if there's nothing to scroll
        if self.total_rows == 0 || self.viewport_height == 0 {
            trace!(
                total_rows = self.total_rows,
                viewport_height = self.viewport_height,
                "No rows/viewport height available; ignoring scroll"
            );
            return;
        }

        if self.table_state.selected().is_some() {
            let i = self.table_state.selected().unwrap_or(0);
            let max_index = self.total_rows.saturating_sub(1);

            // Early return if already at boundary and trying to scroll further
            if delta > 0 && i >= max_index {
                trace!(
                    index = i,
                    max_index,
                    "Already at bottom; ignoring scroll down"
                );
                return;
            }
            if delta < 0 && i == 0 {
                trace!(index = i, "Already at top; ignoring scroll up");
                return;
            }

            if delta > 0 {
                let new_i = i.saturating_add(delta as usize).min(max_index);
                trace!(old = i, new = new_i, "Scrolling table selection down");
                self.table_state.select(Some(new_i));
            } else {
                let new_i = i.saturating_sub((-delta) as usize);
                trace!(old = i, new = new_i, "Scrolling table selection up");
                self.table_state.select(Some(new_i));
            }
        } else {
            // Fast path: simple arithmetic, no clamping overhead
            let max_scroll = self
                .total_rows
                .saturating_sub(self.viewport_height as usize);

            // Early return if already at boundary and trying to scroll further
            if delta > 0 && self.scroll_offset >= max_scroll as u16 {
                trace!(
                    scroll_offset = self.scroll_offset,
                    max_scroll,
                    "Already at bottom; ignoring scroll down"
                );
                return;
            }
            if delta < 0 && self.scroll_offset == 0 {
                trace!(
                    scroll_offset = self.scroll_offset,
                    "Already at top; ignoring scroll up"
                );
                return;
            }

            // Fast path for typical scroll amounts (avoid 64-bit conversions)
            let new_offset = if delta > 0 {
                if delta < i16::MAX as isize {
                    // Fast: common case, small scroll
                    self.scroll_offset.saturating_add(delta as u16)
                } else {
                    // Rare: huge scroll, saturate to max
                    max_scroll as u16
                }
            } else if delta > i16::MIN as isize {
                // Fast: common case, small scroll
                self.scroll_offset.saturating_sub((-delta) as u16)
            } else {
                // Rare: huge scroll, clamp to 0
                0
            };

            // Clamp to valid range
            self.scroll_offset = new_offset.min(max_scroll as u16);
            trace!(
                scroll_offset = self.scroll_offset,
                max_scroll,
                delta,
                "Updated value viewer scroll offset"
            );
        }
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
        self.table_state.select(None);
        self.scrollbar_state = ScrollbarState::default();
    }

    /// Check if we're at a scroll boundary (top or bottom)
    /// Returns Some(true) if at bottom, Some(false) if at top, None if not at boundary
    pub fn is_at_boundary(&self, direction: isize) -> Option<bool> {
        if self.total_rows == 0 || self.viewport_height == 0 {
            return None;
        }

        if self.table_state.selected().is_some() {
            let i = self.table_state.selected().unwrap_or(0);
            let max_index = self.total_rows.saturating_sub(1);
            if direction > 0 && i >= max_index {
                return Some(true); // At bottom
            }
            if direction < 0 && i == 0 {
                return Some(false); // At top
            }
        } else {
            let max_scroll = self
                .total_rows
                .saturating_sub(self.viewport_height as usize);
            if direction > 0 && self.scroll_offset >= max_scroll as u16 {
                return Some(true); // At bottom
            }
            if direction < 0 && self.scroll_offset == 0 {
                return Some(false); // At top
            }
        }
        None // Not at boundary
    }

    fn clear_text_cache(&mut self) {
        self.text_cache = None;
    }

    fn render_header(f: &mut Frame, area: Rect, title: &str, is_active: bool) -> Rect {
        theme::render_panel_header(f, area, title, is_active)
    }

    fn render_header_split(
        f: &mut Frame,
        area: Rect,
        title_left: &str,
        title_right: &str,
        is_active: bool,
    ) -> Rect {
        theme::render_panel_header_split(f, area, title_left, title_right, is_active)
    }

    fn render_scrollbar_for_offset(&mut self, f: &mut Frame, area: Rect, is_active: bool) {
        if self.total_rows <= self.viewport_height as usize || area.width < 2 {
            return;
        }

        let max_scroll = self
            .total_rows
            .saturating_sub(self.viewport_height as usize);
        let clamped_offset = self.scroll_offset.min(max_scroll as u16) as usize;

        let scrollbar_position = if max_scroll > 0 {
            let progress = clamped_offset as f64 / max_scroll as f64;
            (progress * (self.total_rows - 1) as f64) as usize
        } else {
            0
        };

        trace!(
            scroll_offset = self.scroll_offset,
            total_rows = self.total_rows,
            viewport_height = self.viewport_height,
            max_scroll,
            clamped_offset,
            scrollbar_position,
            "Rendering scrollbar for offset (wrapped lines)"
        );

        self.scrollbar_state = self
            .scrollbar_state
            .content_length(self.total_rows)
            .viewport_content_length(self.viewport_height as usize)
            .position(scrollbar_position);

        f.render_stateful_widget(
            theme::scrollbar(is_active),
            theme::scrollbar_area(area),
            &mut self.scrollbar_state,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_paragraph(
        &mut self,
        f: &mut Frame,
        area: Rect,
        is_active: bool,
        title: &str,
        lines: Vec<Line<'static>>,
        style: Style,
        _animation: &AnimationState,
    ) {
        let inner = Self::render_header(f, area, title, is_active);

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
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));

        f.render_widget(widget, inner);

        self.render_scrollbar_for_offset(f, inner, is_active);
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
            props.animation,
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
        _animation: &AnimationState,
        lines_builder: F,
    ) where
        F: FnOnce() -> Vec<Line<'static>>,
    {
        let inner = Self::render_header(f, area, title, is_active);
        let content_width = inner.width.max(1);

        self.viewport_height = inner.height;
        self.ensure_text_cache(value, kind, content_width, paragraph_style, lines_builder);

        let Some(cache_mut) = self.text_cache.as_mut() else {
            return;
        };

        self.total_rows = cache_mut.total_wrapped_lines.max(1);

        // Only wrap visible lines (lazy wrapping)
        // This avoids cloning/wrapping thousands of lines
        let viewport_height = self.viewport_height as usize;
        let scroll_offset =
            self.scroll_offset
                .min(cache_mut.total_wrapped_lines.saturating_sub(1) as u16) as usize;

        // Calculate exact visible range - no buffer needed since we're not using .scroll()
        let start_wrapped_line = scroll_offset;
        let end_wrapped_line = (scroll_offset + viewport_height).min(cache_mut.total_wrapped_lines);

        // Get style before borrow
        let paragraph_style = cache_mut.paragraph_style;

        // CRITICAL: Render directly from cache without intermediate Vec
        // This avoids cloning lines on every frame
        let buf = f.buffer_mut();
        cache_mut.render_visible_lines(
            buf,
            start_wrapped_line,
            end_wrapped_line,
            inner,
            paragraph_style,
        );

        self.render_scrollbar_for_offset(f, inner, is_active);
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
                    props.animation,
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
        _animation: &AnimationState,
    ) {
        let entry_count = rows_data.len();
        let entry_info = if entry_count == 1 {
            "1 entry".to_string()
        } else {
            format!("{} entries", Self::format_number(entry_count))
        };

        // Render header and get content area
        let content_area = Self::render_header_split(f, area, base_title, &entry_info, is_active);

        // Update total rows and viewport for scroll handling
        self.total_rows = rows_data.len();
        self.viewport_height = content_area.height.saturating_sub(1);

        let header_style = Style::default()
            .fg(theme::NEON_CYAN())
            .add_modifier(Modifier::BOLD);

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

                    if is_value_column {
                        if let Ok(json) = serde_json::from_str::<JsonValue>(c) {
                            if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                                let colored_lines = json_formatter.colorize_json(&pretty);
                                formatted_cells.push(colored_lines);
                                continue;
                            }
                        }
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
                            Style::default().fg(theme::NEON_AMBER())
                        } else {
                            Style::default().fg(theme::TEXT_PRIMARY())
                        };
                        Cell::from(lines).style(style)
                    })
                    .collect();

                // Alternate row colors with themed styling
                let row_style = if idx % 2 == 0 {
                    Style::default().bg(theme::BG_PANEL())
                } else {
                    Style::default().bg(theme::BG_SURFACE())
                };

                Row::new(cells).style(row_style).height(height)
            })
            .collect();

        // Ensure we have a selection if we have data
        if !rows.is_empty() && self.table_state.selected().is_none() {
            self.table_state.select(Some(0));
        }
        // Ensure selection is valid
        if let Some(selected) = self.table_state.selected() {
            if selected >= rows.len() {
                self.table_state.select(Some(rows.len().saturating_sub(1)));
            }
        }

        let highlight_style = if is_active {
            theme::list_selected().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let mut table = Table::new(rows, constraints)
            .column_spacing(2)
            .highlight_style(highlight_style);

        if !headers.is_empty() {
            let header_cells: Vec<Cell> = headers
                .iter()
                .map(|h| Cell::from(*h).style(header_style))
                .collect();
            table = table.header(Row::new(header_cells).bottom_margin(0));
        }

        f.render_stateful_widget(table, content_area, &mut self.table_state);

        if self.total_rows > self.viewport_height as usize {
            let position = self.table_state.selected().unwrap_or(0);
            self.scrollbar_state = self
                .scrollbar_state
                .content_length(self.total_rows)
                .viewport_content_length(self.viewport_height as usize)
                .position(position);

            f.render_stateful_widget(
                theme::scrollbar(is_active),
                theme::scrollbar_area(content_area),
                &mut self.scrollbar_state,
            );
        }
    }

    fn format_plain_text(value: &Value, props: &ValueViewerProps<'_>) -> (String, Style) {
        match props.text_formatter.format(value) {
            // Use theme color so it participates in dimming
            Ok(text) => (text, Style::default().fg(theme::TEXT_PRIMARY())),
            Err(_) => (
                "<formatting error>".to_string(),
                Style::default().fg(theme::NEON_RED()),
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

                // Check for overflow: if char is wider than available and we have content, wrap first!
                if consumed_width > available && current_width > 0 {
                    result.push(Self::line_from_spans(
                        std::mem::take(&mut current_spans),
                        line.alignment,
                    ));
                    current_width = 0;
                    // Don't advance start, retry loop iteration on new line
                    continue;
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

    fn count_wrapped_lines(line: &Line<'static>, width: usize) -> usize {
        if width == 0 {
            return 1;
        }

        let mut lines = 0;
        let mut current_width = 0usize;
        let mut had_content = false;
        let mut has_pending_spans = false;

        for span in &line.spans {
            if span.content.is_empty() {
                continue;
            }

            had_content = true;
            let content = &span.content;
            let mut start = 0usize;

            while start < content.len() {
                if current_width >= width {
                    lines += 1;
                    current_width = 0;
                    has_pending_spans = false;
                }

                let available = width.saturating_sub(current_width).max(1);
                let (advance, consumed_width) = Self::split_at_width(&content[start..], available);
                if advance == 0 {
                    break;
                }

                // Check for overflow matching wrap_single_line
                if consumed_width > available && current_width > 0 {
                    lines += 1;
                    current_width = 0;
                    has_pending_spans = false;
                    continue;
                }

                current_width += consumed_width;
                has_pending_spans = true;
                start += advance;
            }
        }

        if has_pending_spans || !had_content {
            lines += 1;
        }

        lines
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
                if let Some(tuple) = item.as_array() {
                    if tuple.len() >= 2 {
                        let member = Self::json_value_to_string(&tuple[0]);
                        let score = Self::json_value_to_string(&tuple[1]);
                        return vec![score, member]; // Score first for ZSet display usually? Or Member? Let's do Score | Member
                    }
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

    fn title_for(
        key_name: Option<&str>,
        _value_type: Option<ValueType>,
        _backend_type: Option<crate::types::BackendType>,
    ) -> String {
        // Show the key name as the title - much more useful!
        key_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Value".to_string())
    }

    /// Format a number with thousand separators (e.g., 1,414)
    fn format_number(n: usize) -> String {
        let s = n.to_string();
        let mut result = String::with_capacity(s.len() + s.len() / 3);
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }
        result.chars().rev().collect()
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
        let base_title = ValueViewer::title_for(
            props.selected_key_name,
            props.selected_key_type,
            props.backend_type,
        );
        let animation = props.animation;

        if let Some(err) = props.error_message {
            self.clear_text_cache();
            self.render_paragraph(
                f,
                area,
                props.is_active,
                &base_title,
                vec![Line::from(Span::styled(
                    format!("✕ Error: {}", err),
                    Style::default().fg(theme::NEON_RED()),
                ))],
                Style::default(),
                animation,
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
                                animation,
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
                                Style::default().fg(theme::NEON_RED()),
                                animation,
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
                                animation,
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
                                Style::default().fg(theme::NEON_RED()),
                                animation,
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
                                animation,
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
                                Style::default().fg(theme::NEON_RED()),
                                animation,
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
                                animation,
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
                                Style::default().fg(theme::NEON_RED()),
                                animation,
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
            vec![Line::from(Span::styled(
                "Select a key to view its value",
                Style::default().fg(theme::TEXT_DIM()),
            ))],
            Style::default(),
            animation,
        );
    }

    fn handle_input(&mut self, _key: KeyEvent, _props: Self::Props<'_>) -> Option<Self::Msg> {
        None
    }
}
