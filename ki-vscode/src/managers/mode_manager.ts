import * as vscode from "vscode";
import type { Dispatcher } from "../dispatcher";
import type { Logger } from "../logger";
import {
    EditorMode,
    type ModeParams,
    type SelectionModeParams,
} from "../protocol/types";
import { Manager } from "./manager";

/**
 * Manages mode synchronization between VSCode and Ki
 */
export class ModeManager extends Manager {
    private currentMode: EditorMode = EditorMode.Normal;
    private currentSelectionMode: SelectionModeParams["mode"] = {
        type: "Line",
    };
    private statusBarItem: vscode.StatusBarItem;
    private context: vscode.ExtensionContext;
    private keyboardLayout = "";

    constructor(
        dispatcher: Dispatcher,
        logger: Logger,
        context: vscode.ExtensionContext,
    ) {
        super(dispatcher, logger);
        this.context = context;

        // Create status bar item
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            100,
        );
        this.registerDisposable(this.statusBarItem);

        this.updateStatusBar();
        this.statusBarItem.show();
    }

    public initialize(): void {
        this.dispatcher.registerKiNotificationHandler(
            "mode.change",
            (params: ModeParams) => {
                this.handleModeChanged(params);
            },
        );
        this.dispatcher.registerKiNotificationHandler(
            "selection_mode.change",
            (params) => {
                this.handleSelectionModeChange(params);
            },
        );
        this.dispatcher.registerKiNotificationHandler(
            "editor.keyboardLayout",
            async (keyboardLayout) => {
                this.keyboardLayout = keyboardLayout;
                this.updateStatusBar();
            },
        );

        this.updateStatusBar();
    }

    /**
     * Handle selection mode change event from Ki
     */
    private handleSelectionModeChange(params: SelectionModeParams): void {
        this.currentSelectionMode = params.mode;

        this.updateStatusBar();
    }

    private handleModeChanged(params: ModeParams): void {
        // Setting `ki.isInsertMode` is necessary so that
        // special keys like tab will not trigger the `ki.specialKey.tab`
        // command.
        vscode.commands.executeCommand(
            "setContext",
            "ki.isInsertMode",
            params.mode === EditorMode.Insert,
        );

        if (
            this.currentMode === EditorMode.Insert &&
            params.mode === EditorMode.Insert
        ) {
            // Don't update cursor position if the current mode is in Insert mode
            // and the incoming mode is Insert mode as well.
            // This is because we should let VS Code handle everything in Insert mode.
            return;
        }

        this.currentMode = this.parseMode(params.mode);

        this.updateStatusBar();

        this.updateCursorStyle();
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
                    return {
                        modeText: "Ki: MULTI",
                        icon: "$(multiple-windows)",
                    };
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

        this.statusBarItem.text = `${icon} ${modeText} ${selectionModeText} [${this.keyboardLayout}]`;
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
        for (const editor of vscode.window.visibleTextEditors) {
            // Skip non-file documents
            if (editor.document.uri.scheme !== "file") {
                continue;
            }

            // Set cursor style for the editor
            const currentOptions = editor.options;
            editor.options = {
                ...currentOptions,
                cursorStyle,
            };
        }
    }

    /**
     * Dispose of resources
     */
    public override dispose(): void {
        super.dispose();
    }
}
