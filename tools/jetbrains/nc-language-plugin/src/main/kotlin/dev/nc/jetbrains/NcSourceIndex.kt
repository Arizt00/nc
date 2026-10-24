package dev.nc.jetbrains

object NcSourceIndex {
    data class Declaration(
        val name: String,
        val kind: String,
        val offset: Int,
        val typeName: String = "",
        val owner: String = ""
    )

    data class Field(
        val owner: String,
        val name: String,
        val typeName: String,
        val offset: Int
    )

    data class Parameter(
        val name: String,
        val typeName: String,
        val offset: Int
    )

    data class Signature(
        val name: String,
        val params: List<Parameter>,
        val returnType: String,
        val offset: Int
    )

    data class Argument(
        val text: String,
        val offset: Int
    )

    data class Call(
        val name: String,
        val nameOffset: Int,
        val openOffset: Int,
        val closeOffset: Int,
        val args: List<Argument>
    )

    fun tokenAt(text: String, offset: Int): String {
        if (text.isEmpty()) return ""
        var cursor = offset.coerceIn(0, text.length - 1)
        if (!isTokenChar(text[cursor]) && cursor > 0 && isTokenChar(text[cursor - 1])) cursor -= 1
        if (!isTokenChar(text[cursor])) return ""
        var start = cursor
        var end = cursor + 1
        while (start > 0 && isTokenChar(text[start - 1])) start -= 1
        while (end < text.length && isTokenChar(text[end])) end += 1
        return text.substring(start, end).trim('@', '.', ';', ':', ',', '[', ']')
    }

    fun tokenStartAt(text: String, offset: Int): Int {
        if (text.isEmpty()) return -1
        var cursor = offset.coerceIn(0, text.length - 1)
        if (!isTokenChar(text[cursor]) && cursor > 0 && isTokenChar(text[cursor - 1])) cursor -= 1
        if (!isTokenChar(text[cursor])) return -1
        while (cursor > 0 && isTokenChar(text[cursor - 1])) cursor -= 1
        return cursor
    }

    fun lineAtOffset(text: String, offset: Int): String {
        val safe = offset.coerceIn(0, text.length)
        val start = text.lastIndexOf('\n', (safe - 1).coerceAtLeast(0)).let { if (it < 0) 0 else it + 1 }
        val end = text.indexOf('\n', safe).let { if (it < 0) text.length else it }
        return text.substring(start, end)
    }

    fun lineStartOffsets(text: String): List<Int> {
        val starts = mutableListOf(0)
        for (index in text.indices) {
            if (text[index] == '\n') starts.add(index + 1)
        }
        return starts
    }

    fun codeMask(text: String): BooleanArray {
        val mask = BooleanArray(text.length) { true }
        var i = 0
        while (i < text.length) {
            when {
                i + 1 < text.length && text[i] == '/' && text[i + 1] == '/' -> {
                    val start = i
                    i += 2
                    while (i < text.length && text[i] != '\n') i += 1
                    for (j in start until i) mask[j] = false
                }
                i + 1 < text.length && text[i] == '/' && text[i + 1] == '*' -> {
                    val start = i
                    i += 2
                    while (i + 1 < text.length && !(text[i] == '*' && text[i + 1] == '/')) i += 1
                    i = (i + 2).coerceAtMost(text.length)
                    for (j in start until i) mask[j] = false
                }
                text[i] == '"' || text[i] == '\'' -> {
                    val quote = text[i]
                    val start = i
                    i += 1
                    var escaped = false
                    while (i < text.length) {
                        val ch = text[i]
                        if (escaped) {
                            escaped = false
                        } else if (ch == '\\') {
                            escaped = true
                        } else if (ch == quote) {
                            i += 1
                            break
                        }
                        i += 1
                    }
                    for (j in start until i.coerceAtMost(text.length)) mask[j] = false
                }
                else -> i += 1
            }
        }
        return mask
    }

    fun declarations(text: String): List<Declaration> {
        val mask = codeMask(text)
        val out = mutableListOf<Declaration>()
        for (match in Regex("""(?m)^\s*([A-Za-z_][A-Za-z0-9_]*)(?:\[[^\]]+])?\s*\{""").findAll(text)) {
            if (!isCode(mask, match.range.first)) continue
            val name = match.groupValues[1]
            out.add(Declaration(name, if (isInTypeSection(text, match.range.first)) "type" else "container", match.range.first + match.value.indexOf(name)))
        }
        for (sig in signatures(text)) {
            out.add(Declaration(sig.name, "function", sig.offset, sig.returnType))
            for (param in sig.params) {
                out.add(Declaration(param.name, "parameter", param.offset, param.typeName, sig.name))
            }
        }
        for (match in Regex("""(?m)^\s*@?([A-Za-z_][A-Za-z0-9_]*)\s*>>\s*(.+?)\s*(?:<-|=|:(?!=)|;)""").findAll(text)) {
            if (!isCode(mask, match.range.first)) continue
            out.add(Declaration(match.groupValues[1], "variable", match.range.first + match.value.indexOf(match.groupValues[1]), cleanType(match.groupValues[2])))
        }
        for (match in Regex("""(?m)^\s*@?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^;=\n]+)(?:[;=]|$)""").findAll(text)) {
            if (!isCode(mask, match.range.first)) continue
            val name = match.groupValues[1]
            if (name in SECTION_NAMES) continue
            out.add(Declaration(name, "variable", match.range.first + match.value.indexOf(name), cleanType(match.groupValues[2])))
        }
        for (field in fieldsByType(text).values.flatten()) {
            out.add(Declaration(field.name, "field", field.offset, field.typeName, field.owner))
        }
        return out.distinctBy { "${it.kind}:${it.owner}:${it.name}:${it.offset}" }
    }

