use super::ConnectionForm;
use super::components::connection_list::ConnectionList;
use super::components::key_browser::KeyBrowser;
use super::components::value_viewer::ValueViewer;
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
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

    pub connection_list: ConnectionList,
    pub key_browser: KeyBrowser,
    pub value_viewer: ValueViewer,
    // Shortcuts for compatibility with existing main.rs logic
    // We expose getters/setters or just public fields if we want to keep refactor minimal
    // But main.rs accesses .connection_state directly.
    // I will expose them via public fields in the components,
    // and add proxy methods or update main.rs to access them via components.
}

impl UiState {
    pub fn new() -> Self {
        Self {
            active_panel: Panel::Connections,
            show_help: false,
            show_connection_form: false,
            connection_form: ConnectionForm::new(),
            form_error: None,

            connection_list: ConnectionList::new(),
            key_browser: KeyBrowser::new(),
            value_viewer: ValueViewer::new(),
        }
    }

    // Helper accessors for main.rs compatibility (temporary until main.rs is fully updated)
    pub fn connection_state(&mut self) -> &mut ratatui::widgets::ListState {
        &mut self.connection_list.state
    }

    pub fn key_state(&mut self) -> &mut ratatui::widgets::ListState {
        &mut self.key_browser.state
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
        let mut cycle = Panel::iter().cycle();
        if let Some(_) = cycle.find(|&p| p == self.active_panel)
            && let Some(next) = cycle.next()
        {
            self.active_panel = next;
        }
    }

    pub fn prev_panel(&mut self) {
        let mut cycle = Panel::iter().rev().cycle();
        if let Some(_) = cycle.find(|&p| p == self.active_panel)
            && let Some(next) = cycle.next()
        {
            self.active_panel = next;
        }
    }

    pub fn next_item(&mut self, connections_len: usize, keys_len: usize) -> bool {
        match self.active_panel {
            Panel::Connections => {
                if connections_len == 0 {
                    return false;
                }
                self.connection_list.next(connections_len);
                true
            }
            Panel::Keys => {
                if keys_len == 0 {
                    self.key_browser.select(None);
                    return false;
                }
                let current = self.key_browser.state.selected().unwrap_or(0);
                let next = if current >= keys_len - 1 {
                    0
                } else {
                    current + 1
                };
                self.key_browser.select(Some(next));
                true
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
                self.connection_list.prev(connections_len);
                true
            }
            Panel::Keys => {
                if keys_len == 0 {
                    self.key_browser.select(None);
                    return false;
                }
                let current = self.key_browser.state.selected().unwrap_or(0);
                let prev = if current == 0 {
                    keys_len.saturating_sub(1)
                } else {
                    current - 1
                };
                self.key_browser.select(Some(prev));
                true
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
