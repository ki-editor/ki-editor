use lsp_types::notification::Notification;
use lsp_types::request::{Initialize, Request};
use lsp_types::{
    lsp_notification, lsp_request, ClientCapabilities, CompletionClientCapabilities,
    CompletionContext, CompletionItemKind, CompletionItemKindCapability, CompletionParams,
    CompletionResponse, CompletionTriggerKind, GeneralClientCapabilities, InitializeParams,
    InitializedParams, PartialResultParams, Position, ServerCapabilities,
    TextDocumentClientCapabilities, TextDocumentIdentifier, TextDocumentPositionParams, Url,
    WorkDoneProgressParams, WorkspaceClientCapabilities,
};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::screen::ScreenMessage;

pub struct LspServerProcess {
    process: process::Child,
    stdin: process::ChildStdin,

    /// This is hacky, but we need to keep the stdout around so that it doesn't get dropped
    stdout: Option<process::ChildStdout>,

    server_capabilities: Option<ServerCapabilities>,
    current_working_directory: PathBuf,
    next_request_id: u64,
    pending_response_requests: HashMap<u64, /* method */ String>,
    screen_message_sender: Sender<ScreenMessage>,

    receiver: Receiver<LspServerProcessMessage>,
    sender: Sender<LspServerProcessMessage>,
}

#[derive(Debug)]
pub enum LspNotification {
    CompletionResponse(CompletionResponse),
}

#[derive(Debug)]
pub enum LspServerProcessMessage {
    FromLspServer(serde_json::Value),
    FromEditor(FromEditor),
}

#[derive(Debug)]
pub enum FromEditor {
    CompletionRequest {
        file_path: PathBuf,
        position: Position,
    },
}

impl LspServerProcess {
    pub fn new(
        command: &str,
        args: Vec<String>,
        screen_message_sender: Sender<ScreenMessage>,
    ) -> anyhow::Result<(Self, Sender<LspServerProcessMessage>)> {
        let mut command = Command::new(command);
        command.stdin(Stdio::piped()).stdout(Stdio::piped());

        args.into_iter().for_each(|arg| {
            command.arg(arg);
        });

        let mut process = command.spawn()?;

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel::<LspServerProcessMessage>();
        let mut lsp_server_process = LspServerProcess {
            process,
            stdin,
            stdout: Some(stdout),
            current_working_directory: std::env::current_dir()?,
            next_request_id: 0,
            pending_response_requests: HashMap::new(),
            server_capabilities: None,
            screen_message_sender,
            receiver,
            sender: sender.clone(),
        };

        lsp_server_process.initialize()?;

        Ok((lsp_server_process, sender))
    }

    fn initialize(&mut self) -> anyhow::Result<()> {
        self.send_request::<lsp_request!("initialize")>(InitializeParams {
            process_id: None,
            root_uri: Some(
                Url::parse(&format!("file://{:?}", self.current_working_directory)).unwrap(),
            ),

            capabilities: ClientCapabilities {
                workspace: Some(WorkspaceClientCapabilities {
                    apply_edit: Some(true),
                    ..WorkspaceClientCapabilities::default()
                }),
                text_document: Some(TextDocumentClientCapabilities {
                    completion: Some(CompletionClientCapabilities {
                        completion_item_kind: Some(CompletionItemKindCapability {
                            value_set: Some(vec![CompletionItemKind::TEXT]),
                        }),
                        ..CompletionClientCapabilities::default()
                    }),
                    ..TextDocumentClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            workspace_folders: None,
            client_info: None,
            ..InitializeParams::default()
        })?;
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

                log::info!("[LspServerProcess] Received reply");
                sender
                    .send(LspServerProcessMessage::FromLspServer(reply))
                    .unwrap_or_else(|error| {
                        log::error!("[LspServerProcess] Error sending reply: {:?}", error);
                    });
            }
        });

        log::info!("[LspServerProcess] Listening for messages from LSP server");
        while let Ok(message) = self.receiver.recv() {
            log::info!("[LspServerProcess] Received message");
            match message {
                LspServerProcessMessage::FromLspServer(json_value) => self.handle_reply(json_value),
                LspServerProcessMessage::FromEditor(from_editor) => {
                    log::info!("[LspServerProcess] FromEditor: {:?}", from_editor);
                    match from_editor {
                        FromEditor::CompletionRequest {
                            file_path,
                            position,
                        } => self.text_document_completion(file_path, position),
                    }
                }
            }
            .unwrap_or_else(|error| {
                log::error!("[LspServerProcess] Error handling reply: {:?}", error);
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
                let method = self.pending_response_requests.remove(&request_id).unwrap();

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
                    }
                    "textDocument/completion" => {
                        let payload: <lsp_request!("textDocument/completion") as Request>::Result =
                            serde_json::from_value(response)?;

                        if let Some(payload) = payload {
                            self.screen_message_sender
                                .send(ScreenMessage::LspNotification(
                                    LspNotification::CompletionResponse(payload),
                                ))
                                .unwrap();
                        }
                    }
                    _ => todo!(),
                }
            }

            // reply is Notification
            Some(method) => {
                let method = method.as_str().unwrap();
                match method {
                    _ => log::info!("Notification: {}", method),
                }
            }
        }

        Ok(())
    }

    pub fn text_document_completion(
        &mut self,
        file_path: PathBuf,
        position: Position,
    ) -> anyhow::Result<()> {
        let result =
            self.send_request::<lsp_request!("textDocument/completion")>(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    position,
                    text_document: TextDocumentIdentifier {
                        uri: Url::parse(&format!(
                            "file://{}",
                            file_path.canonicalize()?.display()
                        ))?,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams {
                    work_done_token: None,
                },
                partial_result_params: PartialResultParams {
                    partial_result_token: None,
                },
                context: Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: None,
                }),
            })?;

        log::info!("{:?}", result);
        Ok(())
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.send_request::<lsp_request!("shutdown")>(())?;
        Ok(())
    }

    fn send_notification<N: Notification>(&mut self, params: N::Params) -> anyhow::Result<()> {
        let notification = json_rpc_types::Request {
            id: None,
            jsonrpc: json_rpc_types::Version::V2,
            method: N::METHOD,
            params: Some(params),
        };

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
    fn send_request<R: Request>(&mut self, params: R::Params) -> anyhow::Result<()>
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

        self.pending_response_requests
            .insert(id, R::METHOD.to_string());

        Ok(())
    }
}
