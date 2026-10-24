package dev.nc.jetbrains

import com.intellij.codeInsight.hints.ChangeListener
import com.intellij.codeInsight.hints.FactoryInlayHintsCollector
import com.intellij.codeInsight.hints.ImmediateConfigurable
import com.intellij.codeInsight.hints.InlayHintsCollector
import com.intellij.codeInsight.hints.InlayHintsProvider
import com.intellij.codeInsight.hints.InlayHintsSink
import com.intellij.codeInsight.hints.SettingsKey
import com.intellij.lang.Language
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import javax.swing.JCheckBox
import javax.swing.JComponent
import javax.swing.JPanel

class NcInlayHintsProvider : InlayHintsProvider<NcInlayHintsProvider.Settings> {
    data class Settings(
        var parameterHints: Boolean = true,
        var typeHints: Boolean = true
    )

    override fun getCollectorFor(
        file: PsiFile,
        editor: Editor,
        settings: Settings,
        sink: InlayHintsSink
    ): InlayHintsCollector? {
        if (file.language != NcLanguage) return null
        if (NcLowLevelCompletionCatalog.rows(file.virtualFile?.extension, file.text ?: "", "") != null) {
            return null
        }
        return object : FactoryInlayHintsCollector(editor) {
            private var emitted = false

            override fun collect(element: PsiElement, editor: Editor, sink: InlayHintsSink): Boolean {
                if (emitted || element !is PsiFile) return true
                emitted = true
                val text = element.text ?: return true
                val hints = linkedMapOf<Int, MutableList<String>>()
                if (settings.parameterHints) {
                    for (hint in parameterHints(text)) addHint(hints, hint.offset, hint.label)
                }
                if (settings.typeHints) {
                    for (hint in typeHints(text)) addHint(hints, hint.offset, hint.label)
                }
                for ((offset, labels) in hints) {
                    for (label in labels.distinct()) {
                        sink.addInlineElement(offset, true, factory.smallText(label), false)
                    }
                }
                return true
            }
        }
    }

    override fun createSettings(): Settings = Settings()

    override val name: String = "nC hints"

    override val key: SettingsKey<Settings> = SettingsKey("nc.inlay.hints")

    override val description: String = "Context-aware parameter and type hints for nC source."

    override val previewText: String = """
        use <io:output, render>;

        demo {
            stc:
                x >> int <- 2;

            fn:
                internal:
                    sum(a >> int, b >> int) -> int {
                        return a + b;
                    }

                public:
                    run() -> bool {
                        println(renderInt(sum(x, 4)));
                        return true;
                    }

            ref main ->> run;

            fa:
                return true;
        }
    """.trimIndent()

    override fun createConfigurable(settings: Settings): ImmediateConfigurable {
        return object : ImmediateConfigurable {
            override fun createComponent(listener: ChangeListener): JComponent {
                val panel = JPanel()
                val params = JCheckBox("Parameter hints", settings.parameterHints)
                val types = JCheckBox("Type hints", settings.typeHints)
                params.addActionListener {
                    settings.parameterHints = params.isSelected
                    listener.settingsChanged()
                }
                types.addActionListener {
                    settings.typeHints = types.isSelected
                    listener.settingsChanged()
                }
                panel.add(params)
                panel.add(types)
                return panel
            }

            override val mainCheckboxText: String = "Show nC inlay hints"
        }
    }

    override fun isLanguageSupported(language: Language): Boolean = language == NcLanguage

    private data class Hint(val offset: Int, val label: String)

    private fun parameterHints(text: String): List<Hint> {
        val signatures = signatures(text)
        val out = mutableListOf<Hint>()
        for (call in NcSourceIndex.calls(text)) {
            val simpleName = call.name.substringAfterLast('.')
            val signature = signatures[simpleName] ?: continue
            for ((index, arg) in call.args.withIndex()) {
                if (index >= signature.params.size) break
                val param = signature.params[index]
                if (param.name.isBlank()) continue
                if (isNamedArgument(arg.text)) continue
                if (argumentAlreadyNamesParameter(arg.text, param.name)) continue
                out.add(Hint(arg.offset, "${param.name}:"))
            }
        }
        return out
    }

