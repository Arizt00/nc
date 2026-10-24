package dev.nc.jetbrains

import com.intellij.execution.configurations.RunProfile
import com.intellij.execution.configurations.RunnerSettings
import com.intellij.execution.executors.DefaultDebugExecutor
import com.intellij.execution.runners.GenericProgramRunner

class NcDebugProgramRunner : GenericProgramRunner<RunnerSettings>() {
    override fun getRunnerId(): String = "NcDebugProgramRunner"

    override fun canRun(executorId: String, profile: RunProfile): Boolean {
        return executorId == DefaultDebugExecutor.EXECUTOR_ID && profile is NcRunConfiguration
    }
}
