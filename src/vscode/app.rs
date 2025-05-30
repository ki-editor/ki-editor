//! VSCode integration app implementation

use crate::components::editor::{Direction, Mode};
use std::collections::HashMap;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::{App, AppMessage, Dimension, Dispatch, StatusLineComponent};
use crate::frontend::crossterm::Crossterm;
use anyhow::Result;
use ki_protocol_types::{
    BufferDiagnostics, InputMessage, MarksParams, OutputMessage, OutputMessageWrapper, PromptItem,
    PromptOpenedParams, ResponseError,
};
use log::{debug, error, info, trace};
use shared::canonicalized_path::CanonicalizedPath;

// Import the new WebSocket IPC handler
use super::ipc::WebSocketIpc;
use super::logger::VSCodeLogger;
use super::utils::*;
use crate::vscode::handlers; // Import the handlers module

/// VSCodeApp serves as a thin IPC layer between VSCode and Ki editor
pub struct VSCodeApp {
    // Core components
    pub(crate) app: Arc<Mutex<App<Crossterm>>>,
    pub(crate) app_sender: mpsc::Sender<AppMessage>, // Sender to communicate TO the App thread
    // Receiver for integration events from the core App (replaces the old notification channel)
    pub(crate) integration_event_receiver:
        mpsc::Receiver<crate::integration_event::IntegrationEvent>,
    // Add receiver for messages *to* the core App (internal queue)
    pub(crate) app_message_receiver: mpsc::Receiver<AppMessage>,
    pub(crate) ipc_handler: WebSocketIpc, // Use the WebSocket IPC handler
    pub(crate) buffer_versions: HashMap<String, u64>,
    #[allow(dead_code)]
    pub(crate) next_message_id: u64,
}

impl VSCodeApp {
    /// Create a new VSCodeApp with the given working directory
    pub fn new(working_directory: Option<CanonicalizedPath>) -> Result<Self> {
        // Set up logging
        let log_level = if std::env::var("KI_DEBUG").is_ok() {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        };
        let logger = VSCodeLogger::new(log_level);
        if let Err(e) = log::set_boxed_logger(Box::new(logger)) {
            eprintln!("Failed to initialize VSCode logger: {}", e);
        } else {
            log::set_max_level(log_level);
        }
        // Create app components
        let frontend = std::rc::Rc::new(std::sync::Mutex::new(Crossterm::new()?));
        let status_line_components = vec![
            StatusLineComponent::Mode,
            StatusLineComponent::SelectionMode,
            StatusLineComponent::LastSearchString,
            StatusLineComponent::Reveal,
            StatusLineComponent::CurrentWorkingDirectory,
            StatusLineComponent::GitBranch,
            StatusLineComponent::KeyboardLayout,
            StatusLineComponent::Help,
            StatusLineComponent::LastDispatch,
        ];

        // Create channels for VSCodeApp communication
        let (real_app_sender, real_app_receiver) = mpsc::channel::<AppMessage>();
        // Removed notification channel in favor of integration events

        // Resolve working directory once
        let resolved_wd = working_directory.unwrap_or("./".try_into()?);

        // Create the integration event channel
        let (integration_event_sender, integration_event_receiver) =
            mpsc::channel::<crate::integration_event::IntegrationEvent>();

        // Create the core App instance correctly
        let core_app = App::from_channel(
            frontend.clone(),               // Clone the Rc for shared ownership
            resolved_wd,                    // Use the resolved working directory
            real_app_sender.clone(),        // Core App gets sender to its *own* queue
            mpsc::channel().1, // Core App gets a dummy receiver it won't use in this mode
            status_line_components.clone(), // Clone the Vec
            Some(integration_event_sender), // Core App gets sender for integration events
            false,             // Disable LSP
        )?;

        // Initialize the WebSocket IPC handler
        let (ipc_handler, _port) = WebSocketIpc::new()?;
        info!("WebSocketIpc initialized. Waiting for VSCode connection...");

        Ok(Self {
            app: Arc::new(Mutex::new(core_app)),
            app_sender: real_app_sender, // Handlers use this sender
            app_message_receiver: real_app_receiver, // VSCodeApp loop polls this receiver
            integration_event_receiver,  // Receiver for integration events FROM App
            ipc_handler,
            buffer_versions: HashMap::new(),
            next_message_id: 1,
        })
    }

