import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { normalizeKiPathToVSCodeUri } from "../utils";
import type { ModeParams } from "../protocol/ModeParams";

/**
 * Supported Ki editor modes
 */
export enum EditorMode {
    Normal = "normal",
    Insert = "insert",
    Visual = "visual",
    Command = "command",
}

/**
 * Handles mode-related events between VSCode and Ki
 */
export class ModeHandler implements vscode.Disposable {
    private dispatcher: Dispatcher;
    private logger: Logger;
    private currentMode: EditorMode = EditorMode.Normal;
    private disposables: vscode.Disposable[] = [];
    private keyInterceptionEnabled: boolean = true;
    private statusBarItem: vscode.StatusBarItem;

    constructor(dispatcher: Dispatcher, logger: Logger, context: vscode.ExtensionContext) {
        this.dispatcher = dispatcher;
        this.logger = logger;

        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
        this.statusBarItem.text = "Ki: Normal";
        this.statusBarItem.tooltip = "Current Ki Editor Mode";
        this.statusBarItem.command = "ki.showModeHelp";
        this.statusBarItem.show();
        this.disposables.push(this.statusBarItem);

        this.setupModeHandling();
        this.updateEditorMode(EditorMode.Normal);
    }

    /**
     * Set up mode handling
     */
    private setupModeHandling(): void {
        // Register event handlers
        this.dispatcher.registerKiNotificationHandler("mode.change", (params) => {
            this.handleModeChange(params);
        });

        // Register commands for forcing modes
        // Use try/catch to gracefully handle any duplicate command registrations
        const registerSafely = (command: string, callback: () => any): void => {
            try {
                this.disposables.push(vscode.commands.registerCommand(command, callback));
                this.logger.log(`Registered command: ${command}`);
            } catch (error) {
                this.logger.error(`Failed to register command "${command}": ${error}`);
                // Command already exists, so we can use the existing one
            }
        };

        // Register mode commands
        registerSafely("ki.mode.normal", () => this.setMode(EditorMode.Normal));
        registerSafely("ki.mode.insert", () => this.setMode(EditorMode.Insert));
        registerSafely("ki.mode.visual", () => this.setMode(EditorMode.Visual));
        registerSafely("ki.mode.command", () => this.setMode(EditorMode.Command));

        // Register key interception commands
        registerSafely("ki.enableKeyInterception", () => this.setKeyInterception(true));
        registerSafely("ki.disableKeyInterception", () => this.setKeyInterception(false));

        // Update initial mode
        this.setMode(EditorMode.Normal);
    }

    /**
     * Handle mode change from Ki
     */
    private handleModeChange(params: ModeParams): void {
        const mode = params.mode;
        this.logger.log(`Received mode change from Ki: ${mode}`);

        // Map Ki mode to EditorMode enum
        switch (mode) {
            case "normal":
                this.updateEditorMode(EditorMode.Normal);
                break;
            case "insert":
                this.updateEditorMode(EditorMode.Insert);
                break;
            case "visual":
            case "v":
            case "V":
            case "extend":
                this.updateEditorMode(EditorMode.Visual);
                break;
            case "command":
                this.updateEditorMode(EditorMode.Command);
                break;
            default:
                this.logger.warn(`Unknown mode from Ki: ${mode}`);
                this.updateEditorMode(EditorMode.Normal);
        }

        // Potentially update editor specific state if buffer_id is present
        if (params.buffer_id) {
            // Normalize the buffer_id before looking up the editor
            const normalizedUri = normalizeKiPathToVSCodeUri(params.buffer_id);
            this.logger.log(`Normalized buffer URI for mode change lookup: ${normalizedUri}`);
            const editor = vscode.window.visibleTextEditors.find((e) => e.document.uri.toString() === normalizedUri);
            if (editor) {
                // Example: Set cursor style based on mode (if desired)
                this.setCursorStyle(editor, mode);
            } else {
                this.logger.warn(
                    `Mode change received for non-visible/found editor. Original: ${params.buffer_id}, Normalized: ${normalizedUri}`,
                );
            }
        }
    }

