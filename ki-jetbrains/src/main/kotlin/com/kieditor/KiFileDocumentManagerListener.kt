package com.kieditor

import com.intellij.openapi.components.service
import com.intellij.openapi.editor.Document
import com.intellij.openapi.fileEditor.FileDocumentManagerListener
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.kieditor.protocol.BufferOpenParams
import com.kieditor.protocol.InputMessage
import kotlinx.coroutines.launch

class KiFileDocumentManagerListener(private val project: Project) : FileDocumentManagerListener {
    override fun fileContentLoaded(file: VirtualFile, document: Document) {
        val uri = extractKiUri(file)
            ?: return

        val content = document.text

        // todo test on large files

        val message = InputMessage.BufferOpen(BufferOpenParams(uri, listOf(), content))

        val service = project.service<KiEditor>()
        service.scope.launch {
            service.sendRequest(message)
        }
    }
}
