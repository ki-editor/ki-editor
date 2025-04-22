import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import type { BufferChange } from "../protocol/BufferChange";
import type { BufferParams } from "../protocol/BufferParams";
import { normalizeKiPathToVSCodeUri } from "../utils";

/**
 * Handles buffer-related events between VSCode and Ki
 */
export class BufferHandler implements vscode.Disposable {
    private dispatcher: Dispatcher;
    private logger: Logger;
    private openBuffers: Map<string, { document: vscode.TextDocument; version: number }> = new Map();
    private ignoreNextBufferChange: boolean = false;
    private pendingChanges: Map<string, vscode.TextDocumentContentChangeEvent[]> = new Map();
    private changeTimeout: NodeJS.Timeout | null = null;

    constructor(dispatcher: Dispatcher, logger: Logger) {
        this.dispatcher = dispatcher;
        this.logger = logger;
        this.registerEventHandlers();

        // Register success handlers
        this.dispatcher.registerSuccessHandler("buffer.open", () => this.onBufferOpenSuccess());
        this.dispatcher.registerSuccessHandler("buffer.close", () => this.onBufferCloseSuccess());
        this.dispatcher.registerSuccessHandler("buffer.save", () => this.onBufferSaveSuccess());
        this.dispatcher.registerSuccessHandler("buffer.change", () => this.onBufferChangeSuccess());
    }

    /**
     * Register event handlers
     */
    private registerEventHandlers(): void {
        // Register VSCode document events
        this.dispatcher.registerEventHandler("document.open", (params) => this.handleDocumentOpen(params.document));
        this.dispatcher.registerEventHandler("document.close", (params) => this.handleDocumentClose(params.document));
        this.dispatcher.registerEventHandler("document.save", (params) => this.handleDocumentSave(params.document));
        this.dispatcher.registerEventHandler("document.change", (params) => this.handleDocumentChange(params.event));
        this.dispatcher.registerEventHandler("editor.active", (params) => this.handleEditorActive(params.editor));

        // Register Ki notification handlers
        this.dispatcher.registerKiNotificationHandler("buffer.update", (params) => this.handleBufferUpdate(params));
        this.dispatcher.registerKiNotificationHandler("buffer.diff", (params) => this.handleBufferDiff(params));
        this.dispatcher.registerKiNotificationHandler("buffer.ack", (params) => this.handleBufferAck(params));
        this.dispatcher.registerEventHandler("ki.sync", () => this.syncBuffers());
    }

    /**
     * Success handlers
     */
    private onBufferOpenSuccess(): void {
        this.logger.log(`Buffer open operation completed successfully`);
    }

    private onBufferCloseSuccess(): void {
        this.logger.log(`Buffer close operation completed successfully`);
    }

    private onBufferSaveSuccess(): void {
        this.logger.log(`Buffer save operation completed successfully`);
    }

    private onBufferChangeSuccess(): void {
        this.logger.log(`Buffer change operation completed successfully`);
        this.ignoreNextBufferChange = false;
    }

    /**
     * Initialize open editors on extension activation
     */
    public initializeOpenEditors(): void {
        this.logger.log("Initializing open editors");

        // Process all open text editors
        vscode.window.visibleTextEditors.forEach((editor) => {
            const document = editor.document;

            // Skip non-file documents
            if (document.uri.scheme !== "file") {
                return;
            }

            // Send open notification to Ki
            this.sendBufferOpenNotification(document);
        });

        // Set active editor
        if (vscode.window.activeTextEditor) {
            this.handleEditorActive({ editor: vscode.window.activeTextEditor });
        }
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
            this.logger.warn(`Skipping large file: ${document.uri.toString()}`);
            return;
        }