    /// Sends a message to VSCode via the WebSocket handler thread
    pub fn send_message_to_vscode(&self, message: OutputMessageWrapper) -> Result<()> {
        let id = message.id;
        let message_type = format!("{:?}", message.message);

        info!(
            "Queueing message for VSCode: type={} id={}",
            message_type, id
        );

        let result = self.ipc_handler.send_message_to_vscode(message);

        if let Err(ref e) = result {
            error!(
                "Failed to send message to IPC handler: type={}, id={}, error={}",
                message_type, id, e
            );
        } else {
            info!(
                "Successfully queued message for VSCode: type={}, id={}",
                message_type, id
            );
        }

        result.map_err(|e| anyhow::anyhow!("Failed to send message to IPC handler: {}", e))
    }

    /// Send a notification (no response expected)
    pub fn send_notification(&self, wrapper: OutputMessageWrapper) -> Result<()> {
        let notification_type = format!("{:?}", wrapper.message);
        trace!(
            "SENDING Notification: Type={:?}, ID={}",
            notification_type,
            wrapper.id // Although notifications often use 0, log the actual ID
        );
        self.send_message_to_vscode(wrapper)
    }

    /// Handle a request received from VSCode via the IPC handler
    fn handle_request(&mut self, id: u64, message: InputMessage, trace_id: &str) -> Result<()> {
        debug!(
            "[{}] Entering handle_request: id={}, message={:?}",
            trace_id,
            id,
            message // Use Debug format for the whole message
        );

        let start_time = std::time::Instant::now();
        let message_type = format!("{:?}", message); // Get message type name
        trace!(
            "[{}] ENTER handle_request: ID={:?}, Type={:?}",
            trace_id,
            id,
            message_type
        );

        // Translate InputMessage to App logic (Dispatch or Event)
        let result = match message {
            InputMessage::Ping(value) => {
                debug!("[{}] Processing ping request", trace_id);
                handlers::ping::handle_ping_request(self, id, value.unwrap_or_default())
            }
            InputMessage::BufferOpen(params) => {
                debug!("[{}] Processing buffer open request", trace_id);
                self.handle_buffer_open_request(id, params)
            }
            InputMessage::BufferClose(params) => {
                debug!("[{}] Processing buffer close request", trace_id);
                self.handle_buffer_close_request(id, params)
            }
            InputMessage::BufferSave(params) => {
                debug!("[{}] Processing buffer save request", trace_id);
                self.handle_buffer_save_request(id, params)
            }
            InputMessage::BufferActive(params) => {
                debug!("[{}] Processing buffer active request", trace_id);
                self.handle_buffer_active_request(id, params)
            }
            InputMessage::BufferChange(params) => {
                debug!("[{}] Processing buffer change request", trace_id);
                self.handle_buffer_change_request(params)
            }
            InputMessage::KeyboardInput(params) => {
                info!(
                    "[{}] Received keyboard.input request: key='{}'",
                    trace_id, params.key
                ); // Log with info level
                self.handle_keyboard_input_request(id, params, trace_id)
            }
            // CursorUpdate has been removed in favor of the unified SelectionSet
            InputMessage::SelectionSet(params) => {
                debug!("[{}] Processing selection set request", trace_id);
                self.handle_selection_set_request(id, params)
            }
            InputMessage::ModeSet(params) => {
                debug!("[{}] Processing mode set request", trace_id);
                self.handle_mode_set_request(id, params, trace_id)
            }
            InputMessage::SelectionModeSet(params) => {
                debug!("[{}] Processing selection mode set request", trace_id);
                self.handle_selection_mode_set_request(id, params, trace_id)
            }
            InputMessage::SearchFind(params) => {
                debug!("[{}] Processing search find request", trace_id);
                self.handle_search_find_request(id, params, trace_id)
            }
            InputMessage::ViewportChange(params) => {
                debug!("[{}] Processing viewport change request", trace_id);
                self.handle_viewport_change_request(id, params, trace_id)
            }
            InputMessage::EditorAction(params) => {
                debug!(
                    "[{}] Processing editor action request: {}",
                    trace_id, params.action
                );
                self.handle_editor_action_request(id, params, trace_id)
            }
            InputMessage::DiagnosticsChange(params) => self.handle_diagnostics_change(params),
            InputMessage::PromptEnter(entry) => self.handle_prompt_enter(entry),
        };

        let duration = start_time.elapsed();
        if let Err(ref e) = result {
            error!("[{}] Error processing request: {}", trace_id, e);
            self.send_error_response(id, &format!("Internal error: {}", e))?;
        }

        debug!("[{}] Exiting handle_request: id={}", trace_id, id);
        trace!(
            target: "vscode_flow",
            "[{}] EXIT handle_request: ID={:?}, Type={:?}, Duration={:?}, Result={}",
            trace_id,
            id,
            message_type,
            duration,
            if result.is_ok() { "Ok" } else { "Err" }
        );

        result
    }

