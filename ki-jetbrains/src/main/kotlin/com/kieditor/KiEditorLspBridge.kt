package com.kieditor

import com.intellij.find.actions.ShowUsagesAction
import com.intellij.openapi.actionSystem.ActionPlaces
import com.intellij.openapi.actionSystem.IdeActions
import com.intellij.openapi.actionSystem.ex.ActionManagerEx
import com.intellij.openapi.application.invokeLater
import com.intellij.openapi.components.Service
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.project.Project


@Service(Service.Level.PROJECT)
class KiEditorLspBridge(val project: Project) {

    fun handleLspCodeAction() {
        executeAction(IdeActions.ACTION_SHOW_INTENTION_ACTIONS, project)
    }

    fun handleLspDeclaration() {
        // there is no distinction between declaration and definition in intellij
        //  so use go to super here
        executeAction(IdeActions.ACTION_GOTO_SUPER, project)
    }

    fun handleLspDefinition() {
        // ki needs shift to press go-to declaration, but it is more often used than go-to definition
        //  so put the go-to declaration into lsp definition instead
        executeAction("GotoDeclarationOnly", project)
    }

    fun handleLspDocumentSymbols() {
        executeAction(IdeActions.ACTION_FILE_STRUCTURE_POPUP, project)
    }

    fun handleLspHover() {
        executeAction(IdeActions.ACTION_QUICK_JAVADOC , project)
    }

    fun handleLspImplementation() {
        executeAction(IdeActions.ACTION_GOTO_IMPLEMENTATION, project)
    }

    fun handleLspReferences() {
        executeAction(ShowUsagesAction.ID, project)
    }

    fun handleLspRename() {
        executeAction(IdeActions.ACTION_RENAME, project)
    }

    fun handleLspTypeDefinition() {
        executeAction(IdeActions.ACTION_GOTO_TYPE_DECLARATION , project)
    }
}

private fun executeAction(actionId: String, project: Project) {
    invokeLater {
        val editor = FileEditorManager.getInstance(project).selectedTextEditor
            ?: return@invokeLater

        val action = ActionManagerEx.getInstanceEx().getAction(actionId)

        ActionManagerEx.getInstanceEx().tryToExecute(
            action,
            null,
            editor.contentComponent, // contentComponent because that is what IdeaVim uses
            ActionPlaces.KEYBOARD_SHORTCUT,
            true
        )
    }
}