    private fun typeHints(text: String): List<Hint> {
        val signatures = signatures(text)
        val symbols = NcSourceIndex.typedSymbols(text)
        val out = mutableListOf<Hint>()
        val starts = NcSourceIndex.lineStartOffsets(text)
        for ((lineIndex, line) in text.lines().withIndex()) {
            val lineStart = starts.getOrElse(lineIndex) { 0 }
            val projection = Regex("""@?([A-Za-z_][A-Za-z0-9_]*)\s*>>\s*([^<-;]+)\s*[<-]\s*(.+?);?\s*$""").find(line)
            if (projection != null) {
                val declared = NcSourceIndex.cleanType(projection.groupValues[2])
                val inferred = NcSourceIndex.inferExprType(projection.groupValues[3], symbols, signatures)
                if (inferred.isNotBlank() && inferred != declared) {
                    out.add(Hint(lineStart + line.length, " expr: $inferred"))
                }
                continue
            }

            val compactAssign = Regex("""^\s*@?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+?);?\s*$""").find(line)
            if (compactAssign != null) {
                val name = compactAssign.groupValues[1]
                val known = symbols[name].orEmpty()
                val inferred = NcSourceIndex.inferExprType(compactAssign.groupValues[2], symbols, signatures)
                val typeName = known.ifBlank { inferred }
                if (typeName.isNotBlank()) {
                    out.add(Hint(lineStart + line.length, " : $typeName"))
                }
            }
        }
        return out
    }

    private fun signatures(text: String): Map<String, NcSourceIndex.Signature> {
        val out = linkedMapOf<String, NcSourceIndex.Signature>()
        for (entry in catalogSignatures()) {
            out.putIfAbsent(entry.name, entry)
        }
        for (entry in NcSourceIndex.signatures(text)) {
            out[entry.name] = entry
        }
        return out
    }

    private fun catalogSignatures(): List<NcSourceIndex.Signature> {
        val rows = NcIntellisenseCatalog.current.symbols.filter { it.kind == "function" }
        val out = mutableListOf<NcSourceIndex.Signature>()
        for (row in rows) {
            val signature = row.signature.ifBlank { row.label }
            val match = Regex("""^([A-Za-z_][A-Za-z0-9_.]*)\s*\(([^)]*)\)\s*(?:->\s*([^;]+))?""").find(signature) ?: continue
            val name = match.groupValues[1].substringAfterLast('.')
            val params = NcSourceIndex.splitArgs(match.groupValues[2], 0).mapIndexed { index, arg ->
                val named = Regex("""@?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^,]+)""").find(arg.text)
                val paramName = named?.groupValues?.get(1) ?: "arg${index + 1}"
                val typeName = named?.groupValues?.get(2)?.let { NcSourceIndex.cleanType(it) }.orEmpty()
                NcSourceIndex.Parameter(paramName, typeName, 0)
            }
            out.add(NcSourceIndex.Signature(name, params, NcSourceIndex.cleanType(match.groupValues.getOrElse(3) { "" }), 0))
        }
        return out + manualSignatures()
    }

    private fun manualSignatures(): List<NcSourceIndex.Signature> {
        fun sig(name: String, returnType: String, vararg params: Pair<String, String>): NcSourceIndex.Signature {
            return NcSourceIndex.Signature(
                name,
                params.map { NcSourceIndex.Parameter(it.first, it.second, 0) },
                returnType,
                0
            )
        }
        return listOf(
            sig("print", "void", "value" to "any"),
            sig("println", "void", "value" to "any"),
            sig("color", "str", "text" to "str", "color" to "str"),
            sig("colorBold", "str", "text" to "str", "color" to "str"),
            sig("renderInt", "str", "value" to "int"),
            sig("renderBool", "str", "value" to "bool"),
            sig("renderBinary", "str", "value" to "int"),
            sig("renderBin", "str", "value" to "int"),
            sig("renderHex", "str", "value" to "int"),
            sig("pOffset", "p<T>", "pointer" to "p<T>", "elements" to "int"),
            sig("pDiff", "int", "right" to "p<T>", "left" to "p<T>")
        )
    }

    private fun addHint(hints: MutableMap<Int, MutableList<String>>, offset: Int, label: String) {
        if (offset < 0 || label.isBlank()) return
        hints.getOrPut(offset) { mutableListOf() }.add(label)
    }

    private fun isNamedArgument(text: String): Boolean {
        return Regex("""^\s*[A-Za-z_][A-Za-z0-9_]*\s*:""").containsMatchIn(text)
    }

    private fun argumentAlreadyNamesParameter(text: String, parameter: String): Boolean {
        return text.trim().removePrefix("@") == parameter
    }
}
