import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { EditorMode } from "../protocol/EditorMode";
import { TypedModeParams } from "../protocol/TypedModeParams";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";

/**
 * Represents the selection mode
 */
export enum SelectionMode {
    Character = "character",
    Line = "line",
    Block = "block",
    Word = "word",
    FineWord = "fine_word",
    Token = "token",
    SyntaxNode = "syntax_node",
    SyntaxNodeFine = "syntax_node_fine",
    LineFull = "line_full",
    Custom = "custom",
}

/**
 * Manages mode synchronization between VSCode and Ki
 */
export class ModeManager extends Manager {
    private currentMode: EditorMode = "normal";
    private currentSelectionMode: SelectionMode = SelectionMode.Character;
    private statusBarItem: vscode.StatusBarItem;
    private context: vscode.ExtensionContext;

    constructor(dispatcher: Dispatcher, logger: Logger, eventHandler: EventHandler, context: vscode.ExtensionContext) {
        super(dispatcher, logger, eventHandler);
        this.context = context;

        // Create status bar item
        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
        this.statusBarItem.command = "ki.toggleMode";
        this.registerDisposable(this.statusBarItem);

        // Update status bar
        this.updateStatusBar();
        this.statusBarItem.show();
    }

    /**
     * Initialize the mode manager
     */
    public initialize(): void {
        // Register integration event handlers
        this.eventHandler.onModeChange((params) => this.handleModeChanged(params));

        // Register selection update handler to track selection mode changes
        this.eventHandler.onSelectionUpdate((params) => this.handleSelectionUpdate(params));

        // Register selection mode change handler
        this.eventHandler.onSelectionModeChange((params) => this.handleSelectionModeChange(params));

        // Set initial selection mode to Line since Ki starts in line mode by default
        this.currentSelectionMode = SelectionMode.Line;

        // Register commands
        this.registerCommands();

        // Update status bar with initial mode
        this.updateStatusBar();
    }

    /**
     * Handle selection update event from Ki
     */
    private handleSelectionUpdate(params: any): void {
        // Check if the selection update includes a mode
        if (params.mode) {
            this.logger.log(`Selection mode from update: ${JSON.stringify(params.mode)}`);

            // The mode can be either a string or an object with a type field
            let modeString: string;
            if (typeof params.mode === "string") {
                modeString = params.mode;
            } else if (params.mode.type) {
                modeString = params.mode.type;
            } else {
                this.logger.warn(`Unknown selection mode format: ${JSON.stringify(params.mode)}`);
                return;
            }

            // Use the parseSelectionMode method to convert the mode string to a SelectionMode enum
            this.currentSelectionMode = this.parseSelectionMode(modeString);

            // Update the status bar to reflect the new selection mode
            this.updateStatusBar();

            // Log the current mode for debugging
            this.logger.log(`Current selection mode is now: ${this.currentSelectionMode}`);
        }
    }

    /**
     * Handle selection mode change event from Ki
     */
    private handleSelectionModeChange(params: any): void {
        this.logger.log(`Selection mode change notification: ${JSON.stringify(params)}`);

        // Extract the mode from the params
        if (params.mode) {
            let modeString: string;
            if (typeof params.mode === "string") {
                modeString = params.mode;
            } else if (params.mode.type) {
                modeString = params.mode.type;
            } else {
                this.logger.warn(`Unknown selection mode format: ${JSON.stringify(params.mode)}`);
                return;
            }

            // Update the current selection mode
            this.currentSelectionMode = this.parseSelectionMode(modeString);

            // Update the status bar
            this.updateStatusBar();

            // Log the current mode for debugging
            this.logger.log(`Selection mode changed to: ${this.currentSelectionMode}`);
        }
    }

    /**
     * Register commands
     */
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

