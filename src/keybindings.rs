use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context for keybindings - determines which bindings are active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingContext {
    Default,
    Search,
    ConnectionForm,
    ConnectionPalette,
    QuitConfirmation,
    Welcome,
    Debug,
}

/// Keybinding configuration - maps command names to key combinations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeybindingsConfig {
    /// Global keybindings - always checked as fallback for all contexts
    #[serde(default)]
    pub global: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub default: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub search: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub connection_form: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub connection_palette: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub quit_confirmation: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub welcome: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub debug: HashMap<String, Vec<String>>,
}

impl KeybindingsConfig {
    /// Get bindings for a specific context
    pub fn get_context_bindings(&self, context: BindingContext) -> &HashMap<String, Vec<String>> {
        match context {
            BindingContext::Default => &self.default,
            BindingContext::Search => &self.search,
            BindingContext::ConnectionForm => &self.connection_form,
            BindingContext::ConnectionPalette => &self.connection_palette,
            BindingContext::QuitConfirmation => &self.quit_confirmation,
            BindingContext::Welcome => &self.welcome,
            BindingContext::Debug => &self.debug,
        }
    }

    /// Merge user config onto defaults - user config overrides defaults
    pub fn merge(defaults: Self, user: Self) -> Self {
        Self {
            global: merge_hashmaps(defaults.global, user.global),
            default: merge_hashmaps(defaults.default, user.default),
            search: merge_hashmaps(defaults.search, user.search),
            connection_form: merge_hashmaps(defaults.connection_form, user.connection_form),
            connection_palette: merge_hashmaps(
                defaults.connection_palette,
                user.connection_palette,
            ),
            quit_confirmation: merge_hashmaps(defaults.quit_confirmation, user.quit_confirmation),
            welcome: merge_hashmaps(defaults.welcome, user.welcome),
            debug: merge_hashmaps(defaults.debug, user.debug),
        }
    }

    /// Get command name for a key event in the given context
    /// First checks context-specific bindings, then falls back to global
    pub fn get_command(&self, key: KeyEvent, context: BindingContext) -> Option<String> {
        // First try context-specific bindings
        if let Some(cmd) = self.match_key_in_bindings(key, self.get_context_bindings(context)) {
            return Some(cmd);
        }

        // Fall back to global bindings
        self.match_key_in_bindings(key, &self.global)
    }

    /// Helper to match a key against a set of bindings
    fn match_key_in_bindings(
        &self,
        key: KeyEvent,
        bindings: &HashMap<String, Vec<String>>,
    ) -> Option<String> {
        for (command, keys) in bindings {
            for key_str in keys {
                if let Some(parsed_key) = parse_key_string(key_str) {
                    // Compare code and modifiers (ignore kind and state)
                    // For character keys, compare case-insensitively
                    let codes_match = match (&parsed_key.code, &key.code) {
                        (KeyCode::Char(c1), KeyCode::Char(c2)) => {
                            c1.to_lowercase().to_string() == c2.to_lowercase().to_string()
                        }
                        _ => parsed_key.code == key.code,
                    };

                    if codes_match && parsed_key.modifiers == key.modifiers {
                        return Some(command.clone());
                    }
                }
            }
        }
        None
    }

    /// Get the first keybinding string for a command in the given context
    /// First checks context-specific bindings, then falls back to global
    pub fn get_first_keybinding(&self, command: &str, context: BindingContext) -> Option<String> {
        let bindings = self.get_context_bindings(context);
        bindings
            .get(command)
            .and_then(|keys| keys.first().cloned())
            .or_else(|| {
                self.global
                    .get(command)
                    .and_then(|keys| keys.first().cloned())
            })
    }
}

fn merge_hashmaps(
    mut defaults: HashMap<String, Vec<String>>,
    user: HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    for (key, value) in user {
        defaults.insert(key, value);
    }
    defaults
}