    /**
     * Update the editor mode and VSCode context
     */
    private updateEditorMode(mode: EditorMode): void {
        this.currentMode = mode;

        // Update VSCode context
        vscode.commands.executeCommand("setContext", "ki.editorMode", mode);

        // Update key interception based on mode
        if (mode === EditorMode.Insert) {
            // In insert mode, we might want to disable key interception for some keys
            // but still intercept basic keys like Escape
            vscode.commands.executeCommand("setContext", "ki.keyInterceptionEnabled", true);
        } else {
            // In other modes, always enable key interception
            vscode.commands.executeCommand("setContext", "ki.keyInterceptionEnabled", true);
        }

        // Update status bar
        this.updateStatusBar();

        this.logger.log(`Mode updated to: ${mode}`);
    }

    /**
     * Update status bar to reflect current mode
     */
    private updateStatusBar(): void {
        let modeText: string;
        let modeColor: string;

        switch (this.currentMode) {
            case EditorMode.Normal:
                modeText = "Ki: Normal";
                modeColor = "statusBarItem.warningBackground";
                break;
            case EditorMode.Insert:
                modeText = "Ki: Insert";
                modeColor = "statusBarItem.errorBackground";
                break;
            case EditorMode.Visual:
                modeText = "Ki: Visual";
                modeColor = "statusBarItem.infoBackground";
                break;
            case EditorMode.Command:
                modeText = "Ki: Command";
                modeColor = "statusBarItem.debuggingBackground";
                break;
            default:
                modeText = `Ki: ${this.currentMode}`;
                modeColor = "";
        }

        this.statusBarItem.text = modeText;
        this.statusBarItem.backgroundColor = new vscode.ThemeColor(modeColor);
        this.statusBarItem.show();
    }

    /**
     * Set the editor mode
     */
    public setMode(mode: EditorMode): void {
        this.logger.log(`Setting mode to: ${mode}`);

        // Only update if mode is different
        if (this.currentMode !== mode) {
            this.updateEditorMode(mode);

            // Send mode change to Ki
            const activeEditor = vscode.window.activeTextEditor;
            if (activeEditor) {
                const buffer_id = activeEditor.document.uri.toString();
                this.dispatcher.sendNotification("mode.set", { buffer_id: buffer_id, mode: mode });
            } else {
                this.logger.warn("No active editor found when trying to send mode.set notification.");
            }
        }
    }

    /**
     * Get the current editor mode
     */
    public getMode(): EditorMode {
        return this.currentMode;
    }

    /**
     * Enable or disable keyboard interception
     */
    public setKeyInterception(enabled: boolean): void {
        this.keyInterceptionEnabled = enabled;
        vscode.commands.executeCommand("setContext", "ki.keyInterceptionEnabled", enabled);
        this.logger.log(`Key interception ${enabled ? "enabled" : "disabled"}`);
    }

    /**
     * Check if keyboard interception is enabled
     */
    public isKeyInterceptionEnabled(): boolean {
        return this.keyInterceptionEnabled;
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.disposables.forEach((d) => d.dispose());
        this.disposables = [];
    }

    // Example helper for cursor style (adapt as needed)
    private setCursorStyle(editor: vscode.TextEditor, mode: string): void {
        let cursorStyle: vscode.TextEditorCursorStyle | undefined;
        switch (mode.toLowerCase()) {
            case "normal":
                cursorStyle = vscode.TextEditorCursorStyle.Block;
                break;
            case "insert":
                cursorStyle = vscode.TextEditorCursorStyle.Line;
                break;
            case "visual":
                cursorStyle = vscode.TextEditorCursorStyle.BlockOutline;
                break;
            default:
                cursorStyle = undefined; // Or default style
        }

        if (cursorStyle !== undefined) {
            editor.options.cursorStyle = cursorStyle;
        }
    }
}
