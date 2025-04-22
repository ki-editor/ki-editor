import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import type { SelectionSet } from "../protocol/SelectionSet";
import { normalizeKiPathToVSCodeUri } from "../utils";

/**
 * Handles selection-related events between VSCode and Ki
 */
export class SelectionHandler implements vscode.Disposable {
    private dispatcher: Dispatcher;
    private logger: Logger;
    private suppressNextSelectionUpdate: boolean = false;
    private lastSelections: Map<string, vscode.Selection[]> = new Map();
    private pendingSelectionUpdate: boolean = false;

    constructor(dispatcher: Dispatcher, logger: Logger) {
        this.dispatcher = dispatcher;
        this.logger = logger;
        this.registerEventHandlers();

        // Register success handlers
        this.dispatcher.registerSuccessHandler("selection.set", () => this.onSelectionSetSuccess());
        this.dispatcher.registerSuccessHandler("selection.get", () => this.onSelectionGetSuccess());
    }

    /**
     * Register event handlers
     */
    private registerEventHandlers(): void {
        // Handle selection changes from VSCode
        this.dispatcher.registerEventHandler("editor.selection", (params) => this.handleSelectionChange(params.event));

        // Handle selection updates from Ki
        this.dispatcher.registerKiNotificationHandler("selection.update", (params) =>
            this.handleSelectionUpdate(params),
        );
        this.dispatcher.registerEventHandler("ki.sync", () => this.syncSelections());

        // Handle internal event from CursorHandler to prevent feedback loops
        this.dispatcher.registerEventHandler("internal.preApplyKiCursorUpdate", (params) => {
            this.logger.log("Received internal pre-apply cursor update event, suppressing next selection change.");
            this.suppressNextSelectionUpdate = true;
        });
    }

    /**
     * Success handlers
     */
    private onSelectionSetSuccess(): void {
        this.logger.log("Selection set operation completed successfully");
        this.suppressNextSelectionUpdate = false;
    }

    private onSelectionGetSuccess(): void {
        this.logger.log("Selection get operation completed successfully");
    }

    /**
     * Handle selection change event from VSCode
     */
    private handleSelectionChange(event: vscode.TextEditorSelectionChangeEvent): void {
        const editor = event.textEditor;

        // Skip if we should suppress this update
        if (this.suppressNextSelectionUpdate || this.pendingSelectionUpdate) {
            this.suppressNextSelectionUpdate = false;
            this.pendingSelectionUpdate = false;
            return;
        }

        // Skip non-file documents
        if (editor.document.uri.scheme !== "file") {
            return;
        }

        const uri = editor.document.uri.toString();
        const selections = event.selections;

        // Skip if there are no selections
        if (selections.length === 0) {
            return;
        }

        // Store last selections to detect changes
        const lastSelections = this.lastSelections.get(uri) || [];

        // Check if selections have changed
        const selectionsChanged = !this.areSelectionsEqual(lastSelections, selections);

        if (selectionsChanged) {
            // Update last selections - create a new array to avoid readonly issues
            this.lastSelections.set(uri, Array.from(selections));

            // Convert to Ki selection format (using anchor/active)
            const kiSelections = selections.map((s) => ({
                // Explicitly map vscode.Position to protocol.Position
                anchor: { line: s.anchor.line, character: s.anchor.character },
                active: { line: s.active.line, character: s.active.character },
                is_extended: !s.isEmpty, // Map isEmpty to is_extended
            }));

            // Find the primary selection index (usually the last one in VSCode)
            const primaryIndex = selections.findIndex((sel) => sel.isEqual(editor.selection)) || 0;

            // Send selection update to Ki as a notification
            this.pendingSelectionUpdate = true;
            this.dispatcher.sendNotification("selection.set", {
                buffer_id: uri,
                primary: primaryIndex,
                selections: kiSelections,
            });
            this.pendingSelectionUpdate = false;
        }
    }

