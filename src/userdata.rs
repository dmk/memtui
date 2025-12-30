use crate::config::Config;
use crate::keybindings::{default_keybindings, KeybindingsConfig};
use crate::types::ConnectionConfig;
use crate::ui::theme::ThemeConfig;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Get the path to the userdata directory (for data files like connections)
pub fn get_userdata_dir() -> Result<PathBuf, io::Error> {
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;

    let userdata_dir = PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("memtui");

    // Create directory if it doesn't exist
    if !userdata_dir.exists() {
        fs::create_dir_all(&userdata_dir)?;
    }

    Ok(userdata_dir)
}

/// Get the path to the config directory (for configuration files)
pub fn get_config_dir() -> Result<PathBuf, io::Error> {
    let home = std::env::var("HOME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;

    let config_dir = PathBuf::from(home).join(".config").join("memtui");

    // Create directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    Ok(config_dir)
}

/// Get the path to the connections file
pub fn get_connections_file() -> Result<PathBuf, io::Error> {
    Ok(get_userdata_dir()?.join("connections.json"))
}

/// Load connections from userdata
pub fn load_connections() -> Result<Vec<ConnectionConfig>, Box<dyn std::error::Error>> {
    let file_path = get_connections_file()?;

    if !file_path.exists() {
        // No file exists yet, return empty list
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(file_path)?;
    let connections: Vec<ConnectionConfig> = serde_json::from_str(&contents)?;

    Ok(connections)
}

/// Save connections to userdata
pub fn save_connections(
    connections: &[ConnectionConfig],
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_connections_file()?;
    let json = serde_json::to_string_pretty(connections)?;
    fs::write(file_path, json)?;

    Ok(())
}

fn get_recent_connections_file() -> Result<PathBuf, io::Error> {
    Ok(get_userdata_dir()?.join("recent_connections.json"))
}

/// Load the ordered list of recently used connection IDs
pub fn load_recent_connection_ids() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file_path = get_recent_connections_file()?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(file_path)?;
    let ids: Vec<String> = serde_json::from_str(&contents)?;
    Ok(ids)
}

pub fn save_recent_connection_ids(ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_recent_connections_file()?;
    let json = serde_json::to_string_pretty(ids)?;
    fs::write(file_path, json)?;
    Ok(())
}

/// Record a connection ID as the most recently used and return the updated list
/// Uses default max_recent_connections (8) for backward compatibility
pub fn record_recent_connection_id(id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    record_recent_connection_id_with_config(id, &Config::default())
}

/// Remove a connection ID from recents (used when deleting configs)
pub fn remove_recent_connection_id(id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut ids = load_recent_connection_ids().unwrap_or_default();
    let len_before = ids.len();
    ids.retain(|existing| existing != id);
    if ids.len() != len_before {
        save_recent_connection_ids(&ids)?;
    }
    Ok(ids)
}

/// Get the path to the config file
pub fn get_config_file() -> Result<PathBuf, io::Error> {
    Ok(get_config_dir()?.join("config.toml"))
}

/// Load configuration from userdata directory
/// Returns default config if file doesn't exist or is invalid
pub fn load_config() -> Config {
    let file_path = match get_config_file() {
        Ok(path) => path,
        Err(_) => return Config::default(),
    };

    load_config_from_path(&file_path)
}

/// Load configuration from a specific path
/// Returns default config if file doesn't exist or is invalid
pub fn load_config_from_path(file_path: &PathBuf) -> Config {
    if !file_path.exists() {
        // Only create default if using standard path
        if file_path == &get_config_file().unwrap_or_default() {
            if let Err(e) = save_config(&Config::default()) {
                eprintln!("Warning: Could not create default config file: {}", e);
            }
        } else {
            eprintln!("Warning: Config file not found: {}", file_path.display());
        }
        return Config::default();
    }

    match fs::read_to_string(file_path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "Warning: Invalid config file ({}), using defaults: {}",
                    file_path.display(),
                    e
                );
                Config::default()
            }
        },
        Err(e) => {
            eprintln!(
                "Warning: Could not read config file ({}), using defaults: {}",
                file_path.display(),
                e
            );
            Config::default()
        }
    }
}

