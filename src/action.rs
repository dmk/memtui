use crate::backend::{Backend, BackendCapabilities};
use crate::types::{ConnectionConfig, KeyMetadata, KeyScanResult, Value};
use crate::ui::Panel;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, tui_dispatch::Action)]
#[action(infer_categories, generate_dispatcher)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Quit,
    ShowQuitConfirmation,
    ConfirmQuit,
    CancelQuit,

    // Navigation
    NextPanel,
    PrevPanel,
    FocusPanel(Panel),
    NextItem,
    PrevItem,
    CycleValueViewMode,
    Enter,
    Escape,
    ToggleHelp,
    OpenConnectionPalette,
    CloseConnectionPalette,
    NextConnectionTab,
    PrevConnectionTab,
    SelectConnectionIndex(usize),
    SelectWelcomeIndex(usize),

    // UI State
    SetPaneRatio(f32),
    SetSearchSelectionIndex(Option<usize>),
    ResetKeySelection,
    ExitSearchMode,
    StartResize,
    EndResize,

    // Connection Form
    OpenConnectionForm,
    CloseConnectionForm,
    SubmitConnectionForm(ConnectionConfig),
    ConnectionFormNextField,
    ConnectionFormPrevField,
    ConnectionFormAddChar(char),
    ConnectionFormDeleteChar,
    ConnectionFormDeleteForward,
    ConnectionFormDeleteWordBack,
    ConnectionFormDeleteWordForward,
    ConnectionFormMoveLeft,
    ConnectionFormMoveRight,
    ConnectionFormMoveWordLeft,
    ConnectionFormMoveWordRight,
    ConnectionFormMoveStart,
    ConnectionFormMoveEnd,
    ConnectionFormNextBackendType,
    ConnectionFormPrevBackendType,

    // Value Viewer
    ValueViewerScrollUp,
    ValueViewerScrollDown,
    ValueViewerScrollBy(i16),

    // Connection List (palette)
    ConnectionListNext,
    ConnectionListPrev,

    // Welcome Screen
    WelcomeNextItem,
    WelcomePrevItem,

    // Connection State
    SetActiveConnection(String),

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
    ConfirmSearch,             // Confirm search (exit input mode, keep selection)
    SearchAddChar(char),       // Add character to search input
    SearchDeleteChar,          // Delete character from search input
    SearchNextResult,          // Navigate to next search result
    SearchPrevResult,          // Navigate to previous search result

    // Async Events (Results)
    DidConnect(String, Arc<RwLock<Box<dyn Backend>>>, BackendCapabilities),
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

    // Debug
    ToggleDebug,
    DebugCopyFrame,
    DebugToggleStateView,
    DebugToggleMouseCapture,

    Error(String),
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Simple variants
            Action::Tick => write!(f, "Tick"),
            Action::Render => write!(f, "Render"),
            Action::Resize(w, h) => f.debug_tuple("Resize").field(w).field(h).finish(),
            Action::Quit => write!(f, "Quit"),
            Action::ShowQuitConfirmation => write!(f, "ShowQuitConfirmation"),
            Action::ConfirmQuit => write!(f, "ConfirmQuit"),
            Action::CancelQuit => write!(f, "CancelQuit"),
            Action::NextPanel => write!(f, "NextPanel"),
            Action::PrevPanel => write!(f, "PrevPanel"),
            Action::FocusPanel(panel) => f.debug_tuple("FocusPanel").field(panel).finish(),
            Action::NextItem => write!(f, "NextItem"),
            Action::PrevItem => write!(f, "PrevItem"),
            Action::CycleValueViewMode => write!(f, "CycleValueViewMode"),
            Action::Enter => write!(f, "Enter"),
            Action::Escape => write!(f, "Escape"),
            Action::ToggleHelp => write!(f, "ToggleHelp"),
            Action::OpenConnectionPalette => write!(f, "OpenConnectionPalette"),
            Action::CloseConnectionPalette => write!(f, "CloseConnectionPalette"),
            Action::NextConnectionTab => write!(f, "NextConnectionTab"),
            Action::PrevConnectionTab => write!(f, "PrevConnectionTab"),
            Action::SelectConnectionIndex(idx) => {
                f.debug_tuple("SelectConnectionIndex").field(idx).finish()
            }
            Action::SelectWelcomeIndex(idx) => {
                f.debug_tuple("SelectWelcomeIndex").field(idx).finish()
            }
            Action::SetPaneRatio(ratio) => f.debug_tuple("SetPaneRatio").field(ratio).finish(),
            Action::SetSearchSelectionIndex(idx) => {
                f.debug_tuple("SetSearchSelectionIndex").field(idx).finish()
            }
            Action::ResetKeySelection => write!(f, "ResetKeySelection"),
            Action::ExitSearchMode => write!(f, "ExitSearchMode"),
            Action::StartResize => write!(f, "StartResize"),
            Action::EndResize => write!(f, "EndResize"),
            Action::OpenConnectionForm => write!(f, "OpenConnectionForm"),
            Action::CloseConnectionForm => write!(f, "CloseConnectionForm"),
            Action::SubmitConnectionForm(cfg) => f
                .debug_tuple("SubmitConnectionForm")
                .field(&cfg.name)
                .finish(),
            Action::ConnectionFormNextField => write!(f, "ConnectionFormNextField"),
            Action::ConnectionFormPrevField => write!(f, "ConnectionFormPrevField"),
            Action::ConnectionFormAddChar(c) => {
                f.debug_tuple("ConnectionFormAddChar").field(c).finish()
            }
            Action::ConnectionFormDeleteChar => write!(f, "ConnectionFormDeleteChar"),
            Action::ConnectionFormDeleteForward => write!(f, "ConnectionFormDeleteForward"),
            Action::ConnectionFormDeleteWordBack => write!(f, "ConnectionFormDeleteWordBack"),
            Action::ConnectionFormDeleteWordForward => write!(f, "ConnectionFormDeleteWordForward"),
            Action::ConnectionFormMoveLeft => write!(f, "ConnectionFormMoveLeft"),
            Action::ConnectionFormMoveRight => write!(f, "ConnectionFormMoveRight"),
            Action::ConnectionFormMoveWordLeft => write!(f, "ConnectionFormMoveWordLeft"),
            Action::ConnectionFormMoveWordRight => write!(f, "ConnectionFormMoveWordRight"),
            Action::ConnectionFormMoveStart => write!(f, "ConnectionFormMoveStart"),
            Action::ConnectionFormMoveEnd => write!(f, "ConnectionFormMoveEnd"),
            Action::ConnectionFormNextBackendType => write!(f, "ConnectionFormNextBackendType"),
            Action::ConnectionFormPrevBackendType => write!(f, "ConnectionFormPrevBackendType"),
            Action::ValueViewerScrollUp => write!(f, "ValueViewerScrollUp"),
            Action::ValueViewerScrollDown => write!(f, "ValueViewerScrollDown"),
            Action::ValueViewerScrollBy(delta) => {
                f.debug_tuple("ValueViewerScrollBy").field(delta).finish()
            }
            Action::ConnectionListNext => write!(f, "ConnectionListNext"),
            Action::ConnectionListPrev => write!(f, "ConnectionListPrev"),
            Action::WelcomeNextItem => write!(f, "WelcomeNextItem"),
            Action::WelcomePrevItem => write!(f, "WelcomePrevItem"),
            Action::SetActiveConnection(id) => {
                f.debug_tuple("SetActiveConnection").field(id).finish()
            }
            Action::Connect(id) => f.debug_tuple("Connect").field(id).finish(),
            Action::Disconnect(id) => f.debug_tuple("Disconnect").field(id).finish(),
            Action::DeleteConnection(id) => f.debug_tuple("DeleteConnection").field(id).finish(),
            Action::FocusConnection(id) => f.debug_tuple("FocusConnection").field(id).finish(),
            Action::LoadKeys => write!(f, "LoadKeys"),
            Action::LoadMoreKeys(idx) => f.debug_tuple("LoadMoreKeys").field(idx).finish(),
            Action::SelectKey(idx) => f.debug_tuple("SelectKey").field(idx).finish(),
            Action::LoadValueDebounced { index, token } => f
                .debug_struct("LoadValueDebounced")
                .field("index", index)
                .field("token", token)
                .finish(),
            Action::LoadValue { index, token } => f
                .debug_struct("LoadValue")
                .field("index", index)
                .field("token", token)
                .finish(),
            Action::StartSearch => write!(f, "StartSearch"),
            Action::UpdateSearchQuery(q) => f.debug_tuple("UpdateSearchQuery").field(q).finish(),
            Action::ClearSearch => write!(f, "ClearSearch"),
            Action::ConfirmSearch => write!(f, "ConfirmSearch"),
            Action::SearchAddChar(c) => f.debug_tuple("SearchAddChar").field(c).finish(),
            Action::SearchDeleteChar => write!(f, "SearchDeleteChar"),
            Action::SearchNextResult => write!(f, "SearchNextResult"),
            Action::SearchPrevResult => write!(f, "SearchPrevResult"),
            // Async results - summarize large data
            Action::DidConnect(id, _, caps) => f
                .debug_struct("DidConnect")
                .field("id", id)
                .field("caps", caps)
                .finish(),
            Action::DidDisconnect(id) => f.debug_tuple("DidDisconnect").field(id).finish(),
            Action::DidFailConnect(id, err) => f
                .debug_tuple("DidFailConnect")
                .field(id)
                .field(err)
                .finish(),
            Action::DidScanKeys {
                keys,
                cursor,
                has_more,
                total_count,
                reset,
                center,
            } => f
                .debug_struct("DidScanKeys")
                .field("keys_count", &keys.len())
                .field("cursor", cursor)
                .field("has_more", has_more)
                .field("total_count", total_count)
                .field("reset", reset)
                .field("center", center)
                .finish(),
            Action::DidFailScanKeys(err) => f.debug_tuple("DidFailScanKeys").field(err).finish(),
            Action::DidLoadValue { value, token } => f
                .debug_struct("DidLoadValue")
                .field("type", &value.value_type)
                .field("bytes", &value.data.len())
                .field("token", token)
                .finish(),
            Action::DidFailLoadValue(err) => f.debug_tuple("DidFailLoadValue").field(err).finish(),
            Action::DidSearchLocal {
                indices,
                match_positions,
                token,
            } => f
                .debug_struct("DidSearchLocal")
                .field("results", &indices.len())
                .field("highlighted", &match_positions.len())
                .field("token", token)
                .finish(),
            Action::DidSearchServer { result, token } => f
                .debug_struct("DidSearchServer")
                .field("keys", &result.keys.len())
                .field("token", token)
                .finish(),
            Action::ToggleDebug => write!(f, "ToggleDebug"),
            Action::DebugCopyFrame => write!(f, "DebugCopyFrame"),
            Action::DebugToggleStateView => write!(f, "DebugToggleStateView"),
            Action::DebugToggleMouseCapture => write!(f, "DebugToggleMouseCapture"),
            Action::Error(err) => f.debug_tuple("Error").field(err).finish(),
        }
    }
}

