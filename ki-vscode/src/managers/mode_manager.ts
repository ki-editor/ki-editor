import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { EditorMode } from "../protocol/EditorMode";
import { SelectionModeParams } from "../protocol/SelectionModeParams";
import { SelectionSet } from "../protocol/SelectionSet";
import { TypedModeParams } from "../protocol/TypedModeParams";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";

/**
 * Manages mode synchronization between VSCode and Ki
 */
export class ModeManager extends Manager {
    private currentMode: EditorMode = "normal";
    private currentSelectionMode: SelectionModeParams["mode"] = { type: "Line" };
    private statusBarItem: vscode.StatusBarItem;
    private context: vscode.ExtensionContext;

    constructor(dispatcher: Dispatcher, logger: Logger, eventHandler: EventHandler, context: vscode.ExtensionContext) {
        super(dispatcher, logger, eventHandler);
        this.context = context;

        // Create status bar item
        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
        this.statusBarItem.command = "ki.toggleMode";
        this.registerDisposable(this.statusBarItem);

        this.updateStatusBar();
        this.statusBarItem.show();
    }

    public initialize(): void {
        this.eventHandler.onModeChange((params) => this.handleModeChanged(params));
        this.eventHandler.onSelectionModeChange((params) => this.handleSelectionModeChange(params));

        this.registerCommands();

        this.updateStatusBar();
    }

    /**
     * Handle selection mode change event from Ki
     */
    private handleSelectionModeChange(params: SelectionModeParams): void {
        this.currentSelectionMode = params.mode;

        this.updateStatusBar();
    }

    private registerCommands(): void {
        // Register toggle mode command
        const toggleModeCommand = vscode.commands.registerCommand("ki.toggleMode", () => {
            this.toggleMode();
        });
        this.registerDisposable(toggleModeCommand);
    }

    /**
     * Toggle between normal and insert mode
     */
    private toggleMode(): void {
        const newMode = this.currentMode === "normal" ? "insert" : "normal";
        this.logger.log(`Toggling mode from ${this.currentMode} to ${newMode}`);

        // Send mode change to Ki
        this.dispatcher.sendNotification("mode.set", {
            buffer_id: vscode.window.activeTextEditor?.document.uri.toString() || "",
            mode: newMode,
        });
    }

    private handleModeChanged(params: TypedModeParams): void {
        // Setting `ki.isInsertMode` is necessary so that
        // special keys like tab will not trigger the `ki.specialKey.tab`
        // command.
        vscode.commands.executeCommand("setContext", "ki.isInsertMode", params.mode === "insert");

        if (this.currentMode === "insert" && params.mode === "insert") {
            // Don't update cursor position if the current mode is in Insert mode
            // and the incoming mode is Insert mode as well.
            // This is because we should let VS Code handle everything in Insert mode.
            return;
        }

        this.logger.log(`Received mode changed event: ${params.mode}`);

        this.currentMode = this.parseMode(params.mode);

        this.updateStatusBar();

        this.updateCursorStyle();

        this.logger.log(`Mode updated to ${this.currentMode} with cursor style updated`);
    }

    private parseMode(mode: string): EditorMode {
        switch (mode.toLowerCase()) {
            case "normal":
                return "normal";
            case "insert":
                return "insert";
            case "multicursor":
                return "multiCursor";
            case "findonechar":
                return "findOneChar";
            case "swap":
                return "swap";
            case "replace":
                return "replace";
            case "extend":
                return "extend";
            default:
                this.logger.warn(`Unknown mode: ${mode}, defaulting to Normal`);
                return "normal";
        }
    }

    private updateStatusBar(): void {
        const { modeText, icon } = (() => {
            switch (this.currentMode) {
                case "normal":
                    return { modeText: "Ki: NORMAL", icon: "$(keyboard)" };
                case "insert":
                    return { modeText: "Ki: INSERT", icon: "$(edit)" };
                case "multiCursor":
                    return { modeText: "Ki: MULTI", icon: "$(multiple-windows)" };
                case "findOneChar":
                    return { modeText: "Ki: FIND", icon: "$(search)" };
                case "swap":
                    return { modeText: "Ki: SWAP", icon: "$(arrow-swap)" };
                case "replace":
                    return { modeText: "Ki: REPLACE", icon: "$(replace)" };
                case "extend":
                    return { modeText: "Ki: EXTEND", icon: "$(selection)" };
            }
        })();

        const selectionModeText = (() => {
            switch (this.currentSelectionMode.type) {
                case "Diagnostic":
                    return this.currentSelectionMode.params;
                case "Find":
                    return `Find ${JSON.stringify(this.currentSelectionMode.params.search)}`;
                default:
                    return this.currentSelectionMode.type;
            }
        })();

        this.statusBarItem.text = `${icon} ${modeText} ${selectionModeText}`;
        this.statusBarItem.tooltip = `Current Ki mode: ${this.currentMode}`;

        // Update color based on mode
        this.statusBarItem.backgroundColor = new vscode.ThemeColor(
            this.currentMode === "normal"
                ? "statusBarItem.warningBackground"
                : this.currentMode === "insert"
                  ? "statusBarItem.errorBackground"
                  : "statusBarItem.prominentBackground",
        );

        // Update VSCode context for keybindings
        this.context.workspaceState.update("kiMode", this.currentMode);

        // Log the mode change
        this.logger.log(`Mode updated to ${this.currentMode} (${this.currentSelectionMode})`);
    }

    public getCurrentMode(): EditorMode {
        return this.currentMode;
    }

    /**
     * Update cursor style based on current mode
     */
    private updateCursorStyle(): void {
        // Set cursor style based on mode
        const cursorStyle = (() => {
            switch (this.currentMode) {
                case "insert":
                    return vscode.TextEditorCursorStyle.Line;
                case "normal":
                    return vscode.TextEditorCursorStyle.Block;
                case "extend":
                case "replace":
                    return vscode.TextEditorCursorStyle.Underline;
                default:
                    return vscode.TextEditorCursorStyle.Block;
            }
        })();

        // Apply cursor style to all visible editors
        vscode.window.visibleTextEditors.forEach((editor) => {
            // Preserve existing options
            const currentOptions = editor.options;
            editor.options = {
                ...currentOptions,
                cursorStyle,
            };
        });

        this.logger.log(`Cursor style updated to ${cursorStyle} for mode ${this.currentMode}`);
    }

    /**
     * Dispose of resources
     */
    public override dispose(): void {
        super.dispose();
    }
}