/// Parse a key string like "q", "esc", "ctrl+p", "shift+tab" into a KeyEvent
pub fn parse_key_string(key_str: &str) -> Option<KeyEvent> {
    let key_str = key_str.trim().to_lowercase();

    if key_str.is_empty() {
        return None;
    }

    // Special case: shift+tab should be BackTab
    if key_str == "shift+tab" || key_str == "backtab" {
        return Some(KeyEvent {
            code: KeyCode::BackTab,
            modifiers: KeyModifiers::SHIFT,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        });
    }

    // Check for modifiers
    let parts: Vec<&str> = key_str.split('+').collect();
    let mut modifiers = KeyModifiers::empty();
    let key_part = parts.last()?.trim();

    if parts.len() > 1 {
        for part in &parts[..parts.len() - 1] {
            match part.trim() {
                "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                "alt" => modifiers |= KeyModifiers::ALT,
                _ => {}
            }
        }
    }

    // Parse the key code
    let code = match key_part {
        "esc" | "escape" => KeyCode::Esc,
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => {
            if modifiers.is_empty() {
                modifiers |= KeyModifiers::SHIFT;
            }
            KeyCode::BackTab
        }
        "backspace" => KeyCode::Backspace,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        // Single character
        c if c.len() == 1 => {
            // For special keys like '?' we need to handle them
            let ch = c.chars().next()?;
            KeyCode::Char(ch)
        }
        _ => return None,
    };

    Some(KeyEvent {
        code,
        modifiers,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    })
}

/// Format a key string for display (e.g., "ctrl+p" -> "^P", "q" -> "q", "tab" -> "Tab")
pub fn format_key_for_display(key_str: &str) -> String {
    let key_str = key_str.trim().to_lowercase();

    // Handle special cases first
    if key_str == "shift+tab" || key_str == "backtab" {
        return "⇧Tab".to_string();
    }

    // Check for modifiers
    let parts: Vec<&str> = key_str.split('+').collect();
    let mut modifiers = Vec::new();
    let key_part = parts.last().copied().unwrap_or(key_str.as_str());

    if parts.len() > 1 {
        for part in &parts[..parts.len() - 1] {
            match part.trim() {
                "ctrl" | "control" => modifiers.push("^"),
                "shift" => modifiers.push("⇧"),
                "alt" => modifiers.push("⌥"),
                _ => {}
            }
        }
    }

    // Format the key part
    let key_display = match key_part {
        "esc" | "escape" => "Esc".to_string(),
        "enter" | "return" => "Enter".to_string(),
        "tab" => "Tab".to_string(),
        "backspace" => "⌫".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "pageup" => "PgUp".to_string(),
        "pagedown" => "PgDn".to_string(),
        "delete" => "Del".to_string(),
        "insert" => "Ins".to_string(),
        "f1" => "F1".to_string(),
        "f2" => "F2".to_string(),
        "f3" => "F3".to_string(),
        "f4" => "F4".to_string(),
        "f5" => "F5".to_string(),
        "f6" => "F6".to_string(),
        "f7" => "F7".to_string(),
        "f8" => "F8".to_string(),
        "f9" => "F9".to_string(),
        "f10" => "F10".to_string(),
        "f11" => "F11".to_string(),
        "f12" => "F12".to_string(),
        // Single character - capitalize for display
        c if c.len() == 1 => {
            let ch = c.chars().next().unwrap();
            // Keep special characters as-is, capitalize letters
            if ch.is_alphabetic() {
                ch.to_uppercase().collect::<String>()
            } else {
                ch.to_string()
            }
        }
        _ => key_part.to_string(),
    };

    // Combine modifiers with key
    if modifiers.is_empty() {
        key_display
    } else {
        format!("{}{}", modifiers.join(""), key_display)
    }
}

