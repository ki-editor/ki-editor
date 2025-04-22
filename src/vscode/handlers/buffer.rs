//! Buffer-related handlers for VSCode IPC messages

use super::prelude::*;
use crate::app::Dispatch;
use crate::buffer::BufferOwner;
use crate::components::editor::DispatchEditor;
use crate::context::Context;
use crate::vscode::app::VSCodeApp;
use crate::vscode::utils::uri_to_path;
use ki_protocol_types::{
    BufferChange, BufferParams, OutputMessage, OutputMessageWrapper, ResponseError,
};
use log::{error, info, warn};

impl VSCodeApp {
    /// Handle buffer open request from VSCode
    pub fn handle_buffer_open_request(&mut self, id: u64, params: BufferParams) -> Result<()> {
        let BufferParams {
            uri,
            content,
            language_id: _,
            version,
        } = params;

        // Convert URI to path
        if let Some(path) = uri_to_path(&uri) {
            // Store the original buffer_id for versioning
            let buffer_id = uri.to_string();

            // Update buffer version
            self.buffer_versions
                .insert(buffer_id.clone(), version.unwrap_or(0) as u64);

            // First check if the file is already open
            let current_path = self.get_current_file_path();
            if current_path.as_ref() == Some(&path) {
                info!("Requested file is already open");
            } else {
                // Open the file
                let dispatch = Dispatch::OpenFile {
                    path: path.clone(),
                    owner: BufferOwner::User,
                    focus: true,
                };
                let open_result = self.app.lock().unwrap().handle_dispatch(dispatch);

                if let Err(e) = open_result {
                    error!("Failed to open file {}: {}", uri, e);
                    // Create the error response
                    let error_message = format!("Failed to open file: {}", e);
                    let response = OutputMessageWrapper {
                        id,
                        message: OutputMessage::Error(error_message.clone()),
                        error: Some(ResponseError {
                            code: 1,
                            message: error_message,
                            data: None,
                        }),
                    };
                    self.send_message_to_vscode(response)?;
                    return Ok(());
                }
            }

            // Set the content if provided
            if let Some(content_val) = content {
                let app_guard = self.app.lock().unwrap();
                let comp = app_guard.current_component();
                let context = Context::new(path.clone());

                // Scope the mutable borrow to avoid borrow checker issues
                {
                    let mut comp_ref = comp.borrow_mut();
                    if let Err(e) = comp_ref.set_content(&content_val, &context) {
                        error!("Failed to set buffer content: {}", e);
                        // Release the app lock before sending the error response
                        drop(app_guard);

                        // Create error message outside any borrows
                        let error_message = format!("Failed to set buffer content: {}", e);
                        let response = OutputMessageWrapper {
                            id,
                            message: OutputMessage::Error(error_message.clone()),
                            error: Some(ResponseError {
                                code: 2,
                                message: error_message,
                                data: None,
                            }),
                        };
                        self.send_message_to_vscode(response)?;
                        return Ok(());
                    }
                }
            }

            // Send success response
            let response = OutputMessageWrapper {
                id,
                message: OutputMessage::Success(true),
                error: None,
            };
            self.send_message_to_vscode(response)?;

            Ok(())
        } else {
            error!("Failed to convert URI to path: {}", uri);
            // Send error response for bad URI
            let error_message = format!("Failed to convert URI to path: {}", uri);
            let response = OutputMessageWrapper {
                id,
                message: OutputMessage::Error(error_message.clone()),
                error: Some(ResponseError {
                    code: 3,
                    message: error_message,
                    data: None,
                }),
            };
            self.send_message_to_vscode(response)?;
            Ok(())
        }
    }

    /// Handle buffer close request from VSCode
    pub fn handle_buffer_close_request(&mut self, id: u64, params: BufferParams) -> Result<()> {
        let BufferParams { uri, .. } = params;
        info!("Buffer closed: uri={}", uri);

        // Convert to CanonicalizedPath
        let _path = match uri_to_path(&uri) {
            Some(p) => p,
            None => {
                error!("Failed to convert URI to path: {}", uri);
                return Ok(());
            }
        };

        // Close the file in Ki
        // Using CloseCurrentWindow instead of CloseFile
        self.app
            .lock()
            .unwrap()
            .handle_dispatch(Dispatch::CloseCurrentWindow)?;

        // Remove from version tracking
        self.buffer_versions.remove(&uri);

        // Send success response
        let response = OutputMessageWrapper {
            id: id,
            message: OutputMessage::Success(true),
            error: None,
        };
        self.send_message_to_vscode(response)?;
        Ok(())
    }

    /// Handle buffer save request from VSCode
    pub fn handle_buffer_save_request(&mut self, id: u64, params: BufferParams) -> Result<()> {
        let BufferParams {
            uri: _,
            content: _,
            language_id: _,
            version: _,
        } = params;
        self.app
            .lock()
            .unwrap()
            .handle_dispatch(Dispatch::ToEditor(DispatchEditor::ForceSave))?;

        // Send success response
        let response = OutputMessageWrapper {
            id: id,
            message: OutputMessage::Success(true),
            error: None,
        };
        self.send_message_to_vscode(response)?;
        Ok(())
    }

