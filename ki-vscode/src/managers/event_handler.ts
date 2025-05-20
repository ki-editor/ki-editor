import { Dispatcher } from "../dispatcher";
import { ErrorHandler, ErrorSeverity } from "../error_handler";
import { Logger } from "../logger";
import {
    BufferDiffParams,
    BufferParams,
    CommandParams,
    JumpsParams,
    SelectionSet,
    TypedModeParams,
    SelectionModeParams,
} from "../protocol/types";

/**
 * Handles events from Ki and VSCode
 */
export class EventHandler {
    private dispatcher: Dispatcher;
    private logger: Logger;
    private errorHandler: ErrorHandler;

    // Event handlers
    private bufferDiffHandlers: Array<(params: BufferDiffParams) => void> = [];
    private bufferOpenHandlers: Array<(params: BufferParams) => void> = [];
    private bufferCloseHandlers: Array<(params: BufferParams) => void> = [];
    private bufferSaveHandlers: Array<(params: BufferParams) => void> = [];
    private bufferActivatedHandlers: Array<(params: BufferParams) => void> = [];
    private modeChangeHandlers: Array<(params: TypedModeParams) => void> = [];
    private selectionUpdateHandlers: Array<(params: SelectionSet) => void> = [];
    private selectionModeChangeHandlers: Array<(params: any) => void> = [];
    private jumpsChangeHandlers: Array<(params: JumpsParams) => void> = [];
    private commandExecutedHandlers: Array<(params: CommandParams) => void> = [];

    constructor(dispatcher: Dispatcher, logger: Logger, errorHandler: ErrorHandler) {
        this.dispatcher = dispatcher;
        this.logger = logger;
        this.errorHandler = errorHandler;
        this.registerEventHandlers();
    }

