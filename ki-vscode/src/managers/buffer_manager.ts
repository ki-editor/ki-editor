import * as vscode from "vscode";
import type { Dispatcher } from "../dispatcher";
import type { Logger } from "../logger";
import type { BufferParams } from "../protocol/types";
import {
    type BufferDiffParams,
    type DiffEdit,
    EditorAction,
} from "../protocol/types";
import { Manager } from "./manager";
import type { ModeManager } from "./mode_manager";

/**
 * Type for buffer changed events
 */
interface BufferChangedEvent {
    path: string;
    transaction: {
        edits: {
            range: {
                start: number;
                end: number;
            };
            new_text: string;
            // Add protocol_range to store the original range information from the protocol
            protocol_range?: {
                start: {
                    line: number;
                    character: number;
                };
                end: {
                    line: number;
                    character: number;
                };
            };
        }[];
    };
}

/**
 * Manages buffer synchronization between VSCode and Ki
 */
export class BufferManager extends Manager {
    private modeManager: ModeManager;
    private openBuffers: Map<
        string,
        { document: vscode.TextDocument; version: number }
    > = new Map();
    private ignoreBufferChangeCounter = 0;
    private pendingBufferDiffs: Map<string, BufferDiffParams[]> = new Map();
    private processingBufferDiffs: Map<string, boolean> = new Map();
    private pendingChangesets: Map<
        string,
        vscode.TextDocumentContentChangeEvent[][]
    > = new Map();
    private changeTimeout: NodeJS.Timeout | null = null;
    private commandDisposables: vscode.Disposable[] = [];

    constructor(
        dispatcher: Dispatcher,
        logger: Logger,
        modeManager: ModeManager,
    ) {
        super(dispatcher, logger);
        this.modeManager = modeManager;
    }

    /**
     * Initialize the buffer manager
     */
    public initialize(): void {
        // Register VSCode document events
        vscode.workspace.onDidOpenTextDocument((document) =>
            this.handleDocumentOpen(document),
        );
        vscode.workspace.onDidCloseTextDocument((document) =>
            this.handleDocumentClose(document),
        );
        vscode.workspace.onDidSaveTextDocument((document) =>
            this.handleDocumentSave(document),
        );
        vscode.workspace.onDidChangeTextDocument((event) =>
            this.handleDocumentChange(event),
        );
        vscode.window.onDidChangeActiveTextEditor((editor) =>
            this.handleEditorActive({ editor }),
        );

        // Register events from Ki
        this.dispatcher.registerKiNotificationHandler(
            "buffer.diff",
            (params: BufferDiffParams) => {
                this.handleBufferDiff(params);
            },
        );
        this.dispatcher.registerKiNotificationHandler(
            "buffer.open",
            (params: BufferParams) => {
                this.logger.log(`Buffer opened: ${params.uri}`);
            },
        );
        this.dispatcher.registerKiNotificationHandler(
            "buffer.save",
            (params: BufferParams) => {
                this.handleBufferSave(params);
            },
        );

        this.dispatcher.registerKiNotificationHandler(
            "editor.syncBufferRequest",
            (params) => {
                this.handleSyncBuffer(params);
            },
        );

        // Register undo/redo command listeners
        this.registerUndoRedoCommands();

        this.initializeActiveEditor();
    }

    private handleSyncBuffer(params: { uri: string }) {
        const document = vscode.workspace.textDocuments.find(
            (doc) => doc.uri.toString() === params.uri.toString(),
        );
        if (document) {
            this.dispatcher.sendNotification("editor.syncBufferResponse", {
                uri: params.uri,
                content: document.getText(),
            });
        } else {
            this.logger.error(
                `Unable to find document with URI "${params.uri}"`,
            );
        }
    }

    private handleBufferSave(params: BufferParams): void {
        this.saveOpenDocument(vscode.Uri.parse(params.uri));
    }

    private async saveOpenDocument(uri: vscode.Uri): Promise<void> {
        // Find the document if it's already open
        const document = vscode.workspace.textDocuments.find(
            (doc) => doc.uri.toString() === uri.toString(),
        );

        if (document?.isDirty) {
            await document.save();
        }
    }

