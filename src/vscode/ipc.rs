//! WebSocket IPC communication with VSCode

use ki_protocol_types::{InputMessage, InputMessageWrapper, MessageMethod, OutputMessageWrapper};
use log::{debug, error, info, trace, warn};
use serde_json;
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use thiserror::Error;
use tungstenite::{
    accept, error::Error as WsError, protocol::WebSocket, HandshakeError, Message as WsMessage,
};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("Network IO error: {0}")]
    Network(#[from] std::io::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] WsError),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Channel send error: {0}")]
    SendError(String),

    #[error("Channel receive error: {0}")]
    RecvError(#[from] mpsc::RecvError),
}

// Helper type alias for the WebSocket stream
type WsStream = WebSocket<TcpStream>;

/// Manages the WebSocket IPC communication with the VSCode extension.
pub struct WebSocketIpc {
    to_vscode_sender: Sender<OutputMessageWrapper>, // Sends messages *to* the WebSocket thread
    from_vscode_receiver: Receiver<(u64, InputMessage, String)>, // Receives messages *from* the WebSocket thread
    // JoinHandle is stored to ensure the thread is properly waited for on drop
    _handler_thread: Option<thread::JoinHandle<()>>,
}

impl WebSocketIpc {
    /// Sets up WebSocket IPC.
    /// Binds a TCP listener, prints the port, and spawns a handler thread.
    pub fn new() -> Result<(Self, u16), IpcError> {
        info!("Setting up VSCode WebSocket IPC...");

        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();

        // Print the port to stdout for the extension to capture
        println!("KI_LISTENING_ON={}", port);
        use std::io::Write;
        // Ensure it gets printed immediately
        std::io::stdout().flush()?;

        info!("WebSocket server listening on port {}", port);

        // Create channels for communication between the handler thread and VSCodeApp
        let (to_main_sender, from_vscode_receiver) = mpsc::channel();
        let (to_vscode_sender, from_main_receiver) = mpsc::channel();

        let handler_thread = thread::spawn(move || {
            info!("WebSocket handler thread started. Waiting for connection...");
            match listener.accept() {
                Ok((stream, addr)) => {
                    info!("Accepted connection from {}", addr);
                    match accept(stream) {
                        Ok(websocket) => {
                            info!("WebSocket handshake successful.");
                            // Pass ownership of channels and websocket to the handler function
                            Self::handle_connection(websocket, to_main_sender, from_main_receiver);
                        }
                        Err(HandshakeError::Failure(f)) => {
                            error!("WebSocket handshake failed: {}", f);
                        }
                        Err(HandshakeError::Interrupted(_)) => {
                            error!("WebSocket handshake interrupted");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    // Signal failure?
                }
            }
            info!("WebSocket handler thread finished.");
        });

        Ok((
            Self {
                to_vscode_sender,
                from_vscode_receiver,
                _handler_thread: Some(handler_thread),
            },
            port,
        ))
    }

    /// The main logic running in the dedicated handler thread.
    fn handle_connection(
        mut websocket: WsStream,
        to_main_sender: Sender<(u64, InputMessage, String)>,
        from_main_receiver: Receiver<OutputMessageWrapper>,
    ) {
        // Set the stream to non-blocking to allow checking both read and channel
        if let Err(e) = websocket.get_mut().set_nonblocking(true) {
            error!("Failed to set WebSocket stream to non-blocking: {}", e);
            return; // Cannot proceed without non-blocking
        }

        loop {
            // 1. Try to read from WebSocket
            match websocket.read() {
                Ok(msg) => {
                    match msg {
                        WsMessage::Text(text) => {
                            trace!("WebSocket received text message (len={})", text.len());
                            match serde_json::from_str::<InputMessageWrapper>(&text) {
                                Ok(wrapper) => {
                                    debug!(
                                        "Parsed message id={}, method={}",
                                        wrapper.id,
                                        wrapper.message.method_name()
                                    );
                                    let trace_id = Uuid::new_v4().to_string();
                                    if let Err(e) =
                                        to_main_sender.send((wrapper.id, wrapper.message, trace_id))
                                    {
                                        error!(
                                            "Failed to send received message to main thread: {}",
                                            e
                                        );
                                        break; // Channel broken, exit thread
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to parse JSON from WebSocket: {}, raw: {}",
                                        e,
                                        text.chars().take(200).collect::<String>()
                                    );
                                    // Don't break, just log the error and continue
                                }
                            }
                        }
                        WsMessage::Binary(_) => warn!("Received unexpected binary message"),
                        WsMessage::Ping(_) => trace!("Received ping"),
                        WsMessage::Pong(_) => trace!("Received pong"),
                        WsMessage::Close(_) => {
                            info!("WebSocket close message received. Shutting down connection handler.");
                            break;
                        }
                        WsMessage::Frame(_) => { /* Usually handled internally by tungstenite */ }
                    }
                }
                Err(WsError::Io(ref e)) if e.kind() == ErrorKind::WouldBlock => {
                    // No message available right now, proceed to check the channel
                }
                Err(e) => {
                    // Handle other potential WebSocket errors
                    match e {
                        WsError::ConnectionClosed | WsError::AlreadyClosed => {
                            info!("WebSocket connection closed.");
                        }
                        WsError::Io(e) => {
                            error!("WebSocket IO error: {}", e);
                        }
                        _ => {
                            error!("WebSocket error: {}", e);
                        }
                    }
                    break; // Assume fatal error for the connection
                }
            }

            // 2. Try to receive message from main thread to send to VSCode
            match from_main_receiver.try_recv() {
                Ok(wrapper_to_send) => {
                    let id = wrapper_to_send.id;
                    let message_type = format!("{:?}", wrapper_to_send.message);

                    debug!("WebSocket handler: Received message from main thread to send to VSCode: id={}, type={}",
                        id, message_type);

                    match serde_json::to_string(&wrapper_to_send) {
                        Ok(serialized) => {
                            debug!(
                                "WebSocket handler: Serialized message: id={}, type={}, length={}",
                                id,
                                message_type,
                                serialized.len()
                            );

                            match websocket.send(WsMessage::Text(serialized)) {
                                Ok(_) => {
                                    info!(
                                        "WebSocket handler: Successfully sent message to VSCode: id={}, type={}",
                                        id, message_type
                                    );
                                }
                                Err(e) => {
                                    error!("WebSocket handler: Failed to write message to WebSocket: id={}, type={}, error={}",
                                        id, message_type, e);
                                    // If write fails, assume connection is broken
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("WebSocket handler: Failed to serialize message for WebSocket: id={}, type={}, error={}",
                                id, message_type, e);
                            // Log error but don't break, maybe the next message is fine
                        }
                    }
                }
                Err(TryRecvError::Empty) => {
                    // No message from main thread, continue loop
                }
                Err(TryRecvError::Disconnected) => {
                    info!("WebSocket handler: Main thread channel disconnected. Shutting down connection handler.");
                    break; // Channel broken, exit thread
                }
            }

            // Small sleep to prevent busy-waiting if neither read nor write occurs
            thread::sleep(std::time::Duration::from_millis(5));
        }

        // Cleanup: Close the WebSocket connection if possible
        let _ = websocket.close(None);
        info!("WebSocket connection handler terminated.");
    }

    /// Sends a message to the VSCode extension via the handler thread.
    pub fn send_message_to_vscode(&self, message: OutputMessageWrapper) -> Result<(), IpcError> {
        let id = message.id;
        let message_type = format!("{:?}", message.message);

        debug!(
            "WebSocketIpc: Sending message to VSCode: id={}, type={}",
            id, message_type
        );

        match self.to_vscode_sender.send(message) {
            Ok(_) => {
                debug!(
                    "WebSocketIpc: Successfully sent message to handler thread: id={}, type={}",
                    id, message_type
                );
                Ok(())
            }
            Err(e) => {
                error!("WebSocketIpc: Failed to send message to handler thread: id={}, type={}, error={}",
                    id, message_type, e);
                Err(IpcError::SendError(e.to_string()))
            }
        }
    }

    /// Receives a message from the VSCode extension via the handler thread.
    /// This is non-blocking.
    pub fn try_receive_from_vscode(&self) -> Result<(u64, InputMessage, String), TryRecvError> {
        self.from_vscode_receiver.try_recv()
    }
}

impl Drop for WebSocketIpc {
    fn drop(&mut self) {
        info!("Dropping WebSocketIpc. Waiting for handler thread to join...");
        // Take the handle to join it. If it's already None, it means it panicked or finished.
        if let Some(handle) = self._handler_thread.take() {
            if let Err(e) = handle.join() {
                error!("WebSocket handler thread panicked: {:?}", e);
            }
        }
        info!("WebSocketIpc dropped.");
    }
}
