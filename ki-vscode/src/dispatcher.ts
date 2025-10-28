import type * as vscode from "vscode";
import type { IPC } from "./ipc";
import type { Logger } from "./logger";
import type { InputMessage, OutputMessage } from "./protocol/types";

// Helpertesttest type test to Oukik map OutputMessage tags to their parameter types
type OutputMessageParamsMap = {
    [K in OutputMessage["tag"]]: Extract<OutputMessage, { tag: K }>["params"];
};

// Helper type to map InputMessage tags to their parameter types
type InputMessageParamsMap = {
    // For each tag K in InputMessage, extract the specific message type O.
    // If O has a 'params' property, infer its type P, otherwise use 'undefined'.
    [K in InputMessage["tag"]]: Extract<
        InputMessage,
        { tag: K }
    > extends infer O
        ? O extends { params: infer P }
            ? P
            : undefined
        : never;
};

/**
 * Handles event dispatching between VSCode and Ki
 */
export class Dispatcher implements vscode.Disposable {
    private ipc: IPC;
    private logger: Logger;
    private successHandlers: Map<string, () => void> = new Map(); // Track which operations need notification on success
    private vscodeDisposables: vscode.Disposable[] = [];
    private kiNotificationHandlers: Map<
        string,
        ((params: unknown) => Promise<void> | void)[]
    > = new Map();

    constructor(ipc: IPC, logger: Logger) {
        this.ipc = ipc;
        this.logger = logger;
        this.setupEventListeners();
    }

    /**
     * Register a handler for a Ki notification
     */
    public registerKiNotificationHandler<
        M extends keyof OutputMessageParamsMap,
    >(
        method: M,
        handler: (params: OutputMessageParamsMap[M]) => Promise<void> | void,
    ): void {
        const handlers = this.kiNotificationHandlers.get(method);
        if (handlers) {
            const genericHandler = handler as (
                params: unknown,
            ) => Promise<void> | void;
            handlers.push(genericHandler);
            this.logger.log(`Registered additional handler for ${method}`);
        } else {
            this.kiNotificationHandlers.set(method, [
                handler as (params: unknown) => Promise<void> | void,
            ]);
            this.logger.log(`Registered first handler for ${method}`);
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
                this.logger.warn(
                    "Received undefined notification message from IPC.",
                );
                return;
            }
            const { tag, params } = message;

            // Note: Success message handling with ID=0 is now done in IPC.processMessage
            // No need for the specific 'success' check here anymore regarding lastRequestMethod.

            this.processKiNotification(tag, params);
        });
    }

    /**
     * Process a notification from Ki
     */
    private processKiNotification(
        tag: OutputMessage["tag"],
        params: unknown,
    ): void {
        // See if we have any handlers for this notification
        const handlers = this.kiNotificationHandlers.get(tag);
        if (handlers && handlers.length > 0) {
            for (const handler of handlers) {
                try {
                    handler(params);
                } catch (error) {
                    this.logger.error(
                        `Error in Ki notification handler for ${tag}: ${
                            error instanceof Error
                                ? error.message
                                : String(error)
                        }`,
                    );
                }
            }
        } else {
            this.logger.warn(
                `No handlers registered for Ki notification: ${tag}`,
            );
        }
    }

    /**
     * Send a notification to Ki
     */
    public sendNotification<M extends InputMessage["tag"]>(
        tag: M,
        params: InputMessageParamsMap[M],
    ): void {
        this.ipc.sendNotification(tag, params);
    }

    /**
     * Send a request to Ki and wait for response
     */
    public sendRequest<M extends InputMessage["tag"]>(
        tag: M,
        params?: InputMessageParamsMap[M],
    ): Promise<unknown> {
        return this.ipc.sendRequest(tag, params);
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        for (const disposable of this.vscodeDisposables) {
            disposable.dispose();
        }
        this.vscodeDisposables = [];
        // Removed reference to handlers map
        this.successHandlers.clear();
        this.kiNotificationHandlers.clear();
    }
}
