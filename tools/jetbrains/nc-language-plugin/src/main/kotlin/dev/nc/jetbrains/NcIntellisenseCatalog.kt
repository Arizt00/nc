package dev.nc.jetbrains

import com.fasterxml.jackson.databind.ObjectMapper
import com.intellij.openapi.editor.Document
import com.intellij.psi.PsiFile

class NcCatalog {
    var schema: String = ""
    var version: String = ""
    var authority: String = ""
    var symbols: List<NcCatalogEntry> = emptyList()
    var types: List<NcCatalogEntry> = emptyList()
    var diagnostics: List<NcCatalogEntry> = emptyList()
}

class NcCatalogEntry {
    var label: String = ""
    var kind: String = ""
    var signature: String = ""
    var insertText: String = ""
    var legacyImport: String = ""
    var facadeImport: String = ""
    var detail: String = ""
    var documentation: String = ""
    var docs: String = ""
    var risk: String = ""
    var aliases: List<String> = emptyList()
}

object NcIntellisenseCatalog {
    private val mapper = ObjectMapper()
    private data class FacadeImport(val domain: String, val family: String, val items: List<String>)
    private data class FacadeMerge(val line: Int, val startOffset: Int, val endOffset: Int, val newLine: String, val changed: Boolean)
    private data class BuiltinFunction(val name: String, val signature: String)

    private val backendPreludeFamilies = setOf(
        "nC.runtime.string",
        "nC.runtime.char",
        "nC.runtime.terminal",
        "nC.runtime.time",
        "nC.runtime.mobile",
        "nC.runtime.network",
        "nC.runtime.sys"
    )

    private val backendPreludeSymbols = setOf(
        "alloc", "calloc", "realloc", "free",
        "rd", "wr", "off", "dist", "nil", "adr", "from", "asptr",
        "pRead", "pWrite", "pOffset", "pDiff", "pAddr", "pFromAddr", "pCast", "pNull",
        "sizeOf", "alignOf", "offsetOf",
        "atomicLoad", "atomicStore", "fence",
        "sleepMs", "monotonicMs", "epochMs", "exists", "listDir",
        "fswrite", "fread_lines", "delete_file", "rename_file"
    )

