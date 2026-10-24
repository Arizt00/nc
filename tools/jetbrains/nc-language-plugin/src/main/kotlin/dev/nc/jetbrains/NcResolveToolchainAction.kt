package dev.nc.jetbrains

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.ui.Messages

class NcResolveToolchainAction : AnAction() {
    override fun actionPerformed(event: AnActionEvent) {
        try {
            val result = NcToolchainResolver.resolveNccBinary()
            Messages.showInfoMessage(
                "toolchain=${result.toolchain_id}\npath=${result.path}\nsource=${result.source}",
                "nC Toolchain"
            )
        } catch (err: Exception) {
            Messages.showErrorDialog("${err.message}", "nC Toolchain")
        }
    }
}
