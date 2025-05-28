import * as vscode from "vscode";
import { Dispatcher } from "../dispatcher";
import { Logger } from "../logger";
import { JumpsParams, SelectionSet, MarksParams } from "../protocol/types";
import { EventHandler } from "./event_handler";
import { Manager } from "./manager";
import { ModeManager } from "./mode_manager";

export const JUMP_SAFETY_PADDING = 10;

/**
 * Manages selection synchronization between VSCode and Ki
 */
export class DecorationManager extends Manager {
    private activeEditor: vscode.TextEditor | undefined;
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

    constructor(dispatcher: Dispatcher, logger: Logger, eventHandler: EventHandler) {
        super(dispatcher, logger, eventHandler);
    }

    /**
     * Initialize the selection manager
     */
    public initialize(): void {
        // Register VSCode event handlers
        this.registerVSCodeEventHandler("editor.visibleRanges", (params) =>
            this.handleVisibleRangesChanged(params.event),
        );

        this.eventHandler.onJumpsChange((params) => this.handleJumpsChanged(params));
        this.eventHandler.onMarksChange((params) => this.handleMarksChanged(params));

        // Initialize with active editor
        this.activeEditor = vscode.window.activeTextEditor;
    }

    private handleJumpsChanged(jumps: JumpsParams): void {
        const editor = this.activeEditor;
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
        const editor = this.activeEditor;
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