    /**
     * Register undo/redo command listeners
     */
    private registerUndoRedoCommands(): void {
        // Register our handlers for the main undo/redo commands
        this.commandDisposables.push(
            vscode.commands.registerCommand("undo", () =>
                this.handleUndoRedo(true),
            ),
            vscode.commands.registerCommand("redo", () =>
                this.handleUndoRedo(false),
            ),
            // Also override the default commands to prevent them from being executed
            vscode.commands.registerCommand("default:undo", () =>
                this.handleUndoRedo(true),
            ),
            vscode.commands.registerCommand("default:redo", () =>
                this.handleUndoRedo(false),
            ),
            // And the keyboard shortcuts
            vscode.commands.registerCommand("editor.action.undo", () =>
                this.handleUndoRedo(true),
            ),
            vscode.commands.registerCommand("editor.action.redo", () =>
                this.handleUndoRedo(false),
            ),
        );

        // Add command disposables to our disposables array
        this.commandDisposables.forEach((d) => this.registerDisposable(d));
    }

    public initializeActiveEditor(): void {
        this.logger.log("Initializing open editors");

        // Process all open text editors
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const document = editor.document;

        // Skip non-file documents
        if (document.uri.scheme !== "file") {
            return;
        }

        // Send open notification to Ki
        this.sendBufferOpenNotificationToKi(document);
    }

    /**
     * Handle document open event
     */
    private handleDocumentOpen(document: vscode.TextDocument): void {
        // Skip non-file documents
        if (document.uri.scheme !== "file") {
            return;
        }

        // Skip large files (over 5MB)
        if (document.getText().length > 5 * 1024 * 1024) {
            return;
        }

        // Send open notification to Ki
        this.sendBufferOpenNotificationToKi(document);
    }

    /**
     * Handle document close event
     */
    private handleDocumentClose(document: vscode.TextDocument): void {
        // Skip non-file documents
        if (document.uri.scheme !== "file") {
            return;
        }

        const uri = document.uri.toString();
        this.logger.log(`Document closed: ${uri}`);

        // Send close notification to Ki
        this.dispatcher.sendRequest("buffer.close", {
            uri: uri,
        });

        // Remove from openBuffers
        this.openBuffers.delete(uri);
    }

    /**
     * Handle document save event
     */
    private handleDocumentSave(document: vscode.TextDocument): void {
        // Skip non-file documents
        if (document.uri.scheme !== "file") {
            return;
        }

        const uri = document.uri.toString();
        this.logger.log(`Document saved: ${uri}`);

        // Send save notification to Ki
        this.dispatcher.sendRequest("buffer.save", {
            uri: uri,
        });
    }

    private handleDocumentChange(event: vscode.TextDocumentChangeEvent): void {
        const document = event.document;

        // Skip non-file documents and empty changes
        if (
            document.uri.scheme !== "file" ||
            event.contentChanges.length === 0
        ) {
            return;
        }

        const uri = document.uri.toString();

        // If we're ignoring changes (due to Ki-initiated updates), skip it
        if (this.ignoreBufferChangeCounter > 0) {
            this.logger.log(
                `Ignoring document change (Ki-initiated): ${uri}, counter: ${this.ignoreBufferChangeCounter}`,
            );
            return;
        }

        // Normal case: collect changes and debounce
        this.addPendingChange(
            uri,
            event.contentChanges.map((x) => x),
        );

        // Debounce changes to avoid sending too many updates
        this.debouncePendingChanges();
    }

    /**
     * Add a pending change to the queue
     */
    private addPendingChange(
        uri: string,
        changes: vscode.TextDocumentContentChangeEvent[],
    ): void {
        if (!this.pendingChangesets.has(uri)) {
            this.pendingChangesets.set(uri, []);
        }

        const pendingChangeset = this.pendingChangesets.get(uri);
        pendingChangeset?.push(changes);
    }

    /**
     * Debounce pending changes to avoid sending too many updates
     */
    private debouncePendingChanges(): void {
        if (this.changeTimeout) {
            clearTimeout(this.changeTimeout);
        }

        this.changeTimeout = setTimeout(() => {
            this.processPendingChanges();
            this.changeTimeout = null;
        }, 50); // 50ms debounce
    }

