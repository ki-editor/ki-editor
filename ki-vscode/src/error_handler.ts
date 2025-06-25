import * as vscode from "vscode";
import type { Logger } from "./logger";

/**
 * Error severity levels
 */
export enum ErrorSeverity {
    /**
     * Informational message, not an error
     */
    Info = "info",

    /**
     * Warning message, non-critical error
     */
    Warning = "warning",

    /**
     * Error message, critical error
     */
    Error = "error",

    /**
     * Fatal error, application cannot continue
     */
    Fatal = "fatal",
}

/**
 * Error context information
 */
interface ErrorContext {
    /**
     * Component where the error occurred
     */
    component: string;

    /**
     * Operation that was being performed
     */
    operation: string;

    /**
     * Additional context information
     */
    details?: Record<string, unknown>;
}

/**
 * Centralized error handling service
 */
export class ErrorHandler {
    private logger: Logger;

    constructor(logger: Logger) {
        this.logger = logger;
    }

    /**
     * Handle an error
     *
     * @param error Error object or message
     * @param context Error context information
     * @param severity Error severity level
     * @param showToUser Whether to show the error to the user
     */
    public handleError(
        error: unknown,
        context: ErrorContext,
        severity: ErrorSeverity = ErrorSeverity.Error,
        showToUser = false,
    ): void {
        // Extract error message
        const errorMessage = this.formatErrorMessage(error);

        // Format context information
        const contextInfo = `[${context.component}] ${context.operation}`;

        // Log the error with context
        switch (severity) {
            case ErrorSeverity.Info:
                this.logger.log(`${contextInfo}: ${errorMessage}`);
                break;
            case ErrorSeverity.Warning:
                this.logger.warn(`${contextInfo}: ${errorMessage}`);
                break;
            case ErrorSeverity.Error:
            case ErrorSeverity.Fatal:
                this.logger.error(
                    `${contextInfo}: ${errorMessage}`,
                    context.details,
                );
                break;
        }

        // Show error to user if requested
        if (showToUser) {
            this.showErrorToUser(errorMessage, severity);
        }

        // Additional handling for fatal errors
        if (severity === ErrorSeverity.Fatal) {
            // Log additional information
            this.logger.error(
                "Fatal error occurred, application may be unstable",
                {
                    error: errorMessage,
                    context,
                },
            );
        }
    }

    /**
     * Format an error message
     *
     * @param error Error object or message
     * @returns Formatted error message
     */
    private formatErrorMessage(error: unknown): string {
        if (error instanceof Error) {
            return `${error.message}\n${error.stack || ""}`;
        }
        if (typeof error === "string") {
            return error;
        }
        return String(error);
    }

    /**
     * Show an error message to the user
     *
     * @param message Error message
     * @param severity Error severity level
     */
    private showErrorToUser(message: string, severity: ErrorSeverity): void {
        switch (severity) {
            case ErrorSeverity.Info:
                vscode.window.showInformationMessage(`Ki: ${message}`);
                break;
            case ErrorSeverity.Warning:
                vscode.window.showWarningMessage(`Ki: ${message}`);
                break;
            case ErrorSeverity.Error:
            case ErrorSeverity.Fatal:
                vscode.window.showErrorMessage(`Ki: ${message}`);
                break;
        }
    }
}
