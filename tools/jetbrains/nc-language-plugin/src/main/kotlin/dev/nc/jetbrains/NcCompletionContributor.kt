package dev.nc.jetbrains

import com.intellij.codeInsight.completion.CompletionContributor
import com.intellij.codeInsight.completion.CompletionParameters
import com.intellij.codeInsight.completion.CompletionProvider
import com.intellij.codeInsight.completion.CompletionResultSet
import com.intellij.codeInsight.completion.CompletionType
import com.intellij.codeInsight.completion.PrioritizedLookupElement
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.patterns.PlatformPatterns
import com.intellij.psi.PsiFile
import com.intellij.util.ProcessingContext

class NcCompletionContributor : CompletionContributor() {
    init {
        extend(
            CompletionType.BASIC,
            PlatformPatterns.psiElement(),
            object : CompletionProvider<CompletionParameters>() {
                override fun addCompletions(
                    parameters: CompletionParameters,
                    context: ProcessingContext,
                    result: CompletionResultSet
                ) {
                    val file = parameters.originalFile
                    if (file.language != NcLanguage) return
                    val text = file.text ?: ""
                    val offset = parameters.offset.coerceIn(0, text.length)
                    val prefix = linePrefix(text, offset)
                    val word = currentWord(prefix)

					val lowLevelRows = NcLowLevelCompletionCatalog.rows(file.virtualFile?.extension, text, prefix)
					if (lowLevelRows != null) {
						for (row in lowLevelRows) {
							result.addElement(
								PrioritizedLookupElement.withPriority(lowLevelElement(row), row.priority),
							)
						}
						result.stopHere()
						return
					}

                    val fieldRows = structFieldElements(text, offset)
                    val entryRows = entrypointSnippets(text)

                    if (isImportContext(prefix)) {
                        for (row in importFamilySnippets()) {
                            result.addElement(PrioritizedLookupElement.withPriority(row, 280.0))
                        }
                        for (row in projectModuleElements(file, text, offset)) {
                            result.addElement(PrioritizedLookupElement.withPriority(row, 270.0))
                        }
                        return
                    }

                    if (fieldRows.isNotEmpty() && isFieldContext(prefix)) {
                        for (row in fieldRows) {
                            result.addElement(PrioritizedLookupElement.withPriority(row, 280.0))
                        }
                        return
                    }

                    if (entryRows.isNotEmpty() && isEntrypointContext(prefix)) {
                        for (row in entryRows) {
                            result.addElement(PrioritizedLookupElement.withPriority(row, 280.0))
                        }
                        return
                    }

                    val showLargeCatalog = word.length >= 2 || prefix.trimEnd().endsWith("@") || prefix.trimEnd().endsWith(".")

                    for (row in importedFunctionElements(text)) {
                        result.addElement(PrioritizedLookupElement.withPriority(row, 260.0))
                    }
                    for (row in fieldRows) {
                        result.addElement(PrioritizedLookupElement.withPriority(row, 250.0))
                    }
                    for (row in entryRows) {
                        result.addElement(PrioritizedLookupElement.withPriority(row, 240.0))
                    }

                    if (showLargeCatalog) {
                        for (row in NcIntellisenseCatalog.current.symbols) {
                            result.addElement(symbolElement(row, NcIntellisenseCatalog.isSourceFile(file)))
                        }
                        for (row in NcIntellisenseCatalog.current.types) {
                            result.addElement(typeElement(row))
                        }
                    }
                    for (snippet in structuralSnippets()) {
                        result.addElement(snippet)
                    }
                }
            }
        )
    }

    private data class NcField(val owner: String, val name: String, val typeName: String)

    private fun linePrefix(text: String, offset: Int): String {
        val safeOffset = offset.coerceIn(0, text.length)
        val start = text.lastIndexOf('\n', (safeOffset - 1).coerceAtLeast(0)).let { if (it < 0) 0 else it + 1 }
        return text.substring(start, safeOffset)
    }

    private fun currentWord(prefix: String): String {
        return Regex("""[A-Za-z_][A-Za-z0-9_]*$""").find(prefix)?.value ?: ""
    }

