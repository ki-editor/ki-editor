import * as vscode from "vscode";
import type { JumpsParams, MarksParams } from "../protocol/types";
import { Manager } from "./manager";

export const JUMP_SAFETY_PADDING = 10;

/**
 * Manages selection synchronization between VSCode and Ki
 */
export class DecorationManager extends Manager {
    private jumpCharDecoration = vscode.window.createTextEditorDecorationType({
        backgroundColor: "red",
        color: "transparent",
        before: {
            // Width zero is necessary for the character to render on-top instead of before the expected position
            width: "0",
            color: "white",
        },
    });

    private marksDecoration = vscode.window.createTextEditorDecorationType({
        backgroundColor: new vscode.ThemeColor("editor.findMatchHighlightBackground"),
    });

    public initialize(): void {
        vscode.window.onDidChangeTextEditorVisibleRanges((event) => {
            // This condition is necessary, otherwise the visible ranges changes
            // of non-file editor, say, Output, will also be sent to Ki
            if (event.textEditor.document.uri.scheme !== "file") {
                return;
            }
            this.handleVisibleRangesChanged(event);
        });

        this.dispatcher.registerKiNotificationHandler("editor.jump", (params: JumpsParams) => {
            this.handleJumpsChanged(params);
        });
        this.dispatcher.registerKiNotificationHandler("editor.mark", (params: MarksParams) => {
            this.handleMarksChanged(params);
        });
    }

    private handleJumpsChanged(jumps: JumpsParams): void {
        const uri = jumps.uri;
        const editor = vscode.window.visibleTextEditors.find((editor) => editor.document.uri.path === uri);
        if (!editor) return;

        const decorations: vscode.DecorationOptions[] = jumps.targets.map((jump) => {
            const start = new vscode.Position(jump.position.line, jump.position.character);
            const end = new vscode.Position(start.line, start.character + 1);
            const result: vscode.DecorationOptions = {
                range: new vscode.Range(start, end),
                renderOptions: {
                    before: { contentText: jump.key },
                },
            };
            return result;
        });
        editor.setDecorations(this.jumpCharDecoration, []);
        editor.setDecorations(this.jumpCharDecoration, decorations);
    }

    private handleMarksChanged(params: MarksParams): void {
        const editor = vscode.window.visibleTextEditors.find((editor) => editor.document.uri.path === params.uri);
        if (!editor) return;

        const decorations: vscode.DecorationOptions[] = params.marks.map((mark) => {
            const start = new vscode.Position(mark.start.line, mark.start.character);
            const end = new vscode.Position(mark.end.line, mark.end.character);
            const result: vscode.DecorationOptions = {
                range: new vscode.Range(start, end),
            };
            return result;
        });
        editor.setDecorations(this.marksDecoration, []);
        editor.setDecorations(this.marksDecoration, decorations);
    }

    private handleVisibleRangesChanged(event: vscode.TextEditorVisibleRangesChangeEvent): void {
        this.dispatcher.sendNotification("viewport.change", {
            buffer_id: event.textEditor.document.uri.toString(),
            visible_line_ranges: event.visibleRanges.map((range) => ({
                start: Math.max(0, range.start.line - JUMP_SAFETY_PADDING),
                end: range.end.line + JUMP_SAFETY_PADDING,
            })),
        });
    }
}
