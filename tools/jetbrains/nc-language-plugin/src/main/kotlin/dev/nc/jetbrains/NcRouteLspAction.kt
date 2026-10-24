package dev.nc.jetbrains

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.ui.Messages

class NcRouteLspAction : AnAction() {
    override fun actionPerformed(event: AnActionEvent) {
        val vf = event.getData(com.intellij.openapi.actionSystem.CommonDataKeys.VIRTUAL_FILE)
        if (vf == null) {
            Messages.showInfoMessage("No file selected", "nC LSP Route")
            return
        }
        val text = try {
            String(vf.contentsToByteArray())
        } catch (_: Exception) {
            ""
        }
        val backend = NcLspModeRouter.chooseBackend(vf.path, text)
        Messages.showInfoMessage("backend=$backend\nfile=${vf.path}", "nC LSP Route")
    }
}
