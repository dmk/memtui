mod support;

use memtui::action::Action;
use memtui::ui::components::connection_form::FormField;
use rstest::rstest;

use support::harness::AppHarness;

#[rstest]
fn open_connection_form_shows_form() {
    let mut app = AppHarness::new();

    assert!(!app.ui_state.show_connection_form);

    assert!(app.dispatch_action(Action::OpenConnectionForm));

    assert!(app.ui_state.show_connection_form);
}

#[rstest]
fn close_connection_form_hides_form() {
    let mut app = AppHarness::new();

    app.dispatch_action(Action::OpenConnectionForm);
    assert!(app.ui_state.show_connection_form);

    assert!(app.dispatch_action(Action::CloseConnectionForm));
    assert!(!app.ui_state.show_connection_form);
}

#[rstest]
fn form_field_navigation_next() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    // Default field is BackendType
    assert_eq!(
        app.ui_state.connection_form.active_field,
        FormField::BackendType
    );

    // Navigate through fields
    assert!(app.dispatch_action(Action::ConnectionFormNextField));
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Name);

    assert!(app.dispatch_action(Action::ConnectionFormNextField));
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Host);

    assert!(app.dispatch_action(Action::ConnectionFormNextField));
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Port);
}

#[rstest]
fn form_field_navigation_prev() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    // Navigate to a middle field
    app.dispatch_many([
        Action::ConnectionFormNextField,
        Action::ConnectionFormNextField,
        Action::ConnectionFormNextField,
    ]);
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Port);

    // Navigate backwards
    assert!(app.dispatch_action(Action::ConnectionFormPrevField));
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Host);

    assert!(app.dispatch_action(Action::ConnectionFormPrevField));
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Name);
}

#[rstest]
fn form_add_char_updates_name_field() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    // Move to name field
    app.dispatch_action(Action::ConnectionFormNextField);
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Name);

    // Type characters
    app.dispatch_action(Action::ConnectionFormAddChar('t'));
    app.dispatch_action(Action::ConnectionFormAddChar('e'));
    app.dispatch_action(Action::ConnectionFormAddChar('s'));
    app.dispatch_action(Action::ConnectionFormAddChar('t'));

    assert_eq!(app.ui_state.connection_form.name.value(), "test");
}

#[rstest]
fn form_delete_char_removes_character() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    // Move to name field and type
    app.dispatch_action(Action::ConnectionFormNextField);
    app.dispatch_action(Action::ConnectionFormAddChar('a'));
    app.dispatch_action(Action::ConnectionFormAddChar('b'));
    app.dispatch_action(Action::ConnectionFormAddChar('c'));

    assert_eq!(app.ui_state.connection_form.name.value(), "abc");

    // Delete last character
    app.dispatch_action(Action::ConnectionFormDeleteChar);
    assert_eq!(app.ui_state.connection_form.name.value(), "ab");
}

#[rstest]
fn form_opens_with_reset_state() {
    let mut app = AppHarness::new();

    // Open form and add some data
    app.dispatch_action(Action::OpenConnectionForm);
    app.dispatch_action(Action::ConnectionFormNextField);
    app.dispatch_action(Action::ConnectionFormAddChar('x'));

    assert_eq!(app.ui_state.connection_form.name.value(), "x");

    // Close and reopen
    app.dispatch_action(Action::CloseConnectionForm);
    app.dispatch_action(Action::OpenConnectionForm);

    // Form should be reset
    assert_eq!(app.ui_state.connection_form.name.value(), "");
    assert_eq!(
        app.ui_state.connection_form.active_field,
        FormField::BackendType
    );
}

#[rstest]
fn form_host_field_has_default() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    // Navigate to host field
    app.dispatch_many([
        Action::ConnectionFormNextField, // Name
        Action::ConnectionFormNextField, // Host
    ]);
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Host);

    // Host should have default value "localhost"
    assert_eq!(app.ui_state.connection_form.host.value(), "localhost");

    // Appending characters works
    for c in "2".chars() {
        app.dispatch_action(Action::ConnectionFormAddChar(c));
    }
    assert_eq!(app.ui_state.connection_form.host.value(), "localhost2");
}

#[rstest]
fn form_port_field_has_default() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    // Navigate to port field
    app.dispatch_many([
        Action::ConnectionFormNextField, // Name
        Action::ConnectionFormNextField, // Host
        Action::ConnectionFormNextField, // Port
    ]);
    assert_eq!(app.ui_state.connection_form.active_field, FormField::Port);

    // Port should have default value "6379"
    assert_eq!(app.ui_state.connection_form.port.value(), "6379");
}
