use super::ConnectionForm;
use super::components::connection_list::ConnectionList;
use super::components::key_browser::KeyBrowser;
use super::components::value_viewer::ValueViewer;
use ratatui::layout::Rect;
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum Panel {
    Keys,
    Value,
}

#[derive(Debug, Clone)]
pub struct TabRegion {
    pub id: String,
    pub area: Rect,
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
    pub last_key_area: Option<Rect>,
    pub last_value_area: Option<Rect>,
    pub tab_regions: Vec<TabRegion>,
    pub tab_bar_area: Option<Rect>,
    pub show_connection_palette: bool,
    pub connection_palette_area: Option<Rect>,
    pub recent_connection_ids: Vec<String>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            active_panel: Panel::Keys,
            show_help: false,
            show_connection_form: false,
            connection_form: ConnectionForm::new(),
            form_error: None,

            connection_list: ConnectionList::new(),
            key_browser: KeyBrowser::new(),
            value_viewer: ValueViewer::new(),
            last_key_area: None,
            last_value_area: None,
            tab_regions: Vec::new(),
            tab_bar_area: None,
            show_connection_palette: false,
            connection_palette_area: None,
            recent_connection_ids: Vec::new(),
        }
    }

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

    pub fn open_connection_palette(&mut self) {
        self.show_connection_palette = true;
    }

    pub fn close_connection_palette(&mut self) {
        self.show_connection_palette = false;
        self.connection_palette_area = None;
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

    pub fn next_item(&mut self, keys_len: usize) -> bool {
        match self.active_panel {
            Panel::Keys => {
                if keys_len == 0 {
                    self.key_browser.select(None);
                    return false;
                }
                let current = self.key_browser.state.selected().unwrap_or(0);
                let next = if current >= keys_len.saturating_sub(1) {
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

    pub fn previous_item(&mut self, keys_len: usize) -> bool {
        match self.active_panel {
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