#[cfg(test)]
mod category_tests {
    use super::*;

    #[test]
    fn test_category_inference() {
        // Test connection_form category
        assert_eq!(
            Action::ConnectionFormNextField.category(),
            Some("connection_form")
        );
        assert_eq!(
            Action::ConnectionFormAddChar('x').category(),
            Some("connection_form")
        );
        assert!(Action::ConnectionFormNextField.is_connection_form());

        // Test value_viewer category
        assert_eq!(Action::ValueViewerScrollUp.category(), Some("value_viewer"));
        assert!(Action::ValueViewerScrollDown.is_value_viewer());

        // Test search category
        assert_eq!(Action::SearchAddChar('a').category(), Some("search"));
        assert!(Action::SearchNextResult.is_search());

        // Test async_result category (Did* prefix)
        assert_eq!(
            Action::DidDisconnect("test".into()).category(),
            Some("async_result")
        );
        assert!(Action::DidFailConnect("id".into(), "error".into()).is_async_result());

        // Test connection_list category
        assert_eq!(
            Action::ConnectionListNext.category(),
            Some("connection_list")
        );
        assert!(Action::ConnectionListPrev.is_connection_list());

        // Test welcome category
        assert_eq!(Action::WelcomeNextItem.category(), Some("welcome"));

        // Test debug category
        assert_eq!(Action::DebugCopyFrame.category(), Some("debug"));
        assert!(Action::DebugToggleStateView.is_debug());

        // Uncategorized actions
        assert_eq!(Action::Tick.category(), None);
        assert_eq!(Action::Quit.category(), None);
        assert_eq!(Action::NextItem.category(), None);
    }

    #[test]
    fn test_category_enum() {
        // super::ActionCategory is the generated enum (not the trait)
        let action = Action::SearchAddChar('x');
        assert_eq!(action.category_enum(), super::ActionCategory::Search);

        let action = Action::Tick;
        assert_eq!(action.category_enum(), super::ActionCategory::Uncategorized);
    }
}
