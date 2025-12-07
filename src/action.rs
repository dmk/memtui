use crate::backend::Backend;
use crate::types::{ConnectionConfig, KeyMetadata, KeyScanResult, Value};
use crossterm::event::{KeyEvent, MouseEvent};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Quit,
    ShowQuitConfirmation,
    ConfirmQuit,
    CancelQuit,

    // User Inputs
    Key(KeyEvent),
    Mouse(MouseEvent),
    Scroll {
        column: u16,
        row: u16,
        delta: isize,
    },

    // Navigation
    NextPanel,
    PrevPanel,
    NextItem,
    PrevItem,
    Enter,
    Escape,
    ToggleHelp,
    OpenConnectionPalette,
    CloseConnectionPalette,
    NextConnectionTab,
    PrevConnectionTab,

    // Connection Form
    OpenConnectionForm,
    CloseConnectionForm,
    SubmitConnectionForm(ConnectionConfig),
    ConnectionFormNextField,
    ConnectionFormPrevField,
    ConnectionFormAddChar(char),
    ConnectionFormDeleteChar,

    // Connection Actions (Intent)
    Connect(String),
    Disconnect(String),
    DeleteConnection(String),
    FocusConnection(String),

    // Data Actions (Intent)
    LoadKeys,
    LoadMoreKeys(usize), // index to load around
    SelectKey(usize),
    LoadValueDebounced {
        index: usize,
        token: u64,
    },
    LoadValue {
        index: usize,
        token: u64,
    },

    // Search Actions
    StartSearch,               // Open search input
    UpdateSearchQuery(String), // User typed in search
    ClearSearch,               // Reset to normal view (Esc or empty query)
    SearchAddChar(char),       // Add character to search input
    SearchDeleteChar,          // Delete character from search input

    // Async Events (Results)
    DidConnect(String, Arc<RwLock<Box<dyn Backend>>>),
    DidDisconnect(String),
    DidFailConnect(String, String), // id, error

    DidScanKeys {
        keys: Vec<KeyMetadata>,
        cursor: Option<String>,
        has_more: bool,
        total_count: Option<u64>,
        reset: bool,           // true if this is a fresh load (clearing previous keys)
        center: Option<usize>, // if loading more, this is the center of attention
    },
    DidFailScanKeys(String),

    DidLoadValue {
        value: Value,
        token: u64,
    },
    DidFailLoadValue(String),

    // Search async results
    DidSearchLocal {
        indices: Vec<usize>, // Indices of matching keys in loaded keys array
        match_positions: HashMap<usize, Vec<u32>>, // Match positions for highlighting (key index -> char positions)
        token: u64,
    },
    DidSearchServer {
        result: KeyScanResult,
        token: u64,
    },

    Error(String),
}
