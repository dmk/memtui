//! Modal component helpers
//!
//! Modals use raw (undimmed) theme colors while the background is dimmed.
//! The dimming is handled by the main render() function which calls
//! theme::dim_theme() when a modal is active.

use ratatui::{
    layout::Rect,
    widgets::{Block, Clear},
    Frame,
};

use crate::ui::theme::raw;

/// Render a modal with proper styling.
///
/// This clears the modal area and renders a block with raw (undimmed) colors.
/// The background dimming should already be active from the main render().
///
/// Returns the inner area (inside the block borders) for content rendering.
pub fn render_modal(f: &mut Frame, modal_area: Rect, title: &str) -> Rect {
    // Clear the modal area
    f.render_widget(Clear, modal_area);

    // Render the modal block with raw (undimmed) colors
    let block = raw::modal_block(title);
    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    inner
}

/// Confirmation dialog block using raw colors
pub fn confirm_block_raw(title: impl Into<String>) -> Block<'static> {
    raw::modal_block(title)
}
