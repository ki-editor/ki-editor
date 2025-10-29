package com.kieditor

import com.intellij.openapi.project.Project
import com.intellij.openapi.util.NlsContexts
import com.intellij.openapi.wm.StatusBarWidget
import com.intellij.openapi.wm.impl.status.EditorBasedWidget
import com.intellij.openapi.wm.impl.status.widget.StatusBarEditorBasedWidgetFactory
import com.kieditor.protocol.EditorMode
import com.kieditor.protocol.SelectionMode
import org.jetbrains.annotations.NonNls
import java.awt.Component.CENTER_ALIGNMENT

class KiModeStatusBarFactory : StatusBarEditorBasedWidgetFactory() {
    companion object {
        const val ID: String = "KiEditor"
    }

    override fun getId(): String {
        return ID
    }

    override fun getDisplayName(): String {
        return "Ki Editor Widget"
    }

    override fun createWidget(project: Project): StatusBarWidget {
        return KiModeWidget(project)
    }
}

class KiModeWidget(project: Project) : EditorBasedWidget(project), StatusBarWidget.TextPresentation {

    override fun getPresentation(): StatusBarWidget.WidgetPresentation {
        return this
    }

    override fun ID(): @NonNls String {
        return KiModeStatusBarFactory.ID
    }

    override fun getText(): @NlsContexts.Label String {
        val editor = getEditor()
            ?: return "" // todo this is not being updated on initial load

        val mode = modeAsString(editor.kiEditorMode)
        val selectionMode = selectionModeAsString(editor.kiEditorSelectionMode)

        return "Ki: $mode - $selectionMode"
    }

    override fun getAlignment(): Float = CENTER_ALIGNMENT

    override fun getTooltipText(): @NlsContexts.Tooltip String? = null
}

private fun modeAsString(mode: EditorMode): String {
    return when (mode) {
        EditorMode.Normal -> "NORMAL"
        EditorMode.Insert -> "INSERT"
        EditorMode.MultiCursor -> "MULTI"
        EditorMode.FindOneChar -> "FIND"
        EditorMode.Swap -> "SWAP"
        EditorMode.Replace -> "REPLACE"
    }
}

private fun selectionModeAsString(mode: SelectionMode): String {
    return when (mode) {
        SelectionMode.Character -> "Character"
        SelectionMode.Custom -> "Custom" // todo what is this?
        is SelectionMode.Diagnostic -> "Diagnostic (${mode.params.string})"
        is SelectionMode.Find -> "Find" // todo the param can be too long, how to display
        SelectionMode.GitHunk -> "GitHunk"
        SelectionMode.Line -> "Line"
        SelectionMode.LineFull -> "LineFull"
        SelectionMode.LocalQuickfix -> "LocalQuickfix"
        SelectionMode.Mark -> "Mark"
        SelectionMode.Subword -> "Subword"
        SelectionMode.SyntaxNode -> "SyntaxNode"
        SelectionMode.SyntaxNodeFine -> "SyntaxNodeFine"
        SelectionMode.Word -> "Word"
    }
}
