import * as vscode from "vscode";
import type { Dispatcher } from "../dispatcher";
import type { Logger } from "../logger";
import type { SelectionSet } from "../protocol/types";
import { JUMP_SAFETY_PADDING } from "./decoration_manager";
import { Manager } from "./manager";
import type { ModeManager } from "./mode_manager";

/**
 * Manages selection synchronization between VSCode and Ki
 */
export class SelectionManager extends Manager {
    private modeManager: ModeManager;
    private activeEditor: vscode.TextEditor | undefined;
    private ignoreSelectionChange = false;

    constructor(dispatcher: Dispatcher, logger: Logger, modeManager: ModeManager) {
        super(dispatcher, logger);
        this.modeManager = modeManager;
    }
    public initialize(): void {
        // Register VSCode event handlers
        vscode.window.onDidChangeActiveTextEditor((editor) => {
            this.handleEditorActive({ editor });
        });
        vscode.window.onDidChangeTextEditorSelection((event) => this.handleSelectionChange(event));

        // Register integration event handlers
        this.dispatcher.registerKiNotificationHandler("selection.update", (params: SelectionSet) => {
            this.handleSelectionChanged(params);
        });

        // Initialize with active editor
        this.activeEditor = vscode.window.activeTextEditor;
    }

    private handleEditorActive(params: { editor: vscode.TextEditor | undefined }): void {
        this.activeEditor = params.editor;
    }

    /**
     * Handle selection change event from VSCode
     */
    private handleSelectionChange(event: vscode.TextEditorSelectionChangeEvent): void {
        const editor = event.textEditor;
        if (!editor || editor.document.uri.scheme !== "file") {
            return;
        }

        // Skip if we're ignoring selection changes (due to Ki-initiated updates)
        if (this.ignoreSelectionChange) {
            this.logger.log("Ignoring selection change from VSCode (initiated by Ki)");
            return;
        }

        const uri = editor.document.uri.toString();
        const selections = event.selections;

        // Check if this selection change was caused by a mouse interaction or a command
        // Example of command is LSP Go to Definition, Go to References, etc.
        // We only want to handle selection changes that are not caused by keyboard input
        // unless we are in insert mode
        const nonKeyboardChanges =
            event.kind === vscode.TextEditorSelectionChangeKind.Mouse ||
            event.kind === vscode.TextEditorSelectionChangeKind.Command;

        if (!nonKeyboardChanges && this.modeManager.getCurrentMode() !== "insert") {
            this.logger.log(`Ignoring non-mouse and non-insert mode selection change in VSCode: ${uri}`);
            return;
        }

        // Only send selection updates to Ki for mouse interactions
        // Set the flag to ignore the next selection change that will come from Ki
        this.ignoreSelectionChange = true;

        try {
            // Convert VSCode selections to Ki format

            const offset = this.modeManager.getCurrentMode() === "insert" ? 1 : 0;
            // The column/character needs to minus 1 in insert mode
            // because of how Ki and VS Code treats cursor differently in insert mode
            const kiSelections = selections.map((sel) => {
                return {
                    anchor: {
                        line: sel.anchor.line,
                        character: sel.anchor.character - offset,
                    },
                    active: {
                        line: sel.active.line,
                        character: sel.active.character - offset,
                    },
                    is_extended: false, // We don't have this information from VSCode
                };
            });

            // Send selection update to Ki
            this.dispatcher.sendNotification("selection.set", {
                buffer_id: uri,
                selections: kiSelections,
                primary: 0, // Always use the first selection as primary
            });
        } finally {
            // Reset the flag after a short delay to allow Ki to process the selection
            setTimeout(() => {
                this.ignoreSelectionChange = false;
            }, 50);
        }
    }

    /**
     * Handle selection changed event from Ki
     */
    private handleSelectionChanged(params: SelectionSet): void {
        if (this.modeManager.getCurrentMode() === "insert") {
            this.logger.error("ignoring selection change becos inest mode");
            return;
        }

        this.logger.log(`Received selection changed event with ${params.selections.length} selections`);

        // Find the active editor
        if (!this.activeEditor) {
            this.logger.warn("No active editor for selection update");
            return;
        }

        // Skip non-file documents
        if (this.activeEditor.document.uri.scheme !== "file") {
            return;
        }

        // Set flag to ignore selection changes triggered by this update
        this.ignoreSelectionChange = true;

        try {
            // Convert Ki selections to VSCode selections
            const selections: vscode.Selection[] = params.selections.map((sel) => {
                return new vscode.Selection(
                    new vscode.Position(sel.anchor.line, sel.anchor.character),
                    new vscode.Position(sel.active.line, sel.active.character),
                );
            });

            // Store current visible ranges before applying selections
            const visibleRanges = this.activeEditor.visibleRanges;

            // Apply selections to the active editor if we have any
            if (selections.length > 0) {
                this.activeEditor.selections = selections;

                // Ensure the primary selection is visible only if it's not already visible
                const primarySelection = selections[0];
                const primaryActive = primarySelection.active;

                // Check if the primary active position is already visible
                const isVisible = visibleRanges.some(
                    (range) => primaryActive.line >= range.start.line && primaryActive.line <= range.end.line,
                );

                // Only reveal if not already visible
                if (!isVisible) {
                    this.activeEditor.revealRange(
                        new vscode.Range(primaryActive, primaryActive),
                        vscode.TextEditorRevealType.InCenterIfOutsideViewport,
                    );
                }
            }

            // Return the latest visible ranges after revealing the primary selection
            this.dispatcher.sendNotification("viewport.change", {
                buffer_id: this.activeEditor.document.uri.toString(),
                visible_line_ranges: this.activeEditor.visibleRanges.map((range) => ({
                    start: Math.max(0, range.start.line - JUMP_SAFETY_PADDING),
                    end: range.end.line + JUMP_SAFETY_PADDING,
                })),
            });
        } finally {
            // Reset flag immediately after applying the selection
            // This ensures that subsequent key presses are processed correctly
            this.ignoreSelectionChange = false;
        }
    }

    /**
     * Dispose of resources
     */
    public override dispose(): void {
        super.dispose();
    }
}
