use crate::canonicalized_path::CanonicalizedPath;
use crate::position::Position;
use crate::screen::RequestParams;
use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::*;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::components::component::ComponentId;
use crate::screen::ScreenMessage;
use crate::utils::consolidate_errors;

use super::completion::{Completion, CompletionItem};
use super::goto_definition_response::GotoDefinitionResponse;
use super::hover::Hover;
use super::language::Language;
use crate::quickfix_list::Location;

struct LspServerProcess {
    language: Language,
    stdin: process::ChildStdin,

    /// This is hacky, but we need to keep the stdout around so that it doesn't get dropped
    stdout: Option<process::ChildStdout>,

    server_capabilities: Option<ServerCapabilities>,
    current_working_directory: CanonicalizedPath,
    next_request_id: RequestId,
    pending_response_requests: HashMap<RequestId, PendingResponseRequest>,
    screen_message_sender: Sender<ScreenMessage>,

    receiver: Receiver<LspServerProcessMessage>,
    sender: Sender<LspServerProcessMessage>,
}

type RequestId = u64;

#[derive(Debug)]
struct PendingResponseRequest {
    method: String,

    /// This indicates that this request was sent by a component,
    /// and the response should be sent back to that component.
    ///
    /// If the response of this request need not be sent back to a component,
    /// just use the default value `ComponentId::default()`.
    ///
    /// This field is purposefully not an `Option` so that we do not need to
    /// use `unwrap()` to obtain the `component_id`.
    component_id: ComponentId,
}

#[derive(Debug)]
pub enum LspNotification {
    Initialized(Language),
    PublishDiagnostics(PublishDiagnosticsParams),
    Completion(ComponentId, Completion),
    Hover(ComponentId, Hover),
    Definition(ComponentId, GotoDefinitionResponse),
    References(ComponentId, Vec<Location>),
}

#[derive(Debug)]
enum LspServerProcessMessage {
    FromLspServer(serde_json::Value),
    FromEditor(FromEditor),
}

#[derive(Debug)]
enum FromEditor {
    RequestHover(RequestParams),
    RequestCompletion(RequestParams),
    RequestDefinition(RequestParams),
    RequestReferences(RequestParams),
    TextDocumentDidOpen {
        file_path: CanonicalizedPath,
        language_id: String,
        version: usize,
        content: String,
    },
    Shutdown,
    TextDocumentDidChange {
        file_path: CanonicalizedPath,
        version: i32,
        content: String,
    },
    TextDocumentDidSave {
        file_path: CanonicalizedPath,
    },
}

pub struct LspServerProcessChannel {
    language: Language,
    join_handle: JoinHandle<JoinHandle<()>>,
    sender: Sender<LspServerProcessMessage>,
    is_initialized: bool,
}

impl LspServerProcessChannel {
    pub fn new(
        language: Language,
        screen_message_sender: Sender<ScreenMessage>,
    ) -> Result<LspServerProcessChannel, anyhow::Error> {
        LspServerProcess::start(language, screen_message_sender)
    }

    pub fn request_hover(&self, params: RequestParams) -> Result<(), anyhow::Error> {
        self.send(LspServerProcessMessage::FromEditor(
            FromEditor::RequestHover(params),
        ))
    }

    pub fn request_definition(&self, params: RequestParams) -> Result<(), anyhow::Error> {
        self.send(LspServerProcessMessage::FromEditor(
            FromEditor::RequestDefinition(params),
        ))
    }

    pub fn request_references(&self, params: RequestParams) -> Result<(), anyhow::Error> {
        self.send(LspServerProcessMessage::FromEditor(
            FromEditor::RequestReferences(params),
        ))
    }

    pub fn request_completion(&self, params: RequestParams) -> anyhow::Result<()> {
        self.send(LspServerProcessMessage::FromEditor(
            FromEditor::RequestCompletion(params),
        ))
    }

    pub fn shutdown(self) -> anyhow::Result<()> {
        self.send(LspServerProcessMessage::FromEditor(FromEditor::Shutdown))?;
        self.join_handle
            .join()
            .map_err(|err| anyhow::anyhow!("Unable to join lsp server process [1]: {:?}", err))?
            .join()
            .map_err(|err| anyhow::anyhow!("Unable to join lsp server process [2]: {:?}", err))
    }

