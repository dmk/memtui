use crate::backend::{Backend, BackendCapabilities, CommandStatus};
use crate::types::{ConnectionConfig, KeyMetadata, KeyScanResult, Value};
use crate::ui::Panel;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Command line mode (vim-style bottom line)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CmdLineMode {
    #[default]
    Search, // / - fuzzy search keys
    Command, // : - execute commands
    Goto,    // g - jump to exact key
}

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
    SetCmdLineSelectionIndex(Option<usize>),
    ResetKeySelection,
    ExitCmdLineMode,
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

    // CmdLine Actions (vim-style command line)
    SetCmdLineMode(CmdLineMode), // Enter cmdline mode
    CmdLineUpdateQuery(String),  // Direct query update (for async results)
    CmdLineClear,                // Reset to normal view (Esc or empty query)
    CmdLineHide,                 // Hide cmdline without resetting selection
    CmdLineConfirm,              // Execute/confirm based on mode
    CmdLineAddChar(char),        // Insert character at cursor
    CmdLineDeleteChar,           // Backspace at cursor
    CmdLineDeleteForward,        // Delete at cursor
    CmdLineDeleteWordBack,       // Delete word backwards (Ctrl+Backspace)
    CmdLineDeleteWordForward,    // Delete word forwards (Ctrl+Delete)
    CmdLineMoveLeft,             // Move cursor left
    CmdLineMoveRight,            // Move cursor right
    CmdLineMoveWordLeft,         // Move cursor word left (Ctrl+Left)
    CmdLineMoveWordRight,        // Move cursor word right (Ctrl+Right)
    CmdLineMoveStart,            // Move cursor to start (Home)
    CmdLineMoveEnd,              // Move cursor to end (End)
    CmdLineNextResult,           // Navigate to next search result (search mode)
    CmdLinePrevResult,           // Navigate to previous search result (search mode)
    CmdLineHistoryPrev,          // Previous history entry
    CmdLineHistoryNext,          // Next history entry

    // Command results
    CmdLineSetCommandResult {
        command: String,
        output: String,
        status: CommandStatus,
    },
    CmdLineClearCommandResult,

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
            Action::SetCmdLineSelectionIndex(idx) => f
                .debug_tuple("SetCmdLineSelectionIndex")
                .field(idx)
                .finish(),
            Action::ResetKeySelection => write!(f, "ResetKeySelection"),
            Action::ExitCmdLineMode => write!(f, "ExitCmdLineMode"),
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
            Action::SetCmdLineMode(mode) => f.debug_tuple("SetCmdLineMode").field(mode).finish(),
            Action::CmdLineUpdateQuery(q) => f.debug_tuple("CmdLineUpdateQuery").field(q).finish(),
            Action::CmdLineClear => write!(f, "CmdLineClear"),
            Action::CmdLineHide => write!(f, "CmdLineHide"),
            Action::CmdLineConfirm => write!(f, "CmdLineConfirm"),
            Action::CmdLineAddChar(c) => f.debug_tuple("CmdLineAddChar").field(c).finish(),
            Action::CmdLineDeleteChar => write!(f, "CmdLineDeleteChar"),
            Action::CmdLineDeleteForward => write!(f, "CmdLineDeleteForward"),
            Action::CmdLineDeleteWordBack => write!(f, "CmdLineDeleteWordBack"),
            Action::CmdLineDeleteWordForward => write!(f, "CmdLineDeleteWordForward"),
            Action::CmdLineMoveLeft => write!(f, "CmdLineMoveLeft"),
            Action::CmdLineMoveRight => write!(f, "CmdLineMoveRight"),
            Action::CmdLineMoveWordLeft => write!(f, "CmdLineMoveWordLeft"),
            Action::CmdLineMoveWordRight => write!(f, "CmdLineMoveWordRight"),
            Action::CmdLineMoveStart => write!(f, "CmdLineMoveStart"),
            Action::CmdLineMoveEnd => write!(f, "CmdLineMoveEnd"),
            Action::CmdLineNextResult => write!(f, "CmdLineNextResult"),
            Action::CmdLinePrevResult => write!(f, "CmdLinePrevResult"),
            Action::CmdLineHistoryPrev => write!(f, "CmdLineHistoryPrev"),
            Action::CmdLineHistoryNext => write!(f, "CmdLineHistoryNext"),
            Action::CmdLineSetCommandResult {
                command,
                output,
                status,
            } => f
                .debug_struct("CmdLineSetCommandResult")
                .field("command", command)
                .field("output", output)
                .field("status", status)
                .finish(),
            Action::CmdLineClearCommandResult => write!(f, "CmdLineClearCommandResult"),
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

        // Test cmd_line category
        assert_eq!(Action::CmdLineAddChar('a').category(), Some("cmd_line"));
        assert!(Action::CmdLineNextResult.is_cmd_line());

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
        let action = Action::CmdLineAddChar('x');
        assert_eq!(action.category_enum(), super::ActionCategory::CmdLine);

        let action = Action::Tick;
        assert_eq!(action.category_enum(), super::ActionCategory::Uncategorized);
    }
}
