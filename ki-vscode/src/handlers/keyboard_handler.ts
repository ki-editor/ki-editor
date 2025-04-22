import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { ModeHandler } from "./mode_handler";
import type { KeyboardParams } from "../protocol/KeyboardParams";

/**
 * Handles keyboard input between VSCode and Ki
 */
export class KeyboardHandler implements vscode.Disposable {
    private dispatcher: Dispatcher;
    private logger: Logger;
    private modeHandler: ModeHandler;
    private disposables: vscode.Disposable[] = [];
    private composing: boolean = false;
    private lastKeysSent: Map<string, number> = new Map();
    private pendingPromise: Promise<any> | null = null;

    constructor(dispatcher: Dispatcher, logger: Logger, modeHandler: ModeHandler) {
        this.dispatcher = dispatcher;
        this.logger = logger;
        this.modeHandler = modeHandler;

        this.setupKeyboardHandling();
        this.registerEventHandlers();
    }

    /**
     * Set up keyboard handling
     */
    private setupKeyboardHandling(): void {
        // Intercept keyboard events in the editor
        this.registerKeyboardCommands();

        // Register sync handler for sending queued keys
        /* // Commenting out interval for now as processQueuedKeys is empty
        const syncInterval = setInterval(() => {
            this.processQueuedKeys();
        }, 100); // Process every 100ms

        this.disposables.push({ dispose: () => clearInterval(syncInterval) });
        */
    }

    /**
     * Register event handlers
     */
    private registerEventHandlers(): void {
        // Register key press handler from Ki
        this.dispatcher.registerEventHandler("keyboard.press", (params) => this.handleKeyboardPress(params));

        // Register sync notification to update state after commands
        this.dispatcher.registerKiNotificationHandler("cursor.update", () => {
            // Update last sent time to prevent throttling after cursor update
            this.lastKeysSent.clear();
        });

        // Clear throttling after a selection update from Ki as well
        this.dispatcher.registerKiNotificationHandler("selection.update", () => {
            this.lastKeysSent.clear();
        });
    }

    /**
     * Process any queued keyboard events
     */
    private processQueuedKeys(): void {
        this.logger.warn("[!!!KeyboardHandler!!!] processQueuedKeys called");
        // Nothing to process for now, but could be used for batching
    }

    /**
     * Register keyboard commands for all keys we want to intercept
     */
    private registerKeyboardCommands(): void {
        this.logger.warn("[!!!KeyboardHandler!!!] registerKeyboardCommands STARTING");
        const letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        const numbers = "0123456789";
        const specialKeys = "`~!@#$%^&*()-_=+[{]}\\|;:'\",<.>/? ";

        // Register all keys we want to intercept
        const allKeys = letters + numbers + specialKeys;
        for (const key of allKeys) {
            const disposable = vscode.commands.registerCommand(`ki.type.${key}`, () => {
                this.handleKeyPress(key);
                return true; // Return true to prevent further handling
            });
            this.disposables.push(disposable);
        }

        // Register special commands for keys that don't have character representations
        const specialCommands = [
            { command: "ki.type.enter", key: "Enter" },
            { command: "ki.type.escape", key: "Escape" },
            { command: "ki.type.backspace", key: "Backspace" },
            { command: "ki.type.delete", key: "Delete" },
            { command: "ki.type.tab", key: "Tab" },
            { command: "ki.type.up", key: "ArrowUp" },
            { command: "ki.type.down", key: "ArrowDown" },
            { command: "ki.type.left", key: "ArrowLeft" },
            { command: "ki.type.right", key: "ArrowRight" },
            { command: "ki.type.home", key: "Home" },
            { command: "ki.type.end", key: "End" },
            { command: "ki.type.pageup", key: "PageUp" },
            { command: "ki.type.pagedown", key: "PageDown" },
        ];

        for (const { command, key } of specialCommands) {
            const disposable = vscode.commands.registerCommand(command, () => {
                this.handleKeyPress(key);
                return true; // Return true to prevent further handling
            });
            this.disposables.push(disposable);
        }

        // Register direct keyboard event handler
        const typeHandler = vscode.commands.registerCommand("type", (args) => {
            this.logger.log(`[KeyboardHandler] Generic 'type' command triggered. Args: ${JSON.stringify(args)}`);

            if (args.text && this.modeHandler.isKeyInterceptionEnabled()) {
                // this.handleKeyPress(args.text); // <<< REMOVE THIS LINE AGAIN
                // Returning here might be necessary if the generic type handler should
                // NOT proceed when interception is enabled and specific commands handle it.
                return; // Keep this return? Need to confirm desired behavior.
            }
            return vscode.commands.executeCommand("default:type", args);
        });
        this.disposables.push(typeHandler);

        // Register composition event listeners
        const compositionStart = vscode.commands.registerCommand("ki.composition.start", () => {
            this.composing = true;
            this.logger.log("Composition started");
            return true;
        });

        const compositionEnd = vscode.commands.registerCommand("ki.composition.end", () => {
            this.composing = false;
            this.logger.log("Composition ended");
            return true;
        });

        this.disposables.push(compositionStart, compositionEnd);

        // Create key bindings for normal/insert mode
        this.setupModalKeybindings();
    }

