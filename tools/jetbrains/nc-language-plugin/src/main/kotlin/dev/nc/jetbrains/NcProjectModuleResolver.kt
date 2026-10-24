package dev.nc.jetbrains

import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.project.DumbService
import com.intellij.openapi.project.Project
import com.intellij.openapi.roots.ProjectRootManager
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.PsiFile
import com.intellij.psi.search.FilenameIndex
import com.intellij.psi.search.GlobalSearchScope

internal class NcProjectModuleResolver(
    private val project: Project
) {
    private val extensions = listOf("n", "nc", "nm", "ncp", "nb", "nbb")

    fun resolve(contextFile: PsiFile, moduleName: String): List<VirtualFile> {
        if (!NC_MODULE_NAME_REGEX.matches(moduleName)) return emptyList()

        val ranked = linkedMapOf<String, Pair<VirtualFile, Int>>()
        val preferredDirectories = preferredDirectories(contextFile)

        preferredDirectories.forEachIndexed { directoryRank, directory ->
            for (extension in extensions) {
                val candidate = directory.findChild("$moduleName.$extension")
                if (candidate != null && candidate.isValid && !candidate.isDirectory) {
                    addCandidate(ranked, candidate, directoryRank)
                }
            }
        }

        if (!DumbService.getInstance(project).isDumb) {
            val scope = GlobalSearchScope.projectScope(project)
            val fileIndex = ProjectRootManager.getInstance(project).fileIndex

            for (extension in extensions) {
                ProgressManager.checkCanceled()
                val fileName = "$moduleName.$extension"

                for (candidate in FilenameIndex.getVirtualFilesByName(fileName, scope)) {
                    ProgressManager.checkCanceled()
                    if (!candidate.isValid || candidate.isDirectory) continue
                    if (!fileIndex.isInContent(candidate) && !isBelowProjectBase(candidate)) continue

                    addCandidate(
                        ranked = ranked,
                        file = candidate,
                        rank = 100 + pathDepth(candidate)
                    )
                }
            }
        }

        if (ranked.isEmpty()) return emptyList()

        val bestRank = ranked.values.minOf { it.second }
        return ranked.values
            .filter { it.second == bestRank }
            .map { it.first }
            .distinctBy { it.path }
    }

    private fun preferredDirectories(contextFile: PsiFile): List<VirtualFile> {
        val directories = linkedSetOf<VirtualFile>()
        val current = contextFile.virtualFile?.parent
        val localFileSystem = LocalFileSystem.getInstance()

        current?.let(directories::add)
        current?.findChild("src")?.takeIf { it.isDirectory }?.let(directories::add)
        current?.parent?.let(directories::add)
        current?.parent?.findChild("src")?.takeIf { it.isDirectory }?.let(directories::add)

        for (contentRoot in ProjectRootManager.getInstance(project).contentRoots) {
            directories += contentRoot
            contentRoot.findChild("src")
                ?.takeIf { it.isDirectory }
                ?.let(directories::add)
        }

        project.basePath
            ?.replace('\\', '/')
            ?.let(localFileSystem::findFileByPath)
            ?.let { projectRoot ->
                directories += projectRoot
                projectRoot.findChild("src")
                    ?.takeIf { it.isDirectory }
                    ?.let(directories::add)
            }

        return directories.toList()
    }

    private fun addCandidate(
        ranked: MutableMap<String, Pair<VirtualFile, Int>>,
        file: VirtualFile,
        rank: Int
    ) {
        val existing = ranked[file.path]
        if (existing == null || rank < existing.second) {
            ranked[file.path] = file to rank
        }
    }

    private fun isBelowProjectBase(file: VirtualFile): Boolean {
        val base = project.basePath?.replace('\\', '/') ?: return false
        return file.path == base || file.path.startsWith("$base/")
    }

    private fun pathDepth(file: VirtualFile): Int {
        return file.path.count { it == '/' }
    }
}
