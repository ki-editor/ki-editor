package com.kieditor

import com.intellij.openapi.application.edtWriteAction
import com.intellij.openapi.application.readAction
import com.intellij.openapi.command.writeCommandAction
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.diagnostic.thisLogger
import com.intellij.openapi.editor.CaretState
import com.intellij.openapi.editor.CaretVisualAttributes
import com.intellij.openapi.editor.CaretVisualAttributes.Shape
import com.intellij.openapi.editor.CaretVisualAttributes.Weight
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.LogicalPosition
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Key
import com.intellij.openapi.vfs.VirtualFileManager
import com.intellij.openapi.wm.WindowManager
import com.kieditor.protocol.*

@Service(Service.Level.PROJECT)
class KiStateManager(val project: Project) {

    suspend fun handleBufferDiff(message: OutputMessage.BufferDiff) {
        // todo why buffer id is not a uri?
        val bufferId = message.params.buffer_id

        val uri = bufferIdToUri(bufferId)
            ?: return

        val editor = EditorFactory.getInstance().editorList
            .find { it.kiEditorUri == uri }
            ?: return

        if (editor.kiEditorMode == EditorMode.Insert) {
            // Don't update IDE if Ki is in insert mode
            // Because in insert mode, the modifications is relayed to IDE
            // the buffer sync direction is reversed, where Ki is listening for changes
            // from IDE
            return
        }

        @Suppress("UnstableApiUsage")
        writeCommandAction(project, "Apply Ki Buffer Diff") {
            for (edit in message.params.edits) {
                // ideally api would have worked with offsets directly without requiring editor
                // but VSCode doesn't have offset based edits
                val startOffset = editor.logicalPositionToOffset(
                    LogicalPosition(
                        edit.range.start.line.toInt(),
                        edit.range.start.character.toInt()
                    )
                )
                val endOffset = editor.logicalPositionToOffset(
                    LogicalPosition(
                        edit.range.end.line.toInt(),
                        edit.range.end.character.toInt()
                    )
                )

                editor.document.replaceString(startOffset, endOffset, edit.new_text)
            }
        }
    }


    suspend fun handleBufferSave(message: OutputMessage.BufferSave) {
        val file = VirtualFileManager.getInstance().findFileByUrl(message.params.uri)
            ?: return

        val doc = readAction {
            FileDocumentManager.getInstance().getDocument(file)
        }

        val document = doc
            ?: return

        edtWriteAction {
            FileDocumentManager.getInstance().saveDocument(document)
        }
    }

    suspend fun handleSyncBufferRequest(message: OutputMessage.SyncBufferRequest) {
        val file = VirtualFileManager.getInstance().findFileByUrl(message.params.uri)
            ?: return

        val document = readAction {
            FileDocumentManager.getInstance().getDocument(file)
        }

        val content = document?.text
            ?: return

        val message = InputMessage.SyncBufferResponse(
            SyncBufferResponseParams(
                message.params.uri,
                content
            )
        )

        project.service<KiEditor>().sendNotification(message)
    }

    suspend fun handleModeChange(message: OutputMessage.ModeChange) {
        val bufferId = message.params.buffer_id
            ?: return // todo when is this null

        val uri = bufferIdToUri(bufferId)
            ?: return

        val editor = EditorFactory.getInstance().editorList
            .find { it.kiEditorUri == uri }
            ?: return

        val mode = message.params.mode

        editor.kiEditorMode = mode

        edtWriteAction {
            for (caret in editor.caretModel.allCarets) {

                val shape = when (mode) {
                    EditorMode.Normal -> Shape.BLOCK
                    EditorMode.Insert -> Shape.BAR
                    EditorMode.MultiCursor -> Shape.BLOCK
                    EditorMode.FindOneChar -> Shape.BLOCK
                    EditorMode.Swap -> Shape.BLOCK
                    EditorMode.Replace -> Shape.UNDERSCORE
                }

                caret.visualAttributes = CaretVisualAttributes(null, Weight.NORMAL, shape, 1.0f)
            }
        }

        WindowManager.getInstance().getStatusBar(project).updateWidget(KiModeStatusBarFactory.ID)
    }

    fun handleSelectionModeChange(message: OutputMessage.SelectionModeChange) {
        val bufferId = message.params.buffer_id
            ?: return // todo when is this null

        val uri = bufferIdToUri(bufferId)
            ?: return

        val editor = EditorFactory.getInstance().editorList
            .find { it.kiEditorUri == uri }
            ?: return

        editor.kiEditorSelectionMode = message.params.mode

        WindowManager.getInstance().getStatusBar(project).updateWidget(KiModeStatusBarFactory.ID)
    }

    fun handleKeyboardLayoutChange(message: OutputMessage.KeyboardLayoutChanged) {
        thisLogger().info("Ki: Keyboard layout changed: ${message.params}")
    }

    fun handleJumpsChanged(message: OutputMessage.JumpsChanged) {

    }

    fun handleMarksChanged(message: OutputMessage.MarksChanged) {

    }

    suspend fun handleSelectionUpdate(message: OutputMessage.SelectionUpdate) {
        val uri = message.params.uri
            ?: return // todo when is this null

        val editor = EditorFactory.getInstance().editorList
            .find { it.kiEditorUri == uri }
            ?: return

        val caretModel = editor.caretModel

        val caretStates = message.params.selections.map {
            val caretPosition = LogicalPosition(it.active.line.toInt(), it.active.character.toInt())
            val oppositeEnd = LogicalPosition(it.anchor.line.toInt(), it.anchor.character.toInt())

            CaretState(caretPosition, caretPosition, oppositeEnd)
        }

        edtWriteAction {
            caretModel.caretsAndSelections = caretStates
        }
    }
}

var Editor.kiEditorMode: EditorMode
    get() = getUserData(editorModeKey) ?: EditorMode.Normal
    set(value) = putUserData(editorModeKey, value)

var Editor.kiEditorSelectionMode: SelectionMode
    get() = getUserData(editorSelectionModeKey) ?: SelectionMode.Line
    set(value) = putUserData(editorSelectionModeKey, value)

var Editor.kiEditorUri: String?
    get() = getUserData(editorUriKey)
    set(value) = putUserData(editorUriKey, value)


private val editorModeKey = Key.create<EditorMode>("KI_EDITOR_MODE")
private val editorSelectionModeKey = Key.create<SelectionMode>("KI_EDITOR_SELECTION_MODE")
private val editorUriKey = Key.create<String>("KI_EDITOR_URI")