	private fun lowLevelElement(row: NcLowLevelSuggestion): LookupElementBuilder {
		return LookupElementBuilder.create(row.label)
			.withTypeText(row.kind, true)
			.withTailText("  ${row.detail}", true)
			.withInsertHandler { insertionContext, _ ->
				if (row.insertText != row.label) {
					insertionContext.document.replaceString(
						insertionContext.startOffset,
						insertionContext.tailOffset,
						row.insertText,
					)
				}
			}
	}

    private fun isImportContext(prefix: String): Boolean {
        val clean = prefix.trimStart()
        return clean == "use" || clean.startsWith("use <") || clean.startsWith("use ") ||
            clean == "import" || clean.startsWith("import ")
    }

    private fun isFieldContext(prefix: String): Boolean {
        return Regex("""\.\s*[A-Za-z0-9_]*$""").containsMatchIn(prefix)
    }

    private fun isEntrypointContext(prefix: String): Boolean {
        return Regex("""^\s*ref(?:\s+main)?(?:\s*->>)?\s*$""").matches(prefix)
    }

    private fun symbolElement(row: NcCatalogEntry, canImport: Boolean): LookupElementBuilder {
        val typeText = row.signature.ifBlank { row.kind }
        val preferred = NcIntellisenseCatalog.preferredImport(row)
        val scope = NcIntellisenseCatalog.importScope(row)
        val tail = if (preferred.isNotBlank()) "  $scope  $preferred" else "  $scope"
        var builder = LookupElementBuilder.create(row.label)
            .withTypeText(typeText, true)
            .withTailText(tail, true)
            .withPresentableText(row.label)

        val insert = row.insertText.ifBlank { row.label }
        if (insert != row.label) {
            builder = builder.withLookupString(insert)
        }

        if (canImport && preferred.isNotBlank() && NcIntellisenseCatalog.autoImportAllowed(row)) {
            builder = builder.withInsertHandler { insertionContext, _ ->
                applyCatalogInsertion(insertionContext.document, insertionContext.tailOffset, row)
                NcIntellisenseCatalog.insertImportIfMissing(insertionContext.document, row)
            }
        } else {
            builder = builder.withInsertHandler { insertionContext, _ ->
                applyCatalogInsertion(insertionContext.document, insertionContext.tailOffset, row)
            }
        }
        return builder
    }

    private fun applyCatalogInsertion(document: com.intellij.openapi.editor.Document, tailOffset: Int, row: NcCatalogEntry) {
        val requested = row.insertText
            .replace(Regex("""\$\{\d+:([^}]+)}"""), "$1")
            .replace(Regex("""\$\d+"""), "")
        if (requested.isBlank() || requested == row.label || !requested.startsWith(row.label)) return
        val suffix = requested.removePrefix(row.label)
        if (suffix.isNotBlank()) {
            document.insertString(tailOffset, suffix)
        }
    }

    private fun typeElement(row: NcCatalogEntry): LookupElementBuilder {
        return LookupElementBuilder.create(row.label)
            .withTypeText(row.signature.ifBlank { "nC type" }, true)
            .withTailText(if (row.risk.isNotBlank()) "  type/risk=${row.risk}" else "  nC type", true)
    }

