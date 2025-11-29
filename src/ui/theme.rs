//! 2026 Modern Theme Module
//! Clean, consistent styling with subtle animations

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════════════════════════════════════════
// COLOR PALETTE
// ═══════════════════════════════════════════════════════════════════════════════

/// Primary accent - cyan
pub const ACCENT: Color = Color::Rgb(80, 200, 220);
/// Secondary accent - lighter cyan
pub const ACCENT_DIM: Color = Color::Rgb(60, 150, 170);
/// Highlight accent
pub const ACCENT_BRIGHT: Color = Color::Rgb(100, 220, 240);

/// Neon green for success/connected
pub const NEON_GREEN: Color = Color::Rgb(57, 255, 20);
/// Amber for warnings
pub const NEON_AMBER: Color = Color::Rgb(255, 191, 0);
/// Red for errors
pub const NEON_RED: Color = Color::Rgb(255, 80, 80);
/// Purple for special elements
pub const NEON_PURPLE: Color = Color::Rgb(160, 100, 220);
/// Cyan for accents
pub const NEON_CYAN: Color = Color::Rgb(0, 255, 255);
/// Pink for logo gradient
pub const NEON_PINK: Color = Color::Rgb(255, 100, 150);
/// Electric blue
pub const ELECTRIC_BLUE: Color = Color::Rgb(80, 180, 255);

/// Deep background
pub const BG_DEEP: Color = Color::Rgb(12, 14, 22);
/// Panel background
pub const BG_PANEL: Color = Color::Rgb(18, 21, 32);
/// Surface background - for cards/elevated areas
pub const BG_SURFACE: Color = Color::Rgb(26, 30, 44);
/// Selected item background
pub const BG_SELECTED: Color = Color::Rgb(35, 45, 65);
/// Hover/elevated state
pub const BG_ELEVATED: Color = Color::Rgb(40, 50, 70);
pub const BG_HOVER: Color = Color::Rgb(45, 55, 75);

/// Dim text
pub const TEXT_DIM: Color = Color::Rgb(90, 100, 120);
/// Secondary text
pub const TEXT_SECONDARY: Color = Color::Rgb(140, 150, 170);
/// Primary text
pub const TEXT_PRIMARY: Color = Color::Rgb(210, 215, 230);
/// Bright/white text
pub const TEXT_BRIGHT: Color = Color::Rgb(245, 248, 255);

/// Border colors
pub const BORDER_DIM: Color = Color::Rgb(50, 58, 78);
pub const BORDER_ACTIVE: Color = Color::Rgb(80, 200, 220);

// ═══════════════════════════════════════════════════════════════════════════════
// ANIMATION STATE
// ═══════════════════════════════════════════════════════════════════════════════

/// Gradient color pair for logo
#[derive(Clone, Copy)]
pub struct GradientColors {
    pub start: (u8, u8, u8),
    pub end: (u8, u8, u8),
}

/// Subtle gradient presets - all pleasant color combinations
const GRADIENT_PRESETS: &[GradientColors] = &[
    // Cyan to Blue
    GradientColors { start: (0, 230, 255), end: (60, 160, 255) },
    // Cyan to Teal
    GradientColors { start: (0, 230, 240), end: (0, 180, 200) },
    // Blue to Purple
    GradientColors { start: (100, 180, 255), end: (180, 130, 255) },
    // Teal to Cyan
    GradientColors { start: (0, 200, 200), end: (80, 220, 255) },
    // Purple to Pink
    GradientColors { start: (160, 140, 255), end: (220, 130, 200) },
    // Green to Cyan
    GradientColors { start: (80, 220, 180), end: (0, 200, 230) },
    // Blue to Cyan
    GradientColors { start: (80, 160, 255), end: (0, 220, 240) },
    // Pink to Purple
    GradientColors { start: (230, 140, 200), end: (160, 120, 230) },
];

#[derive(Clone)]
pub struct AnimationState {
    pub start_time: Instant,
    pub gradient: GradientColors,
}

impl Default for AnimationState {
    fn default() -> Self {
        // Pick a random gradient based on current time
        let seed = Instant::now().elapsed().as_nanos() as usize;
        let gradient = GRADIENT_PRESETS[seed % GRADIENT_PRESETS.len()];

        Self {
            start_time: Instant::now(),
            gradient,
        }
    }
}

