package com.kieditor

import com.intellij.openapi.components.service
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.event.EditorFactoryEvent
import com.intellij.openapi.editor.event.EditorFactoryListener
import com.intellij.openapi.editor.ex.EditorEx
import com.intellij.openapi.editor.ex.FocusChangeListener
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.wm.WindowManager
import com.kieditor.protocol.BufferParams
import com.kieditor.protocol.InputMessage
import com.kieditor.protocol.LineRange
import com.kieditor.protocol.ViewportParams
import kotlinx.coroutines.launch

class KiEditorFactoryListener : EditorFactoryListener {
    override fun editorCreated(event: EditorFactoryEvent) {
        val editor = event.editor as? EditorEx
            ?: return

        val file = FileDocumentManager.getInstance().getFile(editor.document)
            ?: return

        editor.kiEditorUri = extractKiUri(file)

        editor.scrollingModel.addVisibleAreaListener { event ->
            val project = editor.project
                ?: return@addVisibleAreaListener

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

            val service = project.service<KiEditor>()
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

    }
}
