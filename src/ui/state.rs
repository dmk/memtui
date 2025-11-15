use super::ConnectionForm;
use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Connections,
    Keys,
    Value,
}

/// UI state - manages display and navigation
pub struct UiState {
    pub active_panel: Panel,
    pub show_help: bool,
    pub show_connection_form: bool,
    pub connection_form: ConnectionForm,
    pub form_error: Option<String>,
    pub connection_state: ListState,
    pub key_state: ListState,
}

impl UiState {
    pub fn new() -> Self {
        let mut state = Self {
            active_panel: Panel::Connections,
            show_help: false,
            show_connection_form: false,
            connection_form: ConnectionForm::new(),
            form_error: None,
            connection_state: ListState::default(),
            key_state: ListState::default(),
        };
        // Select first connection by default
        state.connection_state.select(Some(0));
        state
    }

    pub fn open_connection_form(&mut self) {
        self.connection_form.clear();
        self.form_error = None;
        self.show_connection_form = true;
    }

    pub fn close_connection_form(&mut self) {
        self.show_connection_form = false;
        self.form_error = None;
    }

    pub fn set_form_error(&mut self, error: String) {
        self.form_error = Some(error);
    }

    pub fn next_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Connections => Panel::Keys,
            Panel::Keys => Panel::Value,
            Panel::Value => Panel::Connections,
        };
    }

    pub fn prev_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Connections => Panel::Value,
            Panel::Keys => Panel::Connections,
            Panel::Value => Panel::Keys,
        };
    }

    pub fn next_item(&mut self, connections_len: usize, keys_len: usize) -> bool {
        match self.active_panel {
            Panel::Connections => {
                if connections_len == 0 {
                    return false;
                }
                let i = match self.connection_state.selected() {
                    Some(i) => {
                        if i >= connections_len - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.connection_state.select(Some(i));
                true // Need to potentially switch connections
            }
            Panel::Keys => {
                let i = match self.key_state.selected() {
                    Some(i) => {
                        if i >= keys_len - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.key_state.select(Some(i));
                true // Need to update value
            }
            Panel::Value => false,
        }
    }

    pub fn previous_item(&mut self, connections_len: usize, keys_len: usize) -> bool {
        match self.active_panel {
            Panel::Connections => {
                if connections_len == 0 {
                    return false;
                }
                let i = match self.connection_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            connections_len - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.connection_state.select(Some(i));
                true // Need to potentially switch connections
            }
            Panel::Keys => {
                let i = match self.key_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            keys_len - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.key_state.select(Some(i));
                true // Need to update value
            }
            Panel::Value => false,
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