impl AnimationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Oscillates between 0.0 and 1.0
    pub fn pulse(&self, period_ms: u64) -> f32 {
        let elapsed_ms = self.elapsed().as_millis() as f32;
        let period = period_ms as f32;
        ((elapsed_ms / period * std::f32::consts::PI * 2.0).sin() + 1.0) / 2.0
    }

    /// Cycles through 0.0 to 1.0
    pub fn cycle(&self, period_ms: u64) -> f32 {
        let elapsed_ms = self.elapsed().as_millis() as u64;
        (elapsed_ms % period_ms) as f32 / period_ms as f32
    }

    /// Returns animation frame index
    pub fn frame(&self, frame_count: usize, period_ms: u64) -> usize {
        let elapsed_ms = self.elapsed().as_millis() as u64;
        ((elapsed_ms / period_ms) as usize) % frame_count
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SPINNER/LOADING INDICATORS
// ═══════════════════════════════════════════════════════════════════════════════

pub const SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
pub const SPINNER_DOTS: [&str; 4] = ["⣾", "⣽", "⣻", "⢿"];
pub const SPINNER_PULSE: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub fn spinner(animation: &AnimationState) -> &'static str {
    let frame = animation.frame(SPINNER_FRAMES.len(), 100);
    SPINNER_FRAMES[frame]
}

pub fn spinner_dots(animation: &AnimationState) -> &'static str {
    let frame = animation.frame(SPINNER_DOTS.len(), 150);
    SPINNER_DOTS[frame]
}

pub fn spinner_pulse(animation: &AnimationState) -> &'static str {
    let frame = animation.frame(SPINNER_PULSE.len(), 250);
    SPINNER_PULSE[frame]
}

/// Status indicators
pub const INDICATOR_CONNECTED: &str = "●";
pub const INDICATOR_CONNECTING: &str = "◐";
pub const INDICATOR_DISCONNECTED: &str = "○";
pub const INDICATOR_ERROR: &str = "✕";

// ═══════════════════════════════════════════════════════════════════════════════
// BORDER STYLES
// ═══════════════════════════════════════════════════════════════════════════════

pub fn border_inactive() -> Style {
    Style::default().fg(BORDER_DIM)
}

pub fn border_focused() -> Style {
    Style::default().fg(BORDER_ACTIVE)
}

