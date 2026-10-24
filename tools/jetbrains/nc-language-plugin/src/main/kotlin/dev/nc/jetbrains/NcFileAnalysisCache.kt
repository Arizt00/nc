package dev.nc.jetbrains

import com.intellij.psi.PsiFile
import com.intellij.psi.util.CachedValueProvider
import com.intellij.psi.util.CachedValuesManager

internal class NcFileAnalysisCache {

    fun get(file: PsiFile, currentText: String): NcFileAnalysis {
        /*
         * El documento del editor puede no estar comprometido todavía en PSI.
         * En ese caso se analiza el texto actual sin reutilizar una caché vieja.
         */
        if (file.text != currentText) {
            return buildAnalysis(currentText)
        }

        return CachedValuesManager.getCachedValue(file) {
            CachedValueProvider.Result.create(
                buildAnalysis(file.text),
                file
            )
        }
    }

    private fun buildAnalysis(text: String): NcFileAnalysis {
        val scopeMap = buildNcLexicalScopeMap(text)
        val visibilityMarkers = NC_VISIBILITY_REGEX.findAll(text).map { match ->
            val visibility = when (match.groupValues[1].lowercase()) {
                "public" -> NcVisibility.PUBLIC
                "private" -> NcVisibility.PRIVATE
                else -> NcVisibility.UNKNOWN
            }

            NcVisibilityMarker(
                visibility = visibility,
                offset = match.range.first,
                scopeId = scopeMap.scopeAt(match.range.first).id
            )
        }.toList()

        val declarations = NcSourceIndex.declarations(text).map { declaration ->
            val kind = classifyDeclaration(
                text = text,
                name = declaration.name,
                offset = declaration.offset
            )

            val lexicalScope = scopeMap.scopeAt(declaration.offset)
            val effectiveScope = if (kind == NcDeclarationKind.PARAMETER) {
                scopeMap.firstChildStartingAfter(declaration.offset)?.id ?: lexicalScope.id
            } else {
                lexicalScope.id
            }

            NcDeclarationSnapshot(
                name = declaration.name,
                offset = declaration.offset,
                kind = kind,
                visibility = visibilityAt(
                    declarationOffset = declaration.offset,
                    declarationScopeId = effectiveScope,
                    markers = visibilityMarkers,
                    scopeMap = scopeMap
                ),
                scopeId = effectiveScope
            )
        }

        val fields = NcSourceIndex.fieldsByType(text).mapValues { (_, values) ->
            values.map { field ->
                NcFieldSnapshot(
                    name = field.name,
                    typeName = field.typeName,
                    offset = field.offset
                )
            }
        }

        return NcFileAnalysis(
            imports = ncImportEntries(text),
            declarations = declarations,
            typedSymbols = NcSourceIndex.typedSymbols(text).toMap(),
            fieldsByType = fields,
            scopeMap = scopeMap
        )
    }

    private fun visibilityAt(
        declarationOffset: Int,
        declarationScopeId: Int,
        markers: List<NcVisibilityMarker>,
        scopeMap: NcLexicalScopeMap
    ): NcVisibility {
        return markers
            .asSequence()
            .filter { it.offset < declarationOffset }
            .filter { marker ->
                marker.scopeId == declarationScopeId ||
                    scopeMap.isAncestor(marker.scopeId, declarationScopeId)
            }
            .maxByOrNull(NcVisibilityMarker::offset)
            ?.visibility
            ?: NcVisibility.UNKNOWN
    }

    private fun classifyDeclaration(
        text: String,
        name: String,
        offset: Int
    ): NcDeclarationKind {
        val safeOffset = offset.coerceIn(0, text.length)
        val beforeStart = (safeOffset - 180).coerceAtLeast(0)
        val afterEnd = (safeOffset + name.length + 220).coerceAtMost(text.length)
        val before = text.substring(beforeStart, safeOffset)
        val after = text.substring((safeOffset + name.length).coerceAtMost(text.length), afterEnd)
        val lineStart = text.lastIndexOf('\n', (safeOffset - 1).coerceAtLeast(0))
            .let { if (it < 0) 0 else it + 1 }
        val lineEnd = text.indexOf('\n', safeOffset)
            .let { if (it < 0) text.length else it }
        val line = text.substring(lineStart, lineEnd)

        if (NC_TYPE_PREFIX_REGEX.containsMatchIn(before.takeLast(80))) {
            return NcDeclarationKind.TYPE
        }

        if (NC_CONSTANT_PREFIX_REGEX.containsMatchIn(before.takeLast(80))) {
            return NcDeclarationKind.CONSTANT
        }

        if (after.matchesAtStart(Regex("""\s*::"""))) {
            return NcDeclarationKind.CONTAINER
        }

        if (after.matchesAtStart(Regex("""\s*\("""))) {
            return NcDeclarationKind.FUNCTION
        }

        if (looksLikeFunctionParameter(text, safeOffset)) {
            return NcDeclarationKind.PARAMETER
        }

        if (Regex("""\b${Regex.escape(name)}\s*>>""").containsMatchIn(line)) {
            return NcDeclarationKind.VARIABLE
        }

        return NcDeclarationKind.UNKNOWN
    }

    private fun looksLikeFunctionParameter(text: String, offset: Int): Boolean {
        val leftParen = text.lastIndexOf('(', offset)
        if (leftParen < 0) return false

        val leftBrace = text.lastIndexOf('{', offset)
        if (leftBrace > leftParen) return false

        val rightParen = text.indexOf(')', offset)
        if (rightParen < 0) return false

        val rightBrace = text.indexOf('{', offset)
        if (rightBrace >= 0 && rightBrace < rightParen) return false

        val signatureTailEnd = (rightParen + 96).coerceAtMost(text.length)
        val signatureTail = text.substring(rightParen, signatureTailEnd)

        return "->" in signatureTail || "{" in signatureTail
    }

    private fun String.matchesAtStart(regex: Regex): Boolean {
        return regex.find(this)?.range?.first == 0
    }
}
