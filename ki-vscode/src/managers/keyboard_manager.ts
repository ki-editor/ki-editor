import * as vscode from "vscode";
import * as zlib from "node:zlib";
import type { Dispatcher } from "../dispatcher";
import type { Logger } from "../logger";
import { Manager } from "./manager";
import type { ModeManager } from "./mode_manager";

// Special keys that need to be handled
type SpecialKey =
    | "escape"
    | "enter"
    | "backspace"
    | "delete"
    | "tab"
    | "home"
    | "end"
    | "pageup"
    | "pagedown"
    | "up"
    | "down"
    | "left"
    | "right";

/**
 * Manages keyboard input handling between VSCode and Ki
 */
export class KeyboardManager extends Manager {
    private modeManager: ModeManager;
    private typeSubscription: vscode.Disposable | undefined;
    private specialKeySubscriptions: vscode.Disposable[] = [];
    private ignoreNextKey = false;
    private outputChannel = vscode.window.createOutputChannel("Ki: Info");

    // Map of special keys to their VSCode key codes
    private specialKeyMap: Record<SpecialKey, string> = {
        escape: "Escape",
        enter: "Enter",
        backspace: "Backspace",
        delete: "Delete",
        tab: "Tab",
        home: "Home",
        end: "End",
        pageup: "PageUp",
        pagedown: "PageDown",
        up: "ArrowUp",
        down: "ArrowDown",
        left: "ArrowLeft",
        right: "ArrowRight",
    };

    constructor(
        dispatcher: Dispatcher,
        logger: Logger,
        modeManager: ModeManager,
    ) {
        super(dispatcher, logger);
        this.modeManager = modeManager;
    }

    /**
     * Initialize the keyboard manager
     */
    public initialize(): void {
        // Register the type event handler
        this.registerTypeHandler();

        // Register special key handlers
        this.registerSpecialKeyHandlers();

        // Register VSCode event handlers
        vscode.window.onDidChangeActiveTextEditor(() => {
            this.updateTypeHandler();
        });

        this.dispatcher.registerKiNotificationHandler(
            "editor.showInfo",
            (params) => this.showInfo(params.info),
        );
    }

    private showInfo(info: string | undefined): void | Promise<void> {
        if (!info) {
            this.outputChannel.hide();
        } else {
            this.outputChannel.clear();
            this.outputChannel.appendLine(info);

            const preserveFocus = true;
            this.outputChannel.show(preserveFocus);
        }
    }

    /**
     * Register handlers for special keys
     */
    private registerSpecialKeyHandlers(): void {
        // Dispose of existing subscriptions if any
        this.specialKeySubscriptions.forEach((subscription) =>
            subscription.dispose(),
        );
        this.specialKeySubscriptions = [];

        // Register each special key
        Object.keys(this.specialKeyMap).forEach((key) => {
            const specialKey = key as SpecialKey;
            const commandId = `ki.specialKey.${specialKey}`;

            // Register the command
            const subscription = vscode.commands.registerCommand(
                commandId,
                async () => {
                    await this.handleSpecialKey(specialKey);
                    return true; // Let VSCode continue processing
                },
            );

            this.specialKeySubscriptions.push(subscription);
            this.registerDisposable(subscription);

            // Register the keybinding programmatically
            this.registerKeybinding(specialKey, commandId);
        });
    }

    /**
     * Register a keybinding programmatically
     */
    private registerKeybinding(
        specialKey: SpecialKey,
        commandId: string,
    ): void {
        // This doesn't actually register keybindings at runtime
        // Keybindings must be defined in package.json
        // This is just a placeholder for documentation
        this.logger.log(
            `Registered command ${commandId} for special key ${specialKey}`,
        );
    }

    /**
     * Register the type event handler
     */
    private registerTypeHandler(): void {
        // Dispose of existing subscription if any
        if (this.typeSubscription) {
            this.typeSubscription.dispose();
        }

        // Create new subscription
        this.typeSubscription = vscode.commands.registerCommand(
            "type",
            (args) => {
                return this.handleType(args);
            },
        );

        // Add to disposables
        this.registerDisposable(this.typeSubscription);
    }

    /**
     * Update the type handler when the active editor changes
     */
    private updateTypeHandler(): void {
        // Re-register the type handler to ensure it's active
        this.registerTypeHandler();
    }

    /**
     * To let VS Code handle the typing, execute `default:type`.
     * Surprisingly this cannot be found in their official docs, but
     * it is found in an issue related to VSCode Vim at
     * https://github.com/microsoft/vscode/issues/65876#issue-395051915
     */
    private async letVscodeHandleTyping(args: { text: string }) {
        await vscode.commands.executeCommand("default:type", args);
    }

    /**
     * Handle type event
     */
    private async handleType(args: { text: string }): Promise<void> {
        // If we should ignore this key, reset the flag and let VSCode handle it
        if (this.ignoreNextKey) {
            this.ignoreNextKey = false;
            return await this.letVscodeHandleTyping(args);
        }

        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            return await this.letVscodeHandleTyping(args);
        }

        // Skip non-file documents
        if (editor.document.uri.scheme !== "file") {
            return await this.letVscodeHandleTyping(args);
        }

        if (this.modeManager.getCurrentMode() === "insert") {
            return await this.letVscodeHandleTyping(args);
        }

        const uri = editor.document.uri.toString();
        const text = args.text;

        this.logger.log(
            `Key pressed: ${text} in mode ${this.modeManager.getCurrentMode()}`,
        );

        // In normal mode, send the key to Ki and prevent VSCode from handling it
        this.dispatcher
            .sendRequest("keyboard.input", {
                key: text,
                uri,
                content_hash: zlib.crc32(editor.document.getText()),
            })
            .then((response) => {
                this.logger.log(
                    `Keyboard input response in normal mode: ${JSON.stringify(response)}`,
                );
                // No longer forcing a sync after each keystroke
            })
            .catch((error) => {
                this.logger.error(
                    `Error sending keyboard input in normal mode: ${error}`,
                );
            });
    }

    /**
     * Handle special key event
     */
    private handleSpecialKey(specialKey: SpecialKey): void {
        // Get the active editor
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            return; // No active editor
        }

        // Skip non-file documents
        if (editor.document.uri.scheme !== "file") {
            return; // Not a file document
        }

        const uri = editor.document.uri.toString();
        const keyCode = this.specialKeyMap[specialKey];

        this.logger.log(
            `Special key pressed: ${specialKey} (${keyCode}) in mode ${this.modeManager.getCurrentMode()}`,
        );

        // Send the key to Ki
        this.dispatcher
            .sendRequest("keyboard.input", {
                key: keyCode,
                uri,
                content_hash: zlib.crc32(editor.document.getText()),
            })
            .then((response) => {
                this.logger.log(
                    `Special key response: ${JSON.stringify(response)}`,
                );
            })
            .catch((error) => {
                this.logger.error(`Error sending special key: ${error}`);
            });
    }

    /**
     * Set the flag to ignore the next key
     * This is useful for special key combinations that should be handled by VSCode
     */
    public ignoreNextKeypress(): void {
        this.ignoreNextKey = true;
    }

    /**
     * Dispose of resources
     */
    public override dispose(): void {
        if (this.typeSubscription) {
            this.typeSubscription.dispose();
        }

        // Dispose of special key subscriptions
        this.specialKeySubscriptions.forEach((subscription) =>
            subscription.dispose(),
        );
        this.specialKeySubscriptions = [];

        super.dispose();
    }
}
