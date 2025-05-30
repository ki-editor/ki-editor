import * as vscode from "vscode";
import { Manager } from "./manager";

/** Handles LSP Requests fired from Ki. */
export class LspManager extends Manager {
    public initialize(): void {
        this.dispatcher.registerKiNotificationHandler("lsp.definition", async () => {
            await vscode.commands.executeCommand("editor.action.revealDefinition");
        });
    }
}
