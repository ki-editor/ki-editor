import * as cp from "child_process";
import { EventEmitter } from "events";
import { cwd } from "process";
import WebSocket from "ws"; // Import WebSocket library
import { Logger } from "./logger";
import type { InputMessage, InputMessageWrapper, OutputMessage, OutputMessageWrapper } from "./protocol/types";

// Helper type to map InputMessage tags to their parameter types
type InputMessageParamsMap = {
    [K in InputMessage["tag"]]: Extract<InputMessage, { tag: K }> extends {
        params: infer P;
    }
        ? P
        : undefined;
};

// Define the set of message tags that are expected as unsolicited notifications
// These are message types that the backend might send without a prior request from the frontend.
const KNOWN_NOTIFICATION_TAGS: ReadonlySet<OutputMessage["tag"]> = new Set([
    "buffer.diff", // Added buffer.diff to handle buffer synchronization from Ki
    "selection.update", // Now includes cursor information
    "mode.change", // Example: backend informs frontend of mode change
    "selection_mode.change", // Example: backend informs frontend of selection mode change
    "search.results", // Added search.results for completeness
    "editor.action", // Added editor.action for undo/redo operations
    "buffer.activated",
    "buffer.open",
    "buffer.close",
    "buffer.save",
    "error",
    "ping",
    "ki.log",
    "success",
    "external_buffer.created",
    "external_buffer.updated",
    "command.executed",
    "viewport.change",
    "editor.jump",
    "editor.mark",
    "prompt.opened",
    // Add any other tags that are definitely notifications
]);

/**
 * Unified IPC module for communication with Ki editor using WebSockets
 */
export class IPC extends EventEmitter {
    private process: cp.ChildProcess | null = null;
    private logger: Logger;
    private webSocket: WebSocket | null = null; // WebSocket client instance
    private nextMessageId = 1;
    private pendingRequests = new Map<
        number,
        { resolve: (value: unknown) => void; reject: (reason: unknown) => void }
    >();
    private port: number | null = null; // Port the Rust server is listening on
    private connectionPromise: Promise<void> | null = null;
    private resolveConnectionPromise: (() => void) | null = null;
    private rejectConnectionPromise: ((reason?: unknown) => void) | null = null;

    constructor(logger: Logger) {
        super();
        this.logger = logger;
    }

