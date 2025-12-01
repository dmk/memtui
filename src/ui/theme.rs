//! 2026 Modern Theme Module
//! Clean, consistent styling with subtle animations
//!
//! Theme colors can be customized via `~/.config/memtui/theme.json`

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════════════════════════════════════════
// THEME CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Global theme instance, loaded once at startup
static THEME: OnceLock<ThemeConfig> = OnceLock::new();

/// Get the current theme (loads default if not initialized)
pub fn theme() -> &'static ThemeConfig {
    THEME.get_or_init(ThemeConfig::default)
}

/// Initialize the global theme with a custom config
/// Should be called once at startup before any rendering
pub fn init_theme(config: ThemeConfig) {
    let _ = THEME.set(config);
}

/// RGB color representation for JSON serialization
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }
}

impl From<RgbColor> for Color {
    fn from(rgb: RgbColor) -> Self {
        rgb.to_color()
    }
}

/// Complete theme configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Accent colors
    #[serde(default)]
    pub accent: AccentColors,

    /// Neon/semantic colors
    #[serde(default)]
    pub neon: NeonColors,

    /// Background colors
    #[serde(default)]
    pub background: BackgroundColors,

    /// Text colors
    #[serde(default)]
    pub text: TextColors,

    /// Border colors
    #[serde(default)]
    pub border: BorderColors,
}

/// Accent colors configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccentColors {
    /// Primary accent - cyan
    #[serde(default = "default_accent")]
    pub primary: RgbColor,
    /// Secondary accent - lighter cyan
    #[serde(default = "default_accent_dim")]
    pub dim: RgbColor,
    /// Highlight accent
    #[serde(default = "default_accent_bright")]
    pub bright: RgbColor,
}

impl Default for AccentColors {
    fn default() -> Self {
        Self {
            primary: default_accent(),
            dim: default_accent_dim(),
            bright: default_accent_bright(),
        }
    }
}

/// Neon/semantic colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeonColors {
    /// Green for success/connected
    #[serde(default = "default_neon_green")]
    pub green: RgbColor,
    /// Amber for warnings
    #[serde(default = "default_neon_amber")]
    pub amber: RgbColor,
    /// Red for errors
    #[serde(default = "default_neon_red")]
    pub red: RgbColor,
    /// Purple for special elements
    #[serde(default = "default_neon_purple")]
    pub purple: RgbColor,
    /// Cyan for accents
    #[serde(default = "default_neon_cyan")]
    pub cyan: RgbColor,
    /// Pink for logo gradient
    #[serde(default = "default_neon_pink")]
    pub pink: RgbColor,
    /// Electric blue
    #[serde(default = "default_electric_blue")]
    pub electric_blue: RgbColor,
}

impl Default for NeonColors {
    fn default() -> Self {
        Self {
            green: default_neon_green(),
            amber: default_neon_amber(),
            red: default_neon_red(),
            purple: default_neon_purple(),
            cyan: default_neon_cyan(),
            pink: default_neon_pink(),
            electric_blue: default_electric_blue(),
        }
    }
}

/// Background colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundColors {
    /// Deep background
    #[serde(default = "default_bg_deep")]
    pub deep: RgbColor,
    /// Panel background
    #[serde(default = "default_bg_panel")]
    pub panel: RgbColor,
    /// Surface background - for cards/elevated areas
    #[serde(default = "default_bg_surface")]
    pub surface: RgbColor,
    /// Selected item background
    #[serde(default = "default_bg_selected")]
    pub selected: RgbColor,
    /// Elevated state
    #[serde(default = "default_bg_elevated")]
    pub elevated: RgbColor,
    /// Hover state
    #[serde(default = "default_bg_hover")]
    pub hover: RgbColor,
}

impl Default for BackgroundColors {
    fn default() -> Self {
        Self {
            deep: default_bg_deep(),
            panel: default_bg_panel(),
            surface: default_bg_surface(),
            selected: default_bg_selected(),
            elevated: default_bg_elevated(),
            hover: default_bg_hover(),
        }
    }
}

/// Text colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextColors {
    /// Dim text
    #[serde(default = "default_text_dim")]
    pub dim: RgbColor,
    /// Secondary text
    #[serde(default = "default_text_secondary")]
    pub secondary: RgbColor,
    /// Primary text
    #[serde(default = "default_text_primary")]
    pub primary: RgbColor,
    /// Bright/white text
    #[serde(default = "default_text_bright")]
    pub bright: RgbColor,
}

impl Default for TextColors {
    fn default() -> Self {
        Self {
            dim: default_text_dim(),
            secondary: default_text_secondary(),
            primary: default_text_primary(),
            bright: default_text_bright(),
        }
    }
}

/// Border colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderColors {
    /// Dim border
    #[serde(default = "default_border_dim")]
    pub dim: RgbColor,
    /// Active border
    #[serde(default = "default_border_active")]
    pub active: RgbColor,
}

impl Default for BorderColors {
    fn default() -> Self {
        Self {
            dim: default_border_dim(),
            active: default_border_active(),
        }
    }
}

