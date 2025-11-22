use crate::backend::Backend;
use crate::types::{ConnectionConfig, KeyMetadata, Value};
use crossterm::event::KeyEvent;
use std::sync::Arc;
use tokio::sync::RwLock;

pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Quit,

    // User Inputs
    Key(KeyEvent),

    // Navigation
    NextPanel,
    PrevPanel,
    NextItem,
    PrevItem,
    Enter,
    Escape,
    ToggleHelp,

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
    SelectConnection(usize),

    // Data Actions (Intent)
    LoadKeys,
    LoadMoreKeys(usize), // index to load around
    SelectKey(usize),
    LoadValue(usize),

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

    DidLoadValue(Value),
    DidFailLoadValue(String),

    Error(String),
}