    /**
     * Start the Ki process, find its WebSocket port, and establish communication
     */
    public start(command: string, args: string[]): void {
        this.logger.log(`Starting Ki process for WebSocket: ${command} ${args.join(" ")}`);

        // Reset connection state
        this.port = null;
        this.webSocket = null;
        this.connectionPromise = new Promise<void>((resolve, reject) => {
            this.resolveConnectionPromise = resolve;
            this.rejectConnectionPromise = reject;
        });

        try {
            // Log execution environment
            this.logger.log(`Current working directory: ${process.cwd()}`);
            this.logger.log(`Executable path: ${command}`);
            this.logger.log(`Environment: NODE_ENV=${process.env.NODE_ENV}`);

            // Spawn process
            this.process = cp.spawn(command, args, {
                stdio: ["pipe", "pipe", "pipe"], // Keep pipes for stdout/stderr
                env: {
                    ...process.env,
                    RUST_LOG: "trace",
                },
                shell: false,
                windowsHide: true,
            });

            if (!this.process || !this.process.pid) {
                throw new Error("Failed to start Ki process: spawn returned invalid process");
            }

            this.logger.log(`Ki process started with PID: ${this.process.pid}`);

            // --- Listen to stdout for the port number ---
            let stdoutBuffer = "";
            if (this.process.stdout) {
                this.process.stdout.on("data", (data) => {
                    if (this.port) return; // Already found port

                    stdoutBuffer += data.toString();
                    this.logger.log(`Ki stdout chunk: ${data.toString().trim()}`); // Log chunk
                    const match = stdoutBuffer.match(/KI_LISTENING_ON=(\d+)/);
                    if (match && match[1]) {
                        this.port = parseInt(match[1], 10);
                        this.logger.log(`Found Ki WebSocket port: ${this.port}`);
                        // Stop listening to stdout once port is found
                        this.process?.stdout?.removeAllListeners("data");
                        this.connectWebSocket(); // Attempt connection
                    }
                    // Prevent buffer from growing indefinitely if port not found quickly
                    if (stdoutBuffer.length > 1024) {
                        stdoutBuffer = stdoutBuffer.slice(-1024);
                    }
                });
                this.process.stdout.on("close", () => {
                    if (!this.port) {
                        this.logger.error("Ki process stdout closed before port was reported.");
                        this.rejectConnectionPromise?.(new Error("stdout closed before port reported"));
                    }
                });
            } else {
                throw new Error("Ki process stdout is null");
            }
            // -------------------------------------------

            // Set up stderr handling (unchanged)
            if (this.process.stderr) {
                this.process.stderr.on("data", (data) => {
                    this.logger.error(`Ki stderr: ${data.toString().trim()}`);
                });
            } else {
                this.logger.error("Ki process stderr is null");
            }

            // Handle process errors (unchanged)
            this.process.on("error", (err) => {
                this.logger.error(`Ki process error: ${err.message}`);
                this.rejectConnectionPromise?.(err);
                this.emit("error", err);
                this.cleanupWebSocket();
            });

            // Handle process exit (unchanged, but cleanup WebSocket)
            this.process.on("exit", (code, signal) => {
                const exitMsg = `Ki process exited with code ${code}, signal: ${signal || "none"}`;
                if (code !== 0 && code !== null) {
                    this.logger.error(exitMsg);
                    this.rejectConnectionPromise?.(new Error(exitMsg));
                } else {
                    this.logger.log(exitMsg);
                    // Resolve if exit was clean and we didn't connect?
                    // Or maybe reject? Let's reject if not connected.
                    if (!this.webSocket?.readyState || this.webSocket.readyState !== WebSocket.OPEN) {
                        this.rejectConnectionPromise?.(new Error("Process exited before WebSocket connection"));
                    }
                }
                this.emit("exit", code);
                this.cleanupWebSocket();
            });

            this.logger.log("Ki process listeners attached.");
        } catch (err) {
            this.logger.error(`Failed to start or monitor Ki process: ${err}`);
            this.process = null;
            this.rejectConnectionPromise?.(err);
            throw err; // Re-throw after attempting cleanup/rejection
        }
    }

    /**
     * Connects to the WebSocket server once the port is known.
     */
    private connectWebSocket(): void {
        if (!this.port) {
            this.logger.error("Cannot connect WebSocket: Port not found.");
            this.rejectConnectionPromise?.(new Error("Port not found"));
            return;
        }

        const wsUrl = `ws://localhost:${this.port}`;
        this.logger.log(`Attempting to connect WebSocket to: ${wsUrl}`);

        try {
            this.webSocket = new WebSocket(wsUrl);

            this.webSocket.on("open", () => {
                this.logger.log("WebSocket connection established.");
                this.resolveConnectionPromise?.(); // Signal successful connection
                this.emit("ready"); // Emit ready event for handlers
            });

            this.webSocket.on("message", (data: WebSocket.Data) => {
                this.handleWebSocketData(data);
            });

            this.webSocket.on("error", (err: Error) => {
                this.logger.error(`WebSocket error: ${err.message}`);
                this.rejectConnectionPromise?.(err); // Signal connection failure
                this.emit("error", err);
                this.cleanupWebSocket();
            });

            this.webSocket.on("close", (code: number, reason: Buffer) => {
                this.logger.log(`WebSocket connection closed: code=${code}, reason=${reason.toString()}`);
                this.emit("close");
                // If connection promise was still pending, reject it
                this.rejectConnectionPromise?.(new Error(`WebSocket closed: ${code}`));
                this.cleanupWebSocket();
            });
        } catch (err) {
            this.logger.error(`Failed to create WebSocket client: ${err}`);
            this.rejectConnectionPromise?.(err);
            this.cleanupWebSocket();
        }
    }