    /**
     * Process all pending changes and send them as diffs to Ki
     */
    private processPendingChanges(): void {
        for (const [uri, changeset] of this.pendingChangesets.entries()) {
            for (const changes of changeset) {
                if (changes.length === 0) continue;

                const document = vscode.workspace.textDocuments.find(
                    (doc) => doc.uri.toString() === uri,
                );
                if (!document) {
                    this.logger.warn(
                        `Document not found for pending changes: ${uri}`,
                    );
                    continue;
                }

                // Convert VSCode changes to Ki DiffEdit format
                const edits: DiffEdit[] = changes.map((change) => {
                    return {
                        range: {
                            start: {
                                line: change.range.start.line,
                                character: change.range.start.character,
                            },
                            end: {
                                line: change.range.end.line,
                                character: change.range.end.character,
                            },
                        },
                        new_text: change.text,
                    };
                });

                this.logger.log(
                    `Sending buffer.change with ${edits.length} diffs for ${uri}`,
                );

                // Send the diff edits to Ki via buffer.change InputMessage
                this.dispatcher.sendNotification("buffer.change", {
                    buffer_id: uri,
                    edits: edits,
                });

                // Update internal version tracking if necessary (using VSCode's version is safer)
                const currentBufferInfo = this.openBuffers.get(uri);
                if (currentBufferInfo) {
                    this.openBuffers.set(uri, {
                        document,
                        version: document.version,
                    });
                } else {
                    // If buffer wasn't tracked, add it now
                    this.openBuffers.set(uri, {
                        document,
                        version: document.version,
                    });
                }
            }
        }

        // Clear pending changes after processing all URIs
        this.pendingChangesets.clear();
    }

    /**
     * Handle editor active event
     */
    private handleEditorActive(params: {
        editor: vscode.TextEditor | undefined;
    }): void {
        const { editor } = params;
        if (!editor) return;

        const document = editor.document;

        // Skip non-file documents
        if (document.uri.scheme !== "file") {
            return;
        }

        const uri = document.uri.toString();

        // Send active notification to Ki
        this.dispatcher.sendNotification("buffer.active", {
            uri: uri,
        });
    }

    /**
     * Handle buffer diff event from Ki
     */
    private async handleBufferDiff(params: BufferDiffParams): Promise<void> {
        // Don't update VS Code if Ki is in insert mode
        // Because in insert mode, the modifications is relayed to VS Code
        // the buffer sync direction is reversed, where Ki is listening for changes
        // from VS Code.
        if (this.modeManager.getCurrentMode() === "insert") {
            return;
        }

        this.logger.log(`Handling buffer diff event for ${params.buffer_id}`);

        // Skip empty edits
        if (params.edits.length === 0) {
            this.logger.log(
                `Skipping buffer diff with no edits for ${params.buffer_id}`,
            );
            return;
        }

        // Log the edits for debugging
        for (const edit of params.edits) {
            this.logger.log(
                `Edit range: ${edit.range.start.line},${edit.range.start.character} to ${edit.range.end.line},${edit.range.end.character} with text of length ${edit.new_text.length}`,
            );
        }

        // Normalize the buffer_id before looking up the document
        const normalizedUri = params.buffer_id;
        this.logger.log(`Buffer URI for lookup: ${normalizedUri}`);

        // Find the document by normalized URI
        let document = vscode.workspace.textDocuments.find(
            (doc) => doc.uri.toString() === normalizedUri,
        );

        if (!document) {
            // Try to find the document by path
            const path = normalizedUri.replace("file://", "");
            document = vscode.workspace.textDocuments.find(
                (doc) =>
                    doc.uri.fsPath === path ||
                    doc.uri.toString().includes(path),
            );

            if (!document) {
                // Try to find any open document that might match
                for (const doc of vscode.workspace.textDocuments) {
                    if (doc.uri.scheme === "file") {
                        this.logger.log(
                            `Checking if ${doc.uri.toString()} matches ${normalizedUri}`,
                        );
                        if (
                            doc.uri.toString().includes(normalizedUri) ||
                            normalizedUri.includes(doc.uri.fsPath)
                        ) {
                            document = doc;
                            this.logger.log(
                                `Found matching document: ${doc.uri.toString()}`,
                            );
                            break;
                        }
                    }
                }
            }

            if (!document) {
                this.logger.warn(
                    `Document not found for buffer diff: ${normalizedUri}`,
                );
                return;
            }
        }

        // Add the diff to the pending queue
        if (!this.pendingBufferDiffs.has(normalizedUri)) {
            this.pendingBufferDiffs.set(normalizedUri, []);
        }
        this.pendingBufferDiffs.get(normalizedUri)?.push(params);

        // Start processing the queue if not already processing
        if (!this.processingBufferDiffs.get(normalizedUri)) {
            await this.processBufferDiffQueue(normalizedUri);
        }
    }

