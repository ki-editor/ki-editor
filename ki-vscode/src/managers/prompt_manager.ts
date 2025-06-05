import * as vscode from "vscode";
import * as path from "path";
import * as os from "os";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";
import { PromptOpenedParams, PromptItem } from "../protocol/types";

export class PromptManager extends Manager {
    private onDidSaveTextDocumentHandler: vscode.Disposable | undefined;
    constructor(dispatcher: Dispatcher, logger: Logger, eventHandler: EventHandler) {
        super(dispatcher, logger, eventHandler);
    }

    public initialize(): void {
        this.eventHandler.onPromptOpened((params) => this.handlePromptOpened(params));
    }

    private async handlePromptOpened(params: PromptOpenedParams) {
        const result = await showComboInput({ title: params.title, items: params.items });
        this.dispatcher.sendNotification("prompt.enter", result);
    }
}

async function showComboInput(params: { title: string; items: PromptItem[] }): Promise<string | undefined> {
    const quickPick = vscode.window.createQuickPick();
    quickPick.items = params.items;
    quickPick.placeholder = params.title;
    quickPick.canSelectMany = false;

    // Allow custom input
    quickPick.onDidChangeValue(() => {
        const currentValue = quickPick.value;

        if (currentValue && !params.items.some((item) => item.label === currentValue)) {
            // If user types something not in the list, add it as an option
            quickPick.items = [{ label: currentValue, description: "(custom)" }, ...params.items];
        } else {
            quickPick.items = params.items;
        }
    });

    quickPick.show();

    return new Promise((resolve) => {
        quickPick.onDidHide(() => resolve(undefined));
        quickPick.onDidAccept(() => {
            const selectedValue = quickPick.selectedItems[0]?.label || quickPick.value;
            quickPick.hide();
            resolve(selectedValue);
        });
    });
}