    /**
     * Handle mode changed event from Ki
     */
    private handleModeChanged(params: TypedModeParams): void {
        this.logger.log(`Received mode changed event: ${params.mode}`);

        // Update current mode
        this.currentMode = this.parseMode(params.mode);

        // Don't reset the selection mode here, as it will be updated by selection updates
        // Keep the current selection mode instead of defaulting to character

        // Update status bar (which also updates cursor style)
        this.updateStatusBar();

        // Explicitly update cursor style to ensure it's applied
        this.updateCursorStyle();

        this.logger.log(`Mode updated to ${this.currentMode} with cursor style updated`);
    }

    /**
     * Parse mode string to EditorMode
     */
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

    /**
     * Parse selection mode string to SelectionMode enum
     */
    private parseSelectionMode(mode: string): SelectionMode {
        switch (mode.toLowerCase()) {
            case "character":
                return SelectionMode.Character;
            case "line":
            case "line_full":
                return SelectionMode.Line;
            case "block":
            case "word":
            case "fine_word":
            case "token":
            case "syntax_node":
            case "syntax_node_fine":
                return SelectionMode.Block;
            case "custom":
                return SelectionMode.Custom;
            default:
                this.logger.warn(`Unknown selection mode: ${mode}, defaulting to Character`);
                return SelectionMode.Character;
        }
    }

    /**
     * Update the status bar item
     */
    private updateStatusBar(): void {
        let modeText = "";
        let icon = "$(keyboard)";

        switch (this.currentMode) {
            case "normal":
                modeText = "Ki: NORMAL";
                icon = "$(keyboard)";
                break;
            case "insert":
                modeText = "Ki: INSERT";
                icon = "$(edit)";
                break;
            case "multiCursor":
                modeText = "Ki: MULTI";
                icon = "$(multiple-windows)";
                break;
            case "findOneChar":
                modeText = "Ki: FIND";
                icon = "$(search)";
                break;
            case "swap":
                modeText = "Ki: SWAP";
                icon = "$(arrow-swap)";
                break;
            case "replace":
                modeText = "Ki: REPLACE";
                icon = "$(replace)";
                break;
            case "extend":
                modeText = "Ki: EXTEND";
                icon = "$(selection)";
                break;
        }

        // Add selection mode
        switch (this.currentSelectionMode) {
            case SelectionMode.Character:
                modeText += " (CHAR)";
                break;
            case SelectionMode.Line:
                modeText += " (LINE)";
                break;
            case SelectionMode.Block:
                modeText += " (WORD)"; // Display as WORD instead of BLOCK for better user understanding
                break;
            case SelectionMode.Custom:
                modeText += " (CUSTOM)";
                break;
        }

        this.statusBarItem.text = `${icon} ${modeText}`;
        this.statusBarItem.tooltip = `Current Ki mode: ${this.currentMode}`;

        // Update color based on mode
        if (this.currentMode === "normal") {
            this.statusBarItem.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
        } else if (this.currentMode === "insert") {
            this.statusBarItem.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
        } else {
            this.statusBarItem.backgroundColor = new vscode.ThemeColor("statusBarItem.prominentBackground");
        }

        // Update VSCode context for keybindings
        this.context.workspaceState.update("kiMode", this.currentMode);

        // Log the mode change
        this.logger.log(`Mode updated to ${this.currentMode} (${this.currentSelectionMode})`);

        // Update cursor style based on mode
        this.updateCursorStyle();
    }

    /**
     * Get the current mode
     */
    public getCurrentMode(): EditorMode {
        return this.currentMode;
    }

    /**
     * Get the current selection mode
     */
    public getCurrentSelectionMode(): SelectionMode {
        return this.currentSelectionMode;
    }

    /**
     * Update cursor style based on current mode
     */
    private updateCursorStyle(): void {
        // Set cursor style based on mode
        let cursorStyle: vscode.TextEditorCursorStyle;

        switch (this.currentMode) {
            case "insert":
                cursorStyle = vscode.TextEditorCursorStyle.Line;
                break;
            case "normal":
                cursorStyle = vscode.TextEditorCursorStyle.Block;
                break;
            case "replace":
                cursorStyle = vscode.TextEditorCursorStyle.Underline;
                break;
            default:
                cursorStyle = vscode.TextEditorCursorStyle.Block;
                break;
        }

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
