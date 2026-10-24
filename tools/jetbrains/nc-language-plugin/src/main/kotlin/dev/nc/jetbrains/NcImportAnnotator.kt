package dev.nc.jetbrains

import com.intellij.codeInsight.intention.IntentionAction
import com.intellij.lang.annotation.AnnotationHolder
import com.intellij.lang.annotation.Annotator
import com.intellij.lang.annotation.HighlightSeverity
import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.project.Project
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile

class NcImportAnnotator : Annotator {
    override fun annotate(element: PsiElement, holder: AnnotationHolder) {
        val file = element.containingFile ?: return
        if (file.language != NcLanguage || !NcIntellisenseCatalog.isSourceFile(file)) return

        val token = NcIntellisenseCatalog.cleanToken(element.text)
        if (token.length < 2) return
        val row = NcIntellisenseCatalog.lookup(token) ?: return
        if (NcIntellisenseCatalog.preferredImport(row).isBlank()) return
        if (NcIntellisenseCatalog.hasImport(file.text, row)) return

        holder.newAnnotation(
            HighlightSeverity.WEAK_WARNING,
            "nC import candidate: ${NcIntellisenseCatalog.preferredImport(row)}"
        )
            .range(element.textRange)
            .withFix(NcAddImportQuickFix(row.label))
            .create()
    }
}

class NcAddImportQuickFix(private val label: String) : IntentionAction {
    override fun getText(): String {
        val row = NcIntellisenseCatalog.lookup(label)
        val statement = if (row == null) "" else NcIntellisenseCatalog.preferredImport(row)
        return if (statement.isBlank()) "Add nC import" else "Add import: $statement"
    }

    override fun getFamilyName(): String = "nC imports"

    override fun isAvailable(project: Project, editor: Editor?, file: PsiFile?): Boolean {
        val row = NcIntellisenseCatalog.lookup(label) ?: return false
        return NcIntellisenseCatalog.isSourceFile(file) &&
            NcIntellisenseCatalog.preferredImport(row).isNotBlank() &&
            !NcIntellisenseCatalog.hasImport(file?.text ?: "", row)
    }

    override fun invoke(project: Project, editor: Editor?, file: PsiFile?) {
        val document = editor?.document ?: return
        val row = NcIntellisenseCatalog.lookup(label) ?: return
        WriteCommandAction.runWriteCommandAction(project) {
            NcIntellisenseCatalog.insertImportIfMissing(document, row)
        }
    }

    override fun startInWriteAction(): Boolean = false
}
