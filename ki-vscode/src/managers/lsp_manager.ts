import * as vscode from "vscode";
import { Manager } from "./manager";

/** Handles LSP Requests fired from Ki. */
export class LspManager extends Manager {
    public initialize(): void {
        this.dispatcher.registerKiNotificationHandler(
            "lsp.definition",
            async () => {
                await vscode.commands.executeCommand(
                    "editor.action.revealDefinition",
                );
            },
        );

        this.dispatcher.registerKiNotificationHandler("lsp.hover", async () => {
            await vscode.commands.executeCommand("editor.action.showHover");
        });

        this.dispatcher.registerKiNotificationHandler(
            "lsp.references",
            async () => {
                await vscode.commands.executeCommand(
                    "editor.action.goToReferences",
                );
            },
        );

        this.dispatcher.registerKiNotificationHandler(
            "lsp.declaration",
            async () => {
                await vscode.commands.executeCommand(
                    "editor.action.goToDeclaration",
                );
            },
        );

        this.dispatcher.registerKiNotificationHandler(
            "lsp.typeDefinition",
            async () => {
                await vscode.commands.executeCommand(
                    "editor.action.goToTypeDefinition",
                );
            },
        );
        this.dispatcher.registerKiNotificationHandler(
            "lsp.implementation",
            async () => {
                await vscode.commands.executeCommand(
                    "editor.action.goToImplementation",
                );
            },
        );
    }
}
