package dev.nc.jetbrains

import com.intellij.openapi.util.TextRange

internal val NC_PROJECT_IMPORT_REGEX = Regex(
    """use\s*<\s*project\s*:\s*([^>]*)>""",
    RegexOption.IGNORE_CASE
)

internal val NC_MEMBER_RECEIVER_REGEX = Regex(
    """@?([A-Za-z_][A-Za-z0-9_]*(?:\s*(?:\.\s*[A-Za-z_][A-Za-z0-9_]*|\[[^\]]*]|\.\s*get\s*\([^)]*\)))*?)\s*\.\s*$"""
)

internal val NC_DOTTED_PATH_REGEX = Regex(
    """[A-Za-z_][A-Za-z0-9_]*(?:\s*\.\s*[A-Za-z_][A-Za-z0-9_]*)+"""
)

internal val NC_IDENTIFIER_REGEX = Regex("[A-Za-z_][A-Za-z0-9_]*")
internal val NC_MODULE_NAME_REGEX = Regex("[A-Za-z_][A-Za-z0-9_]*")
internal val NC_VISIBILITY_REGEX = Regex("(?m)^\\s*(public|private)\\s*:", RegexOption.IGNORE_CASE)
internal val NC_TYPE_PREFIX_REGEX = Regex("""\b(struct|enum|union|trait|interface|type)\s*$""", RegexOption.IGNORE_CASE)
internal val NC_CONSTANT_PREFIX_REGEX = Regex("""\b(const|constant)\s*$""", RegexOption.IGNORE_CASE)
internal val NC_WHITESPACE_REGEX = Regex("\\s+")

internal fun ncModulesFromImportMatch(match: MatchResult): List<String> {
    val content = match.groups[1]?.value ?: return emptyList()

    return content
        .splitToSequence(',')
        .map(String::trim)
        .filter(String::isNotBlank)
        .distinct()
        .toList()
}

internal fun ncImportedModules(text: String): List<String> {
    return NC_PROJECT_IMPORT_REGEX
        .findAll(text)
        .flatMap { ncModulesFromImportMatch(it).asSequence() }
        .distinct()
        .toList()
}

internal fun ncImportEntries(text: String): List<NcImportEntry> {
    val entries = mutableListOf<NcImportEntry>()

    for (match in NC_PROJECT_IMPORT_REGEX.findAll(text)) {
        for (module in ncModulesFromImportMatch(match)) {
            val moduleMatch = NC_IDENTIFIER_REGEX
                .findAll(match.value)
                .firstOrNull { it.value == module }

            val start = if (moduleMatch == null) {
                match.range.first
            } else {
                match.range.first + moduleMatch.range.first
            }

            entries += NcImportEntry(
                module = module,
                range = TextRange(start, start + module.length)
            )
        }
    }

    return entries
}

internal fun ncDottedPathAt(text: String, offset: Int): NcDottedPath? {
    if (text.isEmpty()) return null

    val safeOffset = offset.coerceIn(0, text.length - 1)
    val probeOffsets = if (safeOffset > 0) intArrayOf(safeOffset, safeOffset - 1) else intArrayOf(safeOffset)

    val lineStart = text.lastIndexOf('\n', (safeOffset - 1).coerceAtLeast(0))
        .let { if (it < 0) 0 else it + 1 }
    val lineEnd = text.indexOf('\n', safeOffset)
        .let { if (it < 0) text.length else it }
    val line = text.substring(lineStart, lineEnd)

    for (match in NC_DOTTED_PATH_REGEX.findAll(line)) {
        val absoluteRange = TextRange(
            lineStart + match.range.first,
            lineStart + match.range.last + 1
        )

        if (probeOffsets.none { absoluteRange.containsOffset(it) }) continue

        val segments = NC_IDENTIFIER_REGEX.findAll(match.value).map { segment ->
            val start = absoluteRange.startOffset + segment.range.first
            NcPathSegment(
                value = segment.value,
                range = TextRange(start, start + segment.value.length)
            )
        }.toList()

        val selected = segments.firstOrNull { segment ->
            probeOffsets.any(segment.range::containsOffset)
        } ?: continue

        return NcDottedPath(
            value = match.value.replace(NC_WHITESPACE_REGEX, ""),
            range = absoluteRange,
            segments = segments,
            selectedSegment = selected
        )
    }

    return null
}