    fun signatures(text: String): List<Signature> {
        val mask = codeMask(text)
        return Regex("""(?m)^\s*(?:public\s+|private\s+|internal\s+|protected\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*(?:::\s*fn\s*)?\(([^)]*)\)\s*->\s*([^{\n]+)\{""")
            .findAll(text)
            .filter { isCode(mask, it.range.first) }
            .map { match ->
                val name = match.groupValues[1]
                val paramsRaw = match.groupValues[2]
                val paramsStart = match.range.first + match.value.indexOf('(') + 1
                val params = splitArgs(paramsRaw, paramsStart).mapNotNull { arg ->
                    val param = Regex("""@?([A-Za-z_][A-Za-z0-9_]*)\s*(?:>>|:)\s*([^,]+)""").find(arg.text) ?: return@mapNotNull null
                    Parameter(param.groupValues[1], cleanType(param.groupValues[2]), arg.offset + arg.text.indexOf(param.groupValues[1]))
                }
                Signature(name, params, cleanType(match.groupValues[3]), match.range.first + match.value.indexOf(name))
            }
            .toList()
    }

    fun typedSymbols(text: String): Map<String, String> {
        val out = linkedMapOf<String, String>()
        for (decl in declarations(text)) {
            if (decl.typeName.isNotBlank() && decl.kind != "field") {
                out.putIfAbsent(decl.name, decl.typeName)
            }
        }
        return out
    }

    fun fieldsByType(text: String): Map<String, List<Field>> {
        val out = linkedMapOf<String, MutableList<Field>>()
        val mask = codeMask(text)
        var inTypeSection = false
        var owner = ""
        var depth = 0
        var offset = 0
        for (line in text.lines()) {
            val lineStart = offset
            val section = Regex("""^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:""").find(line)
            if (section != null && owner.isBlank()) {
                inTypeSection = section.groupValues[1] == "type" || section.groupValues[1] == "struct"
            }
            if (owner.isBlank()) {
                val canonicalType = Regex("""^\s*([A-Za-z_][A-Za-z0-9_]*)\s*::\s*type\s*(?:\[[^\]]+])?\s*\{""").find(line)
                val legacyType = if (inTypeSection) Regex("""^\s*([A-Za-z_][A-Za-z0-9_]*)(?:\[[^\]]+])?\s*\{""").find(line) else null
                val type = canonicalType ?: legacyType
                if (type != null && isCode(mask, lineStart + type.range.first)) {
                    owner = type.groupValues[1]
                    out.putIfAbsent(owner, mutableListOf())
                    depth = line.count { it == '{' } - line.count { it == '}' }
                    val afterBrace = line.substringAfter("{", "")
                    collectFields(afterBrace, lineStart + line.indexOf('{') + 1, owner, out)
                }
            } else if (owner.isNotBlank()) {
                collectFields(line, lineStart, owner, out)
                depth += line.count { it == '{' } - line.count { it == '}' }
                if (depth <= 0) owner = ""
            }
            offset += line.length + 1
        }
        return out
    }

    fun calls(text: String): List<Call> {
        val mask = codeMask(text)
        val out = mutableListOf<Call>()
        var i = 0
        while (i < text.length) {
            if (text[i] != '(' || !isCode(mask, i)) {
                i += 1
                continue
            }
            var nameEnd = i
            while (nameEnd > 0 && text[nameEnd - 1].isWhitespace()) nameEnd -= 1
            var nameStart = nameEnd
            while (nameStart > 0 && isQualifiedNameChar(text[nameStart - 1])) nameStart -= 1
            val name = text.substring(nameStart, nameEnd).trim('.')
            if (name.isBlank() || name.substringAfterLast('.') in CALL_SKIP_NAMES) {
                i += 1
                continue
            }
            val close = findMatchingParen(text, mask, i)
            if (close < 0) {
                i += 1
                continue
            }
            val after = text.substring((close + 1).coerceAtMost(text.length)).trimStart()
            if (after.startsWith("->")) {
                i = close + 1
                continue
            }
            val raw = text.substring(i + 1, close)
            out.add(Call(name, nameStart, i, close, splitArgs(raw, i + 1)))
            i += 1
        }
        return out
    }

