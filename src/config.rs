use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Application configuration loaded from config file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// UI and rendering settings
    #[serde(default)]
    pub ui: UiConfig,

    /// Performance and timing settings
    #[serde(default)]
    pub performance: PerformanceConfig,

    /// Connection settings
    #[serde(default)]
    pub connection: ConnectionConfig,

    /// Data loading settings
    #[serde(default)]
    pub data: DataConfig,

    /// JSON formatting settings
    #[serde(default)]
    pub json: JsonConfig,
}

/// UI and rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Initial viewport height (will be updated based on terminal size)
    #[serde(default = "default_viewport_height")]
    pub viewport_height: usize,

    /// Maximum number of recent connections to remember
    #[serde(default = "default_max_recent_connections")]
    pub max_recent_connections: usize,
}

/// Performance and timing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Tick interval in milliseconds (how often the UI updates)
    #[serde(
        default = "default_tick_interval_ms",
        deserialize_with = "deserialize_ms_to_duration"
    )]
    #[serde(serialize_with = "serialize_duration_to_ms")]
    pub tick_interval: Duration,

    /// Event polling timeout in milliseconds
    #[serde(
        default = "default_event_poll_ms",
        deserialize_with = "deserialize_ms_to_duration"
    )]
    #[serde(serialize_with = "serialize_duration_to_ms")]
    pub event_poll_timeout: Duration,

    /// Event loop sleep duration in milliseconds (yield time)
    #[serde(
        default = "default_event_loop_sleep_ms",
        deserialize_with = "deserialize_ms_to_duration"
    )]
    #[serde(serialize_with = "serialize_duration_to_ms")]
    pub event_loop_sleep: Duration,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Default connection timeout in seconds
    #[serde(
        default = "default_connection_timeout_secs",
        deserialize_with = "deserialize_secs_to_duration"
    )]
    #[serde(serialize_with = "serialize_duration_to_secs")]
    pub default_timeout: Duration,
}

/// Data loading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    /// Number of keys to load per chunk
    #[serde(default = "default_keys_per_chunk")]
    pub keys_per_chunk: usize,

    /// Value load debounce time in milliseconds
    #[serde(
        default = "default_value_load_debounce_ms",
        deserialize_with = "deserialize_ms_to_duration"
    )]
    #[serde(serialize_with = "serialize_duration_to_ms")]
    pub value_load_debounce: Duration,
}

/// JSON formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonConfig {
    /// JSON indentation size
    #[serde(default = "default_json_indent")]
    pub indent: usize,

    /// JSON key color (as string, will be converted to Color)
    #[serde(default = "default_json_key_color")]
    pub key_color: String,

    /// JSON string value color
    #[serde(default = "default_json_string_color")]
    pub string_color: String,

    /// JSON number color
    #[serde(default = "default_json_number_color")]
    pub number_color: String,

    /// JSON boolean color
    #[serde(default = "default_json_boolean_color")]
    pub boolean_color: String,

    /// JSON null color
    #[serde(default = "default_json_null_color")]
    pub null_color: String,

    /// JSON brace color
    #[serde(default = "default_json_brace_color")]
    pub brace_color: String,

    /// JSON bracket color
    #[serde(default = "default_json_bracket_color")]
    pub bracket_color: String,

    /// JSON comma color
    #[serde(default = "default_json_comma_color")]
    pub comma_color: String,

    /// JSON colon color
    #[serde(default = "default_json_colon_color")]
    pub colon_color: String,
}

// Default value functions
fn default_viewport_height() -> usize {
    20
}

fn default_max_recent_connections() -> usize {
    8
}

fn default_tick_interval_ms() -> Duration {
    Duration::from_millis(250)
}

fn default_event_poll_ms() -> Duration {
    Duration::from_millis(100)
}

fn default_event_loop_sleep_ms() -> Duration {
    Duration::from_millis(10)
}

fn default_connection_timeout_secs() -> Duration {
    Duration::from_secs(5)
}

fn default_keys_per_chunk() -> usize {
    200
}

fn default_value_load_debounce_ms() -> Duration {
    Duration::from_millis(120)
}

fn default_json_indent() -> usize {
    2
}

fn default_json_key_color() -> String {
    "cyan".to_string()
}

fn default_json_string_color() -> String {
    "green".to_string()
}

fn default_json_number_color() -> String {
    "magenta".to_string()
}

fn default_json_boolean_color() -> String {
    "yellow".to_string()
}

fn default_json_null_color() -> String {
    "dark_gray".to_string()
}

fn default_json_brace_color() -> String {
    "white".to_string()
}

fn default_json_bracket_color() -> String {
    "white".to_string()
}

fn default_json_comma_color() -> String {
    "white".to_string()
}

fn default_json_colon_color() -> String {
    "white".to_string()
}

// Serde helpers for Duration serialization
fn deserialize_ms_to_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let ms = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(ms))
}

fn serialize_duration_to_ms<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_millis() as u64)
}

fn deserialize_secs_to_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(secs))
}

fn serialize_duration_to_secs<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_secs())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
            performance: PerformanceConfig::default(),
            connection: ConnectionConfig::default(),
            data: DataConfig::default(),
            json: JsonConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            viewport_height: default_viewport_height(),
            max_recent_connections: default_max_recent_connections(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            tick_interval: default_tick_interval_ms(),
            event_poll_timeout: default_event_poll_ms(),
            event_loop_sleep: default_event_loop_sleep_ms(),
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            default_timeout: default_connection_timeout_secs(),
        }
    }
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            keys_per_chunk: default_keys_per_chunk(),
            value_load_debounce: default_value_load_debounce_ms(),
        }
    }
}

impl Default for JsonConfig {
    fn default() -> Self {
        Self {
            indent: default_json_indent(),
            key_color: default_json_key_color(),
            string_color: default_json_string_color(),
            number_color: default_json_number_color(),
            boolean_color: default_json_boolean_color(),
            null_color: default_json_null_color(),
            brace_color: default_json_brace_color(),
            bracket_color: default_json_bracket_color(),
            comma_color: default_json_comma_color(),
            colon_color: default_json_colon_color(),
        }
    }
}

impl Config {
    /// Convert JSON color string to ratatui Color
    pub fn json_key_color(&self) -> Color {
        color_from_string(&self.json.key_color)
    }

    pub fn json_string_color(&self) -> Color {
        color_from_string(&self.json.string_color)
    }

    pub fn json_number_color(&self) -> Color {
        color_from_string(&self.json.number_color)
    }

    pub fn json_boolean_color(&self) -> Color {
        color_from_string(&self.json.boolean_color)
    }

    pub fn json_null_color(&self) -> Color {
        color_from_string(&self.json.null_color)
    }

    pub fn json_brace_color(&self) -> Color {
        color_from_string(&self.json.brace_color)
    }

    pub fn json_bracket_color(&self) -> Color {
        color_from_string(&self.json.bracket_color)
    }

    pub fn json_comma_color(&self) -> Color {
        color_from_string(&self.json.comma_color)
    }

    pub fn json_colon_color(&self) -> Color {
        color_from_string(&self.json.colon_color)
    }
}

/// Convert color string to ratatui Color
fn color_from_string(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" => Color::DarkGray,
        "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_yellow" => Color::LightYellow,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        _ => Color::White, // Default fallback
    }
}