/// Generate default config file content with comments
fn generate_default_config_with_comments() -> String {
    r#"# memtui Configuration File
# This file contains all configurable settings for memtui.
# Edit this file to customize the application behavior.
# The file will be automatically created on first run.

# UI and rendering settings
[ui]
# Initial viewport height (will be updated based on terminal size)
viewport_height = 20

# Maximum number of recent connections to remember
max_recent_connections = 8

# Split ratio between key list and value viewer panes
# 0.2 = narrow key list, 0.8 = wide key list
pane_ratio = 0.4

# Performance and timing settings
[performance]
# Tick interval in milliseconds (debounce interval for scroll events)
# 16ms = ~60 FPS smooth scrolling, 33ms = ~30 FPS, 50ms = ~20 FPS
tick_interval = 16

# Event polling timeout in milliseconds
# How long to wait for input events before checking again
event_poll_timeout = 100

# Event loop sleep duration in milliseconds
# Time to yield to other tasks between event checks
event_loop_sleep = 10

# Connection settings
[connection]
# Default connection timeout in seconds
# Used when creating new connections
default_timeout = 5

# Data loading settings
[data]
# Number of keys to load per chunk when scanning
# Higher values = faster loading but more memory usage
keys_per_chunk = 200

# Value load debounce time in milliseconds
# Delay before loading a value when navigating keys
# Prevents loading values too frequently when scrolling quickly
value_load_debounce = 120

# JSON formatting settings
[json]
# JSON indentation size (number of spaces per indent level)
indent = 2

# JSON syntax highlighting colors
# Available colors: black, red, green, yellow, blue, magenta, cyan, white,
# gray, dark_gray, light_red, light_green, light_yellow, light_blue,
# light_magenta, light_cyan
key_color = "cyan"
string_color = "green"
number_color = "magenta"
boolean_color = "yellow"
null_color = "dark_gray"
brace_color = "white"
bracket_color = "white"
comma_color = "white"
colon_color = "white"
"#
    .to_string()
}

/// Save configuration to userdata directory
pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_config_file()?;

    // If file doesn't exist, create it with comments
    if !file_path.exists() {
        let commented_config = generate_default_config_with_comments();
        fs::write(&file_path, commented_config)?;
        return Ok(());
    }

    // Otherwise, save the current config (without comments to preserve user edits)
    let toml = toml::to_string_pretty(config)?;
    fs::write(file_path, toml)?;
    Ok(())
}

/// Record a connection ID as the most recently used and return the updated list
/// Uses max_recent_connections from config
pub fn record_recent_connection_id_with_config(
    id: &str,
    config: &Config,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut ids = load_recent_connection_ids().unwrap_or_default();
    ids.retain(|existing| existing != id);
    ids.insert(0, id.to_string());
    let max = config.ui.max_recent_connections;
    if ids.len() > max {
        ids.truncate(max);
    }
    save_recent_connection_ids(&ids)?;
    Ok(ids)
}

// ═══════════════════════════════════════════════════════════════════════════════
// JSON WITH COMMENTS SUPPORT
// ═══════════════════════════════════════════════════════════════════════════════