    fun splitArgs(raw: String, baseOffset: Int): List<Argument> {
        val out = mutableListOf<Argument>()
        var start = 0
        var depth = 0
        var quote: Char? = null
        var escaped = false
        fun push(end: Int) {
            val original = raw.substring(start, end)
            val leading = original.takeWhile { it.isWhitespace() }.length
            val trimmed = original.trim()
            if (trimmed.isNotBlank()) out.add(Argument(trimmed, baseOffset + start + leading))
        }
        for (index in raw.indices) {
            val ch = raw[index]
            if (quote != null) {
                if (escaped) escaped = false
                else if (ch == '\\') escaped = true
                else if (ch == quote) quote = null
                continue
            }
            when (ch) {
                '"', '\'' -> quote = ch
                '(', '[', '{', '<' -> depth += 1
                ')', ']', '}', '>' -> depth = (depth - 1).coerceAtLeast(0)
                ',' -> if (depth == 0) {
                    push(index)
                    start = index + 1
                }
            }
        }
        push(raw.length)
        return out
    }

    fun inferExprType(expr: String, symbols: Map<String, String>, signatures: Map<String, Signature>): String {
        val clean = expr.removeSuffix(";").trim()
        Regex("""^@?([A-Za-z_][A-Za-z0-9_]*)$""").find(clean)?.let {
            return symbols[it.groupValues[1]] ?: ""
        }
        Regex("""^([A-Za-z_][A-Za-z0-9_.]*)\s*\(""").find(clean)?.let {
            return signatures[it.groupValues[1].substringAfterLast('.')]?.returnType ?: ""
        }
        return when {
            clean == "true" || clean == "false" -> "bool"
            Regex("""^"(?:\\.|[^"\\])*"$""").matches(clean) -> "str"
            Regex("""^'(?:\\.|[^'\\])'$""").matches(clean) -> "char"
            Regex("""^(?:0x[0-9A-Fa-f]+|0b[01]+|[0-9]+)$""").matches(clean) -> "int"
            Regex("""^[0-9]+\.[0-9]+(?:f32|F32)?$""").matches(clean) -> "ft64"
            else -> ""
        }
    }

    fun baseAggregateType(typeName: String): String = cleanType(typeName).substringBefore("<").trim().let { base ->
        if (base in setOf("array", "list", "vector", "queue", "stack", "span", "slice", "dinArray", "dinarray")) elementAggregateType(typeName) else cleanType(typeName)
    }

    fun elementAggregateType(typeName: String): String {
        val clean = cleanType(typeName)
        Regex("""<\s*([A-Za-z_][A-Za-z0-9_]*)""").find(clean)?.let { return it.groupValues[1] }
        Regex("""array\s*<\s*([A-Za-z_][A-Za-z0-9_]*)""").find(clean)?.let { return it.groupValues[1] }
        return ""
    }

    fun cleanType(raw: String): String {
        return raw.trim()
            .removeSuffix("{")
            .removeSuffix(";")
            .substringBefore("=")
            .substringBefore(":")
            .trim()
    }

    private fun collectFields(source: String, baseOffset: Int, owner: String, out: MutableMap<String, MutableList<Field>>) {
        val cleanSource = source.replace(Regex("""\b(?:public|private|internal|protected)\s*:"""), " ")
        for (match in Regex("""(?:^|[;\s])@?([A-Za-z_][A-Za-z0-9_]*)\s*(?:>>|:)\s*([^;<-\{\}]+)""").findAll(cleanSource)) {
            val name = match.groupValues[1]
            if (name in SECTION_NAMES) continue
            out.getOrPut(owner) { mutableListOf() }
                .add(Field(owner, name, cleanType(match.groupValues[2]), baseOffset + match.range.first + match.value.indexOf(name)))
        }
    }

    private fun findMatchingParen(text: String, mask: BooleanArray, open: Int): Int {
        var depth = 0
        for (i in open until text.length) {
            if (!isCode(mask, i)) continue
            when (text[i]) {
                '(' -> depth += 1
                ')' -> {
                    depth -= 1
                    if (depth == 0) return i
                }
            }
        }
        return -1
    }

    private fun isInTypeSection(text: String, offset: Int): Boolean {
        val before = text.substring(0, offset.coerceIn(0, text.length)).lines().asReversed()
        for (line in before) {
            val match = Regex("""^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:""").find(line) ?: continue
            return match.groupValues[1] == "type" || match.groupValues[1] == "struct"
        }
        return false
    }

    private fun isCode(mask: BooleanArray, offset: Int): Boolean {
        return offset in mask.indices && mask[offset]
    }

    private fun isTokenChar(ch: Char): Boolean {
        return ch == '@' || ch == '_' || ch == '.' || ch.isLetterOrDigit()
    }

    private fun isQualifiedNameChar(ch: Char): Boolean {
        return ch == '_' || ch == '.' || ch.isLetterOrDigit()
    }

    private val CALL_SKIP_NAMES = setOf("if", "while", "for", "match", "guard", "unless", "return", "fn")

    private val SECTION_NAMES = setOf(
        "stc", "local", "const", "fn", "fa", "type", "struct", "enum", "binary", "bind",
        "raw", "bridge", "final", "routine", "verify", "report", "seal", "public",
        "private", "internal", "protected"
    )
}