    /// Handle buffer active request from VSCode
    pub fn handle_buffer_active_request(&mut self, _id: u64, params: BufferParams) -> Result<()> {
        let BufferParams { uri, .. } = params;
        let buffer_id = uri.to_string();
        info!("Handling buffer.active: {}", buffer_id);

        // Convert URI to path
        if let Some(path) = uri_to_path(&uri) {
            // Use the app's dispatch system to focus the file
            info!("Focusing file through dispatch: {:?}", path);

            let focus_result = self
                .app
                .lock()
                .unwrap()
                .handle_dispatch(Dispatch::OpenFile {
                    path: path.clone(),
                    owner: BufferOwner::User,
                    focus: true,
                });

            match focus_result {
                Ok(_) => {
                    info!("Successfully focused file: {:?}", path);

                    // Immediately check for changes in cursor and selection
                    // This ensures VSCode is up to date with our state
                    if let Err(err) = self.check_for_cursor_changes() {
                        error!("Failed to check for cursor changes: {}", err);
                    }

                    if let Err(err) = self.check_for_selection_changes() {
                        error!("Failed to check for selection changes: {}", err);
                    }

                    Ok(())
                }
                Err(err) => {
                    error!("Failed to focus file {:?}: {}", path, err);
                    Ok(())
                }
            }
        } else {
            warn!("Failed to convert URI to path: {}", uri);
            Ok(())
        }
    }

    /// Handle buffer change request from VSCode
    /// This is treated as a notification (_id might not be relevant for response)
    pub fn handle_buffer_change_request(
        &mut self,
        _id: u64, // Typically unused for notifications/updates like this
        buffer_change: BufferChange,
    ) -> Result<()> {
        // Destructure according to ki-protocol-types/src/lib.rs
        let BufferChange {
            buffer_id,
            start_line,
            end_line,
            content,
            version,
            message_id,     // ID of the message from VSCode side, used for ACK
            retry_count: _, // Currently unused
        } = buffer_change;

        info!(
            "Handling buffer change request: buffer_id={}, version={}, msg_id={}, range=L{}-L{}",
            buffer_id,
            version, // Use version directly
            message_id,
            start_line + 1,
            end_line + 1 // end_line seems inclusive based on common text editor APIs
        );

        // Convert buffer_id (URI) to path
        if let Some(path) = uri_to_path(&buffer_id) {
            let current_path = self.get_current_file_path();

            // Only apply changes if the path matches the current buffer
            // TODO: Decide if we should queue changes for non-active buffers or handle them differently
            if current_path.as_ref() != Some(&path) {
                warn!(
                    "Received buffer change for non-active buffer: {}. Ignoring.",
                    buffer_id
                );
                // Acknowledge receipt even if ignored to prevent retries?
                // Let's send ACK for now.
                let ack_response = OutputMessageWrapper {
                    id: 0, // Use ID 0 for notifications/acknowledgments
                    message: OutputMessage::BufferAck(message_id),
                    error: None,
                };
                self.send_message_to_vscode(ack_response)?;
                return Ok(());
            }

            // Check versioning (using the single 'version' field)
            let current_version = self.buffer_versions.entry(buffer_id.clone()).or_insert(0);
            if (version as u64) <= *current_version {
                warn!(
                    "Ignoring stale buffer update for {}. Current: {}, Received: {}",
                    buffer_id, *current_version, version
                );
                // Send ACK even for stale updates
                let ack_response = OutputMessageWrapper {
                    id: 0, // Use ID 0 for notifications/acknowledgments
                    message: OutputMessage::BufferAck(message_id),
                    error: None,
                };
                self.send_message_to_vscode(ack_response)?;
                return Ok(());
            }
            // Update version only after successful application? Or before? Let's update before.
            *current_version = version as u64;

            // Apply the single change using the component's methods
            debug!("[{}] Attempting to lock core App mutex...", message_id); // Use message_id as trace_id
            let app_guard = self.app.lock().unwrap();
            debug!("[{}] Core App mutex locked.", message_id);

            let comp = app_guard.current_component();
            let context = Context::new(path.clone());

            // Lock the component
            debug!("[{}] Attempting to borrow component mutably...", message_id);
            let mut comp_ref = comp.borrow_mut();
            debug!("[{}] Component borrowed mutably.", message_id);

            // Determine if it's a full content replace or a range replace
            // This needs a clear convention. Let's assume start=0, end=max_lines (or similar) means full replace.
            // For now, we'll rely on a method that can handle the range.
            // We need start/end columns too for precise range replacement.
            // The current BufferChange struct LACKS column info!
            // This is a significant protocol limitation.
            // WORKAROUND: Assume the change applies to the full lines for now.
            warn!("BufferChange protocol lacks column information. Applying change to full lines {}-{}", start_line + 1, end_line + 1);

            // TODO: Implement a proper replace_lines method in the component if it doesn't exist.
            // Using set_content as a placeholder if the range covers the whole document,
            // otherwise, we need a range-based replace.

            // Placeholder logic: Assume set_content for now until range logic is clearer/protocol updated
            if start_line == 0 && end_line == usize::MAX {
                // A potential convention for full replace? Unlikely.
                info!("Applying full content replace (heuristic)");
                if let Err(e) = comp_ref.set_content(&content, &context) {
                    debug!("[{}] Post-set_content (full replace).", message_id);
                    error!("Failed to apply full content change: {}", e);
                    // Don't send ACK on failure? Or send error? Protocol unclear.
                    return Err(anyhow!("Failed to apply full content change: {}", e));
                }
            } else {
                info!("Applying range replace (line-based workaround)");
                debug!("[{}] Pre-set_content (range placeholder).", message_id);
                // This requires a component method like `replace_text_lines` or similar.
                // Let's simulate with `set_content` for now, which is incorrect for ranges.
                // This highlights the need for component/protocol refinement.
                warn!("Using set_content as a placeholder for replace_text_range/lines due to protocol/component limitations.");
                if let Err(e) = comp_ref.set_content(&content, &context) {
                    debug!("[{}] Post-set_content (range placeholder).", message_id);
                    error!(
                        "Failed to apply range content change using set_content placeholder: {}",
                        e
                    );
                    return Err(anyhow!("Failed to apply range content change: {}", e));
                }

                /* // Ideal future implementation (requires component method and possibly protocol update for columns)
                if let Err(e) = comp_ref.replace_text_range(
                    start_line, 0, // Assuming start of start_line
                    end_line, usize::MAX, // Assuming end of end_line (exclusive/inclusive?)
                    &content,
                    &context,
                ) {
                    error!("Failed to apply replace change: {}", e);
                    return Err(anyhow!("Failed to apply replace change: {}", e));
                }
                */
            }

            // Drop guards explicitly before sending ACK to release locks
            drop(comp_ref);
            drop(app_guard);
            debug!("[{}] Locks released.", message_id);

            // Send acknowledgment back to VSCode
            debug!("[{}] Preparing BufferAck response...", message_id);
            let ack_response = OutputMessageWrapper {
                id: 0, // Use ID 0 for notifications/acknowledgments
                message: OutputMessage::BufferAck(message_id),
                error: None,
            };
            debug!("[{}] Sending BufferAck response to VSCode...", message_id);
            self.send_message_to_vscode(ack_response)?;
            debug!("[{}] BufferAck response sent.", message_id);

            // Update last known VSCode selection if applicable
            // This might need refinement based on how selections should behave after buffer changes
            // TODO: Implement this

            Ok(())
        } else {
            error!(
                "Failed to convert URI to path for change request: {}",
                buffer_id
            );
            // Don't ACK if URI is invalid? Or send error?
            // Let's return Err here, VSCode might retry or handle it.
            Err(anyhow!(
                "Invalid URI for buffer change request: {}",
                buffer_id
            ))
        }
    }