    /**
     * Register event handlers with the dispatcher
     */
    private registerEventHandlers(): void {
        // Register handlers for protocol events from Ki
        this.dispatcher.registerKiNotificationHandler("buffer.diff", (params: BufferDiffParams) => {
            this.logger.log(`Received buffer.diff notification for ${params.buffer_id}`);
            this.handleBufferDiff(params);
        });

        this.dispatcher.registerKiNotificationHandler("mode.change", (params: TypedModeParams) => {
            this.logger.log(`Received mode.change notification: ${JSON.stringify(params)}`);
            this.handleModeChange(params);
        });

        this.dispatcher.registerKiNotificationHandler("selection.update", (params: SelectionSet) => {
            this.logger.log(`Received selection.update notification for ${params.buffer_id}`);
            this.handleSelectionUpdate(params);
        });

        // Register handlers for buffer events
        this.dispatcher.registerKiNotificationHandler("buffer.open", (params: BufferParams) => {
            this.logger.log(`Received buffer.open notification for ${params.uri}`);
            this.handleBufferOpen(params);
        });

        this.dispatcher.registerKiNotificationHandler("buffer.close", (params: BufferParams) => {
            this.logger.log(`Received buffer.close notification for ${params.uri}`);
            this.handleBufferClose(params);
        });

        this.dispatcher.registerKiNotificationHandler("buffer.save", (params: BufferParams) => {
            this.logger.log(`Received buffer.save notification for ${params.uri}`);
            this.handleBufferSave(params);
        });

        this.dispatcher.registerKiNotificationHandler("buffer.activated", (params: BufferParams) => {
            this.logger.log(`Received buffer.activated notification for ${params.uri}`);
            this.handleBufferActivated(params);
        });

        // Register handler for command executed events
        this.dispatcher.registerKiNotificationHandler("command.executed", (params: CommandParams) => {
            this.logger.log(`Received command.executed notification: ${params.name}`);
            this.handleCommandExecuted(params);
        });

        // Register handler for selection mode change notifications
        this.dispatcher.registerKiNotificationHandler("selection_mode.change", (params: any) => {
            this.logger.log(`Received selection_mode.change notification: ${JSON.stringify(params)}`);
            this.handleSelectionModeChange(params);
        });

        this.dispatcher.registerKiNotificationHandler("editor.jump", (params: JumpsParams) => {
            this.logger.log(`Received editor.jump notification`);
            this.handleJumpsChange(params);
        });

        // Register handler for success notifications
        this.dispatcher.registerKiNotificationHandler("success", (params: boolean) => {
            this.logger.log(`Received success notification: ${params}`);
            // Success notifications are handled by the IPC layer for requests with IDs
            // This handler is just to avoid the "No handlers registered" warning
        });

        // The ki.sync event handler has been removed.
        // Ki should be the source of truth, and we should only be reacting to events from Ki,
        // not proactively syncing or creating feedback loops.
    }
    /**
     * Handle buffer diff event
     */
    private handleBufferDiff(params: BufferDiffParams): void {
        this.logger.log(`Processing buffer diff for ${params.buffer_id}`);
        this.bufferDiffHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    { component: "EventHandler", operation: "BufferDiff", details: { buffer_id: params.buffer_id } },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    /**
     * Handle buffer open event
     */
    private handleBufferOpen(params: BufferParams): void {
        this.logger.log(`Processing buffer open for ${params.uri}`);
        this.bufferOpenHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    { component: "EventHandler", operation: "BufferOpen", details: { uri: params.uri } },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    /**
     * Handle buffer close event
     */
    private handleBufferClose(params: BufferParams): void {
        this.logger.log(`Processing buffer close for ${params.uri}`);
        this.bufferCloseHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    { component: "EventHandler", operation: "BufferClose", details: { uri: params.uri } },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    /**
     * Handle buffer save event
     */
    private handleBufferSave(params: BufferParams): void {
        this.logger.log(`Processing buffer save for ${params.uri}`);
        this.bufferSaveHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    { component: "EventHandler", operation: "BufferSave", details: { uri: params.uri } },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    /**
     * Handle buffer activated event
     */
    private handleBufferActivated(params: BufferParams): void {
        this.logger.log(`Processing buffer activated for ${params.uri}`);
        this.bufferActivatedHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    { component: "EventHandler", operation: "BufferActivated", details: { uri: params.uri } },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    /**
     * Handle mode change event
     */
    private handleModeChange(params: TypedModeParams): void {
        this.logger.log(`Processing mode change: ${params.mode}`);
        this.modeChangeHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    { component: "EventHandler", operation: "ModeChange", details: { mode: params.mode } },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    /**
     * Handle selection update event
     */
    private handleSelectionUpdate(params: SelectionSet): void {
        this.logger.log(`Processing selection update for ${params.buffer_id}`);
        this.selectionUpdateHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    {
                        component: "EventHandler",
                        operation: "SelectionUpdate",
                        details: { buffer_id: params.buffer_id },
                    },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    /**
     * Handle selection mode change event
     */
    private handleSelectionModeChange(params: any): void {
        this.logger.log(`Processing selection mode change: ${JSON.stringify(params)}`);
        this.selectionModeChangeHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    {
                        component: "EventHandler",
                        operation: "SelectionModeChange",
                        details: { buffer_id: params.buffer_id },
                    },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    // The handleCursorUpdate method has been removed in favor of the unified selection update

    /**
     * Handle command executed event
     */
    private handleCommandExecuted(params: CommandParams): void {
        this.logger.log(`Processing command executed: ${params.name}`);
        this.commandExecutedHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    { component: "EventHandler", operation: "CommandExecuted", details: { command: params.name } },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    private handleJumpsChange(params: JumpsParams): void {
        this.jumpsChangeHandlers.forEach((handler) => {
            try {
                handler(params);
            } catch (error) {
                this.errorHandler.handleError(
                    error,
                    {
                        component: "EventHandler",
                        operation: "JumpsChange",
                        details: {},
                    },
                    ErrorSeverity.Error,
                );
            }
        });
    }

    /**
     * Register a handler for buffer diff events
     */
    public onBufferDiff(handler: (params: BufferDiffParams) => void): void {
        this.bufferDiffHandlers.push(handler);
    }

    /**
     * Register a handler for buffer open events
     */
    public onBufferOpen(handler: (params: BufferParams) => void): void {
        this.bufferOpenHandlers.push(handler);
    }

    /**
     * Register a handler for buffer close events
     */
    public onBufferClose(handler: (params: BufferParams) => void): void {
        this.bufferCloseHandlers.push(handler);
    }

    /**
     * Register a handler for buffer save events
     */
    public onBufferSave(handler: (params: BufferParams) => void): void {
        this.bufferSaveHandlers.push(handler);
    }

    /**
     * Register a handler for buffer activated events
     */
    public onBufferActivated(handler: (params: BufferParams) => void): void {
        this.bufferActivatedHandlers.push(handler);
    }

    /**
     * Register a handler for mode change events
     */
    public onModeChange(handler: (params: TypedModeParams) => void): void {
        this.modeChangeHandlers.push(handler);
    }

    /**
     * Register a handler for selection update events
     */
    public onSelectionUpdate(handler: (params: SelectionSet) => void): void {
        this.selectionUpdateHandlers.push(handler);
    }

    /**
     * Register a handler for selection mode change events
     */
    public onSelectionModeChange(handler: (params: any) => void): void {
        this.selectionModeChangeHandlers.push(handler);
    }

    // The onCursorUpdate method has been removed in favor of the unified selection update

    /**
     * Register a handler for command executed events
     */
    public onCommandExecuted(handler: (params: CommandParams) => void): void {
        this.commandExecutedHandlers.push(handler);
    }

    public onJumpsChange(handler: (params: JumpsParams) => void): void {
        this.jumpsChangeHandlers.push(handler);
    }
}
