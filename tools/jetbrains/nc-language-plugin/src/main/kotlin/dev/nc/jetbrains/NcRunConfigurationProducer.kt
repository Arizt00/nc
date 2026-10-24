package dev.nc.jetbrains

import com.intellij.execution.actions.ConfigurationContext
import com.intellij.execution.actions.LazyRunConfigurationProducer
import com.intellij.openapi.util.Ref
import com.intellij.psi.PsiElement

class NcRunConfigurationProducer : LazyRunConfigurationProducer<NcRunConfiguration>() {
    override fun getConfigurationFactory() = ncRunFactory()

    override fun setupConfigurationFromContext(
        configuration: NcRunConfiguration,
        context: ConfigurationContext,
        sourceElement: Ref<PsiElement>
    ): Boolean {
        val file = context.location?.virtualFile ?: return false
        if (!NcRunSupport.isRunnableFile(file)) return false
        configuration.name = NcRunSupport.stableName(file)
        configuration.filePath = file.path
        return true
    }

    override fun isConfigurationFromContext(
        configuration: NcRunConfiguration,
        context: ConfigurationContext
    ): Boolean {
        val file = context.location?.virtualFile ?: return false
        return NcRunSupport.isRunnableFile(file) && configuration.filePath == file.path
    }
}
