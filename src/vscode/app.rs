//! VSCode integration app implementation

use crate::components::editor::Direction;
use std::collections::HashMap;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::{App, AppMessage, StatusLineComponent};
use crate::frontend::crossterm::Crossterm;
use anyhow::Result;
use ki_protocol_types::{InputMessage, OutputMessage, OutputMessageWrapper, ResponseError};
use log::{debug, error, info, trace};
use shared::canonicalized_path::CanonicalizedPath;

// Import the new WebSocket IPC handler
use super::ipc::WebSocketIpc;
use super::logger::VSCodeLogger;
use super::utils::*;
use crate::vscode::handlers; // Import the handlers module

/// VSCodeApp serves as a thin IPC layer between VSCode and Ki editor
pub struct VSCodeApp {
    /// Suppression flag to prevent feedback loop when applying backend-driven cursor updates
    pub suppress_next_cursor_update: bool,
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
    pub(crate) last_vscode_selection: Option<ki_protocol_types::SelectionSet>,
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
        let resolved_wd = working_directory.unwrap_or(".".try_into()?);

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
        )?;

        // Initialize the WebSocket IPC handler
        let (ipc_handler, _port) = WebSocketIpc::new()?;
        info!("WebSocketIpc initialized. Waiting for VSCode connection...");

        Ok(Self {
            suppress_next_cursor_update: false,
            app: Arc::new(Mutex::new(core_app)),
            app_sender: real_app_sender, // Handlers use this sender
            app_message_receiver: real_app_receiver, // VSCodeApp loop polls this receiver
            integration_event_receiver,  // Receiver for integration events FROM App
            ipc_handler,
            buffer_versions: HashMap::new(),
            next_message_id: 1,
            last_vscode_selection: None,
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
                self.handle_buffer_change_request(id, params)
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
    ) -> Result<()> {
        use crate::integration_event::IntegrationEvent;

        // Translate the integration event to VSCode protocol messages
        match event {
            IntegrationEvent::BufferChanged {
                component_id: _,
                path,
                edits,
            } => {
                // Extract edits from the transaction
                let buffer_id = path.display_absolute();

                // Convert the transaction to buffer diffs and send to VSCode
                // This is similar to what we do in the BufferEditTransaction handler
                if let Some(component) = self.get_editor_component_by_path(&path) {
                    let component_ref = component.borrow();
                    let editor = component_ref.editor();

                    if !edits.is_empty() {
                        let diff_params = ki_protocol_types::BufferDiffParams { buffer_id, edits };

                        // Log the transaction details for debugging
                        trace!(
                            "Sending buffer diff from integration event: {:?}",
                            diff_params
                        );

                        // Send buffer diff notification to VSCode
                        self.send_notification(OutputMessageWrapper {
                            id: 0,
                            message: OutputMessage::BufferDiff(diff_params),
                            error: None,
                        })?;
                    }
                }
            }
            IntegrationEvent::BufferOpened {
                component_id: _,
                path,
                language_id,
            } => {
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
                })?;
            }
            IntegrationEvent::BufferClosed {
                component_id: _,
                path,
            } => {
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
                })?;
            }
            IntegrationEvent::BufferSaved {
                component_id: _,
                path,
            } => {
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
                })?;
            }
            IntegrationEvent::ModeChanged {
                component_id,
                mode,
                selection_mode,
            } => {
                // Get buffer ID if needed
                let buffer_id = self.get_buffer_id_from_component_id(component_id);

                // Convert string mode to EditorMode enum
                let editor_mode = match mode.as_str() {
                    "Normal" => ki_protocol_types::EditorMode::Normal,
                    "Insert" => ki_protocol_types::EditorMode::Insert,
                    "MultiCursor" => ki_protocol_types::EditorMode::MultiCursor,
                    s if s.starts_with("FindOneChar") => ki_protocol_types::EditorMode::FindOneChar,
                    "Swap" => ki_protocol_types::EditorMode::Swap,
                    "Replace" => ki_protocol_types::EditorMode::Replace,
                    "Extend" => ki_protocol_types::EditorMode::Extend,
                    _ => ki_protocol_types::EditorMode::Normal, // Default to Normal for unknown modes
                };

                // Convert SelectionMode to protocol SelectionMode enum
                let protocol_selection_mode = match selection_mode {
                    crate::selection::SelectionMode::Character => {
                        ki_protocol_types::SelectionMode::Character
                    }
                    crate::selection::SelectionMode::Line
                    | crate::selection::SelectionMode::LineFull => {
                        ki_protocol_types::SelectionMode::Line
                    }
                    _ => {
                        // Default to Character for other modes
                        ki_protocol_types::SelectionMode::Character
                    }
                };

                // Send mode change notification to VSCode
                let mode_params = ki_protocol_types::TypedModeParams {
                    mode: editor_mode,
                    buffer_id: buffer_id.clone(),
                };
                self.send_notification(OutputMessageWrapper {
                    id: 0,
                    message: OutputMessage::ModeChange(mode_params),
                    error: None,
                })?;

                // Send selection mode change notification to VSCode
                let selection_mode_params = ki_protocol_types::SelectionModeParams {
                    mode: protocol_selection_mode,
                    buffer_id,
                };
                self.send_notification(OutputMessageWrapper {
                    id: 0,
                    message: OutputMessage::SelectionModeChange(selection_mode_params),
                    error: None,
                })?;
            }
            IntegrationEvent::SelectionChanged {
                component_id,
                selections,
            } => {
                // Get the buffer ID from the component
                if let Some(buffer_id) = self.get_buffer_id_from_component_id(component_id) {
                    // Get the component to access the buffer
                    if let Some(component) = self.get_component_by_id(component_id) {
                        // Store the component reference to extend its lifetime
                        let component_rc = component.component();
                        let component_ref = component_rc.borrow();
                        let editor = component_ref.editor();
                        let buffer = editor.buffer();

                        // Convert Ki selections to VSCode selections
                        let vscode_selections = selections
                            .iter()
                            .map(|selection| {
                                // Get the extended range from the selection to ensure correct cursor position
                                // This is especially important for word selection mode
                                let range = selection.extended_range();

                                // Convert to positions
                                let start_pos = buffer.char_to_position(range.start).ok();
                                let end_pos = buffer.char_to_position(range.end).ok();

                                // Determine anchor and active positions based on whether it's extended
                                let (anchor, active) = if let Some(initial_range) =
                                    &selection.initial_range
                                {
                                    // Extended selection
                                    let anchor_pos =
                                        buffer.char_to_position(initial_range.start).ok();
                                    let active_pos = buffer.char_to_position(range.end).ok();
                                    (anchor_pos, active_pos)
                                } else {
                                    use crate::components::editor::Direction;
                                    match component.component().borrow().editor().cursor_direction {
                                        Direction::Start => (end_pos, start_pos),
                                        Direction::End => (start_pos, end_pos),
                                    }
                                };

                                // Create VSCode selection
                                ki_protocol_types::Selection {
                                    anchor: anchor.map_or_else(
                                        || ki_protocol_types::Position {
                                            line: 0,
                                            character: 0,
                                        },
                                        |pos| {
                                            crate::vscode::utils::ki_position_to_vscode_position(
                                                &pos,
                                            )
                                        },
                                    ),
                                    active: active.map_or_else(
                                        || ki_protocol_types::Position {
                                            line: 0,
                                            character: 0,
                                        },
                                        |pos| {
                                            crate::vscode::utils::ki_position_to_vscode_position(
                                                &pos,
                                            )
                                        },
                                    ),
                                    is_extended: selection.initial_range.is_some(),
                                }
                            })
                            .collect::<Vec<_>>();

                        // Get the selection mode from the editor
                        let selection_mode = match editor.selection_set.mode {
                            crate::selection::SelectionMode::Character => {
                                Some(ki_protocol_types::SelectionMode::Character)
                            }
                            crate::selection::SelectionMode::Line
                            | crate::selection::SelectionMode::LineFull => {
                                Some(ki_protocol_types::SelectionMode::Line)
                            }
                            crate::selection::SelectionMode::Word { .. } => {
                                Some(ki_protocol_types::SelectionMode::CoarseWord)
                            }
                            _ => None, // Default to None for other modes
                        };

                        // Send selection update notification to VSCode
                        let selection_set = ki_protocol_types::SelectionSet {
                            buffer_id,
                            primary: 0, // Assuming the first selection is primary
                            selections: vscode_selections,
                            mode: selection_mode,
                        };
                        self.send_notification(OutputMessageWrapper {
                            id: 0,
                            message: OutputMessage::SelectionUpdate(selection_set),
                            error: None,
                        })?;
                    }
                }
            }
            // IntegrationEvent::CursorUpdate has been removed in favor of the unified SelectionChanged event
            IntegrationEvent::BufferActivated {
                component_id: _,
                path,
            } => {
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
                })?;
            }
            IntegrationEvent::ViewportChanged {
                component_id,
                start_line,
                end_line,
            } => {
                // Get the buffer ID from the component
                if let Some(buffer_id) = self.get_buffer_id_from_component_id(component_id) {
                    // Send viewport change notification to VSCode
                    let viewport_params = ki_protocol_types::ViewportParams {
                        buffer_id,
                        start_line,
                        end_line,
                    };
                    self.send_notification(OutputMessageWrapper {
                        id: 0,
                        message: OutputMessage::ViewportChange(viewport_params),
                        error: None,
                    })?;
                }
            }
            IntegrationEvent::ExternalBufferCreated {
                component_id: _,
                buffer_id,
                content,
            } => {
                // Send external buffer created notification to VSCode
                let external_buffer_params =
                    ki_protocol_types::ExternalBufferParams { buffer_id, content };
                self.send_notification(OutputMessageWrapper {
                    id: 0,
                    message: OutputMessage::ExternalBufferCreated(external_buffer_params),
                    error: None,
                })?;
            }
            IntegrationEvent::ExternalBufferUpdated {
                component_id: _,
                buffer_id,
                content,
            } => {
                // Send external buffer updated notification to VSCode
                let external_buffer_params =
                    ki_protocol_types::ExternalBufferParams { buffer_id, content };
                self.send_notification(OutputMessageWrapper {
                    id: 0,
                    message: OutputMessage::ExternalBufferUpdated(external_buffer_params),
                    error: None,
                })?;
            }
            IntegrationEvent::CommandExecuted { command, success } => {
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
                })?;
            } // All integration events are now handled
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
        if !anchors.is_empty() && !actives.is_empty() {
            // Create selections from anchors and actives
            let mut vscode_selections = Vec::new();
            for i in 0..anchors.len() {
                let anchor = crate::vscode::utils::ki_position_to_vscode_position(&anchors[i]);
                let active = crate::vscode::utils::ki_position_to_vscode_position(&actives[i]);

                vscode_selections.push(ki_protocol_types::Selection {
                    anchor,
                    active,
                    is_extended: false, // We don't have this information
                });
            }

            // Get the selection mode from the editor
            let selection_mode = match editor.selection_set.mode {
                crate::selection::SelectionMode::Character => {
                    Some(ki_protocol_types::SelectionMode::Character)
                }
                crate::selection::SelectionMode::Line
                | crate::selection::SelectionMode::LineFull => {
                    Some(ki_protocol_types::SelectionMode::Line)
                }
                crate::selection::SelectionMode::Word { .. } => {
                    Some(ki_protocol_types::SelectionMode::CoarseWord)
                }
                _ => None, // Default to None for other modes
            };

            // Send selection update notification to VSCode
            let selection_set = ki_protocol_types::SelectionSet {
                buffer_id,
                primary: 0, // Assuming the first selection is primary
                selections: vscode_selections,
                mode: selection_mode,
            };

            info!("Sending selection update for current buffer");

            self.send_notification(OutputMessageWrapper {
                id: 0,
                message: OutputMessage::SelectionUpdate(selection_set),
                error: None,
            })?;
        }

        Ok(())
    }
}

/// Run the VSCode integration
pub fn run_vscode() -> anyhow::Result<()> {
    eprintln!("== VSCode integration initializing ==");

    // Initialize and run the VSCode integration
    let mut vscode_app = VSCodeApp::new(None)?;

    // Note: Port number is printed within WebSocketIpc::new()
    eprintln!("VSCode integration backend started. Waiting for VSCode extension to connect...");
    info!("VSCode integration backend started. Waiting for VSCode extension to connect...");

    // Run the main loop
    vscode_app.run()
}
