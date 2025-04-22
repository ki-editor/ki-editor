import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import type { CursorParams } from "../protocol/CursorParams";
import type { Position } from "../protocol/Position";
import { normalizeKiPathToVSCodeUri } from "../utils";

// Define editor mode enum if it doesn't exist elsewhere
enum EditorMode {
    Normal = "normal",
    Insert = "insert",
    Visual = "visual",
}

/**
 * Handles cursor events between VSCode and Ki
 */
export class CursorHandler implements vscode.Disposable {
    private suppressNextCursorUpdate: boolean = false;
    private lastSelections: Map<string, vscode.Selection[]> = new Map();
    private disposables: vscode.Disposable[] = [];

    constructor(private dispatcher: Dispatcher, private logger: Logger) {
        this.registerEventHandlers();
    }

    /**
     * Register event handlers
     */
    private registerEventHandlers(): void {
        // Register VSCode events
        this.disposables.push(
            vscode.window.onDidChangeTextEditorSelection((e) => {
                this.handleSelectionChange(e);
            }),
        );

        // Register Ki notifications
        this.dispatcher.registerKiNotificationHandler("cursor.update", (params: CursorParams) =>
            this.handleCursorUpdateFromKi(params),
        );
        this.dispatcher.registerEventHandler("ki.sync", () => this.syncCursors()); // Changed to registerEventHandler

    }

    /**
     * Sync cursors for all visible editors
     */
    private syncCursors(): void {
        this.logger.log("Syncing cursor positions for all editors");

        vscode.window.visibleTextEditors.forEach((editor) => {
            if (editor.document.uri.scheme === "file") {
                const uri = editor.document.uri.toString();
                // Extract both anchor and active positions from selections
                const anchors = editor.selections.map((sel) => this.convertToCursorPosition(sel.anchor));
                const actives = editor.selections.map((sel) => this.convertToCursorPosition(sel.active));

                // Send if there are any selections
                if (actives.length > 0) {
                    this.dispatcher.sendNotification("cursor.update", {
                        buffer_id: uri,
                        anchors: anchors, // Use correct field name
                        actives: actives, // Use correct field name
                    });
                }
            }
        });
    }

    /**
     * Convert VSCode position to Ki cursor position
     */
    private convertToCursorPosition(position: vscode.Position) {
        return {
            line: position.line,
            character: position.character,
        };
    }

    /**
     * Convert Ki cursor position to VSCode position
     */
    private convertFromCursorPosition(position: Position): vscode.Position {
        return new vscode.Position(position.line, position.character);
    }

    /**
     * Send cursor position to Ki with detailed logging
     */
    public sendCursorUpdate(): void {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            this.logger.warn("Cannot send cursor update: No active editor");
            return;
        }

        const position = editor.selection.active;
        const anchorPosition = editor.selection.anchor;
        const filePath = editor.document.uri.fsPath;
        const uri = editor.document.uri.toString();

        this.logger.log(`=== DEBUG: Sending cursor update to Ki ===`);
        this.logger.log(`Active Position: ${position.line}:${position.character}`);
        this.logger.log(`Anchor Position: ${anchorPosition.line}:${anchorPosition.character}`);
        this.logger.log(`File: ${filePath}`);
        this.logger.log(`URI: ${uri}`);