    private fun structuralSnippets(): List<LookupElementBuilder> {
        return listOf(
            LookupElementBuilder.create("to main").withTypeText("entry family", true),
            LookupElementBuilder.create("ref main").withTypeText("entry family", true),
            LookupElementBuilder.create("use <io:output, render, input>;").withTypeText("import family", true),
            LookupElementBuilder.create("use <io:output, render>;").withTypeText("rich output import", true),
            LookupElementBuilder.create("use <memory:ptr, heap, region>;").withTypeText("import family", true),
            LookupElementBuilder.create("use <collections:list, queue, span, slice>;").withTypeText("import family", true),
            LookupElementBuilder.create("use <data:json>;").withTypeText("JSON import", true),
            LookupElementBuilder.create("use <task:async, scheduler, cancel>;").withTypeText("task import", true),
            LookupElementBuilder.create("use <text:string, format, parse>;").withTypeText("text import", true),
            LookupElementBuilder.create("use <sync:atomic, lock, channel>;").withTypeText("synchronization import", true),
            LookupElementBuilder.create("use <audit:diagnostic, trace, debug>;").withTypeText("audit import", true),
            LookupElementBuilder.create("use <terminal:terminal>;").withTypeText("terminal import", true),
            LookupElementBuilder.create("use <mobile:telephony, tts_engine, permissions>;").withTypeText("mobile import", true),
            LookupElementBuilder.create("use <network:server, client, send_bytes, receive_bytes>;").withTypeText("mobile network import", true),
            LookupElementBuilder.create("use <project:module>;").withTypeText("project import", true),
            LookupElementBuilder.create("use <core.types:option, result>;").withTypeText("core types import", true),
            LookupElementBuilder.create("use <core.memory:ptr, region, layout>;").withTypeText("core memory import", true),
            LookupElementBuilder.create("use <core.sync:atomic>;").withTypeText("core atomic import", true),
            LookupElementBuilder.create("use <alloc.collections:list, queue, vector>;").withTypeText("allocator collections import", true),
            LookupElementBuilder.create("use <dom.compiler:parse, sema, diagnostics>;").withTypeText("compiler import", true),
            LookupElementBuilder.create("use <auth.system:file, process, permission>;").withTypeText("authority import", true),
            LookupElementBuilder.create("container block").withTypeText("nC compact container", true)
                .withInsertHandler { insertionContext, _ ->
                    val skeleton = listOf(
                        "contador {",
                        "    const:",
                        "        LIMIT >> int <- 10;",
                        "",
                        "    stc:",
                        "        valor >> int <- 0;",
                        "",
                        "    fn:",
                        "        public:",
                        "            run() -> bool {",
                        "                ",
                        "                return valor < LIMIT;",
                        "            }",
                        "",
                        "    ref main ->> run;",
                        "",
                        "    fa:",
                        "        return true;",
                        "}"
                    ).joinToString("\n")
                    insertionContext.document.replaceString(insertionContext.startOffset, insertionContext.tailOffset, skeleton)
                },
            LookupElementBuilder.create("color(${'$'}{1:text}, \"cyan\")").withTypeText("rich output", true),
            LookupElementBuilder.create("colorBold(${'$'}{1:text}, \"cyan\")").withTypeText("rich output", true),
            LookupElementBuilder.create("\\n").withTypeText("print newline escape", true),
            LookupElementBuilder.create("\\&").withTypeText("print literal ampersand", true),
            LookupElementBuilder.create("\\?").withTypeText("print literal question", true),
            LookupElementBuilder.create("\\#").withTypeText("print literal hash", true),
            LookupElementBuilder.create("##").withTypeText("print separator control", true)
        )
    }

    private fun importFamilySnippets(): List<LookupElementBuilder> {
        return listOf(
            LookupElementBuilder.create("use <io:output, render, input>;")
                .withTypeText("input/output/render import", true),
            LookupElementBuilder.create("use <memory:ptr, heap, region>;")
                .withTypeText("pointer/heap/region import", true),
            LookupElementBuilder.create("use <collections:list, queue, span, slice>;")
                .withTypeText("collections import", true),
            LookupElementBuilder.create("use <data:json>;")
                .withTypeText("JSON import", true),
            LookupElementBuilder.create("use <task:async, scheduler, cancel>;")
                .withTypeText("task/async import", true),
            LookupElementBuilder.create("use <text:string, format, parse>;")
                .withTypeText("text import", true),
            LookupElementBuilder.create("use <sync:atomic, lock, channel>;")
                .withTypeText("synchronization import", true),
            LookupElementBuilder.create("use <audit:diagnostic, trace, debug>;")
                .withTypeText("audit import", true),
            LookupElementBuilder.create("use <terminal:terminal>;")
                .withTypeText("terminal import", true),
            LookupElementBuilder.create("use <mobile:telephony, tts_engine, permissions>;")
                .withTypeText("mobile import", true),
            LookupElementBuilder.create("use <network:server, client, send_bytes, receive_bytes>;")
                .withTypeText("mobile network import", true),
            LookupElementBuilder.create("use <core.types:option, result>;")
                .withTypeText("core types import", true),
            LookupElementBuilder.create("use <core.memory:ptr, region, layout>;")
                .withTypeText("core memory import", true),
            LookupElementBuilder.create("use <core.sync:atomic>;")
                .withTypeText("core atomic import", true),
            LookupElementBuilder.create("use <alloc.collections:list, queue, vector>;")
                .withTypeText("allocator collections import", true),
            LookupElementBuilder.create("use <project:module>;")
                .withTypeText("project child import", true)
        )
    }

