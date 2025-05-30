import * as vscode from "vscode";
import { Dispatcher, EventName, EventParams } from "../dispatcher";
import { Logger } from "../logger";
import { EventHandler } from "./event_handler";

/**
 * Base class for all managers in the Ki VSCode extension
 */
export abstract class Manager implements vscode.Disposable {
    protected dispatcher: Dispatcher;
    protected logger: Logger;
    protected eventHandler: EventHandler;
    protected disposables: vscode.Disposable[] = [];

    constructor(dispatcher: Dispatcher, logger: Logger, eventHandler: EventHandler) {
        this.dispatcher = dispatcher;
        this.logger = logger;
        this.eventHandler = eventHandler;
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

    /**
     * Register a VSCode event handler
     */
    protected registerVSCodeEventHandler<T extends EventName>(
        event: T,
        handler: (params: EventParams<T>) => void,
    ): void {
        this.dispatcher.registerEventHandler(event, handler as (params: unknown) => void);
    }
}
