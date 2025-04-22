import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import type { ModeParams } from "../protocol/ModeParams";

/**
 * Handles selection mode events between VSCode and Ki
 */
export class SelectionModeHandler implements vscode.Disposable {
    private dispatcher: Dispatcher;
    private logger: Logger;
    private currentSelectionMode: string = "normal";
    private statusBarItem: vscode.StatusBarItem;

    constructor(dispatcher: Dispatcher, logger: Logger) {
        this.dispatcher = dispatcher;
        this.logger = logger;

        // Create status bar item for selection mode
        this.dispatcher.registerKiNotificationHandler("selection_mode.change", (params) =>
            this.handleSelectionModeChange(params),
        );
        this.dispatcher.registerEventHandler("ki.sync", () => this.syncSelectionMode());

        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 99);
        this.updateStatusBar();
        this.statusBarItem.show();

        this.registerEventHandlers();
    }

    /**
     * Register event handlers
     */
    private registerEventHandlers(): void {
        // Register for selection mode change notifications
    }

    /**
     * Handle selection mode change notification from Ki
     */
    private handleSelectionModeChange(params: ModeParams): void {
        // Validate params
        if (!params.mode) {
            this.logger.warn("Invalid selection_mode.change notification params");
            return;
        }

        this.logger.log(`Selection mode changed to: ${params.mode}`);

        // Update current selection mode
        this.currentSelectionMode = params.mode;

        // Update status bar
        this.updateStatusBar();
    }

    /**
     * Set the current selection mode
     */
    public setSelectionMode(mode: string): void {
        // Only send if mode is changing
        if (this.currentSelectionMode === mode) {
            return;
        }

        this.logger.log(`Setting selection mode to: ${mode}`);

        // Update context key
        vscode.commands.executeCommand("setContext", "ki.selection_mode", mode);

        // Notify Ki about the selection mode change
        const activeEditor = vscode.window.activeTextEditor;
        if (activeEditor) {
            const buffer_id = activeEditor.document.uri.toString();
            // Note: SelectionModeParams is aliased to ModeParams in the protocol
            this.dispatcher.sendNotification("selection_mode.set", { buffer_id: buffer_id, mode: mode });
        } else {
            this.logger.warn("No active editor found when trying to send selection_mode.set notification.");
        }

        // Update current selection mode (actual update will come from Ki)
        this.currentSelectionMode = mode;
        this.updateStatusBar();
    }

    /**
     * Update status bar to reflect current selection mode
     */
    private updateStatusBar(): void {
        this.statusBarItem.text = `$(list-selection) ${this.formatSelectionMode(this.currentSelectionMode)}`;
        this.statusBarItem.tooltip = "Ki Selection Mode";
    }

    /**
     * Format selection mode for display
     */
    private formatSelectionMode(mode: string): string {
        return mode.charAt(0).toUpperCase() + mode.slice(1) + " Select";
    }

    /**
     * Sync selection mode with Ki
     */
    private syncSelectionMode(): void {
        // No need to sync selection mode to Ki as it's controlled by Ki
        // and we just reflect its state
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.statusBarItem.dispose();
    }
}
