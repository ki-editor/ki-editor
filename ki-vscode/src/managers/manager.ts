import type * as vscode from "vscode";
import type { Dispatcher } from "../dispatcher";
import type { Logger } from "../logger";

/**
 * Base class for all managers in the Ki VSCode extension
 */
export abstract class Manager implements vscode.Disposable {
    protected dispatcher: Dispatcher;
    protected logger: Logger;
    protected disposables: vscode.Disposable[] = [];

    constructor(dispatcher: Dispatcher, logger: Logger) {
        this.dispatcher = dispatcher;
        this.logger = logger;
    }

    /**
     * Initialize the manager
     * This method should be called after all managers are created
     */
    public abstract initialize(): void;

    /**
     * Dispose of resources
     */
    public dispose(): void {
        this.disposables.forEach((d) => d.dispose());
        this.disposables = [];
    }

    /**
     * Register a disposable
     */
    protected registerDisposable(disposable: vscode.Disposable): void {
        this.disposables.push(disposable);
    }
}
