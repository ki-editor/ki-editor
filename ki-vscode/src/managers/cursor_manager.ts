import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { SelectionSet } from "../protocol/SelectionSet";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";

/**
 * Manages cursor synchronization between VSCode and Ki
 */
export class CursorManager extends Manager {
    private activeEditor: vscode.TextEditor | undefined;
    private ignoreSelectionChange: boolean = false;
    /**
     * Initialize the cursor manager
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
        this.eventHandler.onSelectionUpdate((event) => this.handleSelectionUpdate(event));

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
     * Handle selection update event from Ki
     */
    private handleSelectionUpdate(params: SelectionSet): void {
        this.logger.log(
            `Received selection update from Ki with ${params.selections.length} selections for buffer ${params.buffer_id}`,
        );

        // Find the editor for this buffer
        let editor = this.activeEditor;

        // If the active editor doesn't match the buffer_id, try to find the correct editor
        if (!editor || editor.document.uri.toString() !== params.buffer_id) {
            // Try to find an editor with a matching URI
            const editors = vscode.window.visibleTextEditors;
            for (const e of editors) {
                if (e.document.uri.toString() === params.buffer_id) {
                    editor = e;
                    break;
                }

                // Also try with and without file:// prefix
                const editorPath = e.document.uri.toString().replace(/^file:\/\//, "");
                const paramsPath = params.buffer_id.replace(/^file:\/\//, "");
                if (editorPath === paramsPath) {
                    editor = e;
                    break;
                }
            }
        }

        if (!editor) {
            this.logger.warn(`No editor found for selection update on buffer ${params.buffer_id}`);
            return;
        }

        // Skip non-file documents
        if (editor.document.uri.scheme !== "file") {
            return;
        }

        // Set flag to ignore selection changes triggered by this update
        this.ignoreSelectionChange = true;

        try {
            // Convert Ki selections to VSCode selections
            const selections: vscode.Selection[] = [];
            for (const sel of params.selections) {
                const anchor = sel.anchor;
                const active = sel.active;

                // Validate positions are within document bounds
                const docLineCount = editor.document.lineCount;

                // Clamp line values to document bounds
                const anchorLine = Math.max(0, Math.min(anchor.line, docLineCount - 1));
                const activeLine = Math.max(0, Math.min(active.line, docLineCount - 1));

                // Clamp character values to line length bounds
                const anchorLineLength = editor.document.lineAt(anchorLine).text.length;
                const activeLineLength = editor.document.lineAt(activeLine).text.length;

                const anchorChar = Math.max(0, Math.min(anchor.character, anchorLineLength));
                const activeChar = Math.max(0, Math.min(active.character, activeLineLength));

                // Create VSCode selection with validated positions
                // Note: In VSCode, the anchor is where the selection starts and active is where the cursor is
                const selection = new vscode.Selection(
                    new vscode.Position(anchorLine, anchorChar),
                    new vscode.Position(activeLine, activeChar),
                );
                selections.push(selection);

                this.logger.log(`Setting cursor at ${activeLine},${activeChar} (anchor: ${anchorLine},${anchorChar})`);
            }

            // Store current visible ranges and cursor position before applying selections
            const visibleRanges = editor.visibleRanges;
            const previousCursorLine = editor.selection.active.line;

            // Apply selections to the editor
            editor.selections = selections;

            // Ensure the primary selection is visible with smart scrolling behavior
            if (selections.length > 0) {
                const primarySelection = selections[0];
                const primaryActive = primarySelection.active;

                // Check if the primary active position is already visible
                const isVisible = visibleRanges.some(
                    (range) => primaryActive.line >= range.start.line && primaryActive.line <= range.end.line,
                );

                if (!isVisible) {
                    // Cursor is not visible, reveal it
                    editor.revealRange(
                        new vscode.Range(primaryActive, primaryActive),
                        vscode.TextEditorRevealType.InCenterIfOutsideViewport,
                    );
                } else {
                    // Cursor is visible, but we need to handle special cases

                    // Calculate how far the cursor has moved
                    const lineDelta = Math.abs(primaryActive.line - previousCursorLine);

                    // Get the top and bottom visible lines
                    const topVisibleLine = Math.min(...visibleRanges.map((r) => r.start.line));
                    const bottomVisibleLine = Math.max(...visibleRanges.map((r) => r.end.line));

                    // Check if cursor is near the edge of the viewport
                    const isNearTop = primaryActive.line - topVisibleLine < 3;
                    const isNearBottom = bottomVisibleLine - primaryActive.line < 3;

                    // If cursor moved significantly but is near the edge, reveal with context
                    if (lineDelta > 1 && (isNearTop || isNearBottom)) {
                        this.logger.log(`Cursor near edge of viewport, revealing with context`);
                        editor.revealRange(
                            new vscode.Range(primaryActive, primaryActive),
                            vscode.TextEditorRevealType.Default,
                        );
                    }
                    // For selection mode changes (like pressing 'w'), don't scroll
                    // This prevents the cursor from jumping when changing selection modes
                }
            }
        } catch (error) {
            this.logger.error(`Error applying selection update: ${error}`);
        } finally {
            // Reset flag immediately to ensure the next key press is processed correctly
            // This is critical for proper handling of key sequences in word mode
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
