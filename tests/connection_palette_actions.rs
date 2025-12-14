mod support;

use memtui::action::Action;
use memtui::types::ConnectionConfig;
use rstest::rstest;

use support::{fixtures, harness::AppHarness};

fn alpha_and_beta() -> (ConnectionConfig, ConnectionConfig) {
    let mut alpha = fixtures::connection("alpha");
    alpha.name = "Alpha".to_string();
    let mut beta = fixtures::connection("beta");
    beta.name = "Beta".to_string();
    (alpha, beta)
}

#[rstest]
fn open_palette_selects_active_connection() {
    let (alpha, beta) = alpha_and_beta();

    let mut app = AppHarness::new().with_connection_config(beta.clone());
    app = app.with_connection_config(alpha);
    app.app_state.connection_manager.set_active(&beta.id);

    // Start from a different selection to ensure OpenConnectionPalette recalculates.
    app.ui_state.connection_list.state.select(Some(0));

    assert!(app.dispatch_action(Action::OpenConnectionPalette));
    assert!(app.ui_state.show_connection_palette);
    assert_eq!(
        app.ui_state.connection_list.state.selected(),
        Some(1),
        "active connection should be highlighted based on sorted configs"
    );
}

#[rstest]
fn open_palette_without_connections_clears_selection() {
    let mut app = AppHarness::new();

    assert!(app.dispatch_action(Action::OpenConnectionPalette));
    assert!(app.ui_state.show_connection_palette);
    assert_eq!(
        app.ui_state.connection_list.state.selected(),
        None,
        "palette should have no selection when there are no connections"
    );
}
