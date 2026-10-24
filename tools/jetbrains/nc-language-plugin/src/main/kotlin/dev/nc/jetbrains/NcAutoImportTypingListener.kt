package dev.nc.jetbrains

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.Disposable
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.editor.Document
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.editor.event.DocumentEvent
import com.intellij.openapi.editor.event.DocumentListener
import com.intellij.openapi.editor.event.EditorFactoryEvent
import com.intellij.openapi.editor.event.EditorFactoryListener
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.psi.PsiManager
import java.util.Collections
import java.util.WeakHashMap

class NcAutoImportTypingListener : EditorFactoryListener {
    private data class ListenerRegistration(val listener: DocumentListener, val disposable: Disposable)

    private val listeners = Collections.synchronizedMap(WeakHashMap<Editor, ListenerRegistration>())
    private val inFlight = Collections.synchronizedSet(mutableSetOf<Document>())

    override fun editorCreated(event: EditorFactoryEvent) {
        val editor = event.editor
        val listener = object : DocumentListener {
            override fun documentChanged(documentEvent: DocumentEvent) {
                maybeCloseDoubleQuote(editor, documentEvent)
                maybeExpandCompactEmptyBlock(editor, documentEvent)
                maybeInsertImport(editor, documentEvent)
            }
        }
        val disposable = Disposer.newDisposable("nC auto import listener")
        listeners[editor] = ListenerRegistration(listener, disposable)
        editor.document.addDocumentListener(listener, disposable)
    }

    override fun editorReleased(event: EditorFactoryEvent) {
        val editor = event.editor
        val registration = listeners.remove(editor) ?: return
        Disposer.dispose(registration.disposable)
    }

    private fun maybeInsertImport(editor: Editor, event: DocumentEvent) {
        val project = editor.project ?: return
        val document = editor.document
        if (inFlight.contains(document) || !NcIntellisenseCatalog.isTokenBoundary(event.newFragment.toString())) return

        ApplicationManager.getApplication().invokeLater {
            val file = FileDocumentManager.getInstance().getFile(document) ?: return@invokeLater
            val psiFile = PsiManager.getInstance(project).findFile(file) ?: return@invokeLater
            if (!NcIntellisenseCatalog.isSourceFile(psiFile)) return@invokeLater
            if (lineLooksLikeImport(document, document.getLineNumber(event.offset))) return@invokeLater

            val row = NcIntellisenseCatalog.autoImportCandidateAtOffset(
                document.text,
                event.offset + event.newLength
            ) ?: return@invokeLater

            inFlight.add(document)
            try {
                WriteCommandAction.runWriteCommandAction(project, "nC auto import", null, Runnable {
                    NcIntellisenseCatalog.insertImportIfMissing(document, row)
                }, psiFile)
            } finally {
                inFlight.remove(document)
            }
        }
    }

    private fun maybeCloseDoubleQuote(editor: Editor, event: DocumentEvent) {
        val project = editor.project ?: return
        val document = editor.document
        if (inFlight.contains(document) || event.newFragment.toString() != "\"") return

        ApplicationManager.getApplication().invokeLater {
            val file = FileDocumentManager.getInstance().getFile(document) ?: return@invokeLater
            val psiFile = PsiManager.getInstance(project).findFile(file) ?: return@invokeLater
            if (!NcIntellisenseCatalog.isNcFile(psiFile)) return@invokeLater
            val offset = event.offset.coerceIn(0, document.textLength)
            val text = document.text
            if (offset >= text.length || text[offset] != '"') return@invokeLater
            if (isEscaped(text, offset)) return@invokeLater

            inFlight.add(document)
            try {
                WriteCommandAction.runWriteCommandAction(project, "nC close quote", null, Runnable {
                    val freshText = document.text
                    if (offset + 1 < freshText.length && freshText[offset + 1] == '"' && isInsideStringBefore(freshText, offset)) {
                        document.deleteString(offset, offset + 1)
                        editor.caretModel.moveToOffset((offset + 1).coerceAtMost(document.textLength))
                    } else if (shouldAutoCloseQuote(freshText, offset)) {
                        document.insertString(offset + 1, "\"")
                        editor.caretModel.moveToOffset(offset + 1)
                    }
                }, psiFile)
            } finally {
                inFlight.remove(document)
            }
        }
    }

