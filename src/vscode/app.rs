//! VSCode integration app implementation

use std::collections::HashMap;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::{App, AppMessage, StatusLineComponent};
use crate::frontend::crossterm::Crossterm;
use anyhow::Result;
use ki_protocol_types::{InputMessage, OutputMessage, OutputMessageWrapper, ResponseError};
use log::{debug, error, info, trace, warn};
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
    // Add receiver for notifications *from* the core App
    pub(crate) from_app_receiver: mpsc::Receiver<OutputMessageWrapper>,
    // Add receiver for messages *to* the core App (internal queue)
    pub(crate) app_message_receiver: mpsc::Receiver<AppMessage>,
    pub(crate) ipc_handler: WebSocketIpc, // Use the WebSocket IPC handler
    pub(crate) buffer_versions: HashMap<String, u64>,
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
            StatusLineComponent::Help,
            StatusLineComponent::KeyboardLayout,
            StatusLineComponent::CurrentWorkingDirectory,
            StatusLineComponent::GitBranch,
            StatusLineComponent::ViewAlignment,
            StatusLineComponent::Reveal,
            StatusLineComponent::Mode,
            StatusLineComponent::SelectionMode,
            StatusLineComponent::LocalSearchConfig,
            StatusLineComponent::LastDispatch,
        ];

        // Create channels for VSCodeApp communication
        let (real_app_sender, real_app_receiver) = mpsc::channel::<AppMessage>();
        let (real_notification_sender, real_notification_receiver) =
            mpsc::channel::<ki_protocol_types::OutputMessageWrapper>();

        // Resolve working directory once
        let resolved_wd = working_directory.unwrap_or(".".try_into()?);

        // Create the core App instance correctly
        let core_app = App::from_channel(
            frontend.clone(),               // Clone the Rc for shared ownership
            resolved_wd,                    // Use the resolved working directory
            real_app_sender.clone(),        // Core App gets sender to its *own* queue
            mpsc::channel().1, // Core App gets a dummy receiver it won't use in this mode
            status_line_components.clone(), // Clone the Vec
            Some(real_notification_sender), // Core App gets sender for notifications
        )?;

        // Initialize the WebSocket IPC handler
        let (ipc_handler, _port) = WebSocketIpc::new()?;
        info!("WebSocketIpc initialized. Waiting for VSCode connection...");

        Ok(Self {
            suppress_next_cursor_update: false,
            app: Arc::new(Mutex::new(core_app)),
            app_sender: real_app_sender, // Handlers use this sender
            app_message_receiver: real_app_receiver, // VSCodeApp loop polls this receiver
            from_app_receiver: real_notification_receiver, // Receiver for notifications FROM App
            ipc_handler,
            buffer_versions: HashMap::new(),
            next_message_id: 1,
            last_vscode_selection: None,
        })
    }

    /// Sends a message to VSCode via the WebSocket handler thread
    pub fn send_message_to_vscode(&self, message: OutputMessageWrapper) -> Result<()> {
        let id = message.id;
        info!(
            "Queueing message for VSCode: type={:?} id={}",
            message.message, id
        );
        self.ipc_handler
            .send_message_to_vscode(message)
            .map_err(|e| anyhow::anyhow!("Failed to send message to IPC handler: {}", e))
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
            InputMessage::CursorGet => {
                debug!("[{}] Processing cursor get request", trace_id);
                self.handle_cursor_get_request(id)
            }
            InputMessage::SelectionGet => {
                debug!("[{}] Processing selection get request", trace_id);
                self.handle_selection_get_request(id)
            }
            InputMessage::CursorUpdate(params) => {
                debug!("[{}] Processing cursor update request", trace_id);
                self.handle_cursor_update_request(id, params)
            }
            InputMessage::SelectionSet(params) => {
                debug!("[{}] Processing selection set request", trace_id);
                self.handle_selection_set_request(id, params)
            }
            InputMessage::ModeSet(_params) => {
                // TODO: Implement handler call: handlers::mode::handle_mode_set_request(self, id, params)
                warn!(
                    "[{}] Unsupported message type (handler not implemented yet): ModeSet",
                    trace_id
                );
                self.send_error_response(id, "Unsupported message type: ModeSet")
            }
            InputMessage::SelectionModeSet(_params) => {
                // TODO: Implement handler call: handlers::selection_mode::handle_selection_mode_set_request(self, id, params)
                warn!(
                    "[{}] Unsupported message type (handler not implemented yet): SelectionModeSet",
                    trace_id
                );
                self.send_error_response(id, "Unsupported message type: SelectionModeSet")
            }
            InputMessage::SearchFind(_params) => {
                // TODO: Implement handler call: handlers::search::handle_search_find_request(self, id, params)
                warn!(
                    "[{}] Unsupported message type (handler not implemented yet): SearchFind",
                    trace_id
                );
                self.send_error_response(id, "Unsupported message type: SearchFind")
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
            match self.app_message_receiver.try_recv() {
                Ok(app_message) => {
                    received_message = true;
                    trace!("Received message for core App: {:?}", app_message);
                    // Lock the App and process the message
                    match self.app.lock() {
                        Ok(mut app_guard) => {
                            match app_guard.process_message(app_message) {
                                Ok(should_quit) => {
                                    if should_quit {
                                        info!("Core App requested quit. Exiting.");
                                        // TODO: Maybe send a shutdown message to VSCode?
                                        break; // Exit the VSCodeApp loop
                                    }
                                }
                                Err(e) => {
                                    error!("Error processing message in core App: {}", e);
                                    // Optionally send an error notification to VSCode?
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to lock app mutex for processing message: {}", e);
                            // Decide how to handle mutex poisoning - potentially exit?
                            break;
                        }
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

            // 3. Check for internal notifications FROM the core App
            match self.from_app_receiver.try_recv() {
                Ok(app_notification) => {
                    received_message = true;
                    trace!(
                        "Received notification from core App: {:?}",
                        app_notification
                    );

                    if let Err(e) = self.send_notification(app_notification) {
                        error!("Error sending notification based on App update: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No notification from App
                }
                Err(TryRecvError::Disconnected) => {
                    error!("Core App notification channel disconnected! Exiting.");
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
    pub(crate) fn next_id(&mut self) -> u64 {
        let id = self.next_message_id;
        self.next_message_id += 1;
        id
    }

    /// Get the current file URI (accessible within the crate)
    pub(crate) fn get_current_file_uri(&self) -> Option<String> {
        self.get_current_file_path().map(|path| path_to_uri(&path))
    }

    /// Get the current file path (accessible within the crate)
    pub(crate) fn get_current_file_path(&self) -> Option<CanonicalizedPath> {
        let app_guard = match self.app.lock() {
            Ok(guard) => guard,
            Err(_) => return None, // Handle mutex poisoning
        };

        let component = app_guard.current_component();
        let component_ref = component.borrow();
        // Store the result in a variable to ensure borrows live long enough
        let path_opt = component_ref.editor().buffer().path().map(|p| p.clone());
        path_opt // Return the stored option
    }

    /// Send a response back to VSCode for a specific request ID.
    pub(crate) fn send_response(&self, id: u64, message: OutputMessage) -> Result<()> {
        let wrapper = OutputMessageWrapper {
            id,
            message,
            error: None,
        };
        self.send_message_to_vscode(wrapper)
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
