package dev.nc.jetbrains

import com.intellij.lang.documentation.AbstractDocumentationProvider
import com.intellij.psi.PsiElement

class NcDocumentationProvider : AbstractDocumentationProvider() {
    override fun generateDoc(element: PsiElement?, originalElement: PsiElement?): String? {
        val token = NcIntellisenseCatalog.cleanToken(originalElement?.text ?: element?.text)
        val row = NcIntellisenseCatalog.lookup(token) ?: return null
        return NcIntellisenseCatalog.renderDoc(row)
    }

    override fun getQuickNavigateInfo(element: PsiElement?, originalElement: PsiElement?): String? {
        val token = NcIntellisenseCatalog.cleanToken(originalElement?.text ?: element?.text)
        val row = NcIntellisenseCatalog.lookup(token) ?: return null
        val signature = row.signature.ifBlank { row.label }
        val preferred = NcIntellisenseCatalog.preferredImport(row)
        return if (preferred.isBlank()) signature else "$signature  |  $preferred"
    }
}
