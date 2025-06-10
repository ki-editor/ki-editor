import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { ConfigManager } from "./config_manager";
import { Dispatcher } from "./dispatcher";
import { ErrorHandler, ErrorSeverity } from "./error_handler";
import { IPC } from "./ipc";
import { Logger } from "./logger";
import {
    BufferManager,
    DecorationManager,
    CommandManager,
    DiagnosticManager,
    EventHandler,
    KeyboardManager,
    ModeManager,
    SelectionManager,
} from "./managers";
import { LspManager } from "./managers/lsp_manager";
import { PromptManager } from "./managers/prompt_manager";

// Track main extension state
let ipc: IPC | undefined;
let dispatcher: Dispatcher | undefined;
let configManager: ConfigManager | undefined;
let errorHandler: ErrorHandler | undefined;
let disposables: vscode.Disposable[] = [];

/**
 * This method is called when the extension is activated
 */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    // Create logger with minimized verbosity to avoid feedback loops
    const logger = new Logger("Ki", true); // Enable full logging for debugging
    logger.log("Activating Ki extension");

    try {
        // Create config manager, error handler, IPC, and dispatcher
        configManager = new ConfigManager(logger);
        errorHandler = new ErrorHandler(logger);
        ipc = new IPC(logger);
        dispatcher = new Dispatcher(ipc, logger);

        // Get Ki path from config or use fallback paths
        let kiPath = getKiPath(context, configManager, logger);

        logger.log(`Attempting to start Ki process at: ${kiPath}`);

        // Check if file exists
        if (!fs.existsSync(kiPath)) {
            errorHandler.handleError(
                `Ki executable not found at: ${kiPath}`,
                { component: "Extension", operation: "Startup" },
                ErrorSeverity.Fatal,
                true,
            );
            throw new Error(`Ki executable not found at: ${kiPath}`);
        }

        // Log more info about the file
        try {
            const stats = fs.statSync(kiPath);
            logger.log(`Ki executable stats: size=${stats.size}, permissions=${stats.mode.toString(8)}`);
        } catch (err) {
            errorHandler.handleError(
                err,
                { component: "Extension", operation: "GetFileStats", details: { path: kiPath } },
                ErrorSeverity.Warning,
            );
        }

        const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        const cwd = workspaceFolder || process.cwd();

        ipc.start(kiPath, ["@", "vs-code", cwd]);

        // Create event handler
        const eventHandler = new EventHandler(dispatcher, logger, errorHandler);

        // Create managers
        const modeManager = new ModeManager(dispatcher, logger, eventHandler, context);
        const bufferManager = new BufferManager(dispatcher, logger, eventHandler, modeManager);
        const decorationManager = new DecorationManager(dispatcher, logger, eventHandler);
        const keyboardManager = new KeyboardManager(dispatcher, logger, eventHandler, modeManager);
        const selectionManager = new SelectionManager(dispatcher, logger, eventHandler, modeManager);
        const commandManager = new CommandManager(dispatcher, logger, eventHandler);
        const diagnosticManager = new DiagnosticManager(dispatcher, logger, eventHandler);
        const promptManager = new PromptManager(dispatcher, logger, eventHandler);
        const lspManager = new LspManager(dispatcher, logger, eventHandler);

        // Initialize managers
        modeManager.initialize();
        bufferManager.initialize();
        keyboardManager.initialize();
        selectionManager.initialize();
        commandManager.initialize();
        diagnosticManager.initialize();
        promptManager.initialize();
        decorationManager.initialize();
        lspManager.initialize();

        // Add managers and dispatcher to disposables
        disposables = [
            bufferManager,
            keyboardManager,
            modeManager,
            selectionManager,
            commandManager,
            dispatcher,
            diagnosticManager,
            promptManager,
            decorationManager,
            lspManager,
        ];

        // Register all disposables
        context.subscriptions.push(...disposables);

        // The sync command has been removed as it's unnecessary.
        // Ki should be the source of truth, and we should only be reacting to events from Ki,
        // not proactively syncing.

        // Register utility commands
        let outputChannel: vscode.OutputChannel | undefined;
        context.subscriptions.push(
            vscode.commands.registerCommand("ki.showLogs", () => {
                // Create output channel if it doesn't exist
                if (!outputChannel) {
                    outputChannel = vscode.window.createOutputChannel("Ki");
                }
                outputChannel.show();
            }),
            vscode.commands.registerCommand("ki.pingKi", async () => {
                if (dispatcher) {
                    try {
                        const response = await dispatcher.sendRequest("ping");
                        vscode.window.showInformationMessage(`Ki ping response: ${JSON.stringify(response)}`);
                    } catch (err) {
                        errorHandler?.handleError(
                            err,
                            { component: "Extension", operation: "PingKi" },
                            ErrorSeverity.Error,
                            true,
                        );
                    }
                } else {
                    vscode.window.showErrorMessage("Ki is not initialized");
                }
            }),
            vscode.commands.registerCommand("ki.restartKi", () => {
                // Deactivate and reactivate
                deactivate();
                activate(context);
            }),
            vscode.commands.registerCommand("ki.installTreeSitterGrammars", async () => {
                const kiPath = getKiPath(context, configManager, logger);

                function executeCommand(command: string): Promise<void> {
                    return new Promise((resolve, reject) => {
                        const exec = require("child_process").exec;
                        exec(command, (error: Error | null, stdout: string, stderr: string) => {
                            logger.show();
                            logger.log(stdout);

                            if (error) {
                                logger.error(`Error installing grammars: ${error.message}`);
                                vscode.window.showErrorMessage(`Ki: Failed to install grammars: ${error.message}`);
                                reject(error);
                            } else {
                                logger.log(`Grammars installed successfully: ${stdout}`);
                                vscode.window.showInformationMessage(
                                    "Ki: Tree-sitter grammars installed successfully yes",
                                );
                                resolve();
                            }
                            if (stderr) {
                                logger.error(`stderr: ${stderr}`);
                                vscode.window.showErrorMessage(`Ki stderr: ${stderr}`);
                            }
                        });
                    });
                }

                await executeCommand(`${kiPath} @ grammar fetch`);
                await executeCommand(`${kiPath} @ grammar build`);
            }),
        );

        // The periodic sync timer has been removed.
        // Ki should be the source of truth, and we should only be reacting to events from Ki,
        // not proactively syncing.

        logger.log("Ki extension activated successfully");
    } catch (err) {
        if (errorHandler) {
            errorHandler.handleError(
                err,
                { component: "Extension", operation: "Activation" },
                ErrorSeverity.Fatal,
                true,
            );
        } else {
            // Fallback if errorHandler isn't initialized yet
            logger.error("Failed to activate Ki extension:", err);
            vscode.window.showErrorMessage("Failed to initialize Ki extension");
        }

        // Clean up if activation failed
        deactivate();
    }
}