    /// Run the VSCode integration main loop
    pub fn run(&mut self) -> Result<()> {
        trace!("Starting VSCodeApp main loop");

        // Main event loop
        loop {
            let mut received_message = false;

            // 1. Check for messages from VSCode (via WebSocket IPC handler)
            match self.ipc_handler.try_receive_from_vscode() {
                Ok((id, message, trace_id)) => {
                    received_message = true;
                    debug!(
                        "[{}] Received message from VSCode: id={}, message={:?}",
                        &trace_id,
                        id,
                        &message // Use Debug format for the whole message
                    );
                    if let Err(e) = self.handle_request(id, message, &trace_id) {
                        error!("Error handling request from VSCode: {}", e);
                        // Error response is sent within handle_request
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No message from VSCode
                }
                Err(TryRecvError::Disconnected) => {
                    error!("IPC channel from VSCode disconnected! Exiting.");
                    break;
                }
            }

            // 2. Check for internal messages FOR the core App (sent via app_sender)
            // We need to process these messages, but we'll do it in a way that avoids deadlocks
            match self.app_message_receiver.try_recv() {
                Ok(app_message) => {
                    received_message = true;
                    trace!("Received message for core App: {:?}", app_message);

                    // Check for QuitAll message
                    if let AppMessage::QuitAll = &app_message {
                        info!("Core App requested quit. Exiting.");
                        // TODO: Maybe send a shutdown message to VSCode?
                        break; // Exit the VSCodeApp loop
                    }

                    // Try to acquire the App lock without blocking
                    // This prevents deadlocks by giving up immediately if the lock is busy
                    let app_lock_result = match std::sync::Arc::clone(&self.app).try_lock() {
                        Ok(mut guard) => {
                            trace!("Successfully acquired App lock to process message");
                            // Process the message with the acquired lock
                            match guard.process_message(app_message) {
                                Ok(should_quit) => {
                                    if should_quit {
                                        info!("Core App requested quit. Exiting.");
                                        break; // Exit the VSCodeApp loop
                                    }
                                    Ok(())
                                }
                                Err(e) => {
                                    error!("Error processing message in core App: {}", e);
                                    Err(e)
                                }
                            }
                        }
                        Err(_) => {
                            // Could not acquire the lock
                            // This is not an error, it just means the App is busy
                            trace!("Could not acquire App lock to process message, will try again later");
                            Ok(())
                        }
                    };

                    if let Err(e) = app_lock_result {
                        error!("Error processing message: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No internal message for App
                }
                Err(TryRecvError::Disconnected) => {
                    error!("Internal App message channel disconnected! Exiting.");
                    break;
                }
            }

            // 3. Check for integration events FROM the core App (replaces the old notification channel)
            match self.integration_event_receiver.try_recv() {
                Ok(event) => {
                    received_message = true;
                    trace!("Received integration event from core App: {:?}", event);

                    // Process the integration event and translate it to VSCode protocol messages
                    if let Err(e) = self.handle_integration_event(event) {
                        error!("Error handling integration event: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No integration event from App
                }
                Err(TryRecvError::Disconnected) => {
                    error!("Core App integration event channel disconnected! Exiting.");
                    break;
                }
            }

            // Avoid busy-waiting if no messages were processed
            if !received_message {
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        trace!("VSCodeApp main loop stopped");
        Ok(())
    }

    /// Helper method to get the next message ID (accessible within the crate)
    #[allow(dead_code)]
    pub(crate) fn next_id(&mut self) -> u64 {
        let id = self.next_message_id;
        self.next_message_id += 1;
        id
    }

    /// Get the current file URI (accessible within the crate)
    #[allow(dead_code)]
    pub(crate) fn get_current_file_uri(&self) -> Option<String> {
        self.get_current_file_path().map(|path| path_to_uri(&path))
    }

    /// Get the current file path (accessible within the crate)
    pub(crate) fn get_current_file_path(&self) -> Option<CanonicalizedPath> {
        // Use try_lock to avoid deadlocks
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't acquire the lock, we can't get the current file path
                trace!("Could not acquire app lock to get current file path");
                return None;
            }
        };

        let component = app_guard.current_component();
        let component_ref = component.borrow();
        // Store the result in a variable to ensure borrows live long enough
        let path_opt = component_ref.editor().buffer().path().map(|p| p.clone());
        path_opt // Return the stored option
    }

    /// Find an editor component by its file path
    pub(crate) fn get_editor_component_by_path(
        &self,
        path: &CanonicalizedPath,
    ) -> Option<std::rc::Rc<std::cell::RefCell<dyn crate::components::component::Component>>> {
        // Use try_lock to avoid deadlocks
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't acquire the lock, we can't check the components
                trace!("Could not acquire app lock to check components");
                return None;
            }
        };

        // Iterate through all components to find one with matching path
        for component in app_guard.components() {
            let component_rc = component.component();
            let component_ref = component_rc.borrow();

            // Check if this component has the requested path
            if let Some(comp_path) = component_ref.path() {
                if &comp_path == path {
                    return Some(component_rc.clone());
                }
            }
        }

        None // No matching component found
    }

    /// Send a response back to VSCode for a specific request ID.
    pub(crate) fn send_response(&self, id: u64, message: OutputMessage) -> Result<()> {
        info!("Sending response for request ID {}: {:?}", id, message);
        let wrapper = OutputMessageWrapper {
            id,
            message,
            error: None,
        };
        let result = self.send_message_to_vscode(wrapper);
        if let Err(ref e) = result {
            error!("Failed to send response for request ID {}: {}", id, e);
        } else {
            info!("Successfully sent response for request ID {}", id);
        }
        result
    }

    /// Send an error response back to VSCode.
    pub(crate) fn send_error_response(&self, id: u64, error_message: &str) -> Result<()> {
        let error_response = OutputMessageWrapper {
            id,
            message: OutputMessage::Error(error_message.to_string()),
            error: Some(ResponseError {
                code: -32000, // Using a generic JSON-RPC error code
                message: error_message.to_string(),
                data: None,
            }),
        };
        self.send_message_to_vscode(error_response)
    }

    /// Handle integration events from the core App
    fn handle_integration_event(
        &self,
        event: crate::integration_event::IntegrationEvent,
    ) -> anyhow::Result<()> {
        use crate::integration_event::IntegrationEvent;

        // Translate the integration event to VSCode protocol messages
        match event {
            IntegrationEvent::BufferChanged {
                component_id: _,
                path,
                edits,
            } => self.buffer_changed(path, edits)?,
            IntegrationEvent::BufferOpened {
                component_id: _,
                path,
                language_id,
            } => self.buffer_opened(path, language_id)?,
            IntegrationEvent::BufferClosed {
                component_id: _,
                path,
            } => self.buffer_closed(path)?,
            IntegrationEvent::BufferSaved {
                component_id: _,
                path,
            } => self.buffer_saved(path)?,
            IntegrationEvent::ModeChanged { component_id, mode } => {
                self.mode_changed(component_id, mode)?
            }
            IntegrationEvent::SelectionModeChanged {
                component_id,
                selection_mode,
            } => self.selection_mode_changed(component_id, selection_mode)?,
            IntegrationEvent::JumpsChanged {
                component_id,
                jumps,
            } => self.jumps_changed(component_id, jumps)?,
            IntegrationEvent::SelectionChanged {
                component_id,
                selections,
            } => self.selection_changed(component_id, selections)?,
            IntegrationEvent::BufferActivated {
                component_id: _,
                path,
            } => self.buffer_activated(path)?,
            IntegrationEvent::ExternalBufferCreated {
                component_id: _,
                buffer_id,
                content,
            } => self.external_buffer_created(buffer_id, content)?,
            IntegrationEvent::ExternalBufferUpdated {
                component_id: _,
                buffer_id,
                content,
            } => self.external_buffer_updated(buffer_id, content)?,
            IntegrationEvent::CommandExecuted { command, success } => {
                self.command_executed(command, success)?
            }
            IntegrationEvent::PromptOpened { title, items } => self.prompt_opened(title, items)?,
            IntegrationEvent::MarksChanged {
                component_id,
                marks,
            } => self.marks_changed(component_id, marks)?,
            IntegrationEvent::RequestLspDefinition => self.request_lsp_definition()?,
        }

        Ok(())
    }

    /// Helper method to get buffer ID from component ID
    fn get_buffer_id_from_component_id(
        &self,
        component_id: crate::integration_event::ComponentId,
    ) -> Option<String> {
        // Use try_lock to avoid deadlocks
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't acquire the lock, we can't get the component
                trace!("Could not acquire app lock to get component by ID");
                return None;
            }
        };

        // Find the component by ID
        let components = app_guard.components();
        for component in components {
            let component_ref = component.component();
            let borrowed = component_ref.borrow();

            if crate::integration_event::component_id_to_usize(&borrowed.id()) == component_id {
                // Get the buffer path and use it as the buffer ID
                if let Some(path) = borrowed.path() {
                    return Some(path.display_absolute());
                }
            }
        }

        None
    }

    /// Helper method to get component by ID
    fn get_component_by_id(
        &self,
        component_id: crate::integration_event::ComponentId,
    ) -> Option<crate::ui_tree::KindedComponent> {
        // Use try_lock to avoid deadlocks
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't acquire the lock, we can't get the component
                trace!("Could not acquire app lock to get component by ID");
                return None;
            }
        };

