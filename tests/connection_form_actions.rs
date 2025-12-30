mod support;

use memtui::action::Action;
use memtui::config::Config;
use memtui::types::{Auth, BackendType};
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

#[rstest]
fn form_backend_type_selection_updates_defaults() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    assert_eq!(
        app.ui_state.connection_form.backend_type,
        BackendType::Redis
    );
    assert_eq!(app.ui_state.connection_form.port.value(), "6379");

    assert!(app.dispatch_action(Action::ConnectionFormNextBackendType));
    assert_eq!(
        app.ui_state.connection_form.backend_type,
        BackendType::Memcached
    );
    assert_eq!(app.ui_state.connection_form.port.value(), "11211");

    assert!(app.dispatch_action(Action::ConnectionFormNextBackendType));
    assert_eq!(app.ui_state.connection_form.backend_type, BackendType::Etcd);
    assert_eq!(app.ui_state.connection_form.port.value(), "2379");

    assert!(app.dispatch_action(Action::ConnectionFormNextBackendType));
    assert_eq!(
        app.ui_state.connection_form.backend_type,
        BackendType::Redis
    );
    assert_eq!(app.ui_state.connection_form.port.value(), "6379");

    assert!(app.dispatch_action(Action::ConnectionFormPrevBackendType));
    assert_eq!(app.ui_state.connection_form.backend_type, BackendType::Etcd);
    assert_eq!(app.ui_state.connection_form.port.value(), "2379");
}

#[rstest]
fn form_validation_required_fields() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    assert_eq!(app.ui_state.connection_form.name.value(), "");
    assert!(app.ui_state.form_error.is_none());

    assert!(app.dispatch_action(Action::Enter));
    assert_eq!(app.ui_state.form_error.as_deref(), Some("Name is required"));
    assert!(app.ui_state.show_connection_form);

    app.ui_state.connection_form.name.set_value("demo");
    app.ui_state.connection_form.host.set_value("");
    assert!(app.dispatch_action(Action::Enter));
    assert_eq!(app.ui_state.form_error.as_deref(), Some("Host is required"));

    app.ui_state.connection_form.host.set_value("localhost");
    app.ui_state.connection_form.port.set_value("nope");
    assert!(app.dispatch_action(Action::Enter));
    assert_eq!(
        app.ui_state.form_error.as_deref(),
        Some("Port must be a number between 1 and 65535")
    );

    assert_eq!(app.app_state.connection_manager.get_configs().len(), 0);
    assert!(app.drain_actions().is_empty());
}

#[rstest]
fn create_redis_connection() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    app.ui_state.connection_form.name.set_value("redis-main");
    app.ui_state.connection_form.host.set_value("127.0.0.1");
    app.ui_state.connection_form.port.set_value("6380");

    let config = app
        .ui_state
        .connection_form
        .to_config(Config::default().connection.default_timeout)
        .expect("valid config");

    assert_eq!(config.backend_type, BackendType::Redis);
    assert_eq!(config.name, "redis-main");
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 6380);
    assert_eq!(config.auth, None);
    assert_eq!(config.database, Some("0".to_string()));
}

#[rstest]
fn create_redis_with_auth() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    app.ui_state.connection_form.name.set_value("redis-auth");
    app.ui_state.connection_form.password.set_value("secret");

    let config = app
        .ui_state
        .connection_form
        .to_config(Config::default().connection.default_timeout)
        .expect("valid config");

    assert_eq!(config.backend_type, BackendType::Redis);
    assert_eq!(config.auth, Some(Auth::Token("secret".to_string())));
}

#[rstest]
fn create_redis_with_database() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    app.ui_state.connection_form.name.set_value("redis-db");
    app.ui_state.connection_form.database.set_value("5");

    let config = app
        .ui_state
        .connection_form
        .to_config(Config::default().connection.default_timeout)
        .expect("valid config");

    assert_eq!(config.backend_type, BackendType::Redis);
    assert_eq!(config.database, Some("5".to_string()));
}

#[rstest]
fn create_memcached_connection() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);
    app.ui_state
        .connection_form
        .select_backend_type(BackendType::Memcached);
    app.ui_state.connection_form.name.set_value("cache");
    app.ui_state.connection_form.host.set_value("cache.local");

    let config = app
        .ui_state
        .connection_form
        .to_config(Config::default().connection.default_timeout)
        .expect("valid config");

    assert_eq!(config.backend_type, BackendType::Memcached);
    assert_eq!(config.host, "cache.local");
    assert_eq!(config.port, 11211);
    assert_eq!(config.database, None);
}

#[rstest]
fn create_etcd_connection() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);
    app.ui_state
        .connection_form
        .select_backend_type(BackendType::Etcd);
    app.ui_state.connection_form.name.set_value("etcd");
    app.ui_state.connection_form.host.set_value("etcd.local");

    let config = app
        .ui_state
        .connection_form
        .to_config(Config::default().connection.default_timeout)
        .expect("valid config");

    assert_eq!(config.backend_type, BackendType::Etcd);
    assert_eq!(config.host, "etcd.local");
    assert_eq!(config.port, 2379);
    assert_eq!(config.database, None);
}

#[rstest]
fn form_submit_enter_emits_submit_action() {
    let mut app = AppHarness::new();
    app.dispatch_action(Action::OpenConnectionForm);

    app.ui_state.connection_form.name.set_value("submit");
    app.ui_state.connection_form.host.set_value("localhost");
    app.ui_state.connection_form.port.set_value("6379");

    assert!(app.dispatch_action(Action::Enter));

    let actions = app.drain_actions();
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Action::SubmitConnectionForm(config) => {
            assert_eq!(config.name, "submit");
            assert_eq!(config.host, "localhost");
            assert_eq!(config.port, 6379);
        }
        action => panic!("unexpected action: {:?}", action),
    }
}
