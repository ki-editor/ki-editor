import * as vscode from "vscode";
import * as z from "zod";

/**
 * Simple logger for the Ki extension
 */
export class Logger {
    private outputChannel: vscode.OutputChannel;
    private verbose: boolean;

    constructor(name: string, verbose = true) {
        this.outputChannel = vscode.window.createOutputChannel(`Ki: ${name}`);
        this.verbose = verbose;
    }

    /**
     * Log an informational message
     */
    public log(message: string, ...args: unknown[]): void {
        // Skip most log messages in non-verbose mode
        if (
            !this.verbose &&
            !message.includes("error") &&
            !message.includes("fail") &&
            !message.includes("start") &&
            !message.includes("activate") &&
            !message.includes("initialize")
        ) {
            return;
        }
        this.logMessage("INFO", message, args);
    }

    /**
     * Log a warning message
     */
    public warn(message: string, ...args: unknown[]): void {
        // Always log warnings
        this.logMessage("WARN", message, args);
    }

    /**
     * Log an error message
     */
    public error(message: string, ...args: unknown[]): void {
        // Always log errors
        this.logMessage("ERROR", message, args);
    }

    /**
     * Format and log a message with timestamp
     */
    private logMessage(level: string, message: string, args: unknown[]): void {
        const disableDebug = z.enum(["true", "false"]).nullish().parse(process.env.DISABLE_DEBUG) === "true";

        if (disableDebug) {
            // Skip logging if not debugging the extension
            return;
        }
        const timestamp = new Date().toISOString();
        let logMessage = `[${timestamp}] [${level}] ${message}`;

        // Add additional arguments if provided
        if (args && args.length > 0) {
            args.forEach((arg) => {
                if (arg instanceof Error) {
                    logMessage += `\n    ${arg.message}`;
                    if (arg.stack) {
                        logMessage += `\n    ${arg.stack}`;
                    }
                } else if (typeof arg === "object") {
                    try {
                        logMessage += `\n    ${JSON.stringify(arg)}`;
                    } catch (e) {
                        logMessage += "\n    [Object]";
                    }
                } else {
                    logMessage += `\n    ${arg}`;
                }
            });
        }

        this.outputChannel.appendLine(logMessage);
    }

    /**
     * Show the log output channel
     */
    public show(): void {
        this.outputChannel.show();
    }

    /**
     * Dispose of the output channel
     */
    public dispose(): void {
        this.outputChannel.dispose();
    }
}
