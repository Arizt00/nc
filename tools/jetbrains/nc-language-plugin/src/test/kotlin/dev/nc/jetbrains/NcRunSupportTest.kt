package dev.nc.jetbrains

import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.setPosixFilePermissions
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NcRunSupportTest {
    @Test
    fun machineSourcesAreEditableButNotRunnableFromTheIde() {
        assertTrue(NcRunSupport.isRunnablePath("main.n"))
        assertFalse(NcRunSupport.isRunnablePath("kernel.nm"))
    }

    @Test
    fun compatibilityProbeRequiresMapAndDebugAdapter() {
        val root = Files.createTempDirectory("nc-jetbrains-toolchain")
        try {
            val compatible = fakeCompiler(root, "compatible", includeMap = true, includeDebug = true)
            val stale = fakeCompiler(root, "stale", includeMap = false, includeDebug = true)
            val noDebug = fakeCompiler(root, "no-debug", includeMap = true, includeDebug = false)

            assertTrue(NcRunSupport.compatibilityProbeForTests(compatible.toString()))
            assertFalse(NcRunSupport.compatibilityProbeForTests(stale.toString()))
            assertFalse(NcRunSupport.compatibilityProbeForTests(noDebug.toString()))
        } finally {
            root.toFile().deleteRecursively()
        }
    }

    private fun fakeCompiler(root: Path, name: String, includeMap: Boolean, includeDebug: Boolean): Path {
        return if (System.getProperty("os.name").lowercase().contains("win")) {
            val script = root.resolve("$name.cmd")
            val mapEntry = if (includeMap) "{\"family\":\"memory\",\"item\":\"map\"}" else "{\"family\":\"memory\",\"item\":\"ptr\"}"
            val debugStatus = if (includeDebug) "closed_native_dap" else "unavailable"
            Files.writeString(
                script,
                """@echo off
if /I "%1"=="imports" (
  echo {"entries":[$mapEntry]}
  exit /b 0
)
if /I "%1"=="debug-adapter" (
  echo {"protocol":"Debug Adapter Protocol","status":"$debugStatus"}
  exit /b 0
)
exit /b 1
"""
            )
            script
        } else {
            val script = root.resolve(name)
            val mapEntry = if (includeMap) "{\"family\":\"memory\",\"item\":\"map\"}" else "{\"family\":\"memory\",\"item\":\"ptr\"}"
            val debugStatus = if (includeDebug) "closed_native_dap" else "unavailable"
            Files.writeString(
                script,
                """#!/usr/bin/env sh
if [ "${'$'}1" = "imports" ]; then
  printf '%s\n' '{"entries":[$mapEntry]}'
  exit 0
fi
if [ "${'$'}1" = "debug-adapter" ]; then
  printf '%s\n' '{"protocol":"Debug Adapter Protocol","status":"$debugStatus"}'
  exit 0
fi
exit 1
"""
            )
            script.setPosixFilePermissions(setOf(
                java.nio.file.attribute.PosixFilePermission.OWNER_READ,
                java.nio.file.attribute.PosixFilePermission.OWNER_WRITE,
                java.nio.file.attribute.PosixFilePermission.OWNER_EXECUTE
            ))
            script
        }
    }
}