/// Strip comment lines (starting with //) from JSON content
/// This allows users to add comments to JSON config files
fn strip_json_comments(content: &str) -> String {
    content
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ═══════════════════════════════════════════════════════════════════════════════
// THEME CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Get the path to the theme file
pub fn get_theme_file() -> Result<PathBuf, io::Error> {
    Ok(get_config_dir()?.join("theme.json"))
}

/// Load theme configuration from userdata directory
/// Returns default theme if file doesn't exist or is invalid
pub fn load_theme() -> ThemeConfig {
    let file_path = match get_theme_file() {
        Ok(path) => path,
        Err(_) => return ThemeConfig::default(),
    };

    if !file_path.exists() {
        // Create default theme file for user reference
        if let Err(e) = save_default_theme() {
            eprintln!("Warning: Could not create default theme file: {}", e);
        }
        return ThemeConfig::default();
    }

    match fs::read_to_string(&file_path) {
        Ok(contents) => {
            let stripped = strip_json_comments(&contents);
            match serde_json::from_str(&stripped) {
                Ok(theme) => theme,
                Err(e) => {
                    eprintln!(
                        "Warning: Invalid theme file ({}), using defaults: {}",
                        file_path.display(),
                        e
                    );
                    ThemeConfig::default()
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Could not read theme file ({}), using defaults: {}",
                file_path.display(),
                e
            );
            ThemeConfig::default()
        }
    }
}

/// Generate default theme file content with comments
fn generate_default_theme_json() -> String {
    r#"// Theme Configuration for memtui
// Customize the colors by editing this file.
// Colors are specified as RGB objects: { "r": 0-255, "g": 0-255, "b": 255 }
// Lines starting with // are comments and will be ignored.
{
  "accent": {
    // Main accent color (used for active borders, highlights)
    "primary": { "r": 80, "g": 200, "b": 220 },
    // Dimmed accent for secondary elements
    "dim": { "r": 60, "g": 150, "b": 170 },
    // Bright accent for emphasis
    "bright": { "r": 100, "g": 220, "b": 240 }
  },

  "neon": {
    // Success, connected status
    "green": { "r": 57, "g": 255, "b": 20 },
    // Warnings, connecting status
    "amber": { "r": 255, "g": 191, "b": 0 },
    // Errors
    "red": { "r": 255, "g": 80, "b": 80 },
    // Special elements, palette borders
    "purple": { "r": 160, "g": 100, "b": 220 },
    // Key bindings, accents
    "cyan": { "r": 0, "g": 255, "b": 255 },
    // Logo gradient, special highlights
    "pink": { "r": 255, "g": 100, "b": 150 },
    // JSON values
    "electric_blue": { "r": 80, "g": 180, "b": 255 }
  },

  "background": {
    // Main background color
    "deep": { "r": 12, "g": 14, "b": 22 },
    // Panel backgrounds
    "panel": { "r": 18, "g": 21, "b": 32 },
    // Elevated surfaces, cards
    "surface": { "r": 26, "g": 30, "b": 44 },
    // Selected item background
    "selected": { "r": 35, "g": 45, "b": 65 },
    // Elevated/highlighted state
    "elevated": { "r": 40, "g": 50, "b": 70 },
    // Hover state
    "hover": { "r": 45, "g": 55, "b": 75 }
  },

  "text": {
    // Least important text
    "dim": { "r": 90, "g": 100, "b": 120 },
    // Secondary text
    "secondary": { "r": 140, "g": 150, "b": 170 },
    // Main text color
    "primary": { "r": 210, "g": 215, "b": 230 },
    // Most important text
    "bright": { "r": 245, "g": 248, "b": 255 }
  },

  "border": {
    // Inactive borders
    "dim": { "r": 50, "g": 58, "b": 78 },
    // Active/focused borders
    "active": { "r": 80, "g": 200, "b": 220 }
  }
}
"#
    .to_string()
}

/// Save default theme configuration to userdata directory
pub fn save_default_theme() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_theme_file()?;

    // Only create if it doesn't exist
    if !file_path.exists() {
        let theme_json = generate_default_theme_json();
        fs::write(&file_path, theme_json)?;
    }

    Ok(())
}

/// Save theme configuration to userdata directory
pub fn save_theme(theme: &ThemeConfig) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_theme_file()?;
    let json = serde_json::to_string_pretty(theme)?;
    fs::write(file_path, json)?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// KEYBINDINGS CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Get the path to the keybindings file
pub fn get_keybindings_file() -> Result<PathBuf, io::Error> {
    Ok(get_config_dir()?.join("keybindings.json"))
}

/// Load keybindings configuration from userdata directory
/// Returns merged keybindings (defaults + user overrides) if file doesn't exist or is invalid
pub fn load_keybindings() -> KeybindingsConfig {
    let defaults = default_keybindings();
    let file_path = match get_keybindings_file() {
        Ok(path) => path,
        Err(_) => return defaults,
    };

    if !file_path.exists() {
        // Create default keybindings file for user reference
        if let Err(e) = save_default_keybindings() {
            eprintln!("Warning: Could not create default keybindings file: {}", e);
        }
        return defaults;
    }

    match fs::read_to_string(&file_path) {
        Ok(contents) => {
            let stripped = strip_json_comments(&contents);
            match serde_json::from_str::<KeybindingsConfig>(&stripped) {
                Ok(user_config) => KeybindingsConfig::merge(defaults, user_config),
                Err(e) => {
                    eprintln!(
                        "Warning: Invalid keybindings file ({}), using defaults: {}",
                        file_path.display(),
                        e
                    );
                    defaults
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Warning: Could not read keybindings file ({}), using defaults: {}",
                file_path.display(),
                e
            );
            defaults
        }
    }
}

/// Generate default keybindings file content with comments
fn generate_default_keybindings_json() -> String {
    r#"// Keybindings Configuration for memtui
// Customize keyboard shortcuts by editing this file.
// Commands use dot-separated names (e.g., 'quit.show', 'cmdline.search').
// Keys can be simple ('q', 'esc') or with modifiers ('ctrl+p', 'shift+tab').
// Only specify bindings you want to override; others will use defaults.
// Lines starting with // are comments and will be ignored.
{
  // Global keybindings - available everywhere as fallback
  "global": {
    "quit.show": ["q", "esc"],
    "help.toggle": ["?"],
    "connection.palette.toggle": ["ctrl+p"],
    "connection.form.open": ["ctrl+n"],
    "debug.toggle": ["f12", "ctrl+d"]
  },

  // Default context - active when connected and browsing
  "default": {
    "cmdline.search": ["/"],
    "cmdline.command": [":"],
    "cmdline.goto": ["g"],
    "navigation.next_panel": ["tab"],
    "navigation.prev_panel": ["shift+tab"],
    "navigation.next_item": ["down", "j"],
    "navigation.prev_item": ["up", "k"],
    "navigation.enter": ["enter"],
    "connection.tab.next": ["ctrl+right"],
    "connection.tab.prev": ["ctrl+left"],
    "pane.resize.left": ["ctrl+h"],
    "pane.resize.right": ["ctrl+l"]
  },

  // CmdLine mode - active when the command line is focused
  "cmdline": {
    "cmdline.clear": ["esc"],
    "cmdline.confirm": ["enter"],
    "cmdline.next_result": ["down", "ctrl+n"],
    "cmdline.prev_result": ["up", "ctrl+p"]
  },

  // Connection form - active when adding/editing a connection
  "connection_form": {
    "connection.form.submit": ["enter"],
    "connection.form.close": ["esc"],
    "connection.form.next_field": ["tab", "down"],
    "connection.form.prev_field": ["shift+tab", "up"],
    "connection.form.backend_type.next": ["right"],
    "connection.form.backend_type.prev": ["left"]
  },

  // Connection palette - active when the connection palette is open
  "connection_palette": {
    "connection.palette.close": ["esc"],
    "connection.palette.select": ["enter"],
    "connection.palette.next": ["down", "j"],
    "connection.palette.prev": ["up", "k"],
    "connection.palette.delete": ["d"]
  },

  // Quit confirmation dialog
  "quit_confirmation": {
    "quit.confirm": ["y", "Y", "enter"],
    "quit.cancel": ["n", "N", "esc"]
  },

  // Welcome screen - shown on startup
  "welcome": {
    "navigation.next_item": ["down", "j"],
    "navigation.prev_item": ["up", "k"],
    "navigation.enter": ["enter"]
  },

  // Debug mode - active when debug freeze is enabled
  "debug": {
    "debug.toggle": ["esc", "f12", "ctrl+d"],
    "debug.copy_frame": ["y", "c"],
    "debug.state.toggle": ["s"],
    "debug.mouse.toggle": ["i"]
  },

  // Text input - active when editing text fields (connection form, etc.)
  // Note: alt+ keys are for macOS compatibility (Option key)
  "text_input": {
    "input.delete_char_back": ["backspace"],
    "input.delete_char_forward": ["delete"],
    "input.delete_word_back": ["ctrl+backspace", "alt+backspace"],
    "input.delete_word_forward": ["ctrl+delete", "alt+delete"],
    "input.move_word_left": ["ctrl+left", "alt+left"],
    "input.move_word_right": ["ctrl+right", "alt+right"],
    "input.move_start": ["home", "ctrl+a"],
    "input.move_end": ["end", "ctrl+e"]
  }
}
"#
    .to_string()
}

/// Save default keybindings configuration to userdata directory
pub fn save_default_keybindings() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_keybindings_file()?;

    // Only create if it doesn't exist
    if !file_path.exists() {
        let keybindings_json = generate_default_keybindings_json();
        fs::write(&file_path, keybindings_json)?;
    }

    Ok(())
}

/// Save keybindings configuration to userdata directory
pub fn save_keybindings(keybindings: &KeybindingsConfig) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_keybindings_file()?;
    let json = serde_json::to_string_pretty(keybindings)?;
    fs::write(file_path, json)?;
    Ok(())
}