    private val memoryMapFunctions = listOf(
        BuiltinFunction("map", "map(bytes: int) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("mapAnonymous", "mapAnonymous(bytes: int, protection: int, flags: int) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("mapAt", "mapAt(address: addr, bytes: int, protection: int, flags: int) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("mapShared", "mapShared(bytes: int, protection: int) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("mapPrivate", "mapPrivate(bytes: int, protection: int) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("mapFile", "mapFile(path: str, bytes: int, offset: int, protection: int, flags: int) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("mapFileShared", "mapFileShared(path: str, bytes: int, offset: int, protection: int) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("mapFilePrivate", "mapFilePrivate(path: str, bytes: int, offset: int, protection: int) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("unmap", "unmap(ptr: p<byte>) -> Result<bool, MemoryError>"),
        BuiltinFunction("remap", "remap(ptr: p<byte>, bytes: int, mayMove: bool) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("remapAt", "remapAt(ptr: p<byte>, bytes: int, address: addr) -> Result<p<byte>, MemoryError>"),
        BuiltinFunction("mapProtect", "mapProtect(ptr: p<byte>, protection: int) -> Result<bool, MemoryError>"),
        BuiltinFunction("mapSync", "mapSync(ptr: p<byte>, flags: int) -> Result<bool, MemoryError>"),
        BuiltinFunction("mapAdvise", "mapAdvise(ptr: p<byte>, advice: int) -> Result<bool, MemoryError>"),
        BuiltinFunction("mapLock", "mapLock(ptr: p<byte>) -> Result<bool, MemoryError>"),
        BuiltinFunction("mapUnlock", "mapUnlock(ptr: p<byte>) -> Result<bool, MemoryError>"),
        BuiltinFunction("mapLen", "mapLen(ptr: p<byte>) -> int"),
        BuiltinFunction("mapAddress", "mapAddress(ptr: p<byte>) -> p<byte>"),
        BuiltinFunction("mapIsMapped", "mapIsMapped(ptr: p<byte>) -> bool")
    )

    private val kernelCompactFunctions = listOf(
        BuiltinFunction("dbgRegs", "dbgRegs(source: addr, output: addr, slots: int) -> int"),
        BuiltinFunction("dbgDump", "dbgDump(source: addr, output: addr, bytes: int) -> int"),
        BuiltinFunction("dbgTrace", "dbgTrace(frame: addr, output: addr, maximum: int) -> int"),
        BuiltinFunction("dbgSym", "dbgSym(table: addr, count: int, pc: addr, output: addr) -> bool"),
        BuiltinFunction("rspSum", "rspSum(data: addr, bytes: int) -> int"),
        BuiltinFunction("rspFrame", "rspFrame(data: addr, bytes: int, output: addr) -> int"),
        BuiltinFunction("brkSet", "brkSet(code: addr, saved: addr) -> int"),
        BuiltinFunction("brkDel", "brkDel(code: addr, saved: addr, width: int) -> bool"),
        BuiltinFunction("crashWrite", "crashWrite(report: addr, code: int, pc: addr, sp: addr, fault: addr, flags: int) -> bool"),
        BuiltinFunction("kaslr", "kaslr(image: addr, slide: int) -> addr"),
        BuiltinFunction("canarySet", "canarySet(slot: addr, value: int) -> bool"),
        BuiltinFunction("canaryOk", "canaryOk(slot: addr, expected: int) -> bool"),
        BuiltinFunction("sanPoison", "sanPoison(shadow: addr, index: int, tag: int) -> bool"),
        BuiltinFunction("sanClear", "sanClear(shadow: addr, index: int) -> bool"),
        BuiltinFunction("sanOk", "sanOk(shadow: addr, index: int) -> bool")
    )

    val current: NcCatalog by lazy {
        load()
    }

    fun entries(): List<NcCatalogEntry> = current.symbols + current.types + current.diagnostics

    fun sourceEntries(): List<NcCatalogEntry> = current.symbols + current.types

    fun lookup(raw: String?): NcCatalogEntry? {
        val direct = (raw ?: "").trim()
        if (direct.isNotBlank()) {
            entries().firstOrNull { row ->
                row.label == direct ||
                    row.signature == direct ||
                    row.aliases.contains(direct)
            }?.let { return it }
        }
        val token = cleanToken(raw)
        if (token.isBlank()) return null
        return entries().firstOrNull { row ->
            row.label == token ||
                row.signature == token ||
                row.aliases.contains(token) ||
                row.signature.startsWith("$token<") ||
                row.signature.startsWith("$token(")
        }
    }

    fun preferredImport(row: NcCatalogEntry): String {
        if (isBackendPrelude(row)) return ""
        if (row.facadeImport.isNotBlank()) return row.facadeImport
        if (row.legacyImport.isNotBlank()) return "use ${row.legacyImport};"
        return ""
    }

    fun legacyImport(row: NcCatalogEntry): String {
        if (isBackendPrelude(row)) return ""
        if (row.legacyImport.isBlank()) return ""
        return "use ${row.legacyImport};"
    }

    fun importScope(row: NcCatalogEntry): String {
        val parsed = parseFacadeImport(preferredImport(row)) ?: return if (legacyImport(row).isNotBlank()) "legacy/use" else row.kind
        val item = if (parsed.items.size == 1) ":${parsed.items.first()}" else ""
        return "${parsed.domain}/${parsed.family}$item"
    }

    fun autoImportAllowed(row: NcCatalogEntry): Boolean {
        if (isBackendPrelude(row)) return false
        val parsed = parseFacadeImport(preferredImport(row))
        if (parsed == null) return legacyImport(row).isNotBlank()
        if (parsed.domain == "auth") return false
        return parsed.domain == "stoud" || parsed.domain == "dom"
    }

    fun hasImport(text: String, row: NcCatalogEntry): Boolean {
        if (isBackendPrelude(row)) return true
        val preferred = preferredImport(row)
        val legacy = legacyImport(row)
        val lines = text.lines().map { it.trim() }
        return (preferred.isNotBlank() && hasFacadeImport(text, preferred)) ||
            (legacy.isNotBlank() && lines.contains(legacy))
    }

    fun insertImportIfMissing(document: Document, row: NcCatalogEntry) {
        if (isBackendPrelude(row)) return
        val statement = preferredImport(row)
        if (statement.isBlank() || hasImport(document.text, row)) return
        val merge = mergeFacadeImport(document, statement)
        if (merge != null && merge.changed) {
            document.replaceString(merge.startOffset, merge.endOffset, mergedImportText(document, merge))
            return
        }
        document.insertString(importInsertionOffset(document), importInsertionText(document, statement))
    }

    fun isNcFile(file: PsiFile?): Boolean {
        val ext = file?.virtualFile?.extension?.lowercase() ?: return false
        return ext in setOf("n", "nc", "nb", "nbb", "nm", "ncp")
    }

    fun isSourceFile(file: PsiFile?): Boolean {
        val ext = file?.virtualFile?.extension?.lowercase() ?: return false
        return ext == "n" || ext == "nc"
    }

    fun cleanToken(raw: String?): String {
        return (raw ?: "")
            .removePrefix("@")
            .substringBefore("(")
            .trim()
            .trim(';', ':', ',', '[', ']')
    }

    fun isTokenBoundary(text: String?): Boolean {
        return Regex("""[\s();,:\[\]{}<>.]""").containsMatchIn(text ?: "")
    }

    fun completedTokenBeforeBoundary(text: String, offset: Int): String {
        val boundedOffset = offset.coerceIn(0, text.length)
        val left = text.substring(0, boundedOffset).replace(Regex("""[\s();,:\[\]{}<>.]+$"""), "")
        val match = Regex("""@?[A-Za-z_][A-Za-z0-9_.]*$""").find(left) ?: return ""
        return cleanToken(match.value.removePrefix("@"))
    }

    fun autoImportCandidateAtOffset(text: String, offset: Int): NcCatalogEntry? {
        val token = completedTokenBeforeBoundary(text, offset)
        if (token.isBlank()) return null
        val row = lookup(token) ?: return null
        if (preferredImport(row).isBlank() || !autoImportAllowed(row) || hasImport(text, row)) return null
        return row
    }

    fun isBackendPrelude(row: NcCatalogEntry): Boolean {
        return row.label in backendPreludeSymbols || row.legacyImport in backendPreludeFamilies
    }

    fun renderDoc(row: NcCatalogEntry): String {
        val signature = row.signature.ifBlank { row.label }
        val preferred = preferredImport(row)
        val legacy = legacyImport(row)
        val importBlock = buildString {
            if (preferred.isNotBlank()) append("<p><b>Preferred import:</b> <code>$preferred</code></p>")
            if (legacy.isNotBlank() && legacy != preferred) append("<p><b>Legacy import:</b> <code>$legacy</code></p>")
        }
        val risk = if (row.risk.isNotBlank()) "<p><b>Risk:</b> <code>${row.risk}</code></p>" else ""
        val docs = if (row.docs.isNotBlank()) "<p><b>Docs:</b> <code>${row.docs}</code></p>" else ""
        val detail = row.detail.ifBlank { row.documentation }
        val body = row.documentation.ifBlank { row.detail }
        return """
            <html>
              <body>
                <h3>${row.label}</h3>
                <p><code>$signature</code></p>
                <p>$detail</p>
                <p>$body</p>
                $importBlock
                $risk
                $docs
                <p><i>ncc remains the semantic authority; this plugin predicts and explains.</i></p>
              </body>
            </html>
        """.trimIndent()
    }

    private fun load(): NcCatalog {
        val catalog = try {
            val stream = javaClass.classLoader.getResourceAsStream("nc_intellisense_catalog.json")
            if (stream == null) fallback() else stream.use { mapper.readValue(it, NcCatalog::class.java) }
        } catch (_: Exception) {
            fallback()
        }
        ensureMemoryMapSurface(catalog)
        ensureKernelCompactSurface(catalog)
        return catalog
    }

    private fun ensureMemoryMapSurface(catalog: NcCatalog) {
        val known = catalog.symbols.map { it.label }.toMutableSet()
        val additions = memoryMapFunctions
            .filter { known.add(it.name) }
            .map { function ->
                entry(
                    function.name,
                    "function",
                    function.signature,
                    "nC.runtime.memory",
                    "use <memory:map>;"
                ).also { row ->
                    row.insertText = "${function.name}()"
                    row.detail = function.signature
                    row.documentation = "Native mapped-memory operation with Result-based lifecycle errors."
                    row.risk = "high"
                }
            }
        if (additions.isNotEmpty()) catalog.symbols = catalog.symbols + additions
    }

    private fun ensureKernelCompactSurface(catalog: NcCatalog) {
        val known = catalog.symbols.map { it.label }.toMutableSet()
        val additions = kernelCompactFunctions
            .filter { known.add(it.name) }
            .map { function ->
                entry(function.name, "function", function.signature, "", "").also { row ->
                    row.insertText = "${function.name}()"
                    row.detail = function.signature
                    row.documentation = "Manual kernel intrinsic; all addresses, buffers and policy choices belong to the caller."
                    row.risk = "high"
                }
            }
        if (additions.isNotEmpty()) catalog.symbols = catalog.symbols + additions
    }

    private fun fallback(): NcCatalog {
        val catalog = NcCatalog()
        catalog.schema = "nc.ide.intellisense.catalog.v1"
        catalog.version = "fallback"
        catalog.authority = "editor-assist-only; ncc remains semantic authority"
        catalog.symbols = listOf(entry("readInt", "function", "readInt() -> int", "nC.runtime.input", "use <io:input>;"))
        catalog.types = listOf(entry("p", "type", "p<T>", "", ""), entry("memRegion", "type", "memRegion<T>", "", ""))
        return catalog
    }

    private fun entry(label: String, kind: String, signature: String, legacyImport: String, facadeImport: String): NcCatalogEntry {
        val row = NcCatalogEntry()
        row.label = label
        row.kind = kind
        row.signature = signature
        row.legacyImport = legacyImport
        row.facadeImport = facadeImport
        row.detail = signature
        row.documentation = signature
        return row
    }

    private fun parseFacadeImport(line: String): FacadeImport? {
        val canonical = Regex("""^\s*use\s*<\s*([A-Za-z_][A-Za-z0-9_]*)(?:\.([A-Za-z_][A-Za-z0-9_]*))?\s*:\s*([^>]*)>\s*;?\s*$""")
            .matchEntire(line.trim())
        if (canonical != null) {
            val explicitDomain = canonical.groupValues[2]
            val domain = if (explicitDomain.isNotBlank()) canonical.groupValues[1]
                else if (canonical.groupValues[1] == "project") "project" else "stoud"
            val family = if (explicitDomain.isNotBlank()) explicitDomain else canonical.groupValues[1]
            val items = canonical.groupValues[3].split(",").map { it.trim() }.filter { it.isNotBlank() }
            return FacadeImport(domain, family, items)
        }
        val block = Regex("""^\s*use\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*\{\s*([^}]*)\s*}\s*;?\s*$""")
            .matchEntire(line.trim())
        if (block != null) {
            val path = block.groupValues[1].split(".")
            val items = block.groupValues[2].split(",").map { it.trim() }.filter { it.isNotBlank() }
            val domain = if (path.size == 1 && path[0] == "project") "project" else path.first()
            val family = if (path.size == 1 && path[0] == "project") "project" else path.drop(1).joinToString(".")
            return FacadeImport(domain, family, items)
        }
        val match = Regex("""^\s*use::([A-Za-z_][A-Za-z0-9_]*)::([A-Za-z_][A-Za-z0-9_]*)::(?:<\s*([^>]*)\s*>|([A-Za-z_][A-Za-z0-9_]*))\s*;?\s*$""")
            .matchEntire(line.trim()) ?: return null
        val itemsRaw = match.groupValues[3].ifBlank { match.groupValues[4] }
        val items = itemsRaw
            .split(",")
            .map { it.trim() }
            .filter { it.isNotBlank() }
        return FacadeImport(match.groupValues[1], match.groupValues[2], items)
    }

    private fun hasFacadeImport(text: String, statement: String): Boolean {
        val expected = parseFacadeImport(statement)
        return text.lines().any { line ->
            val trimmed = line.trim()
            if (trimmed == statement.trim()) return@any true
            val actual = parseFacadeImport(trimmed) ?: return@any false
            if (expected == null) return@any false
            actual.domain == expected.domain &&
                actual.family == expected.family &&
                expected.items.all { actual.items.contains(it) }
        }
    }

    private fun mergeFacadeImport(document: Document, statement: String): FacadeMerge? {
        val expected = parseFacadeImport(statement) ?: return null
        if (expected.items.isEmpty()) return null
        var line = 0
        while (line < document.lineCount) {
            val start = document.getLineStartOffset(line)
            val end = document.getLineEndOffset(line)
            val text = document.text.substring(start, end)
            val actual = parseFacadeImport(text)
            if (actual != null && actual.domain == expected.domain && actual.family == expected.family) {
                val merged = actual.items.toMutableList()
                for (item in expected.items) {
                    if (!merged.contains(item)) merged.add(item)
                }
                val indent = Regex("""^\s*""").find(text)?.value ?: ""
                val module = if (actual.domain == "stoud" || actual.domain == "project") {
                    actual.family
                } else {
                    "${actual.domain}.${actual.family}"
                }
                val replacement = "${indent}use <${module}:${merged.joinToString(", ")}>;"
                return FacadeMerge(
                    line,
                    start,
                    end,
                    replacement,
                    merged.size != actual.items.size
                )
            }
            line += 1
        }
        return null
    }

    private fun importInsertionOffset(document: Document): Int {
        val line = importInsertionLine(document)
        return if (line >= document.lineCount) document.textLength else document.getLineStartOffset(line)
    }

    private fun importInsertionLine(document: Document): Int {
        var lastImport = -1
        var line = 0
        while (line < document.lineCount) {
            val start = document.getLineStartOffset(line)
            val end = document.getLineEndOffset(line)
            val text = document.text.substring(start, end).trim()
            if (text.isBlank()) {
                line += 1
                continue
            }
            if (lineLooksLikeImport(document, line)) {
                lastImport = line
                line += 1
                continue
            }
            break
        }
        return if (lastImport >= 0) lastImport + 1 else 0
    }

    private fun importInsertionText(document: Document, statement: String): String {
        val line = importInsertionLine(document)
        val needsBlankAfter = !lineIsBlank(document, line)
        return "$statement\n${if (needsBlankAfter) "\n" else ""}"
    }

    private fun mergedImportText(document: Document, merge: FacadeMerge): String {
        val nextLine = merge.line + 1
        val nextIsImport = nextLine < document.lineCount && lineLooksLikeImport(document, nextLine)
        val needsBlankAfter = !nextIsImport && !lineIsBlank(document, nextLine)
        return "${merge.newLine}${if (needsBlankAfter) "\n" else ""}"
    }

    private fun lineIsBlank(document: Document, line: Int): Boolean {
        if (line < 0 || line >= document.lineCount) return true
        val start = document.getLineStartOffset(line)
        val end = document.getLineEndOffset(line)
        return document.text.substring(start, end).trim().isBlank()
    }

    private fun lineLooksLikeImport(document: Document, line: Int): Boolean {
        if (line < 0 || line >= document.lineCount) return false
        val start = document.getLineStartOffset(line)
        val end = document.getLineEndOffset(line)
        val text = document.text.substring(start, end).trim()
        return text.startsWith("use ") || text.startsWith("use::") || text.startsWith("import ")
    }
}
