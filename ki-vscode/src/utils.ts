/**
 * Utility functions for the Ki-VSCode extension
 */

import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";

/**
 * Creates a debounced function that delays invoking the provided function
 * until after the specified wait time has elapsed since the last invocation.
 *
 * @param func The function to debounce
 * @param wait The number of milliseconds to delay
 * @returns A debounced version of the function
 */
export function debounce<T extends (...args: unknown[]) => unknown>(func: T, wait: number): (...args: Parameters<T>) => void {
    let timeout: NodeJS.Timeout | null = null;

    return function (this: ThisParameterType<T>, ...args: Parameters<T>): void {
        const later = () => {
            timeout = null;
            func.apply(this, args);
        };

        if (timeout !== null) {
            clearTimeout(timeout);
        }

        timeout = setTimeout(later, wait);
    };
}

/**
 * Throttles a function to execute at most once in the specified time period
 *
 * @param func The function to throttle
 * @param limit The time limit in milliseconds
 * @returns A throttled version of the function
 */
export function throttle<T extends (...args: unknown[]) => unknown>(func: T, limit: number): (...args: Parameters<T>) => void {
    let inThrottle = false;

    return function (this: ThisParameterType<T>, ...args: Parameters<T>): void {
        if (!inThrottle) {
            func.apply(this, args);
            inThrottle = true;

            setTimeout(() => {
                inThrottle = false;
            }, limit);
        }
    };
}

/**
 * Checks if two arrays are equal by comparing their elements
 *
 * @param a First array
 * @param b Second array
 * @returns True if arrays are equal, false otherwise
 */
export function arraysEqual<T>(a: T[], b: T[]): boolean {
    if (a === b) return true;
    if (a.length !== b.length) return false;

    for (let i = 0; i < a.length; i++) {
        if (a[i] !== b[i]) return false;
    }

    return true;
}

/**
 * Performs a deep clone of an object
 *
 * @param obj The object to clone
 * @returns A deep clone of the object
 */
export function deepClone<T>(obj: T): T {
    return JSON.parse(JSON.stringify(obj));
}

/**
 * Formats a file path for display, truncating if necessary
 *
 * @param path The file path to format
 * @param maxLength Maximum length before truncation
 * @returns Formatted path string
 */
export function formatPath(path: string, maxLength = 50): string {
    if (path.length <= maxLength) return path;

    const parts = path.split("/");
    const fileName = parts.pop() || "";

    // Keep the filename intact
    const remainingLength = maxLength - fileName.length - 4; // 4 for ".../"

    if (remainingLength <= 0) {
        return "..." + fileName.slice(-maxLength + 3);
    }

    return ".../" + fileName;
}

/**
 * Converts a VSCode URI to a file path
 *
 * @param uri The VSCode URI
 * @returns File path as string
 */