// Default value functions
fn default_accent() -> RgbColor {
    RgbColor::new(80, 200, 220)
}
fn default_accent_dim() -> RgbColor {
    RgbColor::new(60, 150, 170)
}
fn default_accent_bright() -> RgbColor {
    RgbColor::new(100, 220, 240)
}

fn default_neon_green() -> RgbColor {
    RgbColor::new(57, 255, 20)
}
fn default_neon_amber() -> RgbColor {
    RgbColor::new(255, 191, 0)
}
fn default_neon_red() -> RgbColor {
    RgbColor::new(255, 80, 80)
}
fn default_neon_purple() -> RgbColor {
    RgbColor::new(160, 100, 220)
}
fn default_neon_cyan() -> RgbColor {
    RgbColor::new(0, 255, 255)
}
fn default_neon_pink() -> RgbColor {
    RgbColor::new(255, 100, 150)
}
fn default_electric_blue() -> RgbColor {
    RgbColor::new(80, 180, 255)
}

fn default_bg_deep() -> RgbColor {
    RgbColor::new(12, 14, 22)
}
fn default_bg_panel() -> RgbColor {
    RgbColor::new(18, 21, 32)
}
fn default_bg_surface() -> RgbColor {
    RgbColor::new(26, 30, 44)
}
fn default_bg_selected() -> RgbColor {
    RgbColor::new(35, 45, 65)
}
fn default_bg_elevated() -> RgbColor {
    RgbColor::new(40, 50, 70)
}
fn default_bg_hover() -> RgbColor {
    RgbColor::new(45, 55, 75)
}

fn default_text_dim() -> RgbColor {
    RgbColor::new(90, 100, 120)
}
fn default_text_secondary() -> RgbColor {
    RgbColor::new(140, 150, 170)
}
fn default_text_primary() -> RgbColor {
    RgbColor::new(210, 215, 230)
}
fn default_text_bright() -> RgbColor {
    RgbColor::new(245, 248, 255)
}

fn default_border_dim() -> RgbColor {
    RgbColor::new(50, 58, 78)
}
fn default_border_active() -> RgbColor {
    RgbColor::new(80, 200, 220)
}

// ═══════════════════════════════════════════════════════════════════════════════
// COLOR PALETTE (backward compatible functions that read from theme)
// These use SCREAMING_CASE to match the original const-based API
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(non_snake_case)]
/// Primary accent - cyan
pub fn ACCENT() -> Color {
    theme().accent.primary.to_color()
}
#[allow(non_snake_case)]
/// Secondary accent - lighter cyan
pub fn ACCENT_DIM() -> Color {
    theme().accent.dim.to_color()
}
#[allow(non_snake_case)]
/// Highlight accent
pub fn ACCENT_BRIGHT() -> Color {
    theme().accent.bright.to_color()
}

#[allow(non_snake_case)]
/// Neon green for success/connected
pub fn NEON_GREEN() -> Color {
    theme().neon.green.to_color()
}
#[allow(non_snake_case)]
/// Amber for warnings
pub fn NEON_AMBER() -> Color {
    theme().neon.amber.to_color()
}
#[allow(non_snake_case)]
/// Red for errors
pub fn NEON_RED() -> Color {
    theme().neon.red.to_color()
}
#[allow(non_snake_case)]
/// Purple for special elements
pub fn NEON_PURPLE() -> Color {
    theme().neon.purple.to_color()
}
#[allow(non_snake_case)]
/// Cyan for accents
pub fn NEON_CYAN() -> Color {
    theme().neon.cyan.to_color()
}
#[allow(non_snake_case)]
/// Pink for logo gradient
pub fn NEON_PINK() -> Color {
    theme().neon.pink.to_color()
}
#[allow(non_snake_case)]
/// Electric blue
pub fn ELECTRIC_BLUE() -> Color {
    theme().neon.electric_blue.to_color()
}

#[allow(non_snake_case)]
/// Deep background
pub fn BG_DEEP() -> Color {
    theme().background.deep.to_color()
}
#[allow(non_snake_case)]
/// Panel background
pub fn BG_PANEL() -> Color {
    theme().background.panel.to_color()
}
#[allow(non_snake_case)]
/// Surface background - for cards/elevated areas
pub fn BG_SURFACE() -> Color {
    theme().background.surface.to_color()
}
#[allow(non_snake_case)]
/// Selected item background
pub fn BG_SELECTED() -> Color {
    theme().background.selected.to_color()
}
#[allow(non_snake_case)]
/// Hover/elevated state
pub fn BG_ELEVATED() -> Color {
    theme().background.elevated.to_color()
}
#[allow(non_snake_case)]
pub fn BG_HOVER() -> Color {
    theme().background.hover.to_color()
}

#[allow(non_snake_case)]
/// Dim text
pub fn TEXT_DIM() -> Color {
    theme().text.dim.to_color()
}
#[allow(non_snake_case)]
/// Secondary text
pub fn TEXT_SECONDARY() -> Color {
    theme().text.secondary.to_color()
}
#[allow(non_snake_case)]
/// Primary text
pub fn TEXT_PRIMARY() -> Color {
    theme().text.primary.to_color()
}
#[allow(non_snake_case)]
/// Bright/white text
pub fn TEXT_BRIGHT() -> Color {
    theme().text.bright.to_color()
}

