package com.kieditor.actions

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.components.service
import com.kieditor.KiEditor
import com.kieditor.protocol.InputMessage
import kotlinx.coroutines.launch

class KiEditorPingAction : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project
            ?: throw RuntimeException("Unable to send Ping request to Ki Editor when no project is open")

        val message = InputMessage.Ping("intellij")

        val service = project.service<KiEditor>()
        service.scope.launch {
            val response = service.sendRequest(message)

            NotificationGroupManager.getInstance()
                .getNotificationGroup("Custom Notification Group")
                .createNotification(response.params, NotificationType.INFORMATION)
                .notify(project)
        }
    }
}