export function uriToPath(uri: string): string {
    // Remove the 'file://' prefix
    return uri.replace(/^file:\/\//, "");
}

/**
 * Reads a file asynchronously and returns its contents as a string
 *
 * @param filePath Path to the file to read
 * @returns Promise resolving to file contents or null if file doesn't exist
 */
export async function readFile(filePath: string): Promise<string | null> {
    try {
        return await fs.promises.readFile(filePath, "utf8");
    } catch (error) {
        console.error(`Error reading file ${filePath}:`, error);
        return null;
    }
}

/**
 * Gets the path to the OS temporary directory
 *
 * @returns Path to the temp directory
 */
export function getTempDir(): string {
    return os.tmpdir();
}

/**
 * Check if a document should be processed by Ki
 * @param document TextDocument to check
 * @returns boolean True if the document should be processed
 */
export function shouldProcessDocument(document: vscode.TextDocument): boolean {
    // Only process file and untitled documents
    if (document.uri.scheme !== "file" && document.uri.scheme !== "untitled") {
        return false;
    }

    // Skip output channels and logs
    const uri = document.uri.toString();
    if (
        uri.includes("output:") ||
        uri.includes("extension-output") ||
        uri.includes("Output-") ||
        uri.includes("Ki:") || // Our own output panel
        uri.includes("output/") ||
        uri.includes("debug-console") ||
        uri.includes("terminal-") ||
        document.languageId === "log"
    ) {
        return false;
    }

    // Skip documents that are too large (over 5MB)
    const contentLength = document.getText().length;
    if (contentLength > 5000000) {
        return false;
    }

    return true;
}

/**
 * Normalizes buffer IDs received from Ki (which might be `CanonicalizedPath(...)` or `file://CanonicalizedPath(...)`)
 * into a standard VSCode URI string (`file:///...`).
 *
 * @param buffer_id The buffer ID string from Ki.
 * @returns A normalized URI string compatible with VSCode editor lookups.
 */
export function normalizeKiPathToVSCodeUri(buffer_id: string): string {
    if (!buffer_id) return buffer_id;

    let pathPart = buffer_id;

    // Handle potential file:// prefix
    if (pathPart.startsWith("file://")) {
        pathPart = pathPart.slice(7);
    }

    // Extract path from CanonicalizedPath wrapper
    const match = pathPart.match(/^CanonicalizedPath\("(.+)"\)$/);
    if (match && match[1]) {
        pathPart = match[1];
    }

    // Ensure it starts with a leading slash if it's a Unix-like path
    if (!pathPart.startsWith("/") && pathPart.includes("/")) {
        pathPart = "/" + pathPart;
    }

    // Construct the final file:/// URI
    // Use vscode.Uri.file to handle platform differences (e.g., drive letters on Windows)
    return vscode.Uri.file(pathPart).toString();
}

/**
 * Get the path to the Ki debug log file
 */
export function getKiDebugLogPath(): string {
    const homeDir = os.homedir();
    return path.join(homeDir, ".ki", "debug.log");
}

/**
 * A promise that can be resolved or rejected from outside the promise constructor.
 * Useful for coordinating asynchronous operations across components.
 */
export class ManualPromise<T> {
    public promise: Promise<T>;
    public resolve!: (value: T | PromiseLike<T>) => void;
    public reject!: (reason?: unknown) => void;

    constructor() {
        this.promise = new Promise<T>((resolve, reject) => {
            this.resolve = resolve;
            this.reject = reject;
        });
        this.promise.catch((_err) => {
            // Prevent unhandled rejection errors
        });
    }
}

/**
 * Checks if a document URI is a "real" document that should be synchronized with Ki
 * @param uri The document URI to check
 * @returns true if this is a document we should sync with Ki
 */
export function isDocumentBufferType(uri: vscode.Uri | string): boolean {
    // Convert string URIs to URI objects when needed
    const uriObj = typeof uri === "string" ? vscode.Uri.parse(uri) : uri;
    const uriString = uriObj.toString();

    // Log details for debugging, but commented out for performance in production
    // console.log(`[Ki] Checking document: ${uriString} (scheme: ${uriObj.scheme})`);

    // Fast-path rejection for common output/debug/terminal schemes
    if (uriObj.scheme !== "file") {
        // Explicitly block these schemes which are never real documents
        if (
            uriObj.scheme === "output" || // Output panels - high priority check
            uriObj.scheme === "debug" ||
            uriObj.scheme === "vscode" ||
            uriObj.scheme === "untitled" || // New unsaved files
            uriObj.scheme === "git" || // Git scheme
            uriObj.scheme === "terminal" || // Terminal
            uriObj.scheme === "webview" || // Webviews
            uriObj.scheme === "vscode-notebook-cell" || // Notebook cells
            uriObj.scheme === "vsls" || // Live Share
            uriObj.scheme === "gitlens" || // GitLens
            uriObj.scheme === "vscode-test-web" // Test web documents
        ) {
            // console.log(`[Ki] Skipping document with blocked scheme: ${uriObj.scheme}`);
            return false;
        }

        // For now, be conservative and only support file scheme
        // console.log(`[Ki] Skipping document with non-file scheme: ${uriObj.scheme}`);
        return false;
    }

    // Comprehensive check for output-related patterns in the URI string
    const outputPatterns = [
        "output:",
        "extension-output",
        "Output-",
        "output-",
        "debug-console",
        "terminal-renderer",
        "webview",
        "output/",
        "Output Panel",
        "console",
        "vscode-extension",
        "ExtensionHost",
        "extension-host",
        "diagnostics",
        "problems-view",
    ];

    // Check for temporary files and special files we want to exclude
    const temporaryFilePatterns = [
        ".git/",
        "/tmp/",
        "/.temp/",
        "/.vscode/",
        "/node_modules/",
        "ki-vscode", // Special case for our extension files to avoid recursive updates
    ];

    // Fast check against the patterns
    for (const pattern of [...outputPatterns, ...temporaryFilePatterns]) {
        if (uriString.includes(pattern)) {
            // console.log(`[Ki] Skipping excluded document (${pattern}): ${uriString}`);
            return false;
        }
    }

    // Check document size to prevent buffer overflow issues
    try {
        const doc = vscode.workspace.textDocuments.find((doc) => doc.uri.toString() === uriString);

        if (doc) {
            // Check for output-related language IDs
            const outputLanguageIds = ["Log", "Output", "log", "output", "debug", "console"];
            if (outputLanguageIds.includes(doc.languageId)) {
                // console.log(`[Ki] Skipping document with output language ID "${doc.languageId}": ${uriString}`);
                return false;
            }

            // Check for large file size - reduced from 5MB to 2MB for added safety
            const size = doc.getText().length;
            if (size > 2000000) {
                // 2MB limit
                console.log(`[Ki] Skipping large file: ${uriString} (size: ${size})`);
                return false;
            }
        }
    } catch (error) {
        console.log(`[Ki] Error checking document: ${error}`);
        // Be conservative - if we can't check it properly, don't process it
        return false;
    }

    // console.log(`[Ki] Processing document: ${uriString}`);
    return true;
}

/**
 * Check if an editor change should be processed
 * @param editor Editor to check
 * @returns boolean True if the editor change should be processed
 */
export function shouldProcessEditorChange(editor: vscode.TextEditor): boolean {
    // Check if the document is processable
    if (!shouldProcessDocument(editor.document)) {
        return false;
    }

    // Additional editor-specific checks can be added here

    return true;
}

/**
 * Get a string representation of an error
 */
export function formatError(error: unknown): string {
    if (error instanceof Error) {
        return `${error.message}\n${error.stack || ""}`;
    }
    return String(error);
}

/**
 * Safely extract text within a range
 */
export function getTextInRange(document: vscode.TextDocument, range: vscode.Range): string {
    // Make sure the range is valid
    const start = range.start;
    const end = range.end;

    if (start.line < 0 || start.line >= document.lineCount) {
        return "";
    }

    return document.getText(range);
}
