import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { EditorMode, SelectionModeParams, SelectionSet, TypedModeParams } from "../protocol/types";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";

/**
 * Manages mode synchronization between VSCode and Ki
 */
export class ModeManager extends Manager {
    private currentMode: EditorMode = EditorMode.Normal;
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
        const newMode = this.currentMode === EditorMode.Normal ? EditorMode.Insert : EditorMode.Normal;
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
        vscode.commands.executeCommand("setContext", "ki.isInsertMode", params.mode === EditorMode.Insert);

        if (this.currentMode === EditorMode.Insert && params.mode === EditorMode.Insert) {
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
                return EditorMode.Normal;
            case "insert":
                return EditorMode.Insert;
            case "multicursor":
                return EditorMode.MultiCursor;
            case "findonechar":
                return EditorMode.FindOneChar;
            case "swap":
                return EditorMode.Swap;
            case "replace":
                return EditorMode.Replace;
            case "extend":
                return EditorMode.Extend;
            default:
                this.logger.warn(`Unknown mode: ${mode}, defaulting to Normal`);
                return EditorMode.Normal;
        }
    }

    private updateStatusBar(): void {
        const modeInfo = (() => {
            switch (this.currentMode) {
                case EditorMode.Normal:
                    return { modeText: "Ki: NORMAL", icon: "$(keyboard)" };
                case EditorMode.Insert:
                    return { modeText: "Ki: INSERT", icon: "$(edit)" };
                case EditorMode.MultiCursor:
                    return { modeText: "Ki: MULTI", icon: "$(multiple-windows)" };
                case EditorMode.FindOneChar:
                    return { modeText: "Ki: FIND", icon: "$(search)" };
                case EditorMode.Swap:
                    return { modeText: "Ki: SWAP", icon: "$(arrow-swap)" };
                case EditorMode.Replace:
                    return { modeText: "Ki: REPLACE", icon: "$(replace)" };
                case EditorMode.Extend:
                    return { modeText: "Ki: EXTEND", icon: "$(selection)" };
                default:
                    return { modeText: "Ki: UNKNOWN", icon: "$(question)" };
            }
        })();

        const { modeText, icon } = modeInfo;

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
            this.currentMode === EditorMode.Normal
                ? "statusBarItem.warningBackground"
                : this.currentMode === EditorMode.Insert
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
                case EditorMode.Insert:
                    return vscode.TextEditorCursorStyle.Line;
                case EditorMode.Normal:
                    return vscode.TextEditorCursorStyle.Block;
                case EditorMode.Extend:
                case EditorMode.Replace:
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