    /**
     * Handle selection update notification from Ki
     */
    private handleSelectionUpdate(params: SelectionSet): void {
        this.logger.log(`Received selection.update: ${params.buffer_id}`);

        // Normalize the buffer_id before using it
        const normalizedUri = normalizeKiPathToVSCodeUri(params.buffer_id);
        this.logger.log(`Normalized buffer URI for lookup: ${normalizedUri}`);

        // Find the editor using the normalized URI
        const editor = this.findEditorForBuffer(normalizedUri);
        if (!editor) {
            // Log both original and normalized for debugging
            this.logger.warn(
                `Editor not found for selection update. Original: ${params.buffer_id}, Normalized: ${normalizedUri}`,
            );
            return;
        }

        // Set suppressNextSelectionUpdate to avoid echo
        this.suppressNextSelectionUpdate = true;

        // Apply selection update to the editor
        this.applySelectionUpdate(editor, params);
    }

    /**
     * Apply selection update to the editor
     */
    private applySelectionUpdate(editor: vscode.TextEditor, params: SelectionSet): void {
        if (!params.selections || params.selections.length === 0) {
            this.logger.warn("No selections in update");
            return;
        }

        try {
            // Create VSCode selections
            const selections = params.selections.map((sel) => {
                // Use anchor/active directly, removing fallback
                const anchorPos = new vscode.Position(sel.anchor.line, sel.anchor.character);
                const activePos = new vscode.Position(sel.active.line, sel.active.character);
                return new vscode.Selection(anchorPos, activePos);
            });

            // Set the selections in the editor
            editor.selections = selections;

            // Ensure the primary selection is visible
            // (using the primary index from Ki if available)
            const primaryIndex = typeof params.primary === "number" ? params.primary : 0;
            const primarySelection = selections[primaryIndex < selections.length ? primaryIndex : 0];

            editor.revealRange(
                new vscode.Range(primarySelection.start, primarySelection.end),
                vscode.TextEditorRevealType.InCenterIfOutsideViewport,
            );

            // Store last selections
            const uri = editor.document.uri.toString();
            this.lastSelections.set(uri, Array.from(selections));

            this.logger.log(`Applied selection update with ${selections.length} selection(s)`);
        } catch (err) {
            this.logger.error(`Error applying selection update: ${err}`);
            this.suppressNextSelectionUpdate = false;
        }
    }

    /**
     * Find the editor for a buffer URI
     */
    private findEditorForBuffer(normalizedUri: string): vscode.TextEditor | undefined {
        // Check the active editor first
        const activeEditor = vscode.window.activeTextEditor;
        // Compare normalized URI
        if (activeEditor && activeEditor.document.uri.toString() === normalizedUri) {
            return activeEditor;
        }

        // Check all visible editors, comparing normalized URI
        return vscode.window.visibleTextEditors.find((editor) => editor.document.uri.toString() === normalizedUri);
    }

    /**
     * Compare two arrays of selections for equality
     */
    private areSelectionsEqual(selectionsA: vscode.Selection[], selectionsB: readonly vscode.Selection[]): boolean {
        if (selectionsA.length !== selectionsB.length) {
            return false;
        }

        return selectionsA.every((selA, index) => {
            const selB = selectionsB[index];
            return (
                selA.start.line === selB.start.line &&
                selA.start.character === selB.start.character &&
                selA.end.line === selB.end.line &&
                selA.end.character === selB.end.character
            );
        });
    }

    /**
     * Synchronize selections between VSCode and Ki
     */
    private syncSelections(): void {
        if (!vscode.window.activeTextEditor) return;

        const editor = vscode.window.activeTextEditor;
        if (editor.document.uri.scheme !== "file") return;

        const uri = editor.document.uri.toString();
        const selections = editor.selections;

        if (selections.length === 0) return;

        // Convert to Ki selection format
        const kiSelections = selections.map((s) => ({
            // Explicitly map vscode.Position to protocol.Position
            anchor: { line: s.anchor.line, character: s.anchor.character },
            active: { line: s.active.line, character: s.active.character },
            is_extended: !s.isEmpty, // Map isEmpty to is_extended
        }));

        // Find the primary selection index
        const primaryIndex = selections.findIndex((sel) => sel.isEqual(editor.selection)) || 0;

        // Send selection update to Ki
        this.dispatcher.sendNotification("selection.set", {
            buffer_id: uri,
            primary: primaryIndex,
            selections: kiSelections,
        });
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.lastSelections.clear();
    }
}
