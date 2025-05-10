import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { SelectionSet } from "../protocol/SelectionSet";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";

/**
 * Manages selection synchronization between VSCode and Ki
 */
export class SelectionManager extends Manager {
    private activeEditor: vscode.TextEditor | undefined;
    private ignoreSelectionChange: boolean = false;

    constructor(dispatcher: Dispatcher, logger: Logger, eventHandler: EventHandler) {
        super(dispatcher, logger, eventHandler);
    }

    /**
     * Initialize the selection manager
     */
    public initialize(): void {
        // Register VSCode event handlers
        this.registerVSCodeEventHandler("editor.active", (params: { editor: vscode.TextEditor | undefined }) =>
            this.handleEditorActive(params),
        );
        this.registerVSCodeEventHandler(
            "editor.selection",
            (params: { event: vscode.TextEditorSelectionChangeEvent }) => this.handleSelectionChange(params.event),
        );

        // Register integration event handlers
        this.eventHandler.onSelectionUpdate((params) => this.handleSelectionChanged(params));

        // Initialize with active editor
        this.activeEditor = vscode.window.activeTextEditor;
    }

    /**
     * Handle editor active event
     */
    private handleEditorActive(params: { editor: vscode.TextEditor | undefined }): void {
        this.activeEditor = params.editor;
    }

    /**
     * Handle selection change event from VSCode
     */
    private handleSelectionChange(event: vscode.TextEditorSelectionChangeEvent): void {
        // Skip if we're ignoring selection changes (due to Ki-initiated updates)
        if (this.ignoreSelectionChange) {
            this.logger.log("Ignoring selection change from VSCode (initiated by Ki)");
            return;
        }

        const editor = event.textEditor;
        if (!editor || editor.document.uri.scheme !== "file") {
            return;
        }

        const uri = editor.document.uri.toString();
        const selections = event.selections;

        // Check if this selection change was caused by a mouse interaction
        // VSCode provides this information through the TextEditorSelectionChangeKind enum
        const isMouseSelection = event.kind === vscode.TextEditorSelectionChangeKind.Mouse;

        // Only send selection updates to Ki for mouse interactions
        if (isMouseSelection) {
            this.logger.log(`Mouse selection changed in VSCode: ${uri} with ${selections.length} selections`);

            // Set the flag to ignore the next selection change that will come from Ki
            this.ignoreSelectionChange = true;

            try {
                // Convert VSCode selections to Ki format
                const kiSelections = selections.map((sel) => {
                    return {
                        anchor: {
                            line: sel.anchor.line,
                            character: sel.anchor.character,
                        },
                        active: {
                            line: sel.active.line,
                            character: sel.active.character,
                        },
                        is_extended: false, // We don't have this information from VSCode
                    };
                });

                // Send selection update to Ki
                this.dispatcher.sendNotification("selection.set", {
                    buffer_id: uri,
                    selections: kiSelections,
                    primary: 0, // Always use the first selection as primary
                    mode: undefined, // We don't know the selection mode from VSCode
                });
            } finally {
                // Reset the flag after a short delay to allow Ki to process the selection
                setTimeout(() => {
                    this.ignoreSelectionChange = false;
                }, 50);
            }
        } else {
            this.logger.log(`Ignoring non-mouse selection change in VSCode: ${uri}`);
        }
    }

    /**
     * Handle selection changed event from Ki
     */
    private handleSelectionChanged(params: SelectionSet): void {
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
