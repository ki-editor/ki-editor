#[derive(Clone)]
pub struct Clipboard {
    history: Vec<String>,
}

impl Clipboard {
    pub(crate) fn new() -> Clipboard {
        Clipboard { history: vec![] }
    }

    /// Get from OS clipboard when available
    pub(crate) fn get_content(&self) -> Option<String> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.get_text())
            .ok()
            .or_else(|| self.history.last().cloned())
    }

    /// Set OS clipboard when available
    pub(crate) fn set_content(&mut self, content: String) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            clipboard.set_text(content.clone()).ok();
        }
        self.history.push(content);
    }
}
