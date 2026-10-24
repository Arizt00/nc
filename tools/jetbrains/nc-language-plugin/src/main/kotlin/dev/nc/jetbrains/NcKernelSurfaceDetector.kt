package dev.nc.jetbrains

object NcKernelSurfaceDetector {
    private val physicalBinding = Regex(
        """(?:^|[;{}])\s*[A-Za-z_][A-Za-z0-9_]*\s+at\s+""",
        RegexOption.MULTILINE,
    )
    private val mmioWrite = Regex("""\*\s*(?:8|16|32|64|128|256|512)\s*\(""")
    private val namedKernelContainer = Regex(
        """^\s*[A-Za-z_][A-Za-z0-9_]*\s*::\s*(?:kernel|system)\s*\{""",
        setOf(RegexOption.IGNORE_CASE, RegexOption.MULTILINE),
    )
    private val directMain = Regex("""\bref\s+main\s*->>""", RegexOption.IGNORE_CASE)

    fun detect(source: CharSequence): Boolean {
        val code = maskNonCode(source)
        return code.contains("[@kernel]", ignoreCase = true) ||
            code.contains("[@system]", ignoreCase = true) ||
            code.contains("@system", ignoreCase = true) ||
            physicalBinding.containsMatchIn(code) ||
            mmioWrite.containsMatchIn(code) ||
            (namedKernelContainer.containsMatchIn(code) && directMain.containsMatchIn(code))
    }

    private fun maskNonCode(source: CharSequence): String {
        val out = StringBuilder(source.length)
        var index = 0
        var lineComment = false
        var blockComment = false
        var quote: Char? = null

        while (index < source.length) {
            val current = source[index]
            val next = source.getOrNull(index + 1)

            if (lineComment) {
                if (current == '\n') {
                    lineComment = false
                    out.append('\n')
                } else {
                    out.append(' ')
                }
                index += 1
                continue
            }
            if (blockComment) {
                if (current == '*' && next == '/') {
                    out.append("  ")
                    index += 2
                    blockComment = false
                } else {
                    out.append(if (current == '\n') '\n' else ' ')
                    index += 1
                }
                continue
            }
            if (quote != null) {
                if (current == '\\' && next != null) {
                    out.append("  ")
                    index += 2
                } else {
                    out.append(if (current == '\n') '\n' else ' ')
                    index += 1
                    if (current == quote) quote = null
                }
                continue
            }
            if (current == '/' && next == '/') {
                out.append("  ")
                index += 2
                lineComment = true
                continue
            }
            if (current == '/' && next == '*') {
                out.append("  ")
                index += 2
                blockComment = true
                continue
            }
            if (current == '"' || current == '\'') {
                quote = current
                out.append(' ')
                index += 1
                continue
            }
            out.append(current)
            index += 1
        }
        return out.toString()
    }
}
