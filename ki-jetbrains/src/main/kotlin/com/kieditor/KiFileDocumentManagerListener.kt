package com.kieditor

import com.intellij.openapi.components.service
import com.intellij.openapi.diagnostic.thisLogger
import com.intellij.openapi.editor.Document
import com.intellij.openapi.fileEditor.FileDocumentManagerListener
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.kieditor.protocol.BufferOpenParams
import com.kieditor.protocol.InputMessage
import kotlinx.coroutines.runBlocking

class KiFileDocumentManagerListener(private val project: Project): FileDocumentManagerListener {
    override fun fileContentLoaded(file: VirtualFile, document: Document) {
        thisLogger().info("fileContentLoaded: ${file.canonicalPath}")

        val uri = extractKiUri(file)
            ?: return

        val content = document.text

        // todo test on large files

        val message = InputMessage.BufferOpen(BufferOpenParams(uri, listOf(), content))

        // todo is this block ok? move into the service
        runBlocking {
            project.service<KiEditor>().sendRequest(message)
        }
    }
}
