import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { kiConfig } from "./config";
import { Dispatcher } from "./dispatcher";
import { BufferHandler } from "./handlers/buffer_handler";
import { CursorHandler } from "./handlers/cursor_handler";
import { KeyboardHandler } from "./handlers/keyboard_handler";
import { ModeHandler } from "./handlers/mode_handler";
import { SearchHandler } from "./handlers/search_handler";
import { SelectionHandler } from "./handlers/selection_handler";
import { SelectionModeHandler } from "./handlers/selection_mode_handler";
import { IPC } from "./ipc";
import { Logger } from "./logger";

// Track main extension state
let ipc: IPC | undefined;
let dispatcher: Dispatcher | undefined;
let handlers: vscode.Disposable[] = [];

/**
 * This method is called when the extension is activated
 */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    // Create logger with minimized verbosity to avoid feedback loops
    const logger = new Logger("Ki", true); // Enable full logging for debugging
    logger.log("Activating Ki extension");

    try {
        // Create IPC and dispatcher
        ipc = new IPC(logger);
        dispatcher = new Dispatcher(ipc, logger);

        // Get Ki path from config or use fallback paths
        let kiPath = kiConfig.getBackendPath();

        if (!kiPath) {
            // First try to use ../target/debug/ki relative to extension
            const debugPath = path.join(context.extensionPath, "..", "target", "debug", "ki");
            logger.log(`Checking for Ki at debug path: ${debugPath}`);

            if (fs.existsSync(debugPath)) {
                kiPath = debugPath;
                logger.log(`Found Ki at debug path: ${kiPath}`);
            } else {
                // Fall back to the bundled executable
                kiPath = context.asAbsolutePath("dist/ki-vscode");
                logger.log(`Debug path not found, using default path: ${kiPath}`);
            }
        } else {
            logger.log(`Using configured Ki path: ${kiPath}`);
        }

        logger.log(`Attempting to start Ki process at: ${kiPath}`);

        // Check if file exists
        if (!fs.existsSync(kiPath)) {
            logger.error(`Ki executable not found at: ${kiPath}`);
            vscode.window.showErrorMessage(`Ki executable not found at: ${kiPath}`);
            throw new Error(`Ki executable not found at: ${kiPath}`);
        }

        // Log more info about the file
        try {
            const stats = fs.statSync(kiPath);
            logger.log(`Ki executable stats: size=${stats.size}, permissions=${stats.mode.toString(8)}`);
        } catch (err) {
            logger.error(`Error getting file stats: ${err}`);
        }

        // Start the Ki process with vscode flag instead of debug
        ipc.start(kiPath, ["--vs-code"]);

        // Create and register handlers
        const bufferHandler = new BufferHandler(dispatcher, logger);
        const cursorHandler = new CursorHandler(dispatcher, logger);
        // Create mode handler first since keyboard handler depends on it
        const modeHandler = new ModeHandler(dispatcher, logger, context);
        const keyboardHandler = new KeyboardHandler(dispatcher, logger, modeHandler);
        const selectionHandler = new SelectionHandler(dispatcher, logger);
        const selectionModeHandler = new SelectionModeHandler(dispatcher, logger);
        const searchHandler = new SearchHandler(dispatcher, logger);

        // Add handlers to our tracking array
        handlers = [
            bufferHandler,
            cursorHandler,
            keyboardHandler,
            modeHandler,
            selectionHandler,
            selectionModeHandler,
            searchHandler,
            dispatcher,
        ];

        // Register all disposables
        context.subscriptions.push(...handlers);

        // Create a sync command
        const syncCommand = vscode.commands.registerCommand("ki.sync", () => {
            logger.log("Manually syncing Ki state");
            dispatcher?.emit("ki.sync", {});
        });
        context.subscriptions.push(syncCommand);

        // Initialize with currently open editors
        bufferHandler.initializeOpenEditors();

        // Set up periodic sync to ensure Ki and VSCode stay in sync
        const syncInterval = setInterval(() => {
            if (dispatcher) {
                dispatcher.emit("ki.sync", {});
            }
        }, 5000); // Sync every 5 seconds

        // Add sync interval to disposables
        context.subscriptions.push({ dispose: () => clearInterval(syncInterval) });

        logger.log("Ki extension activated successfully");
    } catch (err) {
        logger.error("Failed to activate Ki extension:", err);
        vscode.window.showErrorMessage("Failed to initialize Ki extension");

        // Clean up if activation failed
        deactivate();
    }
}

/**
 * This method is called when the extension is deactivated
 */
export function deactivate(): void {
    // Clean up resources
    if (handlers.length > 0) {
        handlers.forEach((handler) => handler.dispose());
        handlers = [];
    }

    if (ipc) {
        ipc.stop();
        ipc = undefined;
    }

    dispatcher = undefined;
}