/// Create default keybindings that match the current hardcoded behavior
pub fn default_keybindings() -> KeybindingsConfig {
    let mut config = KeybindingsConfig::default();

    // Global keybindings - available in all contexts (as fallback)
    let mut global = HashMap::new();
    global.insert(
        "quit.show".to_string(),
        vec!["q".to_string(), "esc".to_string()],
    );
    global.insert("help.toggle".to_string(), vec!["?".to_string()]);
    global.insert(
        "connection.palette.toggle".to_string(),
        vec!["ctrl+p".to_string()],
    );
    global.insert(
        "connection.form.open".to_string(),
        vec!["ctrl+n".to_string()],
    );
    global.insert(
        "debug.toggle".to_string(),
        vec!["f12".to_string(), "ctrl+d".to_string()],
    );
    config.global = global;

    // Default context bindings (when connected and browsing)
    let mut default = HashMap::new();
    default.insert("search.start".to_string(), vec!["/".to_string()]);
    default.insert("navigation.next_panel".to_string(), vec!["tab".to_string()]);
    default.insert(
        "navigation.prev_panel".to_string(),
        vec!["shift+tab".to_string()],
    );
    default.insert(
        "navigation.next_item".to_string(),
        vec!["down".to_string(), "j".to_string()],
    );
    default.insert(
        "navigation.prev_item".to_string(),
        vec!["up".to_string(), "k".to_string()],
    );
    default.insert("navigation.enter".to_string(), vec!["enter".to_string()]);
    default.insert("value.view_mode.cycle".to_string(), vec!["v".to_string()]);
    default.insert(
        "connection.tab.next".to_string(),
        vec!["ctrl+right".to_string()],
    );
    default.insert(
        "connection.tab.prev".to_string(),
        vec!["ctrl+left".to_string()],
    );
    default.insert("pane.resize.left".to_string(), vec!["ctrl+h".to_string()]);
    default.insert("pane.resize.right".to_string(), vec!["ctrl+l".to_string()]);
    config.default = default;

    // Search context bindings
    let mut search = HashMap::new();
    search.insert("search.clear".to_string(), vec!["esc".to_string()]);
    search.insert(
        "search.next_result".to_string(),
        vec!["down".to_string(), "j".to_string()],
    );
    search.insert(
        "search.prev_result".to_string(),
        vec!["up".to_string(), "k".to_string()],
    );
    search.insert("search.confirm".to_string(), vec!["enter".to_string()]);
    config.search = search;

    // Connection form context bindings
    let mut connection_form = HashMap::new();
    connection_form.insert(
        "connection.form.submit".to_string(),
        vec!["enter".to_string()],
    );
    connection_form.insert("connection.form.close".to_string(), vec!["esc".to_string()]);
    connection_form.insert(
        "connection.form.next_field".to_string(),
        vec!["tab".to_string()],
    );
    connection_form.insert(
        "connection.form.prev_field".to_string(),
        vec!["shift+tab".to_string()],
    );
    config.connection_form = connection_form;

    // Connection palette context bindings
    let mut connection_palette = HashMap::new();
    connection_palette.insert(
        "connection.palette.close".to_string(),
        vec!["esc".to_string()],
    );
    connection_palette.insert(
        "connection.palette.select".to_string(),
        vec!["enter".to_string()],
    );
    connection_palette.insert(
        "connection.palette.next".to_string(),
        vec!["down".to_string(), "j".to_string()],
    );
    connection_palette.insert(
        "connection.palette.prev".to_string(),
        vec!["up".to_string(), "k".to_string()],
    );
    connection_palette.insert(
        "connection.palette.delete".to_string(),
        vec!["d".to_string()],
    );
    config.connection_palette = connection_palette;

    // Quit confirmation context bindings
    let mut quit_confirmation = HashMap::new();
    // Note: "Y" and "N" will be handled by case-insensitive matching in parse_key_string
    quit_confirmation.insert(
        "quit.confirm".to_string(),
        vec!["y".to_string(), "enter".to_string()],
    );
    quit_confirmation.insert(
        "quit.cancel".to_string(),
        vec!["n".to_string(), "esc".to_string()],
    );
    config.quit_confirmation = quit_confirmation;

    // Welcome context bindings
    let mut welcome = HashMap::new();
    welcome.insert(
        "navigation.next_item".to_string(),
        vec!["down".to_string(), "j".to_string()],
    );
    welcome.insert(
        "navigation.prev_item".to_string(),
        vec!["up".to_string(), "k".to_string()],
    );
    welcome.insert("navigation.enter".to_string(), vec!["enter".to_string()]);
    config.welcome = welcome;

    // Debug context bindings
    let mut debug = HashMap::new();
    debug.insert(
        "debug.toggle".to_string(),
        vec!["esc".to_string(), "f12".to_string(), "ctrl+d".to_string()],
    );
    debug.insert(
        "debug.copy_frame".to_string(),
        vec!["y".to_string(), "c".to_string()],
    );
    debug.insert("debug.state.toggle".to_string(), vec!["s".to_string()]);
    debug.insert("debug.mouse.toggle".to_string(), vec!["i".to_string()]);
    config.debug = debug;

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_parse_simple_key() {
        let result = parse_key_string("q").unwrap();
        assert_eq!(result.code, KeyCode::Char('q'));
        assert_eq!(result.modifiers, KeyModifiers::empty());
    }

    #[test]
    fn test_parse_esc() {
        let result = parse_key_string("esc").unwrap();
        assert_eq!(result.code, KeyCode::Esc);
    }

    #[test]
    fn test_parse_ctrl_key() {
        let result = parse_key_string("ctrl+p").unwrap();
        assert_eq!(result.code, KeyCode::Char('p'));
        assert!(result.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn test_parse_shift_tab() {
        let result = parse_key_string("shift+tab").unwrap();
        assert_eq!(result.code, KeyCode::BackTab);
        assert!(result.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn test_parse_backtab() {
        let result = parse_key_string("backtab").unwrap();
        assert_eq!(result.code, KeyCode::BackTab);
        assert!(result.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn test_parse_arrow_keys() {
        let result = parse_key_string("up").unwrap();
        assert_eq!(result.code, KeyCode::Up);

        let result = parse_key_string("down").unwrap();
        assert_eq!(result.code, KeyCode::Down);
    }

    #[test]
    fn test_default_keybindings() {
        let bindings = default_keybindings();
        assert!(!bindings.default.is_empty());
        assert!(!bindings.search.is_empty());
    }

    #[test]
    fn test_get_command() {
        let bindings = default_keybindings();
        let key = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        };
        let command = bindings.get_command(key, BindingContext::Default);
        assert_eq!(command, Some("quit.show".to_string()));
    }

    #[test]
    fn test_merge_keybindings() {
        let defaults = default_keybindings();
        let mut user = KeybindingsConfig::default();
        let mut user_global = HashMap::new();
        user_global.insert("quit.show".to_string(), vec!["x".to_string()]);
        user.global = user_global;

        let merged = KeybindingsConfig::merge(defaults.clone(), user);

        // User override should be present in global
        assert_eq!(merged.global.get("quit.show"), Some(&vec!["x".to_string()]));

        // Other global defaults should still be there
        assert!(merged.global.contains_key("help.toggle"));
        assert!(merged.global.contains_key("connection.palette.toggle"));
    }

    #[test]
    fn test_quit_confirmation_keybindings() {
        let bindings = default_keybindings();

        // Test 'y' for quit.confirm
        let key_y = KeyEvent {
            code: KeyCode::Char('y'),
            modifiers: KeyModifiers::empty(),
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        };
        let command = bindings.get_command(key_y, BindingContext::QuitConfirmation);
        assert_eq!(
            command,
            Some("quit.confirm".to_string()),
            "Expected 'y' to map to quit.confirm"
        );

        // Test 'n' for quit.cancel
        let key_n = KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::empty(),
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        };
        let command = bindings.get_command(key_n, BindingContext::QuitConfirmation);
        assert_eq!(
            command,
            Some("quit.cancel".to_string()),
            "Expected 'n' to map to quit.cancel"
        );
    }
}
