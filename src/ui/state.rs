use super::components::connection_list::ConnectionList;
use super::components::key_browser::KeyBrowser;
use super::components::value_viewer::ValueViewer;
use super::components::welcome::WelcomeScreen;
use super::theme::{AnimationState, PaneSplit};
use super::ConnectionForm;
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
    pub welcome_screen: WelcomeScreen,
    pub last_key_area: Option<Rect>,
    pub last_value_area: Option<Rect>,
    pub tab_regions: Vec<TabRegion>,
    pub tab_bar_area: Option<Rect>,
    pub show_connection_palette: bool,
    pub connection_palette_area: Option<Rect>,
    pub recent_connection_ids: Vec<String>,
    pub show_quit_confirmation: bool,

    // 2026 UI Enhancements
    /// Animation state for time-based effects
    pub animation: AnimationState,
    /// Resizable pane split ratio
    pub pane_split: PaneSplit,
    /// Is the user currently resizing panes
    pub is_resizing: bool,
    /// Last body area for resize calculations
    pub last_body_area: Option<Rect>,
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
            welcome_screen: WelcomeScreen::new(),
            last_key_area: None,
            last_value_area: None,
            tab_regions: Vec::new(),
            tab_bar_area: None,
            show_connection_palette: false,
            connection_palette_area: None,
            recent_connection_ids: Vec::new(),
            show_quit_confirmation: false,

            // 2026 UI
            animation: AnimationState::new(),
            pane_split: PaneSplit::default(),
            is_resizing: false,
            last_body_area: None,
        }
    }

    /// Adjust pane split ratio for resizing
    pub fn resize_panes(&mut self, delta: f32) {
        self.pane_split.adjust(delta);
    }

    /// Start pane resize mode
    pub fn start_resize(&mut self) {
        self.is_resizing = true;
    }

    /// End pane resize mode
    pub fn end_resize(&mut self) {
        self.is_resizing = false;
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
        if cycle.any(|p| p == self.active_panel) {
            if let Some(next) = cycle.next() {
                self.active_panel = next;
            }
        }
    }

    pub fn prev_panel(&mut self) {
        let mut cycle = Panel::iter().rev().cycle();
        if cycle.any(|p| p == self.active_panel) {
            if let Some(next) = cycle.next() {
                self.active_panel = next;
            }
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
            Panel::Value => {
                self.value_viewer.scroll_down();
                true
            }
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
            Panel::Value => {
                self.value_viewer.scroll_up();
                true
            }
        }
    }

    /// Scroll key browser by a delta amount (positive = down, negative = up)
    /// More efficient than calling next_item/previous_item in a loop
    /// Matches the wrapping behavior of next_item/previous_item
    pub fn scroll_keys_by(&mut self, keys_len: usize, delta: isize) -> bool {
        if keys_len == 0 {
            self.key_browser.select(None);
            return false;
        }

        let current = self.key_browser.state.selected().unwrap_or(0);
        let new_index = if delta > 0 {
            // Scroll down - matches next_item behavior
            let delta_u = delta as usize;
            let new = current + delta_u;
            if new >= keys_len {
                // Wrap around: for each full cycle, wrap to beginning
                new % keys_len
            } else {
                new
            }
        } else {
            // Scroll up - matches previous_item behavior
            let delta_u = (-delta) as usize;
            if delta_u > current {
                // Need to wrap around from the top
                // Calculate remainder after wrapping
                let remainder = (delta_u - current - 1) % keys_len;
                keys_len - remainder - 1
            } else {
                current - delta_u
            }
        };

        self.key_browser.select(Some(new_index));
        true
    }

    /// Scroll value viewer by a delta amount (positive = down, negative = up)
    pub fn scroll_value_by(&mut self, delta: isize) -> bool {
        self.value_viewer.scroll_by(delta);
        true
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
