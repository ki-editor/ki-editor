use lsp_types::notification::Notification;
use lsp_types::request::Request;
use lsp_types::{
    lsp_notification, lsp_request, ClientCapabilities, CompletionClientCapabilities,
    CompletionContext, CompletionItemKind, CompletionItemKindCapability, CompletionParams,
    CompletionTriggerKind, GeneralClientCapabilities, InitializeParams, InitializedParams,
    PartialResultParams, Position, ServerCapabilities, TextDocumentClientCapabilities,
    TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams,
    WorkspaceClientCapabilities,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};

struct LspServerProcess {
    process: process::Child,
    stdin: process::ChildStdin,
    stdout: process::ChildStdout,
    server_capabilities: Option<ServerCapabilities>,
    current_working_directory: PathBuf,
}

impl LspServerProcess {
    pub fn new(command: &str, args: Vec<String>) -> anyhow::Result<LspServerProcess> {
        let mut command = Command::new(command);
        command.stdin(Stdio::piped()).stdout(Stdio::piped());

        args.into_iter().for_each(|arg| {
            command.arg(arg);
        });

        let mut process = command.spawn()?;

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();
        Ok(LspServerProcess {
            process,
            stdin,
            stdout,
            server_capabilities: None,
            current_working_directory: std::env::current_dir()?,
        })
    }

    fn initialize(&mut self) -> anyhow::Result<()> {
        let result = self.send_request::<lsp_request!("initialize")>(InitializeParams {
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
        self.server_capabilities = Some(result.capabilities);

        self.send_notification::<lsp_notification!("initialized")>(InitializedParams {})?;
        println!("Initialized");
        Ok(())
    }

    fn text_completion(&mut self) -> anyhow::Result<()> {
        let result =
            self.send_request::<lsp_request!("textDocument/completion")>(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: Url::parse(&format!(
                            "file://{:?}{}",
                            self.current_working_directory, "src/lsp/client.rs"
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

        println!("{:?}", result);
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

    fn send_request<R: Request>(&mut self, params: R::Params) -> anyhow::Result<R::Result>
    where
        R::Params: serde::Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        // Convert the request to a JSON-RPC message
        let request = json_rpc_types::Request {
            jsonrpc: json_rpc_types::Version::V2,
            method: R::METHOD,
            params: Some(params),
            id: Some(json_rpc_types::Id::Num(1)),
        };

        let message = serde_json::to_string(&request)?;

        println!("Sending request: {}", message);

        // The message format is according to https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#contentPart
        write!(
            &mut self.stdin,
            "Content-Length: {}\r\n\r\n{}",
            message.len(),
            message
        )?;

        // Read the response from the LSP process
        let mut reader = BufReader::new(&mut self.stdout);
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

        println!("Content-Length: {}", content_length);
        let mut buffer = vec![0; content_length];
        reader.read_exact(&mut buffer).unwrap();

        let response = String::from_utf8(buffer).unwrap();

        println!("Response: {}", response);

        // Deserialize the response
        serde_json::from_str::<
            json_rpc_types::Response<
                R::Result,
                (),
                // Need to specify String here
                // Otherwise the default will be `str_buf::StrBuf<31>`,
                // which says the error message can only be 31 bytes long.
                String,
            >,
        >(&response)
        .map_err(|e| anyhow::anyhow!("Serde error = {:?}", e))?
        .payload
        .map_err(|e| {
            anyhow::anyhow!(
                "LSP JSON-RPC Error: Code={:?} Message={}",
                e.code,
                e.message
            )
        })
    }
}

pub fn run_lsp(cmd: &str) -> anyhow::Result<()> {
    let mut lsp_server = LspServerProcess::new(cmd, vec![]).unwrap();
    lsp_server.initialize()?;

    // Sleep for 5 seconds
    std::thread::sleep(std::time::Duration::from_secs(5));

    lsp_server.text_completion()?;
    panic!("test");
    Ok(())
    // lsp_server.shutdown()
}