    private fun maybeExpandCompactEmptyBlock(editor: Editor, event: DocumentEvent) {
        val project = editor.project ?: return
        val document = editor.document
        if (inFlight.contains(document) || event.newFragment.toString() != "}") return

        ApplicationManager.getApplication().invokeLater {
            val file = FileDocumentManager.getInstance().getFile(document) ?: return@invokeLater
            val psiFile = PsiManager.getInstance(project).findFile(file) ?: return@invokeLater
            if (!NcIntellisenseCatalog.isNcFile(psiFile)) return@invokeLater
            val line = document.getLineNumber(event.offset.coerceIn(0, document.textLength))
            if (line < 0 || line >= document.lineCount) return@invokeLater
            val start = document.getLineStartOffset(line)
            val end = document.getLineEndOffset(line)
            val lineText = document.text.substring(start, end)
            val expanded = expandCompactEmptyBlockLine(lineText, indentUnit(document.text)) ?: return@invokeLater

            inFlight.add(document)
            try {
                WriteCommandAction.runWriteCommandAction(project, "nC expand compact block", null, Runnable {
                    document.replaceString(start, end, expanded)
                    editor.caretModel.moveToOffset(caretOffsetForExpandedBlock(start, expanded))
                }, psiFile)
            } finally {
                inFlight.remove(document)
            }
        }
    }

    private fun expandCompactEmptyBlockLine(line: String, indentUnit: String): String? {
        val match = Regex("""^(\s*)((?:[A-Za-z_][A-Za-z0-9_]*)(?:\[[^\]]+])?|to\s+main|raw|bridge|match\s*\([^)]*\)|if\s*\([^)]*\)|while\s*\([^)]*\)|for\s*\([^)]*\))\s*\{\s*}\s*$""")
            .matchEntire(line) ?: return null
        val indent = match.groupValues[1]
        val header = match.groupValues[2].trim()
        return "$indent$header {\n$indent$indentUnit\n$indent}"
    }

    private fun indentUnit(text: String): String {
        for (line in text.lines()) {
            val indent = line.takeWhile { it == ' ' || it == '\t' }
            if (indent.contains('\t')) return "\t"
            if (indent.length in 2..8 && line.trim().isNotBlank()) {
                return indent.takeLast(4).ifBlank { "    " }
            }
        }
        return "    "
    }

    private fun caretOffsetForExpandedBlock(startOffset: Int, expanded: String): Int {
        val firstNewline = expanded.indexOf('\n')
        if (firstNewline < 0) return startOffset + expanded.length
        val secondNewline = expanded.indexOf('\n', firstNewline + 1)
        return startOffset + if (secondNewline < 0) expanded.length else secondNewline
    }

    private fun lineLooksLikeImport(document: Document, line: Int): Boolean {
        if (line < 0 || line >= document.lineCount) return false
        val start = document.getLineStartOffset(line)
        val end = document.getLineEndOffset(line)
        val text = document.text.substring(start, end).trim()
        return text.startsWith("use ") || text.startsWith("use::") || text.startsWith("import ")
    }

    private fun shouldAutoCloseQuote(text: String, offset: Int): Boolean {
        if (offset !in text.indices) return false
        if (isInLineComment(text, offset) || isInBlockComment(text, offset)) return false
        if (isInsideStringBefore(text, offset)) return false
        val next = text.getOrNull(offset + 1)
        return next == null || next.isWhitespace() || next in charArrayOf(')', ']', '}', ';', ',', ':')
    }

    private fun isInsideStringBefore(text: String, offset: Int): Boolean {
        var quoteOpen = false
        val lineStart = text.lastIndexOf('\n', (offset - 1).coerceAtLeast(0)).let { if (it < 0) 0 else it + 1 }
        var i = lineStart
        while (i < offset && i < text.length) {
            if (text[i] == '"' && !isEscaped(text, i)) quoteOpen = !quoteOpen
            i += 1
        }
        return quoteOpen
    }

    private fun isEscaped(text: String, offset: Int): Boolean {
        var slashCount = 0
        var i = offset - 1
        while (i >= 0 && text[i] == '\\') {
            slashCount += 1
            i -= 1
        }
        return slashCount % 2 == 1
    }

    private fun isInLineComment(text: String, offset: Int): Boolean {
        val lineStart = text.lastIndexOf('\n', (offset - 1).coerceAtLeast(0)).let { if (it < 0) 0 else it + 1 }
        val before = text.substring(lineStart, offset.coerceIn(lineStart, text.length))
        val comment = before.indexOf("//")
        if (comment < 0) return false
        val quote = before.indexOf('"')
        return quote < 0 || comment < quote
    }

    private fun isInBlockComment(text: String, offset: Int): Boolean {
        val safe = offset.coerceIn(0, text.length)
        val open = text.lastIndexOf("/*", safe)
        if (open < 0) return false
        val close = text.lastIndexOf("*/", safe)
        return close < open
    }
}
