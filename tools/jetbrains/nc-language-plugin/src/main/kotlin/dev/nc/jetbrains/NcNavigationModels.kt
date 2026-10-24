package dev.nc.jetbrains

import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiManager

internal enum class NcDeclarationKind {
    CONTAINER,
    TYPE,
    FUNCTION,
    PARAMETER,
    VARIABLE,
    CONSTANT,
    UNKNOWN
}

internal enum class NcVisibility {
    PUBLIC,
    PRIVATE,
    UNKNOWN
}

internal data class NcImportEntry(
    val module: String,
    val range: TextRange
)

internal data class NcVisibilityMarker(
    val visibility: NcVisibility,
    val offset: Int,
    val scopeId: Int
)

internal data class NcDeclarationSnapshot(
    val name: String,
    val offset: Int,
    val kind: NcDeclarationKind,
    val visibility: NcVisibility,
    val scopeId: Int
)

internal data class NcFieldSnapshot(
    val name: String,
    val typeName: String,
    val offset: Int
)

internal data class NcPathSegment(
    val value: String,
    val range: TextRange
)

internal data class NcDottedPath(
    val value: String,
    val range: TextRange,
    val segments: List<NcPathSegment>,
    val selectedSegment: NcPathSegment
)

internal data class NcFileAnalysis(
    val imports: List<NcImportEntry>,
    val declarations: List<NcDeclarationSnapshot>,
    val typedSymbols: Map<String, String>,
    val fieldsByType: Map<String, List<NcFieldSnapshot>>,
    val scopeMap: NcLexicalScopeMap
)

internal data class NcResolutionContext(
    val file: PsiFile,
    val text: String,
    val offset: Int,
    val tokenStart: Int,
    val token: String,
    val analysis: NcFileAnalysis,
    val psiManager: PsiManager
)
