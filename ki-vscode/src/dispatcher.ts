import * as vscode from "vscode";
import { IPC } from "./ipc";
import { Logger } from "./logger";
import type { OutputMessage } from "./protocol/OutputMessage";
import type { InputMessage } from "./protocol/InputMessage";

// Helper type to map OutputMessage tags to their parameter types
type OutputMessageParamsMap = {
    [K in OutputMessage['tag']]: Extract<OutputMessage, { tag: K }>['params'];
};

// Helper type to map InputMessage tags to their parameter types
type InputMessageParamsMap = {
    // For each tag K in InputMessage, extract the specific message type O.
    // If O has a 'params' property, infer its type P, otherwise use 'undefined'.
    [K in InputMessage['tag']]: Extract<InputMessage, { tag: K }> extends infer O ? (O extends { params: infer P } ? P : undefined) : never;
};

/**
 * Handles event dispatching between VSCode and Ki
 */
export class Dispatcher implements vscode.Disposable {
    private ipc: IPC;
    private logger: Logger;
    private handlers: Map<string, (params: any) => Promise<void> | void> = new Map();
    private eventHandlers: Map<string, ((params: any) => void)[]> = new Map();
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
        handler: (params: OutputMessageParamsMap[M]) => Promise<void> | void
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
    public registerEventHandler(event: string, handler: (params: any) => void): void {
        let handlers = this.eventHandlers.get(event);
        if (handlers) {
            handlers.push(handler);
        } else {
            this.eventHandlers.set(event, [handler]);
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

    /**
     * Handle success message from Ki
     */
    private handleSuccessMessage(method: string): void {
        this.logger.log(`Processing success notification for: ${method}`);

        const successHandler = this.successHandlers.get(method);
        if (successHandler) {
            try {
                successHandler();
            } catch (err) {
                this.logger.error(`Error in success handler for ${method}`, err);
            }
        }

        // Force sync after major operations
        if (
            method.includes("buffer") ||
            method.includes("cursor") ||
            method.includes("selection") ||
            method.includes("mode")
        ) {
            this.emitVSCodeEvent("ki.sync", {});
        }
    }

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
        );

        // Editor events
        this.vscodeDisposables.push(
            vscode.window.onDidChangeActiveTextEditor((editor) => this.emitVSCodeEvent("editor.active", { editor })),
            vscode.window.onDidChangeTextEditorSelection((event) =>
                this.emitVSCodeEvent("editor.selection", { event }),
            ),
            vscode.window.onDidChangeTextEditorVisibleRanges((event) =>
                this.emitVSCodeEvent("editor.visible", { event }),
            ),
        );

        // Keyboard input event registration is handled separately
        // in the keyboard handler
    }

    /**
     * Emit a VSCode event to registered handlers
     */
    private emitVSCodeEvent(eventName: string, params: unknown): void {
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
                this.logger.error(`Error in event handler for ${eventName}: ${error}`);
            }
        }
    }

    /**
     * Process a notification from Ki
     */
    private processKiNotification(tag: OutputMessage['tag'], params: unknown): void {
        // Add detailed logging for cursor-related notifications
        if (tag === "cursor.update") {
            this.logger.log(`Processing cursor update from Ki: ${JSON.stringify(params)}`);
        }
        // Special handling for mode changes to ensure they're processed correctly
        else if (tag === "mode.change") {
            this.logger.log(`Processing mode change: ${JSON.stringify(params)}`);
        }

        // See if we have any handlers for this notification
        const handlers = this.kiNotificationHandlers.get(tag);
        if (handlers && handlers.length > 0) {
            this.logger.log(`Found ${handlers.length} handler(s) for ${tag}`);

            handlers.forEach((handler) => {
                try {
                    handler(params);
                } catch (error) {
                    this.logger.error(`Error in Ki notification handler for ${tag}:`, error);
                }
            });
        } else {
            this.logger.warn(`No handlers registered for Ki notification: ${tag}`);
        }
    }

    /**
     * Emit a Ki notification to registered handlers
     */
    private emitKiNotification(notificationType: string, params: unknown): void {
        // Handle important notifications with detailed logging
        if (notificationType.startsWith("cursor.")) {
            this.logger.log(`Emitting Ki notification: ${notificationType} with params: ${JSON.stringify(params)}`);
        } else if (notificationType.startsWith("selection.") || notificationType.startsWith("mode.")) {
            this.logger.log(`Emitting Ki notification: ${notificationType}`);
        }

        // Get handlers for this notification
        const handlers = this.kiNotificationHandlers.get(notificationType) || [];

        if (handlers.length === 0) {
            this.logger.warn(`No handlers found for notification: ${notificationType}`);
            return;
        }

        // Call each handler
        this.logger.log(`Calling ${handlers.length} handler(s) for ${notificationType}`);
        for (const handler of handlers) {
            try {
                handler(params);
            } catch (error) {
                this.logger.error(`Error in notification handler for ${notificationType}: ${error}`);
            }
        }
    }

    /**
     * Emit an event from Ki to registered handlers
     * This is the public method that should be used as API for handlers
     */
    public emit(event: string, params: unknown): void {
        // Use the appropriate emitter method based on event source
        this.emitVSCodeEvent(event, params);
    }

    /**
     * Send a notification to Ki
     */
    public sendNotification<M extends InputMessage['tag']>(
        tag: M,
        params?: InputMessageParamsMap[M]
    ): void {
        this.ipc.sendNotification(tag, params);
    }

    /**
     * Send a request to Ki and wait for response
     */
    public sendRequest<M extends InputMessage['tag']>(
        tag: M,
        params?: InputMessageParamsMap[M]
    ): Promise<unknown> {
        this.lastRequestMethod = tag; // Track last request tag for success handler
        return this.ipc.sendRequest(tag, params);
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.vscodeDisposables.forEach((d) => d.dispose());
        this.vscodeDisposables = [];
        this.handlers.clear();
        this.successHandlers.clear();
        this.kiNotificationHandlers.clear();
        this.eventHandlers.clear();
    }
}
