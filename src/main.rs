use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

use memtui::app::AppState;
use memtui::backend::MockBackend;
use memtui::ui::{self, UiState};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state (backend + data)
    let backend = Box::new(MockBackend::new(true));
    let mut app_state = AppState::new(backend);

    // Create UI state (display + navigation)
    let mut ui_state = UiState::new();

    // Connect to backend and load initial data
    let _ = app_state.connect().await;
    app_state.load_keys().await;

    // Initialize UI selection
    if !app_state.keys.is_empty() {
        ui_state.key_state.select(Some(0));
        app_state.update_value(ui_state.key_state.selected()).await;
    }

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
            ui::render(f, ui_state, &app_state.keys, &app_state.selected_value);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if ui_state.show_help {
                // Close help on any key
                ui_state.show_help = false;
            } else {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') => ui_state.show_help = true,
                    KeyCode::Tab => ui_state.next_panel(),
                    KeyCode::BackTab => ui_state.prev_panel(),
                    KeyCode::Down | KeyCode::Char('j') => {
                        if ui_state.next_item(app_state.keys.len()) {
                            app_state.update_value(ui_state.key_state.selected()).await;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if ui_state.previous_item(app_state.keys.len()) {
                            app_state.update_value(ui_state.key_state.selected()).await;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
