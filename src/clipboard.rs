use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use std::io::{self, Write};

/// Copy text to the terminal clipboard using OSC52.
///
/// This requires terminal/emulator support for OSC52.
pub fn osc52_copy(text: &str) -> io::Result<()> {
    let payload = STANDARD.encode(text.as_bytes());
    let seq = format!("\x1b]52;c;{}\x07", payload);
    let mut stdout = io::stdout();
    stdout.write_all(seq.as_bytes())?;
    stdout.flush()
}
