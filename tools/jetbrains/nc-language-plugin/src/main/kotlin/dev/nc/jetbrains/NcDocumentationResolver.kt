package dev.nc.jetbrains

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiManager
import java.nio.file.Files
import java.nio.file.Path

internal class NcDocumentationResolver(
    private val project: Project
) {
    fun resolve(contextFile: PsiFile, token: String): PsiElement? {
        val row = NcIntellisenseCatalog.lookup(token) ?: return null
        val rawLocation = row.docs.trim().takeIf(String::isNotBlank) ?: return null
        val projectRoot = project.basePath
            ?.let(Path::of)
            ?.toAbsolutePath()
            ?.normalize()
            ?: return null

        val location = parseLocation(rawLocation)
        val unresolved = Path.of(location.path)
        val target = if (unresolved.isAbsolute) unresolved.normalize() else projectRoot.resolve(unresolved).normalize()

        /* El catálogo no puede escapar del proyecto mediante ../../. */
        if (!target.startsWith(projectRoot)) return null
        if (!Files.isRegularFile(target)) return null

        val localFileSystem = LocalFileSystem.getInstance()
        val virtualFile = localFileSystem.findFileByIoFile(target.toFile())
            ?: localFileSystem.refreshAndFindFileByIoFile(target.toFile())
            ?: return null
        val psiFile = PsiManager.getInstance(contextFile.project).findFile(virtualFile) ?: return null
        val offset = location.line?.let { lineStartOffset(psiFile.text, it) } ?: 0

        return navigationElementAt(psiFile, offset)
    }

    private fun parseLocation(raw: String): DocumentationLocation {
        val hashMatch = Regex("""^(.*)#L(\d+)$""").matchEntire(raw)
        if (hashMatch != null) {
            return DocumentationLocation(
                path = hashMatch.groupValues[1],
                line = hashMatch.groupValues[2].toIntOrNull()
            )
        }

        return DocumentationLocation(path = raw, line = null)
    }

    private fun lineStartOffset(text: String, oneBasedLine: Int): Int {
        if (oneBasedLine <= 1) return 0

        var currentLine = 1
        var index = 0

        while (index < text.length && currentLine < oneBasedLine) {
            if (text[index] == '\n') currentLine += 1
            index += 1
        }

        return index.coerceIn(0, text.length.coerceAtLeast(1) - 1)
    }

    private fun navigationElementAt(file: PsiFile, offset: Int): PsiElement {
        if (file.textLength == 0) return file
        val safeOffset = offset.coerceIn(0, file.textLength - 1)
        return file.findElementAt(safeOffset)?.navigationElement ?: file
    }

    private data class DocumentationLocation(
        val path: String,
        val line: Int?
    )
}