        try {
            this.dispatcher.sendNotification("cursor.update", {
                buffer_id: uri,
                // Send both anchor and active positions
                anchors: [this.convertToCursorPosition(anchorPosition)],
                actives: [this.convertToCursorPosition(position)],
            });
            this.logger.log("=== DEBUG: Cursor update sent successfully ===");
        } catch (error) {
            this.logger.error(`Failed to send cursor update: ${error}`);
        }
    }

    /**
     * Handle selection change event from VSCode
     */
    private handleSelectionChange(e: vscode.TextEditorSelectionChangeEvent): void {
        // Prevent feedback loop: if suppression flag is set, clear and return
        if (this.suppressNextCursorUpdate) {
            this.suppressNextCursorUpdate = false;
            return;
        }
        const editor = e.textEditor;
        const documentUri = editor.document.uri.toString();
        const currentSelections = e.selections;

        // Only process file documents
        if (!editor.document.uri.scheme.startsWith("file")) {
            return;
        }

        // Get current mode - assuming dispatcher has a method to get mode
        // If not available, you might need to add a mode tracking mechanism
        const mode = this.getCurrentEditorMode();

        // In normal mode, we need to update Ki with our selections
        if (mode === EditorMode.Normal || mode === EditorMode.Visual) {
            // Check if selections have changed (compare both anchor and active)
            const lastStoredSelections = this.lastSelections.get(documentUri);
            const selectionsMoved =
                !lastStoredSelections ||
                currentSelections.length !== lastStoredSelections.length || // Check array length
                currentSelections.some((sel, i) => !sel.isEqual(lastStoredSelections[i])); // Check each selection

            if (selectionsMoved) {
                // Log selections for debugging
                this.logger.log(
                    `Selection changed: ${currentSelections
                        .map(
                            (s) =>
                                `[A(${s.anchor.line}:${s.anchor.character}) Ac(${s.active.line}:${s.active.character})]`,
                        )
                        .join(", ")}`,
                );

                // Update our stored selections
                this.lastSelections.set(documentUri, [...currentSelections]); // Store full selections

                // Define a helper to check if a position is valid within the document
                const isPositionValid = (pos: vscode.Position): boolean => {
                    if (pos.line < 0 || pos.character < 0) return false;
                    if (pos.line >= editor.document.lineCount) return false;
                    const lineLength = editor.document.lineAt(pos.line).text.length;
                    if (pos.character > lineLength) return false;
                    return true;
                };

                // Filter selections where both anchor and active positions are valid
                const validSelections = currentSelections.filter(
                    (sel) => isPositionValid(sel.anchor) && isPositionValid(sel.active),
                );

                if (validSelections.length === 0 && currentSelections.length > 0) {
                    this.logger.warn("All current selections have invalid positions, not sending update.");
                    return;
                } else if (validSelections.length === 0) {
                    // If there are no selections at all, maybe send an empty update? Or just log.
                    this.logger.log("No valid selections to send.");
                    // Decide if an empty update should be sent or not.
                    // For now, let's not send if there are no valid selections.
                    return;
                }

                // Convert valid selections to protocol format
                const anchors = validSelections.map((sel) => this.convertToCursorPosition(sel.anchor));
                const actives = validSelections.map((sel) => this.convertToCursorPosition(sel.active));

                const params: CursorParams = {
                    buffer_id: documentUri,
                    anchors: anchors,
                    actives: actives,
                };

                // Send cursor update to Ki
                try {
                    this.dispatcher.sendNotification("cursor.update", params);
                    this.logger.log(`Sent cursor update: Anchors=${anchors.length}, Actives=${actives.length}`);
                } catch (err: unknown) {
                    this.logger.error(`Failed to send cursor update: ${err instanceof Error ? err.message : err}`);
                }
            }
        }
    }

    /**
     * Get the current editor mode
     * This is a helper method to get the mode from the dispatcher or another source
     */
    private getCurrentEditorMode(): EditorMode {
        // Since Dispatcher doesn't have a getMode method, default to Normal mode
        // This could be improved by adding a proper mode tracking mechanism
        return EditorMode.Normal;
    }

    /**
     * Handle cursor update notification from Ki
     */
    private handleCursorUpdateFromKi(params: CursorParams): void {
        this.logger.log(`=== DEBUG: Received cursor update from Ki ===`);
        this.logger.log(`Params: ${JSON.stringify(params)}`);

        // Normalize the buffer_id before using it
        const normalizedUri = normalizeKiPathToVSCodeUri(params.buffer_id);
        this.logger.log(`Normalized buffer URI for lookup: ${normalizedUri}`);

        // Find editor using the normalized URI
        const editor = vscode.window.visibleTextEditors.find((e) => e.document.uri.toString() === normalizedUri);

        if (!editor) {
            // Log both original and normalized for debugging
            this.logger.warn(
                `Cannot apply cursor update: Editor not found for buffer. Original: ${params.buffer_id}, Normalized: ${normalizedUri}`,
            );
            return;
        }

        // Check if anchors and actives exist and are not empty
        if (!params.actives || params.actives.length === 0) {
            this.logger.warn("Received cursor update with no active positions");
            return;
        }

        // Assume anchors array length matches actives if present, otherwise use actives for simple cursors
        const anchors =
            params.anchors && params.anchors.length === params.actives.length ? params.anchors : params.actives; // Fallback for simple cursor updates

        try {
            const selections: vscode.Selection[] = params.actives.map((activePos, index) => {
                const anchorPos = anchors[index]; // Get corresponding anchor
                const vscodeAnchor = this.convertFromCursorPosition(anchorPos);
                const vscodeActive = this.convertFromCursorPosition(activePos);
                return new vscode.Selection(vscodeAnchor, vscodeActive);
            });

            this.logger.log(
                `Applying selections: ${JSON.stringify(
                    selections.map((s) => ({
                        anchor: `${s.anchor.line}:${s.anchor.character}`,
                        active: `${s.active.line}:${s.active.character}`,
                    })),
                )}`,
            );

            // Update editor selection
            if (selections.length > 0) {
                this.lastSelections.set(editor.document.uri.toString(), selections);

                // Emit event to signal other handlers (like SelectionHandler) to suppress
                // subsequent selection changes triggered by this update.
                this.dispatcher.emit("internal.preApplyKiCursorUpdate", {
                    uri: editor.document.uri.toString(),
                });

                // Set flag to prevent immediate feedback loop from onDidChangeTextEditorSelection
                this.suppressNextCursorUpdate = true;
                editor.selections = selections;
            }
        } catch (error) {
            this.logger.error(`Error applying cursor positions: ${error}`);
        }
    }

    /**
     * Dispose of disposables
     */
    public dispose(): void {
        this.disposables.forEach((d) => d.dispose());
        this.lastSelections.clear();
    }
}
