mod support;

use insta::assert_snapshot;
use memtui::action::Action;
use memtui::types::ConnectionConfig;
use rstest::rstest;

use support::{
    fixtures,
    harness::AppHarness,
    render::{buffer_rect_to_string, buffer_to_string},
};

fn two_connections() -> (ConnectionConfig, ConnectionConfig) {
    let mut first = fixtures::connection("alpha");
    first.name = "Alpha".to_string();
    let mut second = fixtures::connection("beta");
    second.name = "Beta".to_string();
    (first, second)
}

#[rstest]
fn connection_palette_snapshot() {
    let (first, second) = two_connections();

    let mut app = AppHarness::new().with_connection_config(first.clone());
    app = app.with_connection_config(second.clone());

    // Mark the first as active to show the indicator in the palette.
    app.app_state.connection_manager.set_active(&first.id);
    app.dispatch_action(Action::OpenConnectionPalette);

    let buffer = app.render((80, 20));
    let full = buffer_to_string(&buffer);
    assert_snapshot!("connection_palette_full", full);

    if let Some(area) = app.ui_state.connection_palette_area {
        let modal = buffer_rect_to_string(&buffer, area);
        assert_snapshot!("connection_palette_modal_only", modal);
    }
}
