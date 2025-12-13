use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use std::io::{self, Write};

/// Enable/disable mouse capture.
///
/// When disabled, most terminals allow normal mouse selection (copy/paste).
pub fn set_mouse_capture(enabled: bool) -> io::Result<()> {
    let mut stdout = io::stdout();
    if enabled {
        execute!(stdout, EnableMouseCapture)?;
    } else {
        execute!(stdout, DisableMouseCapture)?;
    }
    stdout.flush()
}