pub fn border_active(_animation: &AnimationState) -> Style {
    border_focused()
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATUS STYLES
// ═══════════════════════════════════════════════════════════════════════════════

pub fn status_connected() -> Style {
    Style::default().fg(NEON_GREEN)
}

pub fn status_connecting(animation: &AnimationState) -> Style {
    let pulse = animation.pulse(800);
    let intensity = (180.0 + 75.0 * pulse) as u8;
    Style::default().fg(Color::Rgb(255, intensity, 0))
}

pub fn status_error() -> Style {
    Style::default().fg(NEON_RED)
}

pub fn status_disconnected() -> Style {
    Style::default().fg(TEXT_DIM)
}

// ═══════════════════════════════════════════════════════════════════════════════
// BLOCK BUILDERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a panel block with clear title styling
pub fn panel_block(title: impl Into<String>, is_active: bool) -> Block<'static> {
    let title_str = title.into();

    if is_active {
        Block::default()
            .title(Line::from(vec![
                Span::styled("│", Style::default().fg(BORDER_ACTIVE)),
                Span::styled(
                    format!(" {} ", title_str),
                    Style::default().fg(ACCENT_BRIGHT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("│", Style::default().fg(BORDER_ACTIVE)),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_focused())
            .style(Style::default().bg(BG_PANEL))
    } else {
        Block::default()
            .title(Line::from(vec![
                Span::styled(
                    format!(" {} ", title_str),
                    Style::default().fg(TEXT_SECONDARY),
                ),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_inactive())
            .style(Style::default().bg(BG_PANEL))
    }
}

/// Alias for backwards compatibility
pub fn themed_block(title: impl Into<String>, is_active: bool) -> Block<'static> {
    panel_block(title, is_active)
}

pub fn animated_block(
    title: impl Into<String>,
    is_active: bool,
    _animation: &AnimationState,
) -> Block<'static> {
    panel_block(title, is_active)
}

/// Modal/dialog block
pub fn modal_block(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .title(Line::from(vec![
            Span::styled(
                format!(" {} ", title.into()),
                Style::default().fg(TEXT_BRIGHT).add_modifier(Modifier::BOLD),
            ),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG_SURFACE))
}

/// Glass-style block for dialogs (kept for compatibility)
pub fn glass_block(title: impl Into<String>) -> Block<'static> {
    modal_block(title)
}

// ═══════════════════════════════════════════════════════════════════════════════
// LIST/SELECTION STYLES
// ═══════════════════════════════════════════════════════════════════════════════

/// Style for selected items - only sets background, preserves text colors
pub fn list_selected() -> Style {
    Style::default().bg(BG_SELECTED)
}

/// Highlighted item (stronger selection indicator)
pub fn list_highlight() -> Style {
    Style::default().bg(BG_ELEVATED).add_modifier(Modifier::BOLD)
}

pub fn list_hover() -> Style {
    Style::default().bg(BG_HOVER)
}

pub fn text_highlight() -> Style {
    Style::default().fg(ACCENT_BRIGHT).add_modifier(Modifier::BOLD)
}

// ═══════════════════════════════════════════════════════════════════════════════
// LAYOUT HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy)]
pub struct PaneSplit {
    pub ratio: f32,
    pub min: f32,
    pub max: f32,
    pub resizing: bool,
}

impl Default for PaneSplit {
    fn default() -> Self {
        Self {
            ratio: 0.4,
            min: 0.2,
            max: 0.8,
            resizing: false,
        }
    }
}

impl PaneSplit {
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.clamp(0.2, 0.8),
            ..Default::default()
        }
    }

    pub fn adjust(&mut self, delta: f32) {
        self.ratio = (self.ratio + delta).clamp(self.min, self.max);
    }

    pub fn left_percent(&self) -> u16 {
        (self.ratio * 100.0) as u16
    }

    pub fn right_percent(&self) -> u16 {
        100 - self.left_percent()
    }

    pub fn is_on_handle(&self, area: Rect, x: u16) -> bool {
        let split_x = area.x + (area.width as f32 * self.ratio) as u16;
        x >= split_x.saturating_sub(1) && x <= split_x.saturating_add(1)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// KEYBINDING DISPLAY
// ═══════════════════════════════════════════════════════════════════════════════

pub fn keybind(key: &str, description: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!(" {} ", key),
            Style::default().fg(BG_DEEP).bg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {} ", description), Style::default().fg(TEXT_SECONDARY)),
    ]
}

pub fn keybind_subtle(key: &str, description: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(key.to_string(), Style::default().fg(NEON_AMBER)),
        Span::styled(format!(" {} ", description), Style::default().fg(TEXT_DIM)),
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// LOGO
// ═══════════════════════════════════════════════════════════════════════════════

pub const LOGO: &[&str] = &[
    "███╗   ███╗███████╗███╗   ███╗████████╗██╗   ██╗██╗",
    "████╗ ████║██╔════╝████╗ ████║╚══██╔══╝██║   ██║██║",
    "██╔████╔██║█████╗  ██╔████╔██║   ██║   ██║   ██║██║",
    "██║╚██╔╝██║██╔══╝  ██║╚██╔╝██║   ██║   ██║   ██║██║",
    "██║ ╚═╝ ██║███████╗██║ ╚═╝ ██║   ██║   ╚██████╔╝██║",
    "╚═╝     ╚═╝╚══════╝╚═╝     ╚═╝   ╚═╝    ╚═════╝ ╚═╝",
];

pub const LOGO_COMPACT: &[&str] = &[
    "┏┳┓┏━╸┏┳┓╺┳╸╻ ╻╻",
    "┃┃┃┣╸ ┃┃┃ ┃ ┃ ┃┃",
    "╹ ╹┗━╸╹ ╹ ╹ ┗━┛╹",
];

/// Create styled logo with smooth animated gradient (random colors each launch)
pub fn logo_lines(animation: &AnimationState) -> Vec<Line<'static>> {
    // 5 second cycle
    let cycle = animation.cycle(5000);
    let gradient = &animation.gradient;

    LOGO.iter()
        .map(|line| {
            let char_count = line.chars().count() as f32;

            let chars: Vec<Span<'static>> = line
                .chars()
                .enumerate()
                .map(|(char_idx, ch)| {
                    if ch == ' ' {
                        return Span::styled(ch.to_string(), Style::default());
                    }

                    // Position along the line (0.0 to 1.0)
                    let pos = char_idx as f32 / char_count;

                    // Smooth sine wave that travels across - no harsh edges
                    let wave = ((pos * std::f32::consts::PI * 2.0) - (cycle * std::f32::consts::PI * 2.0)).sin();
                    // Normalize from -1..1 to 0..1
                    let t = (wave + 1.0) / 2.0;

                    // Interpolate between gradient start and end colors
                    let r = (gradient.start.0 as f32 + (gradient.end.0 as f32 - gradient.start.0 as f32) * t) as u8;
                    let g = (gradient.start.1 as f32 + (gradient.end.1 as f32 - gradient.start.1 as f32) * t) as u8;
                    let b = (gradient.start.2 as f32 + (gradient.end.2 as f32 - gradient.start.2 as f32) * t) as u8;

                    Span::styled(
                        ch.to_string(),
                        Style::default()
                            .fg(Color::Rgb(r, g, b))
                            .add_modifier(Modifier::BOLD),
                    )
                })
                .collect();

            Line::from(chars)
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// UTILITY
// ═══════════════════════════════════════════════════════════════════════════════

pub fn separator_line(width: u16) -> Line<'static> {
    let bar = "─".repeat(width as usize);
    Line::from(Span::styled(bar, Style::default().fg(BORDER_DIM)))
}
