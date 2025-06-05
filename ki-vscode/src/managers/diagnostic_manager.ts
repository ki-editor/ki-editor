import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { DiagnosticSeverity } from "../protocol/types";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";

export class DiagnosticManager extends Manager {
    constructor(dispatcher: Dispatcher, logger: Logger, eventHandler: EventHandler) {
        super(dispatcher, logger, eventHandler);
    }

    public initialize(): void {
        this.registerVSCodeEventHandler("diagnostics.change", (params) => this.handleDiagnosticChange(params));
    }

    private handleDiagnosticChange(params: { uri: vscode.Uri; diagnostics: vscode.Diagnostic[] }[]) {
        this.dispatcher.sendRequest(
            "diagnostics.change",
            params.map(({ uri, diagnostics }) => ({
                path: uri.path,
                diagnostics: diagnostics.map((diagnostic) => ({
                    ...diagnostic,
                    severity: ((): DiagnosticSeverity => {
                        switch (diagnostic.severity) {
                            case vscode.DiagnosticSeverity.Hint:
                                return DiagnosticSeverity.Hint;
                            case vscode.DiagnosticSeverity.Warning:
                                return DiagnosticSeverity.Warning;
                            case vscode.DiagnosticSeverity.Error:
                                return DiagnosticSeverity.Error;
                            case vscode.DiagnosticSeverity.Information:
                                return DiagnosticSeverity.Information;
                        }
                    })(),
                })),
            })),
        );
    }
}