#[allow(non_snake_case)]
/// Border colors
pub fn BORDER_DIM() -> Color {
    theme().border.dim.to_color()
}
#[allow(non_snake_case)]
pub fn BORDER_ACTIVE() -> Color {
    theme().border.active.to_color()
}

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
    GradientColors {
        start: (0, 230, 255),
        end: (60, 160, 255),
    },
    // Cyan to Teal
    GradientColors {
        start: (0, 230, 240),
        end: (0, 180, 200),
    },
    // Blue to Purple
    GradientColors {
        start: (100, 180, 255),
        end: (180, 130, 255),
    },
    // Teal to Cyan
    GradientColors {
        start: (0, 200, 200),
        end: (80, 220, 255),
    },
    // Purple to Pink
    GradientColors {
        start: (160, 140, 255),
        end: (220, 130, 200),
    },
    // Green to Cyan
    GradientColors {
        start: (80, 220, 180),
        end: (0, 200, 230),
    },
    // Blue to Cyan
    GradientColors {
        start: (80, 160, 255),
        end: (0, 220, 240),
    },
    // Pink to Purple
    GradientColors {
        start: (230, 140, 200),
        end: (160, 120, 230),
    },
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
    Style::default().fg(BORDER_DIM())
}

pub fn border_focused() -> Style {
    Style::default().fg(BORDER_ACTIVE())
}

pub fn border_active(_animation: &AnimationState) -> Style {
    border_focused()
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATUS STYLES
// ═══════════════════════════════════════════════════════════════════════════════

pub fn status_connected() -> Style {
    Style::default().fg(NEON_GREEN())
}

pub fn status_connecting(animation: &AnimationState) -> Style {
    let pulse = animation.pulse(800);
    let intensity = (180.0 + 75.0 * pulse) as u8;
    Style::default().fg(Color::Rgb(255, intensity, 0))
}

pub fn status_error() -> Style {
    Style::default().fg(NEON_RED())
}

pub fn status_disconnected() -> Style {
    Style::default().fg(TEXT_DIM())
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
                Span::styled("│", Style::default().fg(BORDER_ACTIVE())),
                Span::styled(
                    format!(" {} ", title_str),
                    Style::default()
                        .fg(ACCENT_BRIGHT())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("│", Style::default().fg(BORDER_ACTIVE())),
            ]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_focused())
            .style(Style::default().bg(BG_PANEL()))
    } else {
        Block::default()
            .title(Line::from(vec![Span::styled(
                format!(" {} ", title_str),
                Style::default().fg(TEXT_SECONDARY()),
            )]))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_inactive())
            .style(Style::default().bg(BG_PANEL()))
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
        .title(Line::from(vec![Span::styled(
            format!(" {} ", title.into()),
            Style::default()
                .fg(TEXT_BRIGHT())
                .add_modifier(Modifier::BOLD),
        )]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT()))
        .style(Style::default().bg(BG_SURFACE()))
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
    Style::default().bg(BG_SELECTED())
}

/// Highlighted item (stronger selection indicator)
pub fn list_highlight() -> Style {
    Style::default()
        .bg(BG_ELEVATED())
        .add_modifier(Modifier::BOLD)
}

pub fn list_hover() -> Style {
    Style::default().bg(BG_HOVER())
}

pub fn text_highlight() -> Style {
    Style::default()
        .fg(ACCENT_BRIGHT())
        .add_modifier(Modifier::BOLD)
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
            Style::default()
                .fg(BG_DEEP())
                .bg(ACCENT())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", description),
            Style::default().fg(TEXT_SECONDARY()),
        ),
    ]
}

pub fn keybind_subtle(key: &str, description: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(key.to_string(), Style::default().fg(NEON_AMBER())),
        Span::styled(
            format!(" {} ", description),
            Style::default().fg(TEXT_DIM()),
        ),
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

pub const LOGO_COMPACT: &[&str] = &["┏┳┓┏━╸┏┳┓╺┳╸╻ ╻╻", "┃┃┃┣╸ ┃┃┃ ┃ ┃ ┃┃", "╹ ╹┗━╸╹ ╹ ╹ ┗━┛╹"];

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
                    let wave = ((pos * std::f32::consts::PI * 2.0)
                        - (cycle * std::f32::consts::PI * 2.0))
                        .sin();
                    // Normalize from -1..1 to 0..1
                    let t = (wave + 1.0) / 2.0;

                    // Interpolate between gradient start and end colors
                    let r = (gradient.start.0 as f32
                        + (gradient.end.0 as f32 - gradient.start.0 as f32) * t)
                        as u8;
                    let g = (gradient.start.1 as f32
                        + (gradient.end.1 as f32 - gradient.start.1 as f32) * t)
                        as u8;
                    let b = (gradient.start.2 as f32
                        + (gradient.end.2 as f32 - gradient.start.2 as f32) * t)
                        as u8;

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
    Line::from(Span::styled(bar, Style::default().fg(BORDER_DIM())))
}