    private fun importedFunctionElements(text: String): List<LookupElementBuilder> {
        return NcIntellisenseCatalog.current.symbols
            .filter { it.kind == "function" && NcIntellisenseCatalog.hasImport(text, it) }
            .map {
                LookupElementBuilder.create(it.label)
                    .withTypeText(it.signature.ifBlank { "imported function" }, true)
                    .withTailText("  imported", true)
                    .withInsertHandler { insertionContext, _ ->
                        applyCatalogInsertion(insertionContext.document, insertionContext.tailOffset, it)
                    }
            }
    }

    private fun entrypointSnippets(text: String): List<LookupElementBuilder> {
        if (Regex("""\b(?:ref\s+main|to\s+main)\b""").containsMatchIn(text)) {
            return emptyList()
        }
        val functions = Regex("""(?m)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*::\s*in\s*\([^)]*\)\s*->\s*[^\{]+\{""")
            .findAll(text)
            .map { it.groupValues[1] }
            .filter { it !in setOf("if", "while", "for", "match") }
            .toList()
            .ifEmpty { listOf("run") }
        return functions.map {
            LookupElementBuilder.create("ref main ->> $it;")
                .withTypeText("entrypoint reference", true)
        }
    }

    private fun projectModuleElements(file: PsiFile, text: String, offset: Int): List<LookupElementBuilder> {
        val prefix = text.substring(0, offset.coerceIn(0, text.length))
        if (!Regex("""^\s*use\s*<\s*project\s*:[^>]*$""")
                .containsMatchIn(prefix.lines().lastOrNull() ?: "")) {
            return emptyList()
        }
        val current = file.virtualFile?.path ?: return emptyList()
        val parent = file.virtualFile?.parent ?: return emptyList()
        return parent.children
            .filter { !it.isDirectory && it.path != current }
            .filter { it.extension?.lowercase() == "n" || it.extension?.lowercase() == "nc" }
            .mapNotNull { it.nameWithoutExtension.takeIf { name -> Regex("""^[A-Za-z_][A-Za-z0-9_]*$""").matches(name) } }
            .distinct()
            .sorted()
            .map { LookupElementBuilder.create(it).withTypeText("project module", true) }
    }

    private fun structFieldElements(text: String, offset: Int): List<LookupElementBuilder> {
        val prefix = text.substring(0, offset.coerceIn(0, text.length))
        val symbols = typedSymbols(text)
        val getReceiver = Regex("""@?([A-Za-z_][A-Za-z0-9_]*)\s*\.\s*get\s*\([^)]*\)\s*\.\s*[A-Za-z0-9_]*$""")
            .find(prefix)?.groupValues?.get(1)
        val indexReceiver = Regex("""@?([A-Za-z_][A-Za-z0-9_]*)\s*\[[^\]]*]\s*\.\s*[A-Za-z0-9_]*$""")
            .find(prefix)?.groupValues?.get(1)
        val directReceiver = Regex("""@?([A-Za-z_][A-Za-z0-9_]*)\s*\.\s*[A-Za-z0-9_]*$""")
            .find(prefix)?.groupValues?.get(1)
        val typeName = when {
            getReceiver != null -> elementAggregateType(symbols[getReceiver] ?: "")
            indexReceiver != null -> elementAggregateType(symbols[indexReceiver] ?: "")
            directReceiver != null -> baseAggregateType(symbols[directReceiver] ?: "")
            else -> ""
        }
        if (typeName.isBlank()) return emptyList()
        val fields = parseStructFields(text)[typeName] ?: return emptyList()
        return fields.map {
            LookupElementBuilder.create(it.name)
                .withTypeText("${it.owner}.${it.name}: ${it.typeName}", true)
                .withTailText("  field", true)
        }
    }

