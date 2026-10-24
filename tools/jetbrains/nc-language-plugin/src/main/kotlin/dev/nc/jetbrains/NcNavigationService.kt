package dev.nc.jetbrains

import com.intellij.openapi.components.Service
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.project.Project
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiManager
import kotlin.math.abs

@Service(Service.Level.PROJECT)
class NcNavigationService(
    private val project: Project
) {
    private val analysisCache = NcFileAnalysisCache()
    private val moduleResolver = NcProjectModuleResolver(project)
    private val documentationResolver = NcDocumentationResolver(project)

    fun resolve(
        file: PsiFile,
        text: String,
        offset: Int,
        sourceText: String
    ): Array<PsiElement>? {
        if (text.isEmpty()) return null

        val safeOffset = offset.coerceIn(0, text.length - 1)
        val token = NcSourceIndex.tokenAt(text, safeOffset).ifBlank {
            NcIntellisenseCatalog.cleanToken(sourceText)
        }
        if (token.isBlank()) return null

        val tokenStart = NcSourceIndex.tokenStartAt(text, safeOffset)
            .takeIf { it >= 0 }
            ?: safeOffset
        val analysis = analysisCache.get(file, text)
        val context = NcResolutionContext(
            file = file,
            text = text,
            offset = safeOffset,
            tokenStart = tokenStart,
            token = token,
            analysis = analysis,
            psiManager = PsiManager.getInstance(project)
        )

        val resolvers: List<() -> List<PsiElement>> = listOf(
            { projectImportTargets(context) },
            { qualifiedProjectTargets(context) },
            { fieldTargets(context) },
            { localTargets(context) },
            { importedProjectTargets(context) },
            { listOfNotNull(documentationResolver.resolve(file, token)) }
        )

        for (resolver in resolvers) {
            ProgressManager.checkCanceled()
            val targets = resolver()
            if (targets.isNotEmpty()) {
                return normalizeTargets(targets)
            }
        }

        return null
    }

    private fun projectImportTargets(context: NcResolutionContext): List<PsiElement> {
        val import = context.analysis.imports.firstOrNull { entry ->
            entry.module == context.token && entry.range.containsOffset(context.tokenStart)
        } ?: return emptyList()

        return moduleResolver.resolve(context.file, import.module).mapNotNull { virtualFile ->
            context.psiManager.findFile(virtualFile)
        }
    }

    private fun qualifiedProjectTargets(context: NcResolutionContext): List<PsiElement> {
        val path = ncDottedPathAt(context.text, context.offset) ?: return emptyList()
        if (path.segments.size < 2) return emptyList()

        val moduleName = path.segments.first().value
        val moduleFiles = moduleResolver.resolve(context.file, moduleName)
        if (moduleFiles.isEmpty()) return emptyList()

        if (path.selectedSegment.value == moduleName) {
            return moduleFiles.mapNotNull(context.psiManager::findFile)
        }

        val targetName = path.selectedSegment.value
        val targets = mutableListOf<PsiElement>()

        for (virtualFile in moduleFiles) {
            ProgressManager.checkCanceled()
            val psiFile = context.psiManager.findFile(virtualFile) ?: continue
            val analysis = analysisCache.get(psiFile, psiFile.text)

            analysis.declarations
                .filter { it.name == targetName }
                .mapNotNull { navigationElementAt(psiFile, it.offset) }
                .forEach(targets::add)

            analysis.fieldsByType.values
                .flatten()
                .filter { it.name == targetName }
                .mapNotNull { navigationElementAt(psiFile, it.offset) }
                .forEach(targets::add)
        }

        return targets
    }

    private fun fieldTargets(context: NcResolutionContext): List<PsiElement> {
        if (context.tokenStart <= 0) return emptyList()

        val beforeToken = context.text.substring(0, context.tokenStart)
        if (!beforeToken.trimEnd().endsWith('.')) return emptyList()

        val receiverWindow = beforeToken.takeLast(2048)
        val receiver = NC_MEMBER_RECEIVER_REGEX
            .find(receiverWindow)
            ?.groupValues
            ?.getOrNull(1)
            ?.trim()
            ?: return emptyList()

        val receiverType = resolveReceiverType(context, receiver)
        if (receiverType.isBlank()) return emptyList()

        val targets = mutableListOf<PsiElement>()
        findFieldTarget(context.file, context.analysis, receiverType, context.token)?.let(targets::add)

        for (module in context.analysis.imports.map(NcImportEntry::module).distinct()) {
            for (virtualFile in moduleResolver.resolve(context.file, module)) {
                ProgressManager.checkCanceled()
                val importedPsi = context.psiManager.findFile(virtualFile) ?: continue
                val importedAnalysis = analysisCache.get(importedPsi, importedPsi.text)
                findFieldTarget(importedPsi, importedAnalysis, receiverType, context.token)?.let(targets::add)
            }
        }

        return targets
    }

    private fun resolveReceiverType(context: NcResolutionContext, receiver: String): String {
        val parts = receiver
            .replace(NC_WHITESPACE_REGEX, "")
            .removePrefix("@")
            .split('.')
            .filter(String::isNotBlank)
        if (parts.isEmpty()) return ""

        val first = parts.first()
        val firstName = first.substringBefore('[')
        var currentType = context.analysis.typedSymbols[firstName] ?: return ""

        if ('[' in first) {
            currentType = NcSourceIndex.elementAggregateType(currentType)
        }

        for (part in parts.drop(1)) {
            currentType = when {
                part.startsWith("get(") || '[' in part -> {
                    NcSourceIndex.elementAggregateType(currentType)
                }

                else -> lookupFieldType(context, currentType, part)
            }

            if (currentType.isBlank()) return ""
        }

        return normalizeAggregateType(currentType)
    }

    private fun lookupFieldType(
        context: NcResolutionContext,
        ownerType: String,
        fieldName: String
    ): String {
        val normalized = normalizeAggregateType(ownerType)
        context.analysis.fieldsByType[normalized]
            ?.firstOrNull { it.name == fieldName }
            ?.typeName
            ?.let { return it }

        for (module in context.analysis.imports.map(NcImportEntry::module).distinct()) {
            for (virtualFile in moduleResolver.resolve(context.file, module)) {
                ProgressManager.checkCanceled()
                val psi = context.psiManager.findFile(virtualFile) ?: continue
                val analysis = analysisCache.get(psi, psi.text)
                analysis.fieldsByType[normalized]
                    ?.firstOrNull { it.name == fieldName }
                    ?.typeName
                    ?.let { return it }
            }
        }

        return ""
    }

    private fun findFieldTarget(
        psiFile: PsiFile,
        analysis: NcFileAnalysis,
        receiverType: String,
        fieldName: String
    ): PsiElement? {
        val normalized = normalizeAggregateType(receiverType)
        val field = analysis.fieldsByType[normalized]
            ?.firstOrNull { it.name == fieldName }
            ?: return null

        return navigationElementAt(psiFile, field.offset)
    }

    private fun normalizeAggregateType(typeName: String): String {
        return NcSourceIndex.baseAggregateType(typeName).substringAfterLast('.')
    }

    private fun localTargets(context: NcResolutionContext): List<PsiElement> {
        val referenceScope = context.analysis.scopeMap.scopeAt(context.tokenStart)
        val candidates = context.analysis.declarations
            .asSequence()
            .filter { it.name == context.token }
            .filter { it.offset != context.tokenStart }
            .filter { declarationVisibleAt(it, context, referenceScope) }
            .map { declaration ->
                declaration to localDeclarationScore(declaration, context, referenceScope)
            }
            .toList()

        if (candidates.isEmpty()) return emptyList()

        val bestScore = candidates.minOf { it.second }
        return candidates
            .filter { it.second == bestScore }
            .mapNotNull { navigationElementAt(context.file, it.first.offset) }
    }

    private fun declarationVisibleAt(
        declaration: NcDeclarationSnapshot,
        context: NcResolutionContext,
        referenceScope: NcLexicalScope
    ): Boolean {
        val scopeMap = context.analysis.scopeMap
        if (!scopeMap.isAncestor(declaration.scopeId, referenceScope.id)) return false

        val declarationScope = scopeMap.scopeById(declaration.scopeId) ?: return false
        val localLike = declaration.kind == NcDeclarationKind.PARAMETER ||
            declaration.kind == NcDeclarationKind.VARIABLE ||
            declaration.kind == NcDeclarationKind.CONSTANT && declarationScope.depth > 0

        return !localLike || declaration.offset < context.tokenStart
    }

    private fun localDeclarationScore(
        declaration: NcDeclarationSnapshot,
        context: NcResolutionContext,
        referenceScope: NcLexicalScope
    ): Long {
        val scopeDistance = context.analysis.scopeMap
            .distanceFromAncestor(declaration.scopeId, referenceScope.id)
            ?: 10_000
        val forwardPenalty = if (declaration.offset > context.tokenStart) 1_000_000_000L else 0L
        val offsetDistance = abs(declaration.offset - context.tokenStart).toLong()

        return scopeDistance.toLong() * 10_000_000L + forwardPenalty + offsetDistance
    }

    private fun importedProjectTargets(context: NcResolutionContext): List<PsiElement> {
        val targets = mutableListOf<PsiElement>()

        for (module in context.analysis.imports.map(NcImportEntry::module).distinct()) {
            for (virtualFile in moduleResolver.resolve(context.file, module)) {
                ProgressManager.checkCanceled()
                val psiFile = context.psiManager.findFile(virtualFile) ?: continue
                val analysis = analysisCache.get(psiFile, psiFile.text)

                analysis.declarations
                    .asSequence()
                    .filter { it.name == context.token }
                    .filter { importedDeclarationIsVisible(it, analysis) }
                    .mapNotNull { navigationElementAt(psiFile, it.offset) }
                    .forEach(targets::add)
            }
        }

        return targets
    }

    private fun importedDeclarationIsVisible(
        declaration: NcDeclarationSnapshot,
        analysis: NcFileAnalysis
    ): Boolean {
        if (declaration.visibility == NcVisibility.PRIVATE) return false

        val scopeDepth = analysis.scopeMap.scopeById(declaration.scopeId)?.depth ?: Int.MAX_VALUE
        return declaration.kind !in setOf(
            NcDeclarationKind.PARAMETER,
            NcDeclarationKind.VARIABLE
        ) && scopeDepth <= 1
    }

    private fun navigationElementAt(file: PsiFile, offset: Int): PsiElement? {
        if (file.textLength == 0) return file
        val safeOffset = offset.coerceIn(0, file.textLength - 1)
        return file.findElementAt(safeOffset)?.navigationElement ?: file
    }

    private fun normalizeTargets(targets: List<PsiElement>): Array<PsiElement>? {
        val unique = targets
            .asSequence()
            .map { it.navigationElement }
            .filter(PsiElement::isValid)
            .distinctBy { element ->
                val path = element.containingFile?.virtualFile?.path.orEmpty()
                "$path:${element.textOffset}"
            }
            .toList()

        return unique.takeIf(List<PsiElement>::isNotEmpty)?.toTypedArray()
    }
}
