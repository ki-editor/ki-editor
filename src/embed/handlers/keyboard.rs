use super::prelude::*;
use crate::app::{AppMessage, Dispatch, FromHostApp};
use crate::embed::app::EmbeddedApp;
use crossterm::event::{KeyCode, KeyModifiers as CrosstermKeyModifiers};
use event::event::Event;

impl EmbeddedApp {
    fn parse_keyboard_input(
        keyboard_input: ki_protocol_types::KeyboardParams,
    ) -> event::event::KeyEvent {
        // Map string keys to crossterm::event::KeyCode
        let code = match keyboard_input.key.as_str() {
            "ArrowUp" => KeyCode::Up,
            "ArrowDown" => KeyCode::Down,
            "ArrowLeft" => KeyCode::Left,
            "ArrowRight" => KeyCode::Right,
            "Enter" => KeyCode::Enter,
            "Escape" => KeyCode::Esc,
            "Backspace" => KeyCode::Backspace,
            "Delete" => KeyCode::Delete,
            "Tab" => KeyCode::Tab,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            "PageUp" => KeyCode::PageUp,
            "PageDown" => KeyCode::PageDown,
            // Handle single characters
            s if s.chars().count() == 1 => KeyCode::Char(s.chars().next().unwrap_or('?')),
            // TODO: Add more mappings (F-keys, etc.)
            _ => KeyCode::Null,
        };

        // Determine modifiers (basic)
        let crossterm_modifiers = CrosstermKeyModifiers::empty();

        // Start empty
        // This is a simplification; real modifier state might be complex.
        // If Host app sends modifier state explicitly, use that instead.
        // For now, we infer based on common prefixes if needed, but ideally
        // the `keyboard_input` struct would have explicit modifier fields.

        // Map crossterm modifiers to event::KeyModifiers enum
        let event_modifiers = match code {
            KeyCode::Char(c) if c.is_ascii_uppercase() => event::KeyModifiers::Shift,
            _ => {
                if crossterm_modifiers.contains(CrosstermKeyModifiers::SHIFT) {
                    event::KeyModifiers::Shift
                } else if crossterm_modifiers.contains(CrosstermKeyModifiers::CONTROL) {
                    event::KeyModifiers::Ctrl
                } else if crossterm_modifiers.contains(CrosstermKeyModifiers::ALT) {
                    event::KeyModifiers::Alt
                } else {
                    event::KeyModifiers::None
                }
            }
        };

        event::KeyEvent::new(code, event_modifiers)
    }

    /// Handle keyboard.input request
    pub(crate) fn handle_keyboard_input_request(
        &self,
        id: u32,
        params: ki_protocol_types::KeyboardParams,
        trace_id: &str,
    ) -> Result<()> {
        let path = uri_to_path(&params.uri).ok();
        let content_hash = params.content_hash;
        let event = Event::Key(Self::parse_keyboard_input(params));
        let app_message =
            AppMessage::ExternalDispatch(Dispatch::FromHostApp(FromHostApp::TargetedEvent {
                event,
                path,
                content_hash,
            }));

        // Send the parameters directly to the App thread via the new AppMessage variant
        if let Err(e) = self.app_sender.send(app_message) {
            error!("[{trace_id}] Failed to send KeyboardInput AppMessage to Core App: {e}");
            self.send_error_response(id, "Failed to process keyboard input")?;
            return Ok(());
        }
        Ok(())
    }
}
