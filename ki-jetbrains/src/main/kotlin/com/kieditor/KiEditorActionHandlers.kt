package com.kieditor

import com.intellij.openapi.actionSystem.DataContext
import com.intellij.openapi.components.service
import com.intellij.openapi.diagnostic.thisLogger
import com.intellij.openapi.editor.Caret
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.actionSystem.EditorActionHandler
import com.kieditor.protocol.InputMessage
import com.kieditor.protocol.KeyboardParams
import kotlinx.coroutines.launch

// todo do we need to add warning if shortcuts for these handlers were changed?
class KiEscEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "Escape")
class KiEnterEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "Enter")
class KiUpEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "ArrowUp")
class KiDownEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "ArrowDown")
class KiLeftEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "ArrowLeft")
class KiRightEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "ArrowRight")
class KiBackspaceEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "Backspace")
class KiDeleteEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "Delete")
class KiTabEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "Tab")
class KiHomeEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "Home")
class KiEndEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "End")
class KiPageUpEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "PageDown")
class KiPageDownEditorActionHandler(nextHandler: EditorActionHandler?) : KiEditorActionHandler(nextHandler, "PageDown")

// todo this breaks text field in find all dialog
// todo this seems to also break escape when doing inline rename

open class KiEditorActionHandler(private val nextHandler: EditorActionHandler?, val key: String): EditorActionHandler() {
    override fun doExecute(editor: Editor, caret: Caret?, dataContext: DataContext?) {
        val project = editor.project

        if (project == null) {
            nextHandler?.execute(editor, caret, dataContext)
            return
        }

        thisLogger().debug("Ki Ide: $key pressed")

//        if (project.service<KiStateManager>().currentMode == EditorMode.Insert) {
//            nextHandler.execute(editor, charTyped, dataContext)
//            return
//        }

        val uri = editor.kiEditorUri
            ?: return

        val message = InputMessage.KeyboardInput(
            KeyboardParams(
                key,
                uri,
                0u // todo hash
            )
        )

        val service = project.service<KiEditor>()
        service.scope.launch {
            service.sendRequest(message)
        }
    }
}
