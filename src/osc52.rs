use base64::{engine::general_purpose, Engine as _};
use std::io::{self, Write};

pub struct ClipboardContext;

impl ClipboardContext {
    pub fn new() -> Self {
        ClipboardContext
    }

    pub fn set_contents(&self, content: &str) -> io::Result<()> {
        let base64_content = general_purpose::STANDARD.encode(content);
        let osc52 = format!("\x1b]52;c;{}\x07", base64_content);
        io::stdout().write_all(osc52.as_bytes())?;
        io::stdout().flush()?;
        Ok(())
    }
}

pub fn copy_to_clipboard(content: &str) -> io::Result<()> {
    ClipboardContext::new().set_contents(content)
}
