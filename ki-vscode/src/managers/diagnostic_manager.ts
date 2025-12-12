import * as vscode from "vscode";
import { DiagnosticSeverity } from "../protocol/types";
import { Manager } from "./manager";

export class DiagnosticManager extends Manager {
    public initialize(): void {
        vscode.languages.onDidChangeDiagnostics((event) =>
            this.handleDiagnosticChange(
                event.uris.map((uri) => ({
                    uri,
                    diagnostics: vscode.languages.getDiagnostics(uri),
                })),
            ),
        );
    }

    private handleDiagnosticChange(
        params: { uri: vscode.Uri; diagnostics: vscode.Diagnostic[] }[],
    ) {
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
