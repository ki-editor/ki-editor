//! VSCode integration app implementation

use crate::components::editor::Mode;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Mutex;
use std::thread;

use crate::app::{App, AppMessage, Dispatch, StatusLineComponent};
use crate::frontend::crossterm::Crossterm;
use anyhow::Result;
use ki_protocol_types::{
    BufferDiagnostics, InputMessage, MarksParams, OutputMessage, OutputMessageWrapper,
    PromptOpenedParams, ResponseError,
};
use log::{debug, error, info, trace};
use shared::canonicalized_path::CanonicalizedPath;

use super::ipc::WebSocketIpc;
use super::logger::VSCodeLogger;
use super::utils::*;
use crate::vscode::handlers;

pub struct VSCodeApp {
    pub(crate) app: Rc<Mutex<App<Crossterm>>>,
    pub(crate) app_sender: mpsc::Sender<AppMessage>,
    pub(crate) integration_event_receiver:
        mpsc::Receiver<crate::integration_event::IntegrationEvent>,
    pub(crate) app_message_receiver: mpsc::Receiver<AppMessage>,
    pub(crate) ipc_handler: WebSocketIpc,
    pub(crate) buffer_versions: HashMap<String, u64>,
    #[allow(dead_code)]
    pub(crate) next_message_id: u64,
}

impl VSCodeApp {
    pub fn new(working_directory: Option<CanonicalizedPath>) -> Result<Self> {
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

        let (real_app_sender, real_app_receiver) = mpsc::channel::<AppMessage>();
        let resolved_wd = working_directory.unwrap_or("./".try_into()?);

        let (integration_event_sender, integration_event_receiver) =
            mpsc::channel::<crate::integration_event::IntegrationEvent>();

        let core_app = App::from_channel(
            frontend.clone(),
            resolved_wd,
            real_app_sender.clone(),
            mpsc::channel().1,
            status_line_components.clone(),
            Some(integration_event_sender),
            false,
            true,
        )?;

        let (ipc_handler, _port) = WebSocketIpc::new()?;
        info!("WebSocketIpc initialized. Waiting for VSCode connection...");

        Ok(Self {
            app: Rc::new(Mutex::new(core_app)),
            app_sender: real_app_sender,
            app_message_receiver: real_app_receiver,
            integration_event_receiver,
            ipc_handler,
            buffer_versions: HashMap::new(),
            next_message_id: 1,
        })
    }

    pub fn send_message_to_vscode(&self, message: OutputMessageWrapper) -> Result<()> {
        self.ipc_handler
            .send_message_to_vscode(message)
            .map_err(|e| anyhow::anyhow!("Failed to send message to IPC handler: {}", e))
    }

    pub fn send_notification(&self, wrapper: OutputMessageWrapper) -> Result<()> {
        let notification_type = format!("{:?}", wrapper.message);
        trace!(
            "SENDING Notification: Type={:?}, ID={}",
            notification_type,
            wrapper.id
        );
        self.send_message_to_vscode(wrapper)
    }