        // Send open notification to Ki
        this.sendBufferOpenNotification(document);
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
            content: null,
            language_id: document.languageId,
            version: document.version,
        });

        // Remove from openBuffers
        this.openBuffers.delete(uri);
    }

    /**
     * Handle document save event
     */
    private handleDocumentSave(params: { document: vscode.TextDocument }): void {
        const { document } = params;

        // Skip non-file documents
        if (document.uri.scheme !== "file") {
            return;
        }

        const uri = document.uri.toString();
        this.logger.log(`Document saved: ${uri}`);

        // Send save notification to Ki
        this.dispatcher.sendRequest("buffer.save", {
            uri: uri,
            content: null,
            language_id: document.languageId,
            version: document.version,
        });
    }

    /**
     * Handle document change event
     */
    private handleDocumentChange(event: vscode.TextDocumentChangeEvent): void {
        const document = event.document;

        // Skip non-file documents and empty changes
        if (document.uri.scheme !== "file" || event.contentChanges.length === 0) {
            return;
        }

        const uri = document.uri.toString();

        // If we're ignoring the next change (due to a Ki-initiated update), skip it
        if (this.ignoreNextBufferChange) {
            this.logger.log(`Ignoring document change (Ki-initiated): ${uri}`);
            this.ignoreNextBufferChange = false;
            return;
        }

        // Collect changes
        this.addPendingChange(uri, event.contentChanges);

        // Debounce changes to avoid sending too many updates
        this.debouncePendingChanges();
    }

    /**
     * Add a pending change to the queue
     */
    private addPendingChange(uri: string, changes: readonly vscode.TextDocumentContentChangeEvent[]): void {
        if (!this.pendingChanges.has(uri)) {
            this.pendingChanges.set(uri, []);
        }

        const pendingChanges = this.pendingChanges.get(uri)!;
        changes.forEach((change) => pendingChanges.push(change));
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
     * Process all pending changes
     */
    private processPendingChanges(): void {
        // Process each uri's changes
        for (const [uri, changes] of this.pendingChanges.entries()) {
            if (changes.length === 0) continue;

            const document = vscode.workspace.textDocuments.find((doc) => doc.uri.toString() === uri);
            if (!document) continue;

            // Update the buffer version
            const version = this.openBuffers.get(uri)?.version || document.version;
            this.openBuffers.set(uri, { document, version: version + 1 });

            // For now, we'll simply send the full content
            // In a more optimized version, we could send just the changes
            // --- TEMPORARILY DISABLED --- //
            // TODO: Implement proper incremental changes or re-enable if backend handles full content better.
            /*
            this.dispatcher.sendRequest("buffer.change", {
                buffer_id: uri,
                start_line: 0, // Placeholder
                end_line: document.lineCount, // Placeholder
                content: document.getText(), // Sending full content is causing issues
                version: version + 1,
                message_id: Date.now(),
                retry_count: 0,
            });
            */
            this.logger.warn(
                `Buffer change detected in VSCode for ${uri}, but sending update to Ki is temporarily disabled.`,
            );
        }

        // Clear pending changes
        this.pendingChanges.clear();
    }

    /**
     * Handle editor active event
     */
    private handleEditorActive(params: { editor: vscode.TextEditor | undefined }): void {
        const { editor } = params;
        if (!editor) return;

        const document = editor.document;

        // Skip non-file documents
        if (document.uri.scheme !== "file") {
            return;
        }

        const uri = document.uri.toString();
        this.logger.log(`Editor active: ${uri}`);

        // Send active notification to Ki
        this.dispatcher.sendNotification("buffer.active", {
            uri: uri,
            content: null,
            language_id: document.languageId,
            version: document.version,
        });

        // Ensure the buffer is open in Ki
        if (!this.openBuffers.has(uri)) {
            this.sendBufferOpenNotification(document);
        }
    }

    /**
     * Handle buffer update notifications from Ki
     */
    private async handleBufferUpdate(params: BufferChange): Promise<void> {
        this.logger.log(`Received buffer.update: ${params.buffer_id}`);

        // Normalize the buffer_id before looking up the document
        const normalizedUri = normalizeKiPathToVSCodeUri(params.buffer_id);
        this.logger.log(`Normalized buffer URI for lookup: ${normalizedUri}`);

        // Find the document by normalized URI
        const document = vscode.workspace.textDocuments.find((doc) => doc.uri.toString() === normalizedUri);

        if (!document) {
            this.logger.warn(
                `Document not found for buffer update. Original: ${params.buffer_id}, Normalized: ${normalizedUri}`,
            );
            return;
        }

        // Apply the partial update using the existing method
        this.applyBufferUpdate(document, params);
    }

    /**
     * Handle buffer diff notification from Ki
     */
    private handleBufferDiff(params: BufferChange): void {
        this.logger.log(`Received buffer.diff: ${params.buffer_id}`);

        // Normalize the buffer_id before looking up the document
        const normalizedUri = normalizeKiPathToVSCodeUri(params.buffer_id);
        this.logger.log(`Normalized buffer URI for lookup: ${normalizedUri}`);

        // Find the document by normalized URI
        const document = vscode.workspace.textDocuments.find((doc) => doc.uri.toString() === normalizedUri);

        if (!document) {
            this.logger.warn(
                `Document not found for buffer diff. Original: ${params.buffer_id}, Normalized: ${normalizedUri}`,
            );
            return;
        }

        // Set ignoreNextBufferChange to avoid echo
        this.ignoreNextBufferChange = true;

        // Apply diff to the document (using the same update logic for now)
        this.applyBufferUpdate(document, params);
    }

    /**
     * Handle buffer acknowledge from Ki
     */
    private handleBufferAck(params: bigint): void {
        this.logger.log(`Received buffer.ack: ${params}`);

        // Find and remove the acknowledged version
    }

    /**
     * Apply buffer update to the document
     */
    private applyBufferUpdate(document: vscode.TextDocument, params: BufferChange): void {
        const edit = new vscode.WorkspaceEdit();

        // Create range for the update
        const startPos = new vscode.Position(params.start_line, 0);
        const endLine = Math.min(params.end_line, document.lineCount - 1);
        const endPos = new vscode.Position(endLine, document.lineAt(endLine).text.length);
        const range = new vscode.Range(startPos, endPos);

        // Apply edit
        edit.replace(document.uri, range, params.content);

        // Execute the edit
        vscode.workspace.applyEdit(edit).then((success) => {
            if (!success) {
                this.logger.error(`Failed to apply buffer update: ${params.buffer_id}`);
            } else {
                this.logger.log(`Successfully applied buffer update: ${params.buffer_id}`);

                // Update version
                const uri = document.uri.toString();
                this.openBuffers.set(uri, { document, version: params.version });
            }
        });
    }

    /**
     * Send buffer open notification to Ki
     */
    private sendBufferOpenNotification(document: vscode.TextDocument): void {
        const uri = document.uri.toString();

        this.logger.log(`Sending buffer.open for ${uri}`);

        // Add to openBuffers map
        this.openBuffers.set(uri, { document, version: document.version });

        // Send request to Ki
        this.dispatcher.sendRequest("buffer.open", {
            uri: uri,
            content: document.getText(),
            language_id: document.languageId,
            version: document.version,
        });
    }

    /**
     * Synchronize buffers between VSCode and Ki
     */
    private syncBuffers(): void {
        if (!vscode.window.activeTextEditor) return;

        const document = vscode.window.activeTextEditor.document;
        if (document.uri.scheme !== "file") return;

        const uri = document.uri.toString();

        // Only sync if we have the buffer tracked
        if (this.openBuffers.has(uri)) {
            const version = this.openBuffers.get(uri)!.version;

            // Update version and send current content
            this.openBuffers.set(uri, { document, version: version + 1 });

            this.dispatcher.sendNotification("buffer.change", {
                buffer_id: uri,
                start_line: 0,
                end_line: document.lineCount,
                content: document.getText(),
                version: version + 1,
                message_id: Date.now(),
                retry_count: 0,
            });
        }
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        if (this.changeTimeout) {
            clearTimeout(this.changeTimeout);
        }
        this.openBuffers.clear();
        this.pendingChanges.clear();
    }
}
