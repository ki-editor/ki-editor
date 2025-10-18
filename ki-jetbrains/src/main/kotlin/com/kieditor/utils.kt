package com.kieditor

import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.vfs.VirtualFileManager
import kotlin.io.path.Path

fun extractKiUri(file: VirtualFile): String? {
    if (!(file.isInLocalFileSystem && file.isValid)) { // todo in-memory buffers
        return null
    }

    return file.url
}

fun bufferIdToUri(bufferId: String): String? {
    val file = VirtualFileManager.getInstance().findFileByNioPath(Path(bufferId))
        ?: return null

    val uri = extractKiUri(file)

    return uri
}

fun uriToBufferId(uri: String): String? {
    val file = VirtualFileManager.getInstance().findFileByUrl(uri)
        ?: return null

    return file.path
}
