// Re-export from tui-dispatch
pub use tui_dispatch::{parse_key_string, BindingContext as BindingContextTrait, Keybindings};

/// Context for keybindings - determines which bindings are active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, tui_dispatch::BindingContext)]
pub enum BindingContext {
    Default,
    CmdLine,
    ConnectionForm,
    ConnectionPalette,
    QuitConfirmation,
    Welcome,
    Help,
    Debug,
    TextInput,
}

/// Keybinding configuration type alias
pub type KeybindingsConfig = Keybindings<BindingContext>;

/// Format a key string for display (e.g., "ctrl+p" -> "^P", "q" -> "q", "tab" -> "Tab")
/// Uses Unicode symbols for a compact, modern look
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
    let mut kb = KeybindingsConfig::new();

    // Global keybindings - available in all contexts (as fallback)
    kb.add_global("quit.show", vec!["q".into(), "esc".into()]);
    kb.add_global("help.toggle", vec!["?".into()]);
    kb.add_global("connection.palette.toggle", vec!["ctrl+p".into()]);
    kb.add_global("connection.form.open", vec!["ctrl+n".into()]);
    kb.add_global("debug.toggle", vec!["f12".into(), "ctrl+d".into()]);

    // Default context bindings (when connected and browsing)
    kb.add(BindingContext::Default, "cmdline.search", vec!["/".into()]);
    kb.add(BindingContext::Default, "cmdline.command", vec![":".into()]);
    kb.add(BindingContext::Default, "cmdline.goto", vec!["g".into()]);
    kb.add(
        BindingContext::Default,
        "navigation.next_panel",
        vec!["tab".into()],
    );
    kb.add(
        BindingContext::Default,
        "navigation.prev_panel",
        vec!["shift+tab".into()],
    );
    kb.add(
        BindingContext::Default,
        "navigation.next_item",
        vec!["down".into(), "j".into()],
    );
    kb.add(
        BindingContext::Default,
        "navigation.prev_item",
        vec!["up".into(), "k".into()],
    );
    kb.add(
        BindingContext::Default,
        "navigation.enter",
        vec!["enter".into()],
    );
    kb.add(
        BindingContext::Default,
        "value.view_mode.cycle",
        vec!["v".into()],
    );
    kb.add(
        BindingContext::Default,
        "connection.tab.next",
        vec!["ctrl+right".into()],
    );
    kb.add(
        BindingContext::Default,
        "connection.tab.prev",
        vec!["ctrl+left".into()],
    );
    kb.add(
        BindingContext::Default,
        "pane.resize.left",
        vec!["ctrl+h".into()],
    );
    kb.add(
        BindingContext::Default,
        "pane.resize.right",
        vec!["ctrl+l".into()],
    );

    // CmdLine context bindings (search, command, goto modes)
    kb.add(BindingContext::CmdLine, "cmdline.clear", vec!["esc".into()]);
    kb.add(
        BindingContext::CmdLine,
        "cmdline.confirm",
        vec!["enter".into()],
    );
    kb.add(
        BindingContext::CmdLine,
        "cmdline.next_result",
        vec!["down".into(), "ctrl+n".into()],
    );
    kb.add(
        BindingContext::CmdLine,
        "cmdline.prev_result",
        vec!["up".into(), "ctrl+p".into()],
    );
    // Note: cmdline.history_prev/next bindings will be added in Phase 7

    // Connection form context bindings
    kb.add(
        BindingContext::ConnectionForm,
        "connection.form.submit",
        vec!["enter".into()],
    );
    kb.add(
        BindingContext::ConnectionForm,
        "connection.form.close",
        vec!["esc".into()],
    );
    kb.add(
        BindingContext::ConnectionForm,
        "connection.form.next_field",
        vec!["tab".into(), "down".into()],
    );
    kb.add(
        BindingContext::ConnectionForm,
        "connection.form.prev_field",
        vec!["shift+tab".into(), "up".into()],
    );
    kb.add(
        BindingContext::ConnectionForm,
        "connection.form.backend_type.next",
        vec!["right".into()],
    );
    kb.add(
        BindingContext::ConnectionForm,
        "connection.form.backend_type.prev",
        vec!["left".into()],
    );

    // Connection palette context bindings
    kb.add(
        BindingContext::ConnectionPalette,
        "connection.palette.close",
        vec!["esc".into()],
    );
    kb.add(
        BindingContext::ConnectionPalette,
        "connection.palette.select",
        vec!["enter".into()],
    );
    kb.add(
        BindingContext::ConnectionPalette,
        "connection.palette.next",
        vec!["down".into(), "j".into()],
    );
    kb.add(
        BindingContext::ConnectionPalette,
        "connection.palette.prev",
        vec!["up".into(), "k".into()],
    );
    kb.add(
        BindingContext::ConnectionPalette,
        "connection.palette.delete",
        vec!["d".into()],
    );

    // Quit confirmation context bindings
    kb.add(
        BindingContext::QuitConfirmation,
        "quit.confirm",
        vec!["y".into(), "enter".into()],
    );
    kb.add(
        BindingContext::QuitConfirmation,
        "quit.cancel",
        vec!["n".into(), "esc".into()],
    );

    // Welcome context bindings
    kb.add(
        BindingContext::Welcome,
        "navigation.next_item",
        vec!["down".into(), "j".into()],
    );
    kb.add(
        BindingContext::Welcome,
        "navigation.prev_item",
        vec!["up".into(), "k".into()],
    );
    kb.add(
        BindingContext::Welcome,
        "navigation.enter",
        vec!["enter".into()],
    );

    // Help context bindings
    kb.add(
        BindingContext::Help,
        "help.close",
        vec!["?".into(), "esc".into(), "q".into()],
    );

    // Debug context bindings
    kb.add(
        BindingContext::Debug,
        "debug.toggle",
        vec!["esc".into(), "f12".into(), "ctrl+d".into()],
    );
    kb.add(
        BindingContext::Debug,
        "debug.copy_frame",
        vec!["y".into(), "c".into()],
    );
    kb.add(
        BindingContext::Debug,
        "debug.state.toggle",
        vec!["s".into()],
    );
    kb.add(
        BindingContext::Debug,
        "debug.mouse.toggle",
        vec!["i".into()],
    );

    // Text input context bindings
    // Note: alt+ keys are for macOS compatibility (Option key)
    kb.add(
        BindingContext::TextInput,
        "input.delete_char_back",
        vec!["backspace".into()],
    );
    kb.add(
        BindingContext::TextInput,
        "input.delete_char_forward",
        vec!["delete".into()],
    );
    kb.add(
        BindingContext::TextInput,
        "input.delete_word_back",
        vec!["ctrl+backspace".into(), "alt+backspace".into()],
    );
    kb.add(
        BindingContext::TextInput,
        "input.delete_word_forward",
        vec!["ctrl+delete".into(), "alt+delete".into()],
    );
    kb.add(
        BindingContext::TextInput,
        "input.move_word_left",
        vec!["ctrl+left".into(), "alt+left".into()],
    );
    kb.add(
        BindingContext::TextInput,
        "input.move_word_right",
        vec!["ctrl+right".into(), "alt+right".into()],
    );
    kb.add(
        BindingContext::TextInput,
        "input.move_start",
        vec!["home".into(), "ctrl+a".into()],
    );
    kb.add(
        BindingContext::TextInput,
        "input.move_end",
        vec!["end".into(), "ctrl+e".into()],
    );

    kb
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
        // Verify some bindings exist
        assert!(bindings
            .get_context_bindings(BindingContext::Default)
            .is_some());
        assert!(bindings
            .get_context_bindings(BindingContext::CmdLine)
            .is_some());
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
        let mut user = KeybindingsConfig::new();
        user.add_global("quit.show", vec!["x".into()]);

        let merged = KeybindingsConfig::merge(defaults.clone(), user);

        // User override should be present in global
        assert_eq!(
            merged.global_bindings().get("quit.show"),
            Some(&vec!["x".to_string()])
        );

        // Other global defaults should still be there
        assert!(merged.global_bindings().contains_key("help.toggle"));
        assert!(merged
            .global_bindings()
            .contains_key("connection.palette.toggle"));
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