    /// Send buffer changes *from* Ki *to* VSCode
    /// This sends an OUTPUT message that acts as a notification.
    pub fn _send_buffer_change(
        &mut self,
        buffer_id: String, // Should be the URI string
        start_line: usize, // Line numbers for the change range
        end_line: usize,   // Line numbers for the change range
        content: String,   // The new content for the specified range/lines
    ) -> Result<()> {
        info!(
            "Sending buffer change notification to VSCode: buffer_id={}, range=L{}-L{}, content_len={}",
            buffer_id,
            start_line + 1,
            end_line + 1, // end_line convention needs clarity (inclusive/exclusive?)
            content.len()
        );

        // Increment version, releasing the borrow immediately
        let version_num = {
            let version = self.buffer_versions.entry(buffer_id.clone()).or_insert(0);
            *version += 1;
            *version // Return the new version number
        };

        // Construct the BufferChange payload matching ki-protocol-types
        // We need a message_id for the VSCode side to potentially ACK. Let's generate one.
        let ki_message_id = self.next_id(); // Generate an ID for *this* Ki->VSCode message

        let buffer_change_payload = BufferChange {
            buffer_id: buffer_id.clone(),
            start_line,
            end_line,
            content: content.clone(),
            version: version_num as i32, // Send updated version
            message_id: ki_message_id,   // ID for this specific change message
            retry_count: 0,              // Initial send
        };

        // Send as a notification (using id=0 convention established in app.rs)
        // The message *payload* (BufferChange) contains its own message_id (ki_message_id).
        let notification_wrapper = OutputMessageWrapper {
            id: 0, // id=0 signifies notification (no specific response expected for *this* wrapper ID)
            message: OutputMessage::BufferChange(buffer_change_payload), // Use BufferChange variant
            error: None,
        };

        // Use the standard send method, which handles id=0 as notification
        self.send_message_to_vscode(notification_wrapper)?;

        Ok(())
    }
}
