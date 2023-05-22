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
use std::sync::{Arc, Mutex};
use std::thread;

struct LspServerProcess {
    process: process::Child,
    stdin: process::ChildStdin,
    server_capabilities: Option<ServerCapabilities>,
    current_working_directory: PathBuf,
    next_request_id: u64,
    pending_response_requests: HashMap<u64, /* method */ String>,
    completion_response: Option<CompletionResponse>,
}

impl LspServerProcess {
    pub fn new(
        command: &str,
        args: Vec<String>,
    ) -> anyhow::Result<(Arc<Mutex<LspServerProcess>>, thread::JoinHandle<()>)> {
        let mut command = Command::new(command);
        command.stdin(Stdio::piped()).stdout(Stdio::piped());

        args.into_iter().for_each(|arg| {
            command.arg(arg);
        });

        let mut process = command.spawn()?;

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();
        let lsp_server_process = Arc::new(Mutex::new(LspServerProcess {
            process,
            stdin,
            current_working_directory: std::env::current_dir()?,
            next_request_id: 0,
            pending_response_requests: HashMap::new(),
            completion_response: None,
            server_capabilities: None,
        }));
        let stdout_join_handle = Self::listen(lsp_server_process.clone(), stdout);
        Ok((lsp_server_process, stdout_join_handle))
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

    fn listen(
        lsp_server_process: Arc<Mutex<LspServerProcess>>,
        stdout: process::ChildStdout,
    ) -> thread::JoinHandle<()> {
        let mut reader = BufReader::new(stdout);
        thread::spawn(move || loop {
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

            log::info!("Content-Length: {}", content_length);
            let mut buffer = vec![0; content_length];
            reader.read_exact(&mut buffer).unwrap();

            let reply = String::from_utf8(buffer).unwrap();

            // Parse as generic JSON value
            let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();

            // Send the JSON value to the LSP server process
            let mut lsp_server_process = lsp_server_process.lock().unwrap();
            lsp_server_process.handle_reply(reply).map_err(|e| {
                log::error!("Handle reply error: {:?}", e);
            });
        })
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

                        self.set_completion_response(payload);
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

    fn text_completion(&mut self) -> anyhow::Result<()> {
        let result =
            self.send_request::<lsp_request!("textDocument/completion")>(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::parse(&format!(
                            "file://{}/{}",
                            self.current_working_directory.display(),
                            "src/lsp/client.rs"
                        ))?,
                    },
                    position: Position {
                        line: 0,
                        character: 0,
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

    fn shutdown(&mut self) -> anyhow::Result<()> {
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

    fn set_completion_response(&mut self, completion_response: Option<CompletionResponse>) {
        self.completion_response = completion_response
    }
}

pub fn run_lsp(cmd: &str) -> anyhow::Result<()> {
    let (lsp_server, join_handle) = LspServerProcess::new(cmd, vec![]).unwrap();
    lsp_server.lock().unwrap().initialize();

    // Sleep for 5 seconds
    std::thread::sleep(std::time::Duration::from_secs(5));

    lsp_server.lock().unwrap().text_completion()?;
    join_handle.join().unwrap();
    panic!("test");
    Ok(())
    // lsp_server.shutdown()
}