    /**
     * Process buffer diff queue for a specific URI
     */
    private async processBufferDiffQueue(uri: string): Promise<void> {
        // Mark as processing
        this.processingBufferDiffs.set(uri, true);

        try {
            // Process all pending diffs in order
            while (
                this.pendingBufferDiffs.has(uri) &&
                (this.pendingBufferDiffs.get(uri)?.length ?? 0) > 0
            ) {
                const params: BufferDiffParams | undefined =
                    this.pendingBufferDiffs.get(uri)?.shift();
                if (!params) return;

                // Convert to our internal format for processing
                const event: BufferChangedEvent = {
                    path: params.buffer_id,
                    transaction: {
                        edits: params.edits.map((edit) => ({
                            range: {
                                start: 0,
                                end: 0,
                            },
                            new_text: edit.new_text,
                            // Preserve the original range information from the protocol
                            protocol_range: edit.range,
                        })),
                    },
                };

                await this.applyBufferDiff(uri, event);
            }
        } finally {
            // Clean up
            this.processingBufferDiffs.set(uri, false);
            if (this.pendingBufferDiffs.has(uri)) {
                this.pendingBufferDiffs.delete(uri);
            }
        }
    }

    /**
     * Apply a single buffer diff
     */
    private async applyBufferDiff(
        uri: string,
        event: BufferChangedEvent,
    ): Promise<void> {
        this.logger.log(`Applying buffer diff for ${uri}`);

        // Try to find the document by URI
        let document = vscode.workspace.textDocuments.find(
            (doc) => doc.uri.toString() === uri,
        );

        // If not found, try to find by path
        if (!document) {
            this.logger.warn(
                `Document not found by URI for buffer diff: ${uri}`,
            );

            // Try with and without file:// prefix
            let path = uri;
            if (uri.startsWith("file://")) {
                path = uri.replace(/^file:\/\//, "");
            } else {
                path = `file://${uri}`;
            }

            // Try to find the document by path instead of URI
            const allDocs = vscode.workspace.textDocuments;
            for (const doc of allDocs) {
                if (
                    doc.uri.fsPath === event.path ||
                    doc.uri.toString().includes(event.path) ||
                    event.path.includes(doc.uri.fsPath) ||
                    doc.uri.toString() === path ||
                    doc.uri.fsPath === path.replace(/^file:\/\//, "")
                ) {
                    this.logger.log(
                        `Found document by path instead: ${doc.uri.toString()}`,
                    );
                    document = doc;
                    break;
                }
            }

            // If still not found, try to find the active editor
            if (!document) {
                const activeEditor = vscode.window.activeTextEditor;
                if (activeEditor) {
                    this.logger.log(
                        `Using active editor as fallback: ${activeEditor.document.uri.toString()}`,
                    );
                    document = activeEditor.document;
                } else {
                    this.logger.error(
                        `Document not found for buffer diff and no active editor: ${uri}`,
                    );
                    return;
                }
            }
        }

        // Increment counter to ignore upcoming change events from VSCode
        this.ignoreBufferChangeCounter++;
        this.logger.log(
            `Incremented ignore counter to ${this.ignoreBufferChangeCounter} for ${event.path}`,
        );

        const editor = vscode.window.activeTextEditor;
        await editor?.edit((editBuilder) => {
            for (const kiEdit of event.transaction.edits) {
                // Check if we have protocol_range information
                if (kiEdit.protocol_range) {
                    const startPos = new vscode.Position(
                        kiEdit.protocol_range.start.line,
                        kiEdit.protocol_range.start.character,
                    );
                    const endPos = new vscode.Position(
                        kiEdit.protocol_range.end.line,
                        kiEdit.protocol_range.end.character,
                    );
                    const vscodeRange = new vscode.Range(startPos, endPos);
                    editBuilder.replace(vscodeRange, kiEdit.new_text);
                }
            }
        });

        // It's crucial to decrement the counter *after* the edit is potentially processed by VSCode's change event listener.
        // Using a small timeout ensures the decrement happens after the current event loop cycle.
        return new Promise<void>((resolve) => {
            setTimeout(() => {
                this.ignoreBufferChangeCounter = Math.max(
                    0,
                    this.ignoreBufferChangeCounter - 1,
                );
                this.logger.log(
                    `Decremented ignore counter to ${this.ignoreBufferChangeCounter} for ${event.path}`,
                );
                resolve();
            }, 10); // Small delay to ensure VSCode processes the edit
        });
    }

    /**
     * Handle undo/redo operations
     */
    private handleUndoRedo(isUndo: boolean): void {
        this.logger.log(
            `${isUndo ? "Undo" : "Redo"} operation detected - intercepting VSCode command and sending to Ki`,
        );

        // Get the active document
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            this.logger.warn("No active editor for undo/redo operation");
            return;
        }

        const document = editor.document;
        if (document.uri.scheme !== "file") {
            this.logger.warn("Active document is not a file");
            return;
        }

        this.logger.log(
            `Handling ${isUndo ? "undo" : "redo"} for document: ${document.uri.toString()}`,
        );

        // Increment counter to ignore upcoming change events from VSCode
        // This prevents feedback loops if the undo/redo causes document changes
        this.ignoreBufferChangeCounter++;
        this.logger.log(
            `Incremented ignore counter to ${this.ignoreBufferChangeCounter} for undo/redo operation`,
        );

        // Send the undo/redo command to Ki
        this.dispatcher
            .sendRequest("editor.action", {
                action: isUndo ? EditorAction.Undo : EditorAction.Redo,
                buffer_id: document.uri.toString(),
            })
            .then((response) => {
                this.logger.log(
                    `Sent ${isUndo ? "undo" : "redo"} command to Ki, response: ${JSON.stringify(response)}`,
                );

                // Ki will send buffer.diff events in response to the undo/redo,
                // which will be handled by our normal buffer diff handling code
            })
            .catch((error) => {
                this.logger.error(
                    `Error sending ${isUndo ? "undo" : "redo"} command to Ki:`,
                    error,
                );

                // Reset counter on error
                this.ignoreBufferChangeCounter = Math.max(
                    0,
                    this.ignoreBufferChangeCounter - 1,
                );
            });
    }

    private sendBufferOpenNotificationToKi(
        document: vscode.TextDocument,
    ): void {
        const uri = document.uri.toString();

        this.openBuffers.set(uri, { document, version: document.version });

        // This setTimeout is necessary, otherwise the `editor.selections` would be the default value
        // which is Line 1.
        //
        // Reference: see https://github.com/microsoft/vscode/issues/114047#issue-782319649
        setTimeout(() => {
            const editor = vscode.window.visibleTextEditors.find(
                (e) => e.document.uri.toString() === uri,
            );

            this.dispatcher.sendRequest("buffer.open", {
                uri: uri,
                content: document.getText(),
                selections:
                    editor?.selections.map((selection) => ({
                        anchor: selection.anchor,
                        active: selection.active,
                    })) ?? [],
            });
        }, 10);
    }

    /**
     * Dispose of resources
     */
    public override dispose(): void {
        if (this.changeTimeout) {
            clearTimeout(this.changeTimeout);
        }

        this.openBuffers.clear();
        this.pendingChangesets.clear();
        this.pendingBufferDiffs.clear();
        this.processingBufferDiffs.clear();

        super.dispose();
    }
}
