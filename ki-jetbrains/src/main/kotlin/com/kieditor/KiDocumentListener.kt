package com.kieditor

import com.intellij.openapi.components.service
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.event.DocumentEvent
import com.intellij.openapi.editor.event.DocumentListener
import com.kieditor.protocol.BufferDiffParams
import com.kieditor.protocol.DiffEdit
import com.kieditor.protocol.InputMessage
import com.kieditor.protocol.Position
import com.kieditor.protocol.Range
import kotlinx.coroutines.launch

class KiDocumentListener() : DocumentListener {
    override fun documentChanged(event: DocumentEvent) {
        val editor = EditorFactory.getInstance().editorList
            .find { it.document == event.document }
            ?: return

        val project = editor.project
            ?: return

        val uri = editor.kiEditorUri
            ?: return

        val bufferId = uriToBufferId(uri)
            ?: return

        val start = editor.offsetToLogicalPosition(event.offset)
        val end = editor.offsetToLogicalPosition(event.offset + event.oldLength)
        val newText = event.newFragment.toString()

        val message = InputMessage.BufferChange(
            BufferDiffParams(
                bufferId,
                listOf(
                    DiffEdit(
                        Range(
                            Position(start.line.toUInt(), start.column.toUInt()),
                            Position(end.line.toUInt(), end.column.toUInt())
                        ),
                        newText
                    )
                )
            )
        )

        val service = project.service<KiEditor>()
        service.scope.launch {
            service.sendNotification(message)
        }
    }
}