    private fun typedSymbols(text: String): Map<String, String> {
        val out = linkedMapOf<String, String>()
        for (match in Regex("""@?([A-Za-z_][A-Za-z0-9_]*)\s*>>\s*([^:;=]+)\s*(?:<-|:|=|;)""").findAll(text)) {
            out[match.groupValues[1]] = cleanDeclaredType(match.groupValues[2])
        }
        for (match in Regex("""(?m)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^;]+);""").findAll(text)) {
            out.putIfAbsent(match.groupValues[1], cleanDeclaredType(match.groupValues[2]))
        }
        return out
    }

    private fun parseStructFields(text: String): Map<String, List<NcField>> {
        val out = linkedMapOf<String, MutableList<NcField>>()
        var inType = false
        var owner = ""
        var depth = 0
        for (line in text.lines()) {
            var openedHere = false
            val section = Regex("""^\s*(stc|local|const|fn|fa|type|struct|enum|binary|bind|raw|bridge|final|routine|verify|report|seal)\s*:""")
                .find(line)
            if (section != null && owner.isBlank()) {
                inType = section.groupValues[1] == "type" || section.groupValues[1] == "struct"
            }
            if (owner.isBlank()) {
                val canonicalType = Regex("""^\s*([A-Za-z_][A-Za-z0-9_]*)\s*::\s*type\s*(?:\[[^\]]+])?\s*\{""").find(line)
                val legacyType = if (inType) Regex("""^\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^\]]+])?\s*\{""").find(line) else null
                val type = canonicalType ?: legacyType
                if (type != null) {
                    owner = type.groupValues[1]
                    out.putIfAbsent(owner, mutableListOf())
                    depth = line.count { it == '{' } - line.count { it == '}' }
                    openedHere = true
                }
            }
            if (owner.isNotBlank()) {
                val fieldSource = if (openedHere && line.contains("{")) line.substringAfter("{") else line
                val normalizedFieldSource = fieldSource.replace(Regex("""\b(?:public|private|internal|protected)\s*:"""), " ")
                val fields = Regex("""(?:^|[;\s])@?([A-Za-z_][A-Za-z0-9_]*)\s*(?:>>|:)\s*([^;<-\{\}]+)""")
                    .findAll(normalizedFieldSource)
                for (field in fields) {
                    out[owner]?.add(NcField(owner, field.groupValues[1], cleanDeclaredType(field.groupValues[2])))
                }
                if (!openedHere) {
                    depth += line.count { it == '{' } - line.count { it == '}' }
                }
                if (depth <= 0) owner = ""
            }
        }
        return out
    }

    private fun cleanDeclaredType(raw: String): String {
        return raw.replace(Regex("""\s+"""), " ")
            .replace(Regex("""\s*(?:;|=|:).*"""), "")
            .trim()
    }

    private fun baseAggregateType(raw: String): String {
        val clean = cleanDeclaredType(raw).removePrefix("@")
        val pointer = Regex("""^(?:ptr|ref|site|ptr\s+mut|ref\s+mut)\s*<\s*([A-Za-z_][A-Za-z0-9_]*)""").find(clean)
        if (pointer != null) return pointer.groupValues[1]
        return Regex("""^([A-Za-z_][A-Za-z0-9_]*)""").find(clean)?.groupValues?.get(1) ?: clean
    }

    private fun elementAggregateType(raw: String): String {
        return Regex("""^(?:vector|list|dinArray|array|slice|span)\s*<\s*([A-Za-z_][A-Za-z0-9_]*)""")
            .find(cleanDeclaredType(raw))
            ?.groupValues
            ?.get(1) ?: ""
    }
}
