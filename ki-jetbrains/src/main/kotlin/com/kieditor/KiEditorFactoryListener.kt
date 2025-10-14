package com.kieditor

import com.intellij.codeInsight.daemon.impl.HighlightInfo
import com.intellij.lang.annotation.HighlightSeverity
import com.intellij.openapi.components.service
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.event.EditorFactoryEvent
import com.intellij.openapi.editor.event.EditorFactoryListener
import com.intellij.openapi.editor.ex.EditorEx
import com.intellij.openapi.editor.ex.FocusChangeListener
import com.intellij.openapi.editor.ex.MarkupModelEx
import com.intellij.openapi.editor.ex.RangeHighlighterEx
import com.intellij.openapi.editor.ex.util.EditorUtil
import com.intellij.openapi.editor.impl.DocumentMarkupModel
import com.intellij.openapi.editor.impl.event.MarkupModelListener
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.wm.WindowManager
import com.kieditor.protocol.*
import kotlinx.coroutines.launch


class KiEditorFactoryListener : EditorFactoryListener {
    override fun editorCreated(event: EditorFactoryEvent) {
        val editor = event.editor as? EditorEx
            ?: return

        val project = editor.project
            ?: return

        val service = project.service<KiEditor>()

        val file = FileDocumentManager.getInstance().getFile(editor.document)
            ?: return

        val uri = extractKiUri(file)
            ?: return

        editor.kiEditorUri = uri

        val bufferId = uriToBufferId(uri)
            ?: return

        editor.scrollingModel.addVisibleAreaListener { event ->
            val uri = editor.kiEditorUri
                ?: return@addVisibleAreaListener

            val bufferId = uriToBufferId(uri)
                ?: return@addVisibleAreaListener

            val message = InputMessage.ViewportChange(
                ViewportParams(
                    bufferId,
                    listOf(
                        LineRange(
                            event.newRectangle.y.toUInt(),
                            event.newRectangle.y.toUInt() + event.newRectangle.height.toUInt(),
                        )
                    )
                )
            )

            service.scope.launch {
                service.sendNotification(message)
            }
        }

        editor.addFocusListener(object : FocusChangeListener {
            override fun focusGained(editor: Editor) {

                val project = editor.project
                    ?: return

                val uri = editor.kiEditorUri
                    ?: return // todo in-memory buffers

                WindowManager.getInstance().getStatusBar(project).updateWidget(KiModeStatusBarFactory.ID)

                val message = InputMessage.BufferActive(BufferParams(uri))

                val service = project.service<KiEditor>()
                service.scope.launch {
                    service.sendNotification(message)
                }
            }
        })

        val listenerDisposable = Disposer.newDisposable()

        val model = DocumentMarkupModel.forDocument(editor.document, project, true) as MarkupModelEx
        model.addMarkupModelListener(listenerDisposable, object : MarkupModelListener {
            override fun afterAdded(highlighter: RangeHighlighterEx) {
                collectDiagnostics(project, editor, bufferId, model)
            }

            override fun beforeRemoved(highlighter: RangeHighlighterEx) {
                collectDiagnostics(project, editor, bufferId, model)
            }

            override fun afterRemoved(highlighter: RangeHighlighterEx) {
                collectDiagnostics(project, editor, bufferId, model)
            }

            override fun attributesChanged(
                highlighter: RangeHighlighterEx,
                renderersChanged: Boolean,
                fontStyleChanged: Boolean,
                foregroundColorChanged: Boolean
            ) {
                collectDiagnostics(project, editor, bufferId, model)
            }
        })

        EditorUtil.disposeWithEditor(editor, listenerDisposable)
    }
}

private fun collectDiagnostics(project: Project, editor: Editor, bufferId: String, model: MarkupModelEx) {
    val highlights = mutableListOf<HighlightInfo>()
    model.processRangeHighlightersOverlappingWith(0, editor.document.textLength) {
        val info = HighlightInfo.fromRangeHighlighter(it);
        if (info != null && info.highlighter != null) {
            highlights.add(info)
        }
        true
    }

    val diagnostics = highlights.map {
        val start = editor.offsetToLogicalPosition(it.startOffset)
        val end = editor.offsetToLogicalPosition(it.endOffset)

        val range = Range(
            Position(
                start.line.toUInt(),
                start.column.toUInt()
            ),
            Position(
                end.line.toUInt(),
                end.column.toUInt()
            )
        )

        val severity = when (it.severity) {
            HighlightSeverity.ERROR -> DiagnosticSeverity.Error
            HighlightSeverity.WARNING -> DiagnosticSeverity.Warning
            HighlightSeverity.WEAK_WARNING -> DiagnosticSeverity.Warning
            HighlightSeverity.INFORMATION -> DiagnosticSeverity.Information
            else -> DiagnosticSeverity.Hint
        }

        Diagnostic(range, it.text, severity)
    }

    val message = InputMessage.DiagnosticsChange(
        listOf(
            BufferDiagnostics(bufferId, diagnostics)
        )
    )

    val service = project.service<KiEditor>()
    service.scope.launch {
        service.sendRequest(message)
    }
}
