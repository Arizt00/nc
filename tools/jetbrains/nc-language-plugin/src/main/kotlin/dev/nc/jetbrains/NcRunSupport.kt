package dev.nc.jetbrains

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import java.io.File

object NcRunSupport {
    private val sourceExtensions = setOf("n", "nb", "nbb", "ncp")

    data class Toolchain(
        val nccPath: String,
        val source: String,
        val toolchainId: String?
    )

    private data class Candidate(val path: String, val source: String, val toolchainId: String?)
    private data class CompatibilityProbe(val stamp: String, val compatible: Boolean)
    private val compatibilityCache = mutableMapOf<String, CompatibilityProbe>()

    fun isRunnableFile(file: VirtualFile?): Boolean {
        if (file == null || file.isDirectory) return false
        return sourceExtensions.contains(file.extension?.lowercase())
    }

    fun isRunnablePath(path: String?): Boolean {
        if (path.isNullOrBlank()) return false
        val ext = path.substringAfterLast('.', "").lowercase()
        return sourceExtensions.contains(ext)
    }

    fun projectRoot(project: Project): File {
        val basePath = project.basePath
        if (!basePath.isNullOrBlank()) return File(basePath)
        return File(System.getProperty("user.dir"))
    }

    fun resolveToolchain(project: Project? = null): Toolchain {
        val candidates = mutableListOf<Candidate>()
        installedSuiteNccExecutable()?.let {
            candidates.add(Candidate(it, "installed:ncc-suite", "installed"))
        }
        try {
            val resolved = NcToolchainResolver.resolveNccBinary()
            val path = executablePathOrNull(resolved.path)
                ?: executablePathOrNull(resolved.bin)
            if (path != null) {
                candidates.add(Candidate(path, resolved.source ?: "ncup", resolved.toolchain_id))
            }
        } catch (_: Exception) {
            // The complete installed suite remains usable when no nominal ncup toolchain is selected.
        }
        resolvePathExecutable("ncc")?.let {
            candidates.add(Candidate(it, "PATH:ncc", "path"))
        }
        resolveLocalNccExecutable(project)?.let {
            candidates.add(Candidate(it, "workspace:ncc-kit", "workspace"))
        }

        for (candidate in candidates.distinctBy { File(it.path).absolutePath.lowercase() }) {
            if (isCompatibleNcc(candidate.path)) {
                return Toolchain(candidate.path, candidate.source, candidate.toolchainId)
            }
        }
        throw IllegalStateException(
            "No compatible ncc was found. Run ncup; the selected compiler must provide stoud::memory::map and debug-adapter."
        )
    }

    fun stableName(file: VirtualFile): String = "nC Run ${file.name}"

    private fun executablePathOrNull(path: String?): String? {
        if (path.isNullOrBlank()) return null
        val file = File(path)
        return if (file.isFile) file.absolutePath else null
    }

    private fun installedSuiteNccExecutable(): String? {
        val candidates = if (isWindows()) {
            listOfNotNull(
                System.getenv("LOCALAPPDATA")?.let { File(it, "nC/bin/ncc.exe") },
                System.getenv("USERPROFILE")?.let { File(it, "AppData/Local/nC/bin/ncc.exe") }
            )
        } else {
            val home = System.getProperty("user.home")
            listOf(
                File(home, ".cargo/bin/ncc"),
                File(home, ".local/bin/ncc"),
                File("/usr/local/bin/ncc")
            )
        }
        return candidates.firstOrNull { it.isFile }?.absolutePath
    }

    private fun isCompatibleNcc(path: String): Boolean {
        val file = File(path)
        if (!file.isFile) return false
        val stamp = "${file.length()}:${file.lastModified()}"
        compatibilityCache[path]?.let { cached ->
            if (cached.stamp == stamp) return cached.compatible
        }

        val imports = runProbe(path, listOf("imports", "manifest", "--json"))
        val hasMap = imports != null &&
            Regex("""\"family\"\s*:\s*\"memory\"""").containsMatchIn(imports) &&
            Regex("""\"item\"\s*:\s*\"map\"""").containsMatchIn(imports)
        val debug = runProbe(path, listOf("debug-adapter", "--probe", "--json"))
        val hasDebugAdapter = debug != null &&
            Regex("""\"protocol\"\s*:\s*\"Debug Adapter Protocol\"""").containsMatchIn(debug) &&
            Regex("""\"status\"\s*:\s*\"closed[^\"]*\"""").containsMatchIn(debug)
        val compatible = hasMap && hasDebugAdapter
        compatibilityCache[path] = CompatibilityProbe(stamp, compatible)
        return compatible
    }

    internal fun compatibilityProbeForTests(path: String): Boolean = isCompatibleNcc(path)

    private fun runProbe(path: String, args: List<String>): String? {
        return try {
            val command = if (isWindows() && (path.endsWith(".cmd", true) || path.endsWith(".bat", true))) {
                listOf("cmd.exe", "/d", "/c", "call", path) + args
            } else {
                listOf(path) + args
            }
            val process = ProcessBuilder(command)
                .redirectErrorStream(true)
                .start()
            val output = process.inputStream.bufferedReader().use { it.readText() }
            if (process.waitFor() == 0) output else null
        } catch (_: Exception) {
            null
        }
    }

    private fun resolveLocalNccExecutable(project: Project?): String? {
        val roots = mutableListOf<File>()
        project?.let { roots.add(projectRoot(it)) }
        roots.add(File(System.getProperty("user.dir")))

        val localNames = if (isWindows()) {
            listOf("ncc.exe", ".ncc/bin/ncc.exe", ".ncc\\bin\\ncc.exe", "ncc.cmd")
        } else {
            listOf("ncc", ".ncc/bin/ncc")
        }

        for (root in roots.distinctBy { it.absolutePath }) {
            for (name in localNames) {
                val candidate = File(root, name)
                if (candidate.isFile) return candidate.absolutePath
            }
        }

        return resolvePathExecutable("ncc")
    }

    private fun resolvePathExecutable(name: String): String? {
        val path = System.getenv("PATH") ?: return null
        val extensions = if (isWindows()) {
            val raw = System.getenv("PATHEXT")
                ?.split(';')
                ?.filter { it.isNotBlank() }
                ?: listOf(".EXE", ".CMD", ".BAT")
            listOf("") + raw
        } else {
            listOf("")
        }

        for (dir in path.split(File.pathSeparatorChar).filter { it.isNotBlank() }) {
            for (ext in extensions) {
                val candidate = File(dir, if (name.endsWith(ext, ignoreCase = true)) name else name + ext.lowercase())
                if (candidate.isFile) return candidate.absolutePath
            }
        }
        return null
    }

    private fun isWindows(): Boolean =
        System.getProperty("os.name").lowercase().contains("win")
}
