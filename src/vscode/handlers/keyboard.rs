//! Keyboard-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::app::AppMessage;
use crate::vscode::app::VSCodeApp;
use crossterm::event::{KeyCode, KeyModifiers as CrosstermKeyModifiers};
use event::event::Event;
use ki_protocol_types::OutputMessage;

impl VSCodeApp {
    fn handle_keyboard_input(
        keyboard_input: ki_protocol_types::KeyboardParams,
    ) -> event::event::KeyEvent {
        trace!("Core App processing KeyboardInput: {:?}", keyboard_input);
        // Convert ki_protocol_types::KeyboardParams to event::event::KeyEvent
        let key_event = {
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
            // TODO: Protocol might need explicit modifier info
            let crossterm_modifiers = CrosstermKeyModifiers::empty(); // Start empty
                                                                      // This is a simplification; real modifier state might be complex.
                                                                      // If VSCode sends modifier state explicitly, use that instead.
                                                                      // For now, we infer based on common prefixes if needed, but ideally
                                                                      // the `keyboard_input` struct would have explicit modifier fields.

            // Map crossterm modifiers to event::KeyModifiers enum
            let event_modifiers = if crossterm_modifiers.contains(CrosstermKeyModifiers::SHIFT) {
                event::KeyModifiers::Shift
            } else if crossterm_modifiers.contains(CrosstermKeyModifiers::CONTROL) {
                event::KeyModifiers::Ctrl
            } else if crossterm_modifiers.contains(CrosstermKeyModifiers::ALT) {
                event::KeyModifiers::Alt
            } else {
                event::KeyModifiers::None
            };

            event::KeyEvent::new(code, event_modifiers)
        };

        trace!("Converted KeyboardInput to KeyEvent: {:?}", key_event);

        key_event
        // // Now handle the converted event using the main event handler
        // app.handle_event(Event::Key(key_event))
        //     .map(|_should_quit| ()) // Map Ok(bool) to Ok(()) to discard the boolean
        //     .map_err(|e| anyhow::anyhow!("Failed to handle converted key event: {}", e))
    }
    /// Handle keyboard.input request
    pub fn handle_keyboard_input_request(
        &self,
        id: u64,
        params: ki_protocol_types::KeyboardParams,
        trace_id: &str,
    ) -> Result<()> {
        debug!("[{}] Creating Event::Key from params...", trace_id);
        let event = Event::Key(Self::handle_keyboard_input(params));
        debug!("[{}] Created Event::Key: {:?}", trace_id, event);

        debug!("[{}] Wrapping Event in AppMessage::Event...", trace_id);
        let app_message = AppMessage::Event(event);
        debug!("[{}] Created AppMessage: {:?}", trace_id, app_message);

        // --- BEGIN ADDED LOGGING ---
        trace!(
            target: "vscode_flow",
            "[{}] SENDING AppMessage: {:?} to Core App",
            trace_id,
            app_message
        );
        // --- END ADDED LOGGING ---

        // Send the parameters directly to the App thread via the new AppMessage variant
        debug!("[{}] Sending AppMessage to core app...", trace_id);
        if let Err(e) = self.app_sender.send(app_message) {
            error!(
                "[{}] Failed to send KeyboardInput AppMessage to Core App: {}",
                trace_id, e
            );
            self.send_error_response(id, "Failed to process keyboard input")?;
            return Ok(());
        }
        debug!("[{}] Successfully sent AppMessage to core app.", trace_id);

        // Acknowledge receipt immediately
        debug!(
            "[{}] Sending Success response to VSCode for ID {}...",
            trace_id, id
        );
        self.send_response(
            id,
            OutputMessage::Success(true), // Indicate the input was received
        )?;
        debug!(
            "[{}] Successfully sent Success response to VSCode.",
            trace_id
        );
        Ok(())
    }
}