        // Find the component by ID
        let components = app_guard.components();
        for component in components {
            let component_ref = component.component();
            let borrowed = component_ref.borrow();

            if crate::integration_event::component_id_to_usize(&borrowed.id()) == component_id {
                return Some(component.clone());
            }
        }

        None
    }

    /// Send cursor position update for the current buffer
    /// This ensures VSCode has the correct cursor position
    pub fn send_cursor_position_for_current_buffer(&self) -> Result<()> {
        // Use try_lock to avoid deadlocks
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't acquire the lock, we can't get the current component
                trace!("Could not acquire app lock to get current component");
                return Ok(());
            }
        };

        // Get the current component
        let component = app_guard.current_component();
        let component_ref = component.borrow();
        let editor = component_ref.editor();

        // Get the buffer ID
        let buffer_id = if let Some(path) = component_ref.path() {
            path.display_absolute()
        } else {
            // No path, can't send cursor update
            return Ok(());
        };

        // Get the cursor positions
        let selections = editor.selection_set.selections();
        if selections.is_empty() {
            // No selections, can't send cursor update
            return Ok(());
        }

        // Extract anchor and active positions from selections
        let mut anchors = Vec::new();
        let mut actives = Vec::new();

        for selection in selections {
            // Use extended_range() instead of range() to get the actual selection range
            // This ensures we get the correct cursor position when in word select mode
            let range = selection.extended_range();
            let buffer = editor.buffer();

            // Convert char indices to positions
            if let Ok(start_pos) = buffer.char_to_position(range.start) {
                anchors.push(start_pos);
            }

            if let Ok(end_pos) = buffer.char_to_position(range.end) {
                actives.push(end_pos);
            }
        }

        // Only send update if we have valid positions
        Ok(())
    }

    fn selection_changed(
        &self,
        component_id: usize,
        selections: Vec<crate::selection::Selection>,
    ) -> anyhow::Result<()> {
        let Some(buffer_id) = self.get_buffer_id_from_component_id(component_id) else {
            return Ok(());
        };
        let Some(component) = self.get_component_by_id(component_id) else {
            return Ok(());
        };
        let component_rc = component.component();
        let component_ref = component_rc.borrow();
        let editor = component_ref.editor();
        let buffer = editor.buffer();

        let vscode_selections = selections
            .iter()
            .map(|selection| {
                let range = selection.extended_range();

                let flipped = match &selection.initial_range {
                    Some(initial_range) => initial_range.start > selection.range().start,
                    _ => matches!(
                        component.component().borrow().editor().cursor_direction,
                        Direction::End
                    ),
                };
                let (primary_cursor, secondary_cursor) = if flipped {
                    (range.end, range.start)
                } else {
                    (range.start, range.end)
                };

                // Create VSCode selection
                use crate::components::editor::Direction;
                Ok(ki_protocol_types::Selection {
                    active: buffer
                        .char_to_position(primary_cursor)?
                        .to_vscode_position(),
                    anchor: buffer
                        .char_to_position(secondary_cursor)?
                        .to_vscode_position(),
                    is_extended: selection.initial_range.is_some(),
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Send selection update notification to VSCode
        let selection_set = ki_protocol_types::SelectionSet {
            buffer_id,
            primary: 0,
            selections: vscode_selections,
        };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::SelectionUpdate(selection_set),
            error: None,
        })
    }

    fn buffer_changed(
        &self,
        path: CanonicalizedPath,
        edits: Vec<ki_protocol_types::DiffEdit>,
    ) -> anyhow::Result<()> {
        let buffer_id = path.display_absolute();

        // Convert the transaction to buffer diffs and send to VSCode
        // This is similar to what we do in the BufferEditTransaction handler
        let Some(component) = self.get_editor_component_by_path(&path) else {
            return Ok(());
        };
        let component_ref = component.borrow();
        let editor = component_ref.editor();

        let diff_params = ki_protocol_types::BufferDiffParams { buffer_id, edits };

        // Send buffer diff notification to VSCode
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::BufferDiff(diff_params),
            error: None,
        })
    }

    fn buffer_opened(
        &self,
        path: CanonicalizedPath,
        language_id: Option<String>,
    ) -> anyhow::Result<()> {
        // Send buffer open notification to VSCode
        let uri = path_to_uri(&path);
        let params = ki_protocol_types::BufferParams {
            uri,
            content: None,
            language_id,
            version: None,
        };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::BufferOpen(params),
            error: None,
        })
    }

    fn buffer_closed(&self, path: CanonicalizedPath) -> anyhow::Result<()> {
        // Send buffer close notification to VSCode
        let uri = path_to_uri(&path);
        let params = ki_protocol_types::BufferParams {
            uri,
            content: None,
            language_id: None,
            version: None,
        };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::BufferClose(params),
            error: None,
        })
    }

    fn buffer_saved(&self, path: CanonicalizedPath) -> anyhow::Result<()> {
        // Send buffer save notification to VSCode
        let uri = path_to_uri(&path);
        let params = ki_protocol_types::BufferParams {
            uri,
            content: None,
            language_id: None,
            version: None,
        };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::BufferSave(params),
            error: None,
        })
    }

    fn mode_changed(&self, component_id: usize, mode: Mode) -> anyhow::Result<()> {
        let buffer_id = self.get_buffer_id_from_component_id(component_id);

        // Convert Ki Mode to VS Code Mode
        let editor_mode = match mode {
            Mode::Normal => ki_protocol_types::EditorMode::Normal,
            Mode::Insert => ki_protocol_types::EditorMode::Insert,
            Mode::MultiCursor => ki_protocol_types::EditorMode::MultiCursor,
            Mode::FindOneChar(_) => ki_protocol_types::EditorMode::FindOneChar,
            Mode::Swap => ki_protocol_types::EditorMode::Swap,
            Mode::Replace => ki_protocol_types::EditorMode::Replace,
            Mode::Extend => ki_protocol_types::EditorMode::Extend,
        };

        // Send mode change notification to VSCode
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::ModeChange(ki_protocol_types::TypedModeParams {
                mode: editor_mode,
                buffer_id: buffer_id.clone(),
            }),
            error: None,
        })
    }

    fn selection_mode_changed(
        &self,
        component_id: usize,
        selection_mode: crate::selection::SelectionMode,
    ) -> anyhow::Result<()> {
        let buffer_id = self.get_buffer_id_from_component_id(component_id);
        let selection_mode = match &selection_mode {
            crate::selection::SelectionMode::Character => {
                ki_protocol_types::SelectionMode::Character
            }
            crate::selection::SelectionMode::Line | crate::selection::SelectionMode::LineFull => {
                ki_protocol_types::SelectionMode::Line
            }
            crate::selection::SelectionMode::Word { skip_symbols: true } => {
                ki_protocol_types::SelectionMode::Word
            }
            crate::selection::SelectionMode::Word {
                skip_symbols: false,
            } => ki_protocol_types::SelectionMode::WordFine,
            crate::selection::SelectionMode::Token { .. } => {
                ki_protocol_types::SelectionMode::Token
            }
            crate::selection::SelectionMode::Custom => ki_protocol_types::SelectionMode::Custom,
            crate::selection::SelectionMode::Find { search } => {
                ki_protocol_types::SelectionMode::Find {
                    search: search.search.clone(),
                }
            }
            crate::selection::SelectionMode::GitHunk(_) => {
                ki_protocol_types::SelectionMode::GitHunk
            }
            crate::selection::SelectionMode::LocalQuickfix { title } => {
                ki_protocol_types::SelectionMode::LocalQuickfix
            }
            crate::selection::SelectionMode::Mark => ki_protocol_types::SelectionMode::Mark,
            crate::selection::SelectionMode::SyntaxNode => {
                ki_protocol_types::SelectionMode::SyntaxNode
            }
            crate::selection::SelectionMode::SyntaxNodeFine => {
                ki_protocol_types::SelectionMode::SyntaxNodeFine
            }
            crate::selection::SelectionMode::Diagnostic(kind) => {
                ki_protocol_types::SelectionMode::Diagnostic(match kind {
                    crate::quickfix_list::DiagnosticSeverityRange::All => {
                        ki_protocol_types::DiagnosticKind::All
                    }
                    crate::quickfix_list::DiagnosticSeverityRange::Error => {
                        ki_protocol_types::DiagnosticKind::Error
                    }
                    crate::quickfix_list::DiagnosticSeverityRange::Warning => {
                        ki_protocol_types::DiagnosticKind::Warning
                    }
                    crate::quickfix_list::DiagnosticSeverityRange::Information => {
                        ki_protocol_types::DiagnosticKind::Information
                    }
                    crate::quickfix_list::DiagnosticSeverityRange::Hint => {
                        ki_protocol_types::DiagnosticKind::Hint
                    }
                })
            }
        };

        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::SelectionModeChange(ki_protocol_types::SelectionModeParams {
                mode: selection_mode,
                buffer_id,
            }),
            error: None,
        })
    }

    fn jumps_changed(
        &self,
        component_id: usize,
        jumps: Vec<(char, crate::selection::CharIndex)>,
    ) -> anyhow::Result<()> {
        let Some(buffer_id) = self.get_buffer_id_from_component_id(component_id) else {
            return Ok(());
        };

        // Get the component to access the buffer
        let Some(component) = self.get_component_by_id(component_id) else {
            return Ok(());
        };
        // Store the component reference to extend its lifetime
        let component_rc = component.component();
        let component_ref = component_rc.borrow();
        let editor = component_ref.editor();
        let buffer = editor.buffer();

        // Convert Ki's jumps to VS Code jumps
        let jumps = jumps
            .into_iter()
            .map(|(char, char_index)| -> anyhow::Result<_> {
                Ok((
                    char,
                    buffer.char_to_position(char_index)?.to_vscode_position(),
                ))
            })
            .collect::<anyhow::Result<Vec<_>, _>>()?;

        // Get the selection mode from the editor
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::JumpsChanged(ki_protocol_types::JumpsParams {
                targets: jumps
                    .into_iter()
                    .map(|(key, position)| ki_protocol_types::JumpTarget { key, position })
                    .collect(),
            }),
            error: None,
        })
    }

    fn buffer_activated(&self, path: CanonicalizedPath) -> anyhow::Result<()> {
        // Send buffer activated notification to VSCode
        let uri = path_to_uri(&path);
        let params = ki_protocol_types::BufferParams {
            uri,
            content: None,
            language_id: None,
            version: None,
        };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::BufferActivated(params),
            error: None,
        })
    }

    fn external_buffer_created(&self, buffer_id: String, content: String) -> anyhow::Result<()> {
        // Send external buffer created notification to VSCode
        let external_buffer_params = ki_protocol_types::ExternalBufferParams { buffer_id, content };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::ExternalBufferCreated(external_buffer_params),
            error: None,
        })
    }

    fn external_buffer_updated(&self, buffer_id: String, content: String) -> anyhow::Result<()> {
        // Send external buffer updated notification to VSCode
        let external_buffer_params = ki_protocol_types::ExternalBufferParams { buffer_id, content };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::ExternalBufferUpdated(external_buffer_params),
            error: None,
        })
    }

    fn command_executed(&self, command: String, success: bool) -> anyhow::Result<()> {
        // Send command executed notification to VSCode
        let command_params = ki_protocol_types::CommandParams {
            name: command,
            args: Vec::new(), // We don't have args in the event currently
            success: Some(success),
        };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::CommandExecuted(command_params),
            error: None,
        })
    }

    fn handle_prompt_enter(&self, entry: String) -> std::result::Result<(), anyhow::Error> {
        let mut app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't acquire the lock, we can't get the current component
                trace!("Could not acquire app lock to get current component");
                return Ok(());
            }
        };
        app_guard.handle_dispatch(Dispatch::PromptEntered(entry))?;
        Ok(())
    }

    fn handle_diagnostics_change(
        &self,
        buffer_diagnosticss: Vec<BufferDiagnostics>,
    ) -> anyhow::Result<()> {
        let mut app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't acquire the lock, we can't get the current component
                trace!("Could not acquire app lock to get current component");
                return Ok(());
            }
        };
        for buffer_diagnostics in buffer_diagnosticss {
            let path = CanonicalizedPath::try_from(buffer_diagnostics.path)?;
            let diagnostics = buffer_diagnostics
                .diagnostics
                .into_iter()
                .map(|diagnostic| lsp_types::Diagnostic {
                    range: lsp_types::Range {
                        start: lsp_types::Position::new(
                            diagnostic.range.start.line as u32,
                            diagnostic.range.start.character as u32,
                        ),
                        end: lsp_types::Position::new(
                            diagnostic.range.end.line as u32,
                            diagnostic.range.end.character as u32,
                        ),
                    },
                    severity: diagnostic.severity.map(|severity| match severity {
                        ki_protocol_types::DiagnosticSeverity::Warning => {
                            lsp_types::DiagnosticSeverity::WARNING
                        }
                        ki_protocol_types::DiagnosticSeverity::Hint => {
                            lsp_types::DiagnosticSeverity::HINT
                        }
                        ki_protocol_types::DiagnosticSeverity::Information => {
                            lsp_types::DiagnosticSeverity::INFORMATION
                        }
                        ki_protocol_types::DiagnosticSeverity::Error => {
                            lsp_types::DiagnosticSeverity::ERROR
                        }
                    }),
                    message: diagnostic.message,
                    ..Default::default()
                })
                .collect();
            app_guard.update_diagnostics(path, diagnostics)?;
        }
        Ok(())
    }

    fn prompt_opened(
        &self,
        title: String,
        items: Vec<ki_protocol_types::PromptItem>,
    ) -> anyhow::Result<()> {
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::PromptOpened(PromptOpenedParams { title, items }),
            error: None,
        })
    }

    fn marks_changed(
        &self,
        component_id: usize,
        marks: Vec<crate::char_index_range::CharIndexRange>,
    ) -> anyhow::Result<()> {
        let Some(buffer_id) = self.get_buffer_id_from_component_id(component_id) else {
            return Ok(());
        };

        // Get the component to access the buffer
        let Some(component) = self.get_component_by_id(component_id) else {
            return Ok(());
        };
        // Store the component reference to extend its lifetime
        let component_rc = component.component();
        let component_ref = component_rc.borrow();
        let editor = component_ref.editor();
        let buffer = editor.buffer();
        let marks = marks
            .into_iter()
            .map(|range| -> anyhow::Result<_> {
                let std::ops::Range { start, end } =
                    buffer.char_index_range_to_position_range(range)?;
                Ok(ki_protocol_types::Range {
                    start: start.to_vscode_position(),
                    end: end.to_vscode_position(),
                })
            })
            .collect::<anyhow::Result<Vec<_>, _>>()?;
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::MarksChanged(MarksParams { marks }),
            error: None,
        })
    }

    fn request_lsp_definition(&self) -> anyhow::Result<()> {
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::RequestLspDefinition,
            error: None,
        })
    }
}

/// Run the VSCode integration
pub fn run_vscode(working_directory: CanonicalizedPath) -> anyhow::Result<()> {
    // TODO: handle cwd from VS Code
    eprintln!("== VSCode integration initializing ==");

    // Initialize and run the VSCode integration
    let mut vscode_app = VSCodeApp::new(Some(working_directory))?;

    // Note: Port number is printed within WebSocketIpc::new()
    eprintln!("VSCode integration backend started. Waiting for VSCode extension to connect...");
    info!("VSCode integration backend started. Waiting for VSCode extension to connect...");

    // Run the main loop
    vscode_app.run()
}