    fn send(&self, message: LspServerProcessMessage) -> anyhow::Result<()> {
        self.sender
            .send(message)
            .map_err(|err| anyhow::anyhow!("Unable to send request: {}", err))
    }

    pub fn documents_did_open(
        &mut self,
        paths: Vec<CanonicalizedPath>,
    ) -> Result<(), anyhow::Error> {
        consolidate_errors(
            "[documents_did_open]",
            paths
                .into_iter()
                .map(|path| self.document_did_open(path))
                .collect(),
        )
    }

    pub fn document_did_open(&self, path: CanonicalizedPath) -> Result<(), anyhow::Error> {
        let content = path.read()?;
        self.send(LspServerProcessMessage::FromEditor(
            FromEditor::TextDocumentDidOpen {
                file_path: path,
                language_id: self.language.id(),
                version: 1,
                content,
            },
        ))
    }

    pub fn document_did_change(
        &self,
        path: &CanonicalizedPath,
        content: &String,
    ) -> Result<(), anyhow::Error> {
        self.send(LspServerProcessMessage::FromEditor(
            FromEditor::TextDocumentDidChange {
                file_path: path.clone(),
                version: 2,
                content: content.clone(),
            },
        ))
    }
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    pub fn initialized(&mut self) {
        self.is_initialized = true
    }

    pub fn document_did_save(&self, path: &CanonicalizedPath) -> Result<(), anyhow::Error> {
        self.send(LspServerProcessMessage::FromEditor(
            FromEditor::TextDocumentDidSave {
                file_path: path.clone(),
            },
        ))
    }
}

