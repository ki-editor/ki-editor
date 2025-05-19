import * as vscode from "vscode";
import { IPC } from "./ipc";
import { Logger } from "./logger";
import type { InputMessage } from "./protocol/InputMessage";
import type { OutputMessage } from "./protocol/OutputMessage";

// Helpertesttest type test to Oukik map OutputMessage tags to their parameter types
type OutputMessageParamsMap = {
    [K in OutputMessage["tag"]]: Extract<OutputMessage, { tag: K }>["params"];
};

// Helper type to map InputMessage tags to their parameter types
type InputMessageParamsMap = {
    // For each tag K in InputMessage, extract the specific message type O.
    // If O has a 'params' property, infer its type P, otherwise use 'undefined'.
    [K in InputMessage["tag"]]: Extract<InputMessage, { tag: K }> extends infer O
        ? O extends { params: infer P }
            ? P
            : undefined
        : never;
};

export type EventName =
    | "document.open"
    | "document.close"
    | "document.change"
    | "document.save"
    | "editor.active"
    | "editor.selection"
    | "editor.visibleRanges"
    | "diagnostics.change";

export type EventParams<T extends EventName> = T extends "document.open" | "document.save" | "document.close"
    ? { document: vscode.TextDocument }
    : T extends "document.change"
      ? { event: vscode.TextDocumentChangeEvent }
      : T extends "editor.active"
        ? { editor: vscode.TextEditor | undefined }
        : T extends "editor.selection"
          ? { event: vscode.TextEditorSelectionChangeEvent }
          : T extends "editor.visibleRanges"
            ? { event: vscode.TextEditorVisibleRangesChangeEvent }
            : T extends "diagnostics.change"
              ? { uri: vscode.Uri; diagnostics: vscode.Diagnostic[] }[]
              : never;

/**
 * Handles event dispatching between VSCode and Ki
 */
export class Dispatcher implements vscode.Disposable {
    private ipc: IPC;
    private logger: Logger;
    // Removed unused handlers map
    private eventHandlers: Map<EventName, ((params: EventParams<EventName>) => void)[]> = new Map();
    private successHandlers: Map<string, () => void> = new Map(); // Track which operations need notification on success
    private vscodeDisposables: vscode.Disposable[] = [];
    private lastRequestMethod: string | null = null;
    private kiNotificationHandlers: Map<string, ((params: unknown) => Promise<void> | void)[]> = new Map();

    constructor(ipc: IPC, logger: Logger) {
        this.ipc = ipc;
        this.logger = logger;
        this.setupEventListeners();
    }

    /**
     * Register a handler for a Ki notification
     */
    public registerKiNotificationHandler<M extends keyof OutputMessageParamsMap>(
        method: M,
        handler: (params: OutputMessageParamsMap[M]) => Promise<void> | void,
    ): void {
        let handlers = this.kiNotificationHandlers.get(method);
        if (handlers) {
            const genericHandler = handler as (params: unknown) => Promise<void> | void;
            handlers.push(genericHandler);
            this.logger.log(`Registered additional handler for ${method}`);
        } else {
            this.kiNotificationHandlers.set(method, [handler as (params: unknown) => Promise<void> | void]);
            this.logger.log(`Registered first handler for ${method}`);
        }
    }

    /**
     * Register an event handler
     */
    public registerEventHandler<T extends EventName>(event: T, handler: (params: EventParams<T>) => void): void {
        let handlers = this.eventHandlers.get(event);
        if (handlers) {
            handlers.push(handler as any);
        } else {
            this.eventHandlers.set(event, [handler as any]);
        }
    }

    /**
     * Register a success handler for a specific operation
     */
    public registerSuccessHandler(method: string, handler: () => void): void {
        this.successHandlers.set(method, handler);
    }

    /**
     * Set up event listeners for IPC and VSCode
     */
    private setupEventListeners(): void {
        // Listen for notifications (OutputMessage) from Ki
        this.ipc.on("notification", (message: OutputMessage | undefined) => {
            if (!message) {
                this.logger.warn("Received undefined notification message from IPC.");
                return;
            }
            const { tag, params } = message;

            // Log only important notifications to avoid noise
            if (
                tag.includes("error") ||
                tag.includes("mode") ||
                tag.includes("start") ||
                tag.includes("ready") ||
                tag === "success" // Keep success check just in case, though IPC should filter ID=0 success
            ) {
                this.logger.log(`Received Ki notification: ${tag}`);
            }

            // Note: Success message handling with ID=0 is now done in IPC.processMessage
            // No need for the specific 'success' check here anymore regarding lastRequestMethod.

            this.processKiNotification(tag, params);
        });

        // Listen for basic VSCode events
        this.registerVSCodeEventListeners();
    }