    /**
     * Cleans up WebSocket resources.
     */
    private cleanupWebSocket(): void {
        if (this.webSocket) {
            // Remove all listeners to prevent further events
            this.webSocket.removeAllListeners();
            if (this.webSocket.readyState === WebSocket.OPEN) {
                this.webSocket.close();
            }
            this.webSocket = null;
        }
        // Ensure connection promise is settled if it wasn't already
        this.rejectConnectionPromise?.(new Error("WebSocket cleanup occurred"));
        this.connectionPromise = null;
        this.resolveConnectionPromise = null;
        this.rejectConnectionPromise = null;
    }

    /**
     * Handle data received from the WebSocket connection
     */
    private handleWebSocketData(data: WebSocket.Data): void {
        let messageStr: string;
        if (Buffer.isBuffer(data)) {
            messageStr = data.toString("utf8");
        } else if (data instanceof ArrayBuffer) {
            messageStr = Buffer.from(data).toString("utf8");
        } else if (Array.isArray(data)) {
            // Should not happen with standard ws library, but handle just in case
            messageStr = Buffer.concat(data).toString("utf8");
        } else {
            this.logger.error(`Received unexpected WebSocket data type: ${typeof data}`);
            return;
        }

        this.logger.log(
            `WebSocket received (len=${messageStr.length}): ${messageStr.substring(0, 200)}${
                messageStr.length > 200 ? "..." : ""
            }`,
        );

        try {
            const message = JSON.parse(messageStr) as OutputMessageWrapper;
            this.processMessage(message);
        } catch (err) {
            this.logger.error(`Failed to parse WebSocket message JSON: ${err}`);
        }
    }

    /**
     * Process an incoming message (Now received via WebSocket)
     * Robustly handles responses and notifications.
     */
    private processMessage(message: OutputMessageWrapper): void {
        const { id, message: outputMsg, error } = message;
        const tag = outputMsg?.tag;

        this.logger.log(`processMessage: Received ID=${id}, Tag=${tag ?? "N/A"}, Error=${!!error}`);

        // 1. Check if it's a response to a pending request (non-zero ID and matches)
        if (id > 0 && this.pendingRequests.has(id)) {
            const pendingRequest = this.pendingRequests.get(id)!;
            this.pendingRequests.delete(id);

            if (error) {
                this.logger.error(`processMessage: Rejecting pending request ID ${id} due to error: ${error.message}`);
                pendingRequest.reject(error);
            } else {
                this.logger.log(`processMessage: Resolving pending request ID ${id} with message tag: ${tag}`);
                pendingRequest.resolve(outputMsg); // Resolve with the message part
            }
            return; // Handled as a response
        }

        // 2. If ID is 0 OR doesn't match a pending request, treat as potential notification
        if (id === 0 || id > 0 /* but not found in pendingRequests */) {
            if (id > 0) {
                // Log if we received a non-zero ID that wasn't pending (likely backend error)
                this.logger.warn(
                    `processMessage: Received message with ID ${id} but no matching pending request was found.`,
                );
            }

            if (outputMsg && KNOWN_NOTIFICATION_TAGS.has(outputMsg.tag)) {
                // It's a known notification type
                this.logger.log(
                    `processMessage: Treating message ID ${id} (Tag=${outputMsg.tag}) as notification. Emitting 'notification' event.`,
                );
                this.emit("notification", outputMsg); // Emit the message part
            } else {
                // Unknown tag for a notification OR an error/success message sent incorrectly
                this.logger.warn(
                    `processMessage: Discarding message with ID ${id} and Tag=${
                        tag ?? "N/A"
                    }. Not a pending response and not a known notification type.`,
                );
            }
            return; // Handled (or discarded) as potential notification
        }

        // Should not happen if logic above is correct, but log just in case
        this.logger.error(
            `processMessage: Unhandled message condition for ID ${id}, Tag=${tag ?? "N/A"}, Error=${!!error}`,
        );
    }

    /**
     * Send a notification message to the Ki process (no response expected).
     */
    public sendNotification<M extends InputMessage["tag"]>(
        tag: M,
        params?: InputMessageParamsMap[M], // Made params optional
    ): void {
        // Construct the core InputMessage object based on the tag and params
        const notification = {
            tag: tag,
            ...(params !== undefined && { params: params }),
        } as InputMessage; // Cast needed because TS can't perfectly map the conditional params

        // Send without expecting a response (pass null for requestId)
        this.sendMessageToProcess(notification, null);
    }