function getKiPath(context: vscode.ExtensionContext, configManager: ConfigManager | undefined, logger: Logger): string {
    function getKiPathInner(context: vscode.ExtensionContext, configManager: ConfigManager | undefined): string {
        const configuredKiPath = configManager?.getBackendPath();
        if (configuredKiPath) {
            return configuredKiPath;
        }

        // First try to use ../target/debug/ki relative to extension
        const debugPath = path.join(context.extensionPath, "..", "target", "debug", "ki");
        if (fs.existsSync(debugPath)) {
            return debugPath;
        }

        // Use the bundled platform-specific executable
        const platform = process.platform;
        const arch = process.arch;

        if (platform === "darwin") {
            // macOS
            if (arch === "arm64") {
                return context.asAbsolutePath(path.join("dist", "bin", "ki-darwin-arm64"));
            }
            return context.asAbsolutePath(path.join("dist", "bin", "ki-darwin-x64"));
        }

        if (platform === "linux") {
            // Linux
            if (arch === "arm64") {
                return context.asAbsolutePath(path.join("dist", "bin", "ki-linux-arm64"));
            }
            return context.asAbsolutePath(path.join("dist", "bin", "ki-linux-x64"));
        }

        if (platform === "win32") {
            // Windows
            return context.asAbsolutePath(path.join("dist", "bin", "ki-win32-x64.exe"));
        }

        throw new Error(`Unsupported platform: ${platform}`);
    }

    // Use the bundled platform-specific executable
    const platform = process.platform;
    const arch = process.arch;
    logger.log(`Detected platform: ${platform}, architecture: ${arch}`);

    const kiPath = getKiPathInner(context, configManager);
    logger.log(`Using platform-specific binary: ${kiPath}`);
    return kiPath;
}

/**
 * This method is called when the extension is deactivated
 */
export function deactivate(): void {
    // Clean up resources
    if (disposables.length > 0) {
        disposables.forEach((disposable) => disposable.dispose());
        disposables = [];
    }

    if (ipc) {
        ipc.stop();
        ipc = undefined;
    }

    // Clean up other resources
    if (configManager) {
        configManager.dispose();
        configManager = undefined;
    }

    dispatcher = undefined;
    errorHandler = undefined;
}
