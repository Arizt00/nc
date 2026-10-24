package dev.nc.jetbrains

import com.intellij.execution.ProgramRunnerUtil
import com.intellij.execution.RunManager
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.ConfigurationTypeUtil
import com.intellij.execution.executors.DefaultDebugExecutor
import com.intellij.execution.executors.DefaultRunExecutor
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.ui.Messages

class NcRunActiveFileAction : AnAction() {
    override fun actionPerformed(event: AnActionEvent) {
        launch(event, debug = false)
    }

    override fun update(event: AnActionEvent) {
        event.presentation.isEnabledAndVisible = runnable(event)
    }
}

class NcDebugActiveFileAction : AnAction() {
    override fun actionPerformed(event: AnActionEvent) {
        launch(event, debug = true)
    }

    override fun update(event: AnActionEvent) {
        event.presentation.isEnabledAndVisible = runnable(event)
    }
}

private fun runnable(event: AnActionEvent): Boolean {
    return event.project != null && NcRunSupport.isRunnableFile(event.getData(CommonDataKeys.VIRTUAL_FILE))
}

private fun launch(event: AnActionEvent, debug: Boolean) {
    val project = event.project
    val file = event.getData(CommonDataKeys.VIRTUAL_FILE)
    if (project == null || !NcRunSupport.isRunnableFile(file)) {
        Messages.showInfoMessage("Select a runnable nC file: .n, .nb, .nbb or .ncp. Start .nm from a terminal.", "nC")
        return
    }

    val factory = ncRunFactory()
    val name = if (debug) "nC Debug ${file!!.name}" else NcRunSupport.stableName(file!!)
    val settings = RunManager.getInstance(project).createConfiguration(name, factory)
    val configuration = settings.configuration as NcRunConfiguration
    configuration.filePath = file.path
    configuration.debugRun = debug
    if (debug) {
        ProgramRunnerUtil.executeConfiguration(settings, DefaultDebugExecutor.getDebugExecutorInstance())
    } else {
        ProgramRunnerUtil.executeConfiguration(settings, DefaultRunExecutor.getRunExecutorInstance())
    }
}

fun ncRunFactory(): ConfigurationFactory {
    return ConfigurationTypeUtil.findConfigurationType(NcRunConfigurationType::class.java)
        .configurationFactories
        .first()
}
