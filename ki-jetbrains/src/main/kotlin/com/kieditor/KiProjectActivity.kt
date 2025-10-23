package com.kieditor

import com.intellij.openapi.components.serviceAsync
import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.ProjectActivity

class KiProjectActivity(): ProjectActivity {
    override suspend fun execute(project: Project) {
        project.serviceAsync<KiEditor>()
    }
}
