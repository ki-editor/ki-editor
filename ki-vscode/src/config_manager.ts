import * as vscode from "vscode";
import type { Logger } from "./logger";

/**
 * Configuration keys
 */
enum ConfigKey {
    /**
     * Path to the Ki backend executable
     */
    BackendPath = "ki.backendPath",

    /**
     * Enable debug logging
     */
    EnableDebugLogging = "ki.enableDebugLogging",

    /**
     * Maximum file size to process (in bytes)
     */
    MaxFileSize = "ki.maxFileSize",
}

/**
 * Configuration manager for the Ki extension
 *
 * This class provides a centralized way to access and modify configuration settings
 * for the Ki extension. It also provides a way to listen for configuration changes.
 *
 * @example
 * ```typescript
 * const configManager = new ConfigManager(logger);
 * const backendPath = configManager.getBackendPath();
 * const isDebugLoggingEnabled = configManager.isDebugLoggingEnabled();
 * ```
 */
export class ConfigManager {
    /** Logger instance */
    private logger: Logger;

    /** VSCode workspace configuration */
    private config: vscode.WorkspaceConfiguration;

    /** Map of configuration change listeners */
    private changeListeners: Map<ConfigKey, ((value: unknown) => void)[]> =
        new Map();

    /** Disposables for event listeners */
    private disposables: vscode.Disposable[] = [];

    /**
     * Creates a new ConfigManager instance
     *
     * @param logger Logger instance for logging
     */
    constructor(logger: Logger) {
        this.logger = logger;
        this.config = vscode.workspace.getConfiguration("ki");

        // Listen for configuration changes
        this.disposables.push(
            vscode.workspace.onDidChangeConfiguration(
                this.handleConfigChange.bind(this),
            ),
        );
    }

    /**
     * Get a configuration value
     *
     * @param key Configuration key
     * @param defaultValue Default value if not set
     * @returns Configuration value
     */
    public get<T>(key: ConfigKey, defaultValue?: T): T {
        const value = this.config.get<T>(
            key.replace("ki.", ""),
            defaultValue as T,
        );
        return value as T;
    }

    /**
     * Set a configuration value
     *
     * @param key Configuration key
     * @param value Configuration value
     * @param target Configuration target
     * @returns Promise that resolves when the value is set
     */
    public async set<T>(
        key: ConfigKey,
        value: T,
        target: vscode.ConfigurationTarget = vscode.ConfigurationTarget.Global,
    ): Promise<void> {
        await this.config.update(key.replace("ki.", ""), value, target);
    }

    /**
     * Register a listener for configuration changes
     *
     * @param key Configuration key
     * @param listener Listener function
     * @returns Disposable to unregister the listener
     */
    public onDidChangeConfiguration(
        key: ConfigKey,
        listener: (value: unknown) => void,
    ): vscode.Disposable {
        let listeners = this.changeListeners.get(key);
        if (!listeners) {
            listeners = [];
            this.changeListeners.set(key, listeners);
        }

        listeners.push(listener);

        return {
            dispose: () => {
                const listeners = this.changeListeners.get(key);
                if (listeners) {
                    const index = listeners.indexOf(listener);
                    if (index !== -1) {
                        listeners.splice(index, 1);
                    }
                }
            },
        };
    }

    /**
     * Handle configuration changes
     *
     * @param event Configuration change event
     */
    private handleConfigChange(event: vscode.ConfigurationChangeEvent): void {
        if (!event.affectsConfiguration("ki")) {
            return;
        }

        // Reload configuration
        this.config = vscode.workspace.getConfiguration("ki");

        // Notify listeners
        for (const [key, listeners] of this.changeListeners.entries()) {
            if (event.affectsConfiguration(key)) {
                const value = this.get(key);
                for (const listener of listeners) {
                    try {
                        listener(value);
                    } catch (error) {
                        this.logger.error(
                            `Error in configuration change listener for ${key}:`,
                            error,
                        );
                    }
                }
            }
        }
    }

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.disposables.forEach((d) => d.dispose());
        this.disposables = [];
        this.changeListeners.clear();
    }

    /**
     * Get the backend path
     *
     * @returns Backend path or undefined if not set
     */
    public getBackendPath(): string | undefined {
        return this.get<string>(ConfigKey.BackendPath);
    }

    /**
     * Get whether debug logging is enabled
     *
     * @returns True if debug logging is enabled
     */
    public isDebugLoggingEnabled(): boolean {
        return this.get<boolean>(ConfigKey.EnableDebugLogging, false);
    }

    /**
     * Get the maximum file size to process (in bytes)
     *
     * @returns Maximum file size in bytes
     */
    public getMaxFileSize(): number {
        return this.get<number>(ConfigKey.MaxFileSize, 2 * 1024 * 1024); // 2MB default
    }
}