    fn handle_request(&mut self, id: u64, message: InputMessage, trace_id: &str) -> Result<()> {
        debug!(
            "[{}] Entering handle_request: id={}, message={:?}",
            trace_id, id, message
        );

        let start_time = std::time::Instant::now();
        let message_type = format!("{:?}", message);
        trace!(
            "[{}] ENTER handle_request: ID={:?}, Type={:?}",
            trace_id,
            id,
            message_type
        );

        let result = match message {
            InputMessage::Ping(value) => {
                debug!("[{}] Processing ping request", trace_id);
                handlers::ping::handle_ping_request(self, id, value.unwrap_or_default())
            }
            InputMessage::BufferOpen(params) => {
                debug!("[{}] Processing buffer open request", trace_id);
                self.handle_buffer_open_request(params)
            }
            InputMessage::BufferClose(params) => {
                debug!("[{}] Processing buffer close request", trace_id);
                self.handle_buffer_close_request(params)
            }
            InputMessage::BufferSave(params) => {
                debug!("[{}] Processing buffer save request", trace_id);
                self.handle_buffer_save_request(params)
            }
            InputMessage::BufferActive(params) => {
                debug!("[{}] Processing buffer active request", trace_id);
                self.handle_buffer_active_request(params)
            }
            InputMessage::BufferChange(params) => {
                debug!("[{}] Processing buffer change request", trace_id);
                self.handle_buffer_change_request(params)
            }
            InputMessage::KeyboardInput(params) => {
                info!(
                    "[{}] Received keyboard.input request: key='{}'",
                    trace_id, params.key
                );
                self.handle_keyboard_input_request(id, params, trace_id)
            }
            InputMessage::SelectionSet(params) => {
                debug!("[{}] Processing selection set request", trace_id);
                self.handle_selection_set_request(params)
            }
            InputMessage::ModeSet(params) => {
                debug!("[{}] Processing mode set request", trace_id);
                self.handle_mode_set_request(params, trace_id)
            }
            InputMessage::SelectionModeSet(params) => {
                debug!("[{}] Processing selection mode set request", trace_id);
                self.handle_selection_mode_set_request(params, trace_id)
            }
            InputMessage::SearchFind(params) => {
                debug!("[{}] Processing search find request", trace_id);
                self.handle_search_find_request(params, trace_id)
            }
            InputMessage::ViewportChange(params) => {
                debug!("[{}] Processing viewport change request", trace_id);
                self.handle_viewport_change_request(params)
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

    pub fn run(&mut self) -> Result<()> {
        trace!("Starting VSCodeApp main loop");

        loop {
            let mut received_message = false;

            match self.ipc_handler.try_receive_from_vscode() {
                Ok((id, message, trace_id)) => {
                    received_message = true;
                    debug!(
                        "[{}] Received message from VSCode: id={}, message={:?}",
                        &trace_id, id, &message
                    );
                    if let Err(e) = self.handle_request(id, message, &trace_id) {
                        error!("Error handling request from VSCode: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    error!("IPC channel from VSCode disconnected! Exiting.");
                    break;
                }
            }

            match self.app_message_receiver.try_recv() {
                Ok(app_message) => {
                    received_message = true;
                    trace!("Received message for core App: {:?}", app_message);

                    if let AppMessage::QuitAll = &app_message {
                        info!("Core App requested quit. Exiting.");
                        break;
                    }

                    let app_lock_result = match Rc::clone(&self.app).try_lock() {
                        Ok(mut guard) => {
                            trace!("Successfully acquired App lock to process message");
                            match guard.process_message(app_message) {
                                Ok(should_quit) => {
                                    if should_quit {
                                        info!("Core App requested quit. Exiting.");
                                        break;
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
                            trace!("Could not acquire App lock to process message, will try again later");
                            Ok(())
                        }
                    };

                    if let Err(e) = app_lock_result {
                        error!("Error processing message: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    error!("Internal App message channel disconnected! Exiting.");
                    break;
                }
            }

            match self.integration_event_receiver.try_recv() {
                Ok(event) => {
                    received_message = true;
                    trace!("Received integration event from core App: {:?}", event);

                    if let Err(e) = self.handle_integration_event(event) {
                        error!("Error handling integration event: {}", e);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    error!("Core App integration event channel disconnected! Exiting.");
                    break;
                }
            }

            if !received_message {
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        trace!("VSCodeApp main loop stopped");
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn next_id(&mut self) -> u64 {
        let id = self.next_message_id;
        self.next_message_id += 1;
        id
    }

    #[allow(dead_code)]
    pub(crate) fn get_current_file_uri(&self) -> Option<String> {
        self.get_current_file_path().map(|path| path_to_uri(&path))
    }

    pub(crate) fn get_current_file_path(&self) -> Option<CanonicalizedPath> {
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                trace!("Could not acquire app lock to get current file path");
                return None;
            }
        };

        let component = app_guard.current_component();
        let component_ref = component.borrow();
        let path_opt = component_ref.editor().buffer().path();
        path_opt
    }

    pub(crate) fn get_editor_component_by_path(
        &self,
        path: &CanonicalizedPath,
    ) -> Option<std::rc::Rc<std::cell::RefCell<dyn crate::components::component::Component>>> {
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                trace!("Could not acquire app lock to check components");
                return None;
            }
        };

        for component in app_guard.components() {
            let component_rc = component.component();
            let component_ref = component_rc.borrow();

            if let Some(comp_path) = component_ref.path() {
                if &comp_path == path {
                    return Some(component_rc.clone());
                }
            }
        }

        None
    }

    pub(crate) fn send_response(&self, id: u64, message: OutputMessage) -> Result<()> {
        info!("Sending response for request ID {}: {:?}", id, message);
        let wrapper = OutputMessageWrapper {
            id,
            message,
            error: None,
        };
        self.send_message_to_vscode(wrapper)
    }

    pub(crate) fn send_error_response(&self, id: u64, error_message: &str) -> Result<()> {
        let error_response = OutputMessageWrapper {
            id,
            message: OutputMessage::Error(error_message.to_string()),
            error: Some(ResponseError {
                code: -32000,
                message: error_message.to_string(),
                data: None,
            }),
        };
        self.send_message_to_vscode(error_response)
    }

    fn handle_integration_event(
        &self,
        event: crate::integration_event::IntegrationEvent,
    ) -> anyhow::Result<()> {
        use crate::integration_event::IntegrationEvent;

        match event {
            IntegrationEvent::BufferChanged {
                component_id: _,
                path,
                edits,
            } => self.buffer_changed(path, edits)?,
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
            IntegrationEvent::RequestLspHover => self.request_lsp_hover()?,
            IntegrationEvent::RequestLspReferences => self.request_lsp_references()?,
            IntegrationEvent::RequestLspDeclaration => self.request_lsp_declaration()?,
            IntegrationEvent::RequestLspImplementation => self.request_lsp_implementation()?,
            IntegrationEvent::RequestLspTypeDefinition => self.request_lsp_type_definition()?,
            IntegrationEvent::KeyboardLayoutChanged(keyboard_layout) => {
                self.keyboard_layout_changed(keyboard_layout)?
            }
        }

        Ok(())
    }

    fn get_buffer_id_from_component_id(
        &self,
        component_id: crate::integration_event::ComponentId,
    ) -> Option<String> {
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                trace!("Could not acquire app lock to get component by ID");
                return None;
            }
        };

        let components = app_guard.components();
        for component in components {
            let component_ref = component.component();
            let borrowed = component_ref.borrow();

            if crate::integration_event::component_id_to_usize(&borrowed.id()) == component_id {
                if let Some(path) = borrowed.path() {
                    return Some(path.display_absolute());
                }
            }
        }

        None
    }

    fn get_component_by_id(
        &self,
        component_id: crate::integration_event::ComponentId,
    ) -> Option<crate::ui_tree::KindedComponent> {
        let app_guard = match self.app.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                trace!("Could not acquire app lock to get component by ID");
                return None;
            }
        };

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
                    Some(initial_range) => initial_range.start < selection.range().start,
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

        let diff_params = ki_protocol_types::BufferDiffParams { buffer_id, edits };

        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::BufferDiff(diff_params),
            error: None,
        })
    }

    fn buffer_saved(&self, path: CanonicalizedPath) -> anyhow::Result<()> {
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

        let editor_mode = match mode {
            Mode::Normal => ki_protocol_types::EditorMode::Normal,
            Mode::Insert => ki_protocol_types::EditorMode::Insert,
            Mode::MultiCursor => ki_protocol_types::EditorMode::MultiCursor,
            Mode::FindOneChar(_) => ki_protocol_types::EditorMode::FindOneChar,
            Mode::Swap => ki_protocol_types::EditorMode::Swap,
            Mode::Replace => ki_protocol_types::EditorMode::Replace,
            Mode::Extend => ki_protocol_types::EditorMode::Extend,
        };

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
            crate::selection::SelectionMode::LocalQuickfix { .. } => {
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

        let Some(component) = self.get_component_by_id(component_id) else {
            return Ok(());
        };
        let component_rc = component.component();
        let component_ref = component_rc.borrow();
        let editor = component_ref.editor();
        let buffer = editor.buffer();

        let jumps = jumps
            .into_iter()
            .map(|(char, char_index)| -> anyhow::Result<_> {
                Ok((
                    char,
                    buffer.char_to_position(char_index)?.to_vscode_position(),
                ))
            })
            .collect::<anyhow::Result<Vec<_>, _>>()?;

        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::JumpsChanged(ki_protocol_types::JumpsParams {
                uri: buffer_id,
                targets: jumps
                    .into_iter()
                    .map(|(key, position)| ki_protocol_types::JumpTarget { key, position })
                    .collect(),
            }),
            error: None,
        })
    }

    fn external_buffer_created(&self, buffer_id: String, content: String) -> anyhow::Result<()> {
        let external_buffer_params = ki_protocol_types::ExternalBufferParams { buffer_id, content };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::ExternalBufferCreated(external_buffer_params),
            error: None,
        })
    }

    fn external_buffer_updated(&self, buffer_id: String, content: String) -> anyhow::Result<()> {
        let external_buffer_params = ki_protocol_types::ExternalBufferParams { buffer_id, content };
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::ExternalBufferUpdated(external_buffer_params),
            error: None,
        })
    }

    fn command_executed(&self, command: String, success: bool) -> anyhow::Result<()> {
        let command_params = ki_protocol_types::CommandParams {
            name: command,
            args: Vec::new(),
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

        let Some(component) = self.get_component_by_id(component_id) else {
            return Ok(());
        };
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
            message: OutputMessage::MarksChanged(MarksParams {
                marks,
                uri: buffer_id,
            }),
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

    fn request_lsp_hover(&self) -> anyhow::Result<()> {
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::RequestLspHover,
            error: None,
        })
    }

    fn request_lsp_references(&self) -> anyhow::Result<()> {
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::RequestLspReferences,
            error: None,
        })
    }

    fn request_lsp_declaration(&self) -> anyhow::Result<()> {
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::RequestLspDeclaration,
            error: None,
        })
    }

    fn request_lsp_type_definition(&self) -> anyhow::Result<()> {
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::RequestLspTypeDefinition,
            error: None,
        })
    }

    fn request_lsp_implementation(&self) -> anyhow::Result<()> {
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::RequestLspImplementation,
            error: None,
        })
    }

    fn keyboard_layout_changed(&self, keyboard_layout: &str) -> anyhow::Result<()> {
        self.send_notification(OutputMessageWrapper {
            id: 0,
            message: OutputMessage::KeyboardLayoutChanged(keyboard_layout.to_string()),
            error: None,
        })
    }
}

pub fn run_vscode(working_directory: CanonicalizedPath) -> anyhow::Result<()> {
    eprintln!("== VSCode integration initializing ==");

    let mut vscode_app = VSCodeApp::new(Some(working_directory))?;

    eprintln!("VSCode integration backend started. Waiting for VSCode extension to connect...");
    info!("VSCode integration backend started. Waiting for VSCode extension to connect...");

    vscode_app.run()
}
