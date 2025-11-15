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
    pub connections: Vec<String>,
    pub connection_state: ListState,
    pub key_state: ListState,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            active_panel: Panel::Keys,
            show_help: false,
            connections: vec!["Mock Backend (localhost:6379)".to_string()],
            connection_state: ListState::default(),
            key_state: ListState::default(),
        }
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

    pub fn next_item(&mut self, keys_len: usize) -> bool {
        match self.active_panel {
            Panel::Connections => {
                let i = match self.connection_state.selected() {
                    Some(i) => {
                        if i >= self.connections.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.connection_state.select(Some(i));
                false
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

    pub fn previous_item(&mut self, keys_len: usize) -> bool {
        match self.active_panel {
            Panel::Connections => {
                let i = match self.connection_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.connections.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.connection_state.select(Some(i));
                false
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