    // Removed unused handleSuccessMessage method

    /**
     * Register VSCode event listeners
     */
    private registerVSCodeEventListeners(): void {
        // Text document events
        this.vscodeDisposables.push(
            vscode.workspace.onDidOpenTextDocument((document) => this.emitVSCodeEvent("document.open", { document })),
            vscode.workspace.onDidCloseTextDocument((document) => this.emitVSCodeEvent("document.close", { document })),
            vscode.workspace.onDidChangeTextDocument((event) => this.emitVSCodeEvent("document.change", { event })),
            vscode.workspace.onDidSaveTextDocument((document) => this.emitVSCodeEvent("document.save", { document })),
            vscode.languages.onDidChangeDiagnostics((event) =>
                this.emitVSCodeEvent(
                    "diagnostics.change",
                    event.uris.map((uri) => ({ uri, diagnostics: vscode.languages.getDiagnostics(uri) })),
                ),
            ),
        );

        // Editor events
        this.vscodeDisposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => this.emitVSCodeEvent("editor.active", { editor })),
            vscode.window.onDidChangeTextEditorSelection((event) =>
                this.emitVSCodeEvent("editor.selection", { event }),
            ),
            vscode.window.onDidChangeTextEditorVisibleRanges((event) => {
                // This condition is necessary, otherwise the visible ranges changes
                // of non-file editor, say, Output, will also be sent to Ki
                if (event.textEditor.document.uri.scheme !== "file") {
                    return;
                }
                return this.emitVSCodeEvent("editor.visibleRanges", { event });
            }),
        );

        // Keyboard input event registration is handled separately
        // in the keyboard handler
    }

    /**
     * Emit a VSCode event to registered handlers
     */
    private emitVSCodeEvent<T extends EventName>(eventName: T, params: EventParams<T>): void {
        // Handle important notifications
        if (eventName.startsWith("cursor.") || eventName.startsWith("selection.") || eventName.startsWith("mode.")) {
            this.logger.log(`Emitting VSCode event: ${eventName}`);
        }

        // Get handlers for this event
        const handlers = this.eventHandlers.get(eventName) || [];

        // Call each handler
        for (const handler of handlers) {
            try {
                handler(params);
            } catch (error) {
                this.logger.error(
                    `Error in event handler for ${eventName}: ${
                        error instanceof Error ? error.message : String(error)
                    }`,
                );
            }
        }
    }

    /**
     * Process a notification from Ki
     */
    private processKiNotification(tag: OutputMessage["tag"], params: unknown): void {
        // Special handling for mode changes to ensure they're processed correctly
        if (tag === "mode.change") {
            this.logger.log(`Processing mode change: ${JSON.stringify(params)}`);
        }
        // Special handling for buffer diff to ensure they're processed correctly
        else if (tag === "buffer.diff") {
            this.logger.log(`Processing buffer diff from Ki`);
        }
        // Special handling for selection updates
        else if (tag === "selection.update") {
            this.logger.log(`Processing selection update from Ki: ${JSON.stringify(params)}`);
        }

        // See if we have any handlers for this notification
        const handlers = this.kiNotificationHandlers.get(tag);
        if (handlers && handlers.length > 0) {
            this.logger.log(`Found ${handlers.length} handler(s) for ${tag}`);

            handlers.forEach((handler) => {
                try {
                    handler(params);
                } catch (error) {
                    this.logger.error(
                        `Error in Ki notification handler for ${tag}: ${
                            error instanceof Error ? error.message : String(error)
                        }`,
                    );
                }
            });
        } else {
            this.logger.warn(`No handlers registered for Ki notification: ${tag}`);
        }
    }

    // Removed unused emitKiNotification method

    /**
     * Emit an event from Ki to registered handlers
     * This is the public method that should be used as API for handlers
     */
    public emit<T extends EventName>(event: T, params: EventParams<T>): void {
        // Use the appropriate emitter method based on event source
        this.emitVSCodeEvent(event, params);
    }

    /**
     * Send a notification to Ki
     */
    public sendNotification<M extends InputMessage["tag"]>(tag: M, params?: InputMessageParamsMap[M]): void {
        this.ipc.sendNotification(tag, params);
    }

    /**
     * Send a request to Ki and wait for response
     */
    public sendRequest<M extends InputMessage["tag"]>(tag: M, params?: InputMessageParamsMap[M]): Promise<unknown> {
        this.lastRequestMethod = tag; // Track last request tag for success handler
        return this.ipc.sendRequest(tag, params);
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.vscodeDisposables.forEach((d) => d.dispose());
        this.vscodeDisposables = [];
        // Removed reference to handlers map
        this.successHandlers.clear();
        this.kiNotificationHandlers.clear();
        this.eventHandlers.clear();
    }
}
