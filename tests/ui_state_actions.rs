mod support;

use memtui::action::{Action, CmdLineMode};
use memtui::ui::Panel;
use rstest::rstest;

use support::{fixtures, harness::AppHarness};

#[rstest]
fn panel_focus_changes_active_panel() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Default is Keys panel
    assert_eq!(app.ui_state.active_panel, Panel::Keys);

    assert!(app.dispatch_action(Action::FocusPanel(Panel::Value)));
    assert_eq!(app.ui_state.active_panel, Panel::Value);

    assert!(app.dispatch_action(Action::FocusPanel(Panel::Keys)));
    assert_eq!(app.ui_state.active_panel, Panel::Keys);
}

#[rstest]
fn pane_resize_updates_ratio() {
    let mut app = AppHarness::new();

    let initial = app.ui_state.pane_split.ratio;
    assert!(initial > 0.0);

    assert!(app.dispatch_action(Action::SetPaneRatio(0.3)));
    assert!((app.ui_state.pane_split.ratio - 0.3).abs() < 0.01);

    // Test clamping to min
    assert!(app.dispatch_action(Action::SetPaneRatio(0.05)));
    assert!(app.ui_state.pane_split.ratio >= app.ui_state.pane_split.min);

    // Test clamping to max
    assert!(app.dispatch_action(Action::SetPaneRatio(0.95)));
    assert!(app.ui_state.pane_split.ratio <= app.ui_state.pane_split.max);
}

#[rstest]
fn resize_start_end_toggles_state() {
    let mut app = AppHarness::new();

    assert!(!app.ui_state.is_resizing);

    assert!(app.dispatch_action(Action::StartResize));
    assert!(app.ui_state.is_resizing);

    assert!(app.dispatch_action(Action::EndResize));
    assert!(!app.ui_state.is_resizing);
}

#[rstest]
fn search_selection_index_updates() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    assert_eq!(app.app_state.search_selection_index, None);

    assert!(app.dispatch_action(Action::SetCmdLineSelectionIndex(Some(2))));
    assert_eq!(app.app_state.search_selection_index, Some(2));

    assert!(app.dispatch_action(Action::SetCmdLineSelectionIndex(None)));
    assert_eq!(app.app_state.search_selection_index, None);
}

#[rstest]
fn reset_key_selection_clears_state() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Set up some state
    app.app_state.error_message = Some("test error".to_string());

    assert!(app.dispatch_action(Action::ResetKeySelection));

    // Key list selection should be cleared
    assert_eq!(app.ui_state.key_list.state.selected(), None);
    // Error message should be cleared
    assert_eq!(app.app_state.error_message, None);
}

#[rstest]
fn exit_search_mode_clears_flag() {
    let mut app = AppHarness::new();

    app.app_state.cmdline_mode = Some(CmdLineMode::Search);
    app.app_state.cmdline_active = true;

    assert!(app.dispatch_action(Action::ExitCmdLineMode));
    assert!(!app.app_state.cmdline_active);
    assert_eq!(app.app_state.cmdline_mode, Some(CmdLineMode::Search));
}

#[rstest]
fn connection_index_selection() {
    let mut app = AppHarness::new().with_connection("local");

    assert!(app.dispatch_action(Action::SelectConnectionIndex(0)));
    assert_eq!(app.ui_state.connection_list.state.selected(), Some(0));
}

#[rstest]
fn ui_toggles_work() {
    let mut app = AppHarness::new();

    // Quit confirmation
    assert!(!app.ui_state.show_quit_confirmation);
    assert!(app.dispatch_action(Action::ShowQuitConfirmation));
    assert!(app.ui_state.show_quit_confirmation);
    assert!(app.dispatch_action(Action::CancelQuit));
    assert!(!app.ui_state.show_quit_confirmation);

    // Help
    assert!(!app.ui_state.show_help);
    assert!(app.dispatch_action(Action::ToggleHelp));
    assert!(app.ui_state.show_help);
    assert!(app.dispatch_action(Action::ToggleHelp));
    assert!(!app.ui_state.show_help);

    // Connection form
    assert!(!app.ui_state.show_connection_form);
    assert!(app.dispatch_action(Action::OpenConnectionForm));
    assert!(app.ui_state.show_connection_form);
    assert!(app.dispatch_action(Action::CloseConnectionForm));
    assert!(!app.ui_state.show_connection_form);

    // Connection palette
    assert!(!app.ui_state.show_connection_palette);
    assert!(app.dispatch_action(Action::OpenConnectionPalette));
    assert!(app.ui_state.show_connection_palette);
    assert!(app.dispatch_action(Action::CloseConnectionPalette));
    assert!(!app.ui_state.show_connection_palette);
}

#[rstest]
fn dispatch_many_sequences_actions() {
    let mut app = AppHarness::new()
        .with_connection("local")
        .with_keys(fixtures::sample_keys());

    // Dispatch a sequence of actions
    let rendered = app.dispatch_many([
        Action::FocusPanel(Panel::Value),
        Action::SetPaneRatio(0.4),
        Action::FocusPanel(Panel::Keys),
    ]);

    assert!(rendered);
    assert_eq!(app.ui_state.active_panel, Panel::Keys);
    assert!((app.ui_state.pane_split.ratio - 0.4).abs() < 0.01);
}