impl LspServerProcess {
    fn start(
        language: Language,
        screen_message_sender: Sender<ScreenMessage>,
    ) -> anyhow::Result<LspServerProcessChannel> {
        let (command, args) = language.get_command_args();

        let mut command = Command::new(command);
        command.stdin(Stdio::piped()).stdout(Stdio::piped());

        args.into_iter().for_each(|arg| {
            command.arg(arg);
        });

        let mut process = command.spawn()?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Unable to obtain stdin"))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Unable to obtain stdout"))?;
        let (sender, receiver) = std::sync::mpsc::channel::<LspServerProcessMessage>();
        let mut lsp_server_process = LspServerProcess {
            language,
            stdin,
            stdout: Some(stdout),
            current_working_directory: std::env::current_dir()?.try_into()?,
            next_request_id: 0,
            pending_response_requests: HashMap::new(),
            server_capabilities: None,
            screen_message_sender,
            receiver,
            sender: sender.clone(),
        };

        lsp_server_process.initialize()?;

        let join_handle = std::thread::spawn(move || lsp_server_process.listen());

        Ok(LspServerProcessChannel {
            language,
            join_handle,
            sender,
            is_initialized: false,
        })
    }

    fn initialize(&mut self) -> anyhow::Result<()> {
        self.send_request::<lsp_request!("initialize")>(
            ComponentId::default(),
            InitializeParams {
                process_id: None,
                root_uri: Some(
                    Url::parse(&format!(
                        "file://{}",
                        self.current_working_directory.display()
                    ))
                    .unwrap(),
                ),

                capabilities: ClientCapabilities {
                    workspace: Some(WorkspaceClientCapabilities {
                        apply_edit: Some(true),
                        ..WorkspaceClientCapabilities::default()
                    }),
                    text_document: Some(TextDocumentClientCapabilities {
                        completion: Some(CompletionClientCapabilities {
                            completion_item: Some(CompletionItemCapability {
                                ..CompletionItemCapability::default()
                            }),
                            completion_item_kind: Some(CompletionItemKindCapability {
                                ..Default::default()
                            }),
                            ..CompletionClientCapabilities::default()
                        }),
                        hover: Some(HoverClientCapabilities {
                            content_format: Some(vec![MarkupKind::PlainText]),
                            ..HoverClientCapabilities::default()
                        }),
                        ..TextDocumentClientCapabilities::default()
                    }),
                    ..ClientCapabilities::default()
                },
                workspace_folders: None,
                client_info: None,
                ..InitializeParams::default()
            },
        )?;
        Ok(())
    }

    pub fn listen(mut self) -> JoinHandle<()> {
        let mut reader = BufReader::new(self.stdout.take().unwrap());
        let sender = self.sender.clone();
        let handle = thread::spawn(move || {
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();

                let content_length = line
                    .split(":")
                    .nth(1)
                    .unwrap()
                    .trim()
                    .parse::<usize>()
                    .unwrap();

                // According to https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#headerPart
                //
                // ... this means that TWO ‘\r\n’ sequences always immediately precede the content part of a message.
                //
                // That's why we have to read an empty line here again.
                reader.read_line(&mut line).unwrap();

                let mut buffer = vec![0; content_length];
                reader.read_exact(&mut buffer).unwrap();

                let reply = String::from_utf8(buffer).unwrap();

                // Parse as generic JSON value
                let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();

                sender
                    .send(LspServerProcessMessage::FromLspServer(reply))
                    .unwrap_or_else(|error| {
                        log::error!("[LspServerProcess] Error sending reply: {:?}", error);
                    });
            }
        });

        log::info!("[LspServerProcess] Listening for messages from LSP server");
        while let Ok(message) = self.receiver.recv() {
            match message {
                LspServerProcessMessage::FromLspServer(json_value) => self.handle_reply(json_value),
                LspServerProcessMessage::FromEditor(from_editor) => match from_editor {
                    FromEditor::RequestCompletion(params) => self.text_document_completion(params),
                    FromEditor::RequestHover(params) => self.text_document_hover(params),
                    FromEditor::RequestDefinition(params) => self.text_document_definition(params),
                    FromEditor::RequestReferences(params) => self.text_document_references(params),
                    FromEditor::TextDocumentDidOpen {
                        file_path,
                        language_id,
                        version,
                        content,
                    } => self.text_document_did_open(file_path, language_id, version, content),
                    FromEditor::Shutdown => self.shutdown(),
                    FromEditor::TextDocumentDidChange {
                        file_path,
                        version,
                        content,
                    } => self.text_document_did_change(file_path, version, content),
                    FromEditor::TextDocumentDidSave { file_path } => {
                        self.text_document_did_save(file_path)
                    }
                },
            }
            .unwrap_or_else(|error| {
                log::info!("[LspServerProcess] Error handling reply: {:?}", error);
            })
        }

        log::info!("[LspServerProcess] Stopped listening for messages from LSP server");
        handle
    }

    fn handle_reply(&mut self, reply: serde_json::Value) -> anyhow::Result<()> {
        // Check if reply is Response or Notification
        // Only Notification contains the `method` field
        match reply.get("method") {
            // reply is Response
            None => {
                // Get the request ID
                let request_id = reply.get("id").unwrap().as_u64().unwrap();

                // Get the method of the request
                let pending_response_request =
                    self.pending_response_requests.remove(&request_id).unwrap();

                // Parse the reply as a Response
                let response = serde_json::from_value::<
                    json_rpc_types::Response<
                        serde_json::Value,
                        (),
                        // Need to specify String here
                        // Otherwise the default will be `str_buf::StrBuf<31>`,
                        // which says the error message can only be 31 bytes long.
                        String,
                    >,
                >(reply)
                .map_err(|e| anyhow::anyhow!("Serde error = {:?}", e))?
                .payload
                .map_err(|e| {
                    anyhow::anyhow!(
                        "LSP JSON-RPC Error: Code={:?} Message={}",
                        e.code,
                        e.message
                    )
                })?;

                let PendingResponseRequest {
                    method,
                    component_id,
                } = pending_response_request;
                match method.as_str() {
                    "initialize" => {
                        log::info!("Initialize response: {:?}", response);
                        let payload: <lsp_request!("initialize") as Request>::Result =
                            serde_json::from_value(response)?;

                        // Get the capabilities
                        self.server_capabilities = Some(payload.capabilities);

                        // Send the initialized notification
                        self.send_notification::<lsp_notification!("initialized")>(
                            InitializedParams {},
                        )?;

                        self.screen_message_sender
                            .send(ScreenMessage::LspNotification(
                                LspNotification::Initialized(self.language),
                            ))?;
                    }
                    "textDocument/completion" => {
                        let payload: <lsp_request!("textDocument/completion") as Request>::Result =
                            serde_json::from_value(response)?;

                        log::info!("Recevied completion");

                        if let Some(payload) = payload {
                            self.screen_message_sender
                                .send(ScreenMessage::LspNotification(LspNotification::Completion(
                                    component_id,
                                    Completion {
                                        trigger_characters: self.trigger_characters(),
                                        items: match payload {
                                            CompletionResponse::Array(items) => items,
                                            CompletionResponse::List(list) => list.items,
                                        }
                                        .into_iter()
                                        .map(CompletionItem::from)
                                        .collect(),
                                    },
                                )))
                                .unwrap();
                        }
                    }
                    "textDocument/hover" => {
                        let payload: <lsp_request!("textDocument/hover") as Request>::Result =
                            serde_json::from_value(response)?;

                        if let Some(payload) = payload {
                            self.screen_message_sender
                                .send(ScreenMessage::LspNotification(LspNotification::Hover(
                                    component_id,
                                    payload.into(),
                                )))
                                .unwrap();
                        }
                    }
                    "textDocument/definition" => {
                        let payload: <lsp_request!("textDocument/definition") as Request>::Result =
                            serde_json::from_value(response)?;

                        if let Some(payload) = payload {
                            self.screen_message_sender
                                .send(ScreenMessage::LspNotification(LspNotification::Definition(
                                    component_id,
                                    payload.try_into()?,
                                )))
                                .unwrap();
                        }
                    }
                    "textDocument/references" => {
                        let payload: <lsp_request!("textDocument/references") as Request>::Result =
                            serde_json::from_value(response)?;

                        if let Some(payload) = payload {
                            self.screen_message_sender
                                .send(ScreenMessage::LspNotification(LspNotification::References(
                                    component_id,
                                    payload
                                        .into_iter()
                                        .map(|r| r.try_into())
                                        .collect::<Result<Vec<_>, _>>()?,
                                )))
                                .unwrap();
                        }
                    }
                    _ => {
                        log::info!("Unknown method: {:#?}", method);
                    }
                }
            }

            // reply is Notification
            Some(_) => {
                let request = serde_json::from_value::<
                    json_rpc_types::Request<
                        serde_json::Value,
                        // Need to specify String here
                        // Otherwise the default will be `str_buf::StrBuf<31>`,
                        // which says the error message can only be 31 bytes long.
                        String,
                    >,
                >(reply)
                .map_err(|e| anyhow::anyhow!("Serde error = {:?}", e))?;

                let method = request.method;
                // Parse the reply as Notification
                match method.as_str() {
                    "textDocument/publishDiagnostics" => {
                        log::info!("Incoming Notification: {}", method);
                        let params: <lsp_notification!("textDocument/publishDiagnostics") as Notification>::Params =
                            serde_json::from_value(request.params.ok_or_else(|| anyhow::anyhow!("Missing params"))?)?;

                        self.screen_message_sender
                            .send(ScreenMessage::LspNotification(
                                LspNotification::PublishDiagnostics(params),
                            ))
                            .unwrap();
                    }
                    _ => log::info!("Incoming Notification: {}", method),
                }
            }
        }

        Ok(())
    }

    pub fn text_document_completion(
        &mut self,
        RequestParams {
            component_id,
            path,
            position,
        }: RequestParams,
    ) -> anyhow::Result<()> {
        let result = self.send_request::<lsp_request!("textDocument/completion")>(
            component_id,
            CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    position: Position {
                        // Need to add 1 for completion after trigger characters to work
                        column: position.column + 1,
                        ..position
                    }
                    .into(),
                    text_document: path_buf_to_text_document_identifier(path)?,
                },
                work_done_progress_params: WorkDoneProgressParams {
                    work_done_token: None,
                },
                partial_result_params: PartialResultParams {
                    partial_result_token: None,
                },
                context: None,
            },
        )?;

        log::info!("{:?}", result);
        Ok(())
    }

    fn trigger_characters(&self) -> Vec<String> {
        self.server_capabilities
            .as_ref()
            .map(|capabilities| {
                capabilities
                    .completion_provider
                    .as_ref()
                    .map(|provider| provider.trigger_characters.clone())
            })
            .flatten()
            .flatten()
            .unwrap_or_else(|| vec![])
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.send_request::<lsp_request!("shutdown")>(ComponentId::default(), ())?;
        Ok(())
    }

    fn send_notification<N: Notification>(&mut self, params: N::Params) -> anyhow::Result<()> {
        let notification = json_rpc_types::Request {
            id: None,
            jsonrpc: json_rpc_types::Version::V2,
            method: N::METHOD,
            params: Some(params),
        };

        log::info!("Sending notification: {:?}", N::METHOD);
        let message = serde_json::to_string(&notification)?;

        write!(
            &mut self.stdin,
            "Content-Length: {}\r\n\r\n{}",
            message.len(),
            message
        )?;

        Ok(())
    }

    /// Returns the request ID
    fn send_request<R: Request>(
        &mut self,
        component_id: ComponentId,
        params: R::Params,
    ) -> anyhow::Result<()>
    where
        R::Params: serde::Serialize,
    {
        let id = {
            let result = self.next_request_id;
            self.next_request_id += 1;
            result
        };
        // Convert the request to a JSON-RPC message
        let request = json_rpc_types::Request {
            jsonrpc: json_rpc_types::Version::V2,
            method: R::METHOD,
            params: Some(params),
            id: Some(json_rpc_types::Id::Num(id)),
        };

        let message = serde_json::to_string(&request)?;

        log::info!("Sending request: {}", message);

        // The message format is according to https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#contentPart
        write!(
            &mut self.stdin,
            "Content-Length: {}\r\n\r\n{}",
            message.len(),
            message
        )?;

        self.pending_response_requests.insert(
            id,
            PendingResponseRequest {
                component_id,
                method: R::METHOD.to_string(),
            },
        );

        Ok(())
    }

    fn text_document_did_open(
        &mut self,
        file_path: CanonicalizedPath,
        language_id: String,
        version: usize,
        content: String,
    ) -> Result<(), anyhow::Error> {
        self.send_notification::<lsp_notification!("textDocument/didOpen")>(
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: path_buf_to_url(file_path)?,
                    language_id,
                    version: version as i32,
                    text: content,
                },
            },
        )
    }

    fn text_document_did_change(
        &mut self,
        file_path: CanonicalizedPath,
        version: i32,
        content: String,
    ) -> Result<(), anyhow::Error> {
        self.send_notification::<lsp_notification!("textDocument/didChange")>(
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: path_buf_to_url(file_path)?,
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: content,
                }],
            },
        )
    }

    fn text_document_did_save(
        &mut self,
        file_path: CanonicalizedPath,
    ) -> Result<(), anyhow::Error> {
        self.send_notification::<lsp_notification!("textDocument/didSave")>(
            DidSaveTextDocumentParams {
                text_document: path_buf_to_text_document_identifier(file_path)?,
                text: None,
            },
        )
    }

    fn text_document_hover(
        &mut self,
        RequestParams {
            component_id,
            path,
            position,
        }: RequestParams,
    ) -> anyhow::Result<()> {
        self.send_request::<lsp_request!("textDocument/hover")>(
            component_id,
            HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    position: position.into(),
                    text_document: path_buf_to_text_document_identifier(path)?,
                },
                work_done_progress_params: WorkDoneProgressParams {
                    work_done_token: None,
                },
            },
        )
    }

    fn text_document_definition(
        &mut self,
        RequestParams {
            component_id,
            path,
            position,
        }: RequestParams,
    ) -> anyhow::Result<()> {
        self.send_request::<lsp_request!("textDocument/definition")>(
            component_id,
            GotoDefinitionParams {
                partial_result_params: Default::default(),
                text_document_position_params: TextDocumentPositionParams {
                    position: position.into(),
                    text_document: path_buf_to_text_document_identifier(path)?,
                },
                work_done_progress_params: Default::default(),
            },
        )
    }

    fn text_document_references(
        &mut self,
        RequestParams {
            component_id,
            path,
            position,
        }: RequestParams,
    ) -> anyhow::Result<()> {
        self.send_request::<lsp_request!("textDocument/references")>(
            component_id,
            ReferenceParams {
                context: ReferenceContext {
                    include_declaration: true,
                },
                partial_result_params: Default::default(),
                text_document_position: TextDocumentPositionParams {
                    position: position.into(),
                    text_document: path_buf_to_text_document_identifier(path)?,
                },
                work_done_progress_params: Default::default(),
            },
        )
    }
}

fn path_buf_to_url(path: CanonicalizedPath) -> Result<Url, anyhow::Error> {
    Ok(Url::parse(&format!("file://{}", path.display()))?)
}

fn path_buf_to_text_document_identifier(
    path: CanonicalizedPath,
) -> Result<TextDocumentIdentifier, anyhow::Error> {
    Ok(TextDocumentIdentifier {
        uri: path_buf_to_url(path)?,
    })
}
