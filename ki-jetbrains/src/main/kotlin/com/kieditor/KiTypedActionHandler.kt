package com.kieditor

import com.intellij.openapi.actionSystem.DataContext
import com.intellij.openapi.components.service
import com.intellij.openapi.diagnostic.thisLogger
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.actionSystem.ActionPlan
import com.intellij.openapi.editor.actionSystem.TypedActionHandler
import com.intellij.openapi.editor.actionSystem.TypedActionHandlerEx
import com.kieditor.protocol.EditorMode
import com.kieditor.protocol.InputMessage
import com.kieditor.protocol.KeyboardParams
import kotlinx.coroutines.launch
import java.util.zip.CRC32

class KiTypedActionHandler(val originalHandler: TypedActionHandler) : TypedActionHandlerEx {

    override fun beforeExecute(editor: Editor, c: Char, context: DataContext, plan: ActionPlan) {
        if (editor.kiEditorMode == EditorMode.Insert) {
            if (originalHandler is TypedActionHandlerEx) {
                originalHandler.beforeExecute(editor, c, context, plan)
            }
        }
    }

    override fun execute(editor: Editor, charTyped: Char, dataContext: DataContext) {
        thisLogger().debug("testing: $charTyped")

        val project = editor.project // TODO what are the cases this is null? ki should also work in those cases
            ?: return

        if (editor.kiEditorMode == EditorMode.Insert) {
            originalHandler.execute(editor, charTyped, dataContext)
            return
        }

        val uri = editor.kiEditorUri
            ?: return

        val crc32 = CRC32()
        crc32.update(editor.document.text.toByteArray())
        val checksum = crc32.value

        val message = InputMessage.KeyboardInput(
            KeyboardParams(
                charTyped.toString(),
                uri,
                checksum.toUInt() // todo is this safe
            )
        )

        val service = project.service<KiEditor>()
        service.scope.launch {
            service.sendRequest(message)
        }
    }
}