    /**
     * Send a request message to the Ki process and return a promise for the response.
     */
    public sendRequest<M extends InputMessage["tag"]>(
        tag: M,
        params?: InputMessageParamsMap[M], // Made params optional
    ): Promise<unknown> {
        return new Promise((resolve, reject) => {
            const id = this.nextMessageId++;
            if (!this.webSocket || this.webSocket.readyState !== WebSocket.OPEN) {
                this.logger.log(`sendRequest ID=${id}: WebSocket not ready. Waiting for connection...`); // Use log instead of info
                // Queue the request to be sent once connected - Reinstated logic
                if (this.connectionPromise) {
                    this.connectionPromise
                        .then(() => {
                            this.logger.log(`sendRequest: Queueing request ID=${id}, Tag=${tag}`); // Use log instead of info
                            this._sendRequestInternal(tag, params, id, resolve, reject);
                        })
                        .catch((err: unknown) => {
                            // Add type unknown
                            this.logger.error(`Connection failed before request ID=${id} could be sent: ${err}`);
                            reject(new Error(`Connection failed: ${err}`));
                        });
                } else {
                    // Should not happen if connect() was called, but handle defensively
                    this.logger.error(`Cannot send request ID=${id}: Connection promise is null.`);
                    reject(new Error("IPC not initialized or connection failed."));
                }
            } else {
                // Send immediately if already connected
                this._sendRequestInternal(tag, params, id, resolve, reject);
            }
        });
    }

    // Internal helper to actually send the request
    private _sendRequestInternal<M extends InputMessage["tag"]>(
        tag: M,
        params: InputMessageParamsMap[M] | undefined, // Update type to include undefined
        id: number,
        resolve: (value: unknown) => void,
        reject: (reason?: unknown) => void,
    ): void {
        // Construct the core InputMessage object
        const request = {
            tag: tag,
            ...(params !== undefined && { params: params }),
        } as InputMessage; // Cast needed

        this.pendingRequests.set(id, { resolve, reject });
        // Pass the actual request ID to sendMessageToProcess
        this.sendMessageToProcess(request, id);
    }

    /**
     * Send a message to the Ki process via WebSocket
     */
    private sendMessageToProcess(message: InputMessage, requestId: number | null): boolean {
        if (this.webSocket && this.webSocket.readyState === WebSocket.OPEN) {
            // Determine the ID for the wrapper. Use requestId if provided (for requests), otherwise use 0 (for notifications).
            const wrapperId = requestId ?? 0;

            // Wrap the message according to the expected InputMessageWrapper structure
            const wrappedMessage: InputMessageWrapper = {
                message: message,
                id: wrapperId,
            }; // Now strongly typed and includes the wrapper ID
            const messageString = JSON.stringify(wrappedMessage);
            this.logger.log(
                `WebSocket sending (WrapperID=${wrapperId}, len=${messageString.length}): ${messageString.substring(
                    0,
                    200,
                )}${messageString.length > 200 ? "..." : ""}`,
            );
            this.webSocket.send(messageString);
            return true;
        } else {
            this.logger.error(
                `Cannot send message Tag=${message.tag}${
                    requestId ? ` (RequestID=${requestId})` : ""
                }: WebSocket not connected or not open.`,
            );
            return false;
        }
    }

    /**
     * Stop the Ki process and WebSocket connection
     */
    public stop(): void {
        this.logger.log("Stopping IPC...");
        this.cleanupWebSocket(); // Close WebSocket and reject pending promises

        if (this.process) {
            this.logger.log("Killing Ki process...");
            this.process.kill();
            this.process = null;
        }

        // Clear any remaining pending requests just in case cleanupWebSocket didn't catch them
        for (const [id, request] of this.pendingRequests.entries()) {
            request.reject(new Error("IPC stopped"));
            this.pendingRequests.delete(id);
        }
        this.logger.log("IPC stopped.");
    }

    /**
     * Check if the IPC connection is active (WebSocket is open)
     */
    public isRunning(): boolean {
        return this.webSocket !== null && this.webSocket.readyState === WebSocket.OPEN;
    }
}