    /**
     * Setup modal keybindings based on current mode
     */
    private setupModalKeybindings(): void {
        // The mode commands (ki.mode.normal, ki.mode.insert, etc.) are already
        // registered by the ModeHandler class, so we don't need to register them here.
        //
        // This avoids the "command already exists" error during extension activation.

        // Enable keyboard interception by default
        this.updateKeyboardInterceptionState(true);
    }

    /**
     * Update keyboard interception state
     */
    private updateKeyboardInterceptionState(enabled: boolean): void {
        // This can be used to dynamically enable/disable keyboard interception
        // based on context (e.g., disable when in insert mode in certain contexts)
        this.modeHandler.setKeyInterception(enabled);
    }

    /**
     * Handle key press from VSCode
     */
    private handleKeyPress(key: string): boolean {
        this.logger.warn(`[!!!KeyboardHandler!!!] handleKeyPress STARTING for key: ${key}`);
        if (this.composing) {
            this.logger.log(`Ignoring key during composition: ${key}`);
            return false;
        }

        if (!this.modeHandler.isKeyInterceptionEnabled()) {
            return false;
        }

        this.logger.log(`[KeyboardHandler] handleKeyPress proceeding for key: ${key}`);

        const currentMode = this.modeHandler.getMode();
        this.logger.warn(`[!!!KeyboardHandler!!!] Sending key press to Ki: ${key} (mode: ${currentMode})`);

        // Ensure the mode is a valid string expected by Ki
        const modeString = currentMode as string;

        // Send to Ki with current mode - use request instead of notification for important feedback
        this.pendingPromise = this.dispatcher
            .sendRequest("keyboard.input", {
                key: key,
                mode: modeString,
                timestamp: Date.now(),
                is_composed: false,
            })
            .catch((err) => {
                this.logger.error(`Error sending keyboard input to Ki: ${err}`);
            });

        return true;
    }

    /**
     * Handle keyboard press notification from Ki
     */
    private handleKeyboardPress(params: KeyboardParams): void {
        // Handle keyboard press notifications from Ki if needed
        // (Ki might want to trigger synthetic keypresses)
        this.logger.log(`Received keyboard press from Ki: ${JSON.stringify(params)}`);

        // Force VSCode to refresh cursor position after Ki keyboard press
        const editor = vscode.window.activeTextEditor;
        if (editor) {
            // Make a copy of the current selections to force refresh
            editor.selections = [...editor.selections];
        }
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.disposables.forEach((d) => d.dispose());
        this.disposables = [];
    }
}
