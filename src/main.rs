use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

use memtui::app::AppState;
use memtui::ui::{self, Panel, UiState};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state (with connection manager)
    let mut app_state = AppState::new();

    // Create UI state (display + navigation)
    let mut ui_state = UiState::new();

    if ui_state.connection_state.selected().is_none() {
        ui_state.connection_state.select(Some(0));
    }
    activate_selected_connection(&mut app_state, &mut ui_state).await;

    // Run app
    let res = run_app(&mut terminal, &mut app_state, &mut ui_state).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app_state: &mut AppState,
    ui_state: &mut UiState,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            ui::render(f, app_state, ui_state);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // Connection form is open
            if ui_state.show_connection_form {
                match key.code {
                    KeyCode::Esc => {
                        ui_state.close_connection_form();
                    }
                    KeyCode::Enter => {
                        // Try to save the connection
                        match ui_state.connection_form.to_config() {
                            Ok(config) => {
                                let new_id = config.id.clone();
                                app_state.connection_manager.add_connection(config);
                                if let Some(idx) = app_state
                                    .connection_manager
                                    .get_configs()
                                    .iter()
                                    .position(|c| c.id == new_id)
                                {
                                    ui_state.connection_state.select(Some(idx));
                                }
                                ui_state.close_connection_form();
                                activate_selected_connection(app_state, ui_state).await;
                            }
                            Err(e) => {
                                ui_state.set_form_error(e);
                            }
                        }
                    }
                    KeyCode::Tab => {
                        ui_state.connection_form.next_field();
                        ui_state.form_error = None;
                    }
                    KeyCode::BackTab => {
                        ui_state.connection_form.prev_field();
                        ui_state.form_error = None;
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        ui_state.connection_form.add_char(c);
                        ui_state.form_error = None;
                    }
                    KeyCode::Backspace => {
                        ui_state.connection_form.delete_char();
                        ui_state.form_error = None;
                    }
                    _ => {}
                }
            }
            // Help is open
            else if ui_state.show_help {
                // Close help on any key
                ui_state.show_help = false;
            }
            // Normal navigation
            else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('?') => ui_state.show_help = true,
                    KeyCode::Char('n') if ui_state.active_panel == Panel::Connections => {
                        ui_state.open_connection_form();
                    }
                    KeyCode::Char('d') if ui_state.active_panel == Panel::Connections => {
                        // Delete selected connection
                        if let Some(idx) = ui_state.connection_state.selected() {
                            let configs: Vec<_> = app_state
                                .connection_manager
                                .get_configs()
                                .iter()
                                .map(|c| c.id.clone())
                                .collect();
                            if let Some(id) = configs.get(idx)
                                && id != "mock"
                            {
                                app_state.connection_manager.remove_config(id);
                                let remaining = app_state.connection_manager.get_configs();
                                if !remaining.is_empty() {
                                    let new_idx = idx.min(remaining.len() - 1);
                                    ui_state.connection_state.select(Some(new_idx));
                                    activate_selected_connection(app_state, ui_state).await;
                                } else {
                                    ui_state.connection_state.select(None);
                                }
                            }
                        }
                    }
                    KeyCode::Enter if ui_state.active_panel == Panel::Connections => {
                        ui_state.active_panel = Panel::Keys;
                    }
                    KeyCode::Tab => ui_state.next_panel(),
                    KeyCode::BackTab => ui_state.prev_panel(),
                    KeyCode::Down | KeyCode::Char('j') => {
                        let connections_len = app_state.connection_manager.get_configs().len();
                        let keys_len = app_state.keys.len();

                        if ui_state.next_item(connections_len, keys_len) {
                            match ui_state.active_panel {
                                Panel::Connections => {
                                    activate_selected_connection(app_state, ui_state).await;
                                }
                                Panel::Keys => {
                                    app_state.update_value(ui_state.key_state.selected()).await;
                                }
                                Panel::Value => {}
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let connections_len = app_state.connection_manager.get_configs().len();
                        let keys_len = app_state.keys.len();

                        if ui_state.previous_item(connections_len, keys_len) {
                            match ui_state.active_panel {
                                Panel::Connections => {
                                    activate_selected_connection(app_state, ui_state).await;
                                }
                                Panel::Keys => {
                                    app_state.update_value(ui_state.key_state.selected()).await;
                                }
                                Panel::Value => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn activate_selected_connection(app_state: &mut AppState, ui_state: &mut UiState) {
    let configs = app_state.connection_manager.get_configs();
    if configs.is_empty() {
        return;
    }

    let mut idx = ui_state.connection_state.selected().unwrap_or(0);
    if idx >= configs.len() {
        idx = configs.len() - 1;
        ui_state.connection_state.select(Some(idx));
    }

    let connection_id = configs[idx].id.clone();

    if app_state.connection_manager.get_active_id() == Some(connection_id.as_str())
        && !app_state.keys.is_empty()
    {
        return;
    }

    if app_state.connect_to(&connection_id).await.is_ok() {
        if !app_state.keys.is_empty() {
            ui_state.key_state.select(Some(0));
            app_state.update_value(ui_state.key_state.selected()).await;
        } else {
            ui_state.key_state.select(None);
            app_state.selected_value.clear();
        }
    }
}
