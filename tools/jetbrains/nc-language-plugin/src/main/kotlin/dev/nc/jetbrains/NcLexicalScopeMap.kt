package dev.nc.jetbrains

import java.util.ArrayDeque

private enum class NcLexicalScannerState {
    NORMAL,
    LINE_COMMENT,
    BLOCK_COMMENT,
    STRING,
    CHARACTER
}

internal data class NcLexicalScope(
    val id: Int,
    val parentId: Int?,
    val startOffset: Int,
    val endOffsetExclusive: Int,
    val depth: Int
) {
    fun contains(offset: Int): Boolean {
        return offset >= startOffset && offset < endOffsetExclusive
    }
}

internal class NcLexicalScopeMap(
    val scopes: List<NcLexicalScope>
) {
    private val byId = scopes.associateBy(NcLexicalScope::id)

    fun scopeAt(offset: Int): NcLexicalScope {
        return scopes
            .asSequence()
            .filter { it.contains(offset) }
            .maxByOrNull(NcLexicalScope::depth)
            ?: scopes.first()
    }

    fun scopeById(id: Int): NcLexicalScope? = byId[id]

    fun isAncestor(ancestorId: Int, childId: Int): Boolean {
        var current: NcLexicalScope? = byId[childId]

        while (current != null) {
            if (current.id == ancestorId) return true
            current = current.parentId?.let(byId::get)
        }

        return false
    }

    fun distanceFromAncestor(ancestorId: Int, childId: Int): Int? {
        var current: NcLexicalScope? = byId[childId]
        var distance = 0

        while (current != null) {
            if (current.id == ancestorId) return distance
            current = current.parentId?.let(byId::get)
            distance += 1
        }

        return null
    }

    fun firstChildStartingAfter(offset: Int, maximumDistance: Int = 2048): NcLexicalScope? {
        return scopes
            .asSequence()
            .filter { it.parentId != null }
            .filter { it.startOffset >= offset }
            .filter { it.startOffset - offset <= maximumDistance }
            .minByOrNull(NcLexicalScope::startOffset)
    }
}

internal fun buildNcLexicalScopeMap(text: String): NcLexicalScopeMap {
    data class MutableScope(
        val id: Int,
        val parentId: Int?,
        val startOffset: Int,
        var endOffsetExclusive: Int,
        val depth: Int
    )

    val mutableScopes = mutableListOf(
        MutableScope(
            id = 0,
            parentId = null,
            startOffset = 0,
            endOffsetExclusive = text.length.coerceAtLeast(1),
            depth = 0
        )
    )
    val stack = ArrayDeque<Int>()
    stack.addLast(0)

    var state = NcLexicalScannerState.NORMAL
    var blockCommentDepth = 0
    var escaped = false
    var index = 0

    while (index < text.length) {
        val current = text[index]
        val next = text.getOrNull(index + 1)

        when (state) {
            NcLexicalScannerState.NORMAL -> when {
                current == '/' && next == '/' -> {
                    state = NcLexicalScannerState.LINE_COMMENT
                    index += 2
                    continue
                }

                current == '/' && next == '*' -> {
                    state = NcLexicalScannerState.BLOCK_COMMENT
                    blockCommentDepth = 1
                    index += 2
                    continue
                }

                current == '"' -> {
                    state = NcLexicalScannerState.STRING
                    escaped = false
                }

                current == '\'' -> {
                    state = NcLexicalScannerState.CHARACTER
                    escaped = false
                }

                current == '{' -> {
                    val parentId = stack.last()
                    val parent = mutableScopes[parentId]
                    val scope = MutableScope(
                        id = mutableScopes.size,
                        parentId = parentId,
                        startOffset = index,
                        endOffsetExclusive = text.length.coerceAtLeast(index + 1),
                        depth = parent.depth + 1
                    )
                    mutableScopes += scope
                    stack.addLast(scope.id)
                }

                current == '}' && stack.size > 1 -> {
                    val closedId = stack.removeLast()
                    mutableScopes[closedId].endOffsetExclusive = index + 1
                }
            }

            NcLexicalScannerState.LINE_COMMENT -> {
                if (current == '\n') state = NcLexicalScannerState.NORMAL
            }

            NcLexicalScannerState.BLOCK_COMMENT -> when {
                current == '/' && next == '*' -> {
                    blockCommentDepth += 1
                    index += 2
                    continue
                }

                current == '*' && next == '/' -> {
                    blockCommentDepth -= 1
                    index += 2
                    if (blockCommentDepth == 0) state = NcLexicalScannerState.NORMAL
                    continue
                }
            }

            NcLexicalScannerState.STRING -> {
                when {
                    escaped -> escaped = false
                    current == '\\' -> escaped = true
                    current == '"' -> state = NcLexicalScannerState.NORMAL
                }
            }

            NcLexicalScannerState.CHARACTER -> {
                when {
                    escaped -> escaped = false
                    current == '\\' -> escaped = true
                    current == '\'' -> state = NcLexicalScannerState.NORMAL
                }
            }
        }

        index += 1
    }

    while (stack.size > 1) {
        mutableScopes[stack.removeLast()].endOffsetExclusive = text.length.coerceAtLeast(1)
    }

    val immutable = mutableScopes.map {
        NcLexicalScope(
            id = it.id,
            parentId = it.parentId,
            startOffset = it.startOffset,
            endOffsetExclusive = it.endOffsetExclusive,
            depth = it.depth
        )
    }

    return NcLexicalScopeMap(immutable)
}
