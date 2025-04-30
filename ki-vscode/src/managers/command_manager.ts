import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { CommandParams } from "../protocol/CommandParams";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";

/**
 * Manages command execution between VSCode and Ki
 */
export class CommandManager extends Manager {
    constructor(dispatcher: Dispatcher, logger: Logger, eventHandler: EventHandler) {
        super(dispatcher, logger, eventHandler);
    }

    /**
     * Initialize the command manager
     */
    public initialize(): void {
        // Register integration event handlers
        this.eventHandler.onCommandExecuted((params) => this.handleCommandExecuted(params));

        // Register commands
        this.registerCommands();
    }

    /**
     * Register commands
     */
    private registerCommands(): void {
        // Register execute command
        const executeCommand = vscode.commands.registerCommand("ki.executeCommand", (command: string) => {
            this.executeCommand(command);
        });
        this.registerDisposable(executeCommand);
    }

    /**
     * Execute a Ki command
     */
    private executeCommand(command: string): void {
        this.logger.log(`Executing Ki command: ${command}`);

        // Get the active editor
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            this.logger.warn("No active editor for command execution");
            return;
        }

        // Skip non-file documents
        if (editor.document.uri.scheme !== "file") {
            return;
        }

        const uri = editor.document.uri.toString();

        // Send command to Ki
        this.dispatcher
            .sendRequest("editor.action", {
                action: command as any, // Cast to any to avoid type error
                buffer_id: uri,
            })
            .then((response) => {
                this.logger.log(`Command executed successfully: ${command}`, response);
            })
            .catch((error) => {
                this.logger.error(`Error executing command: ${command}`, error);
                vscode.window.showErrorMessage(`Error executing Ki command: ${command}`);
            });
    }

    /**
     * Handle command executed event from Ki
     */
    private handleCommandExecuted(params: CommandParams): void {
        this.logger.log(`Received command executed event: ${params.name}`);

        // Show notification for important commands
        if (params.name.startsWith("save") || params.name.startsWith("quit")) {
            vscode.window.showInformationMessage(`Ki command executed: ${params.name}`);
        }
    }

    /**
     * Dispose of resources
     */
    public override dispose(): void {
        super.dispose();
    }
}
