package dev.nc.jetbrains

import com.intellij.codeInsight.navigation.actions.GotoDeclarationHandler
import com.intellij.openapi.components.service
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.project.DumbAware
import com.intellij.psi.PsiElement

/**
 * Adaptador ligero entre la extensión de navegación de IntelliJ y el servicio
 * semántico del plugin. No guarda estado ni recorre el disco.
 */
class NcGotoDeclarationHandler : GotoDeclarationHandler, DumbAware {
    override fun getGotoDeclarationTargets(
        sourceElement: PsiElement?,
        offset: Int,
        editor: Editor?
    ): Array<PsiElement>? {
        val source = sourceElement ?: return null
        val file = source.containingFile ?: return null
        if (file.language != NcLanguage) return null

        val text = editor?.document?.text ?: file.text
        if (text.isEmpty()) return null

        return file.project.service<NcNavigationService>().resolve(
            file = file,
            text = text,
            offset = offset,
            sourceText = source.text
        )
    }
}
