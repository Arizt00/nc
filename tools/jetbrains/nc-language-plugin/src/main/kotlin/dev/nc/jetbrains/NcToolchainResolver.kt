package dev.nc.jetbrains

import com.fasterxml.jackson.databind.ObjectMapper
import java.io.BufferedReader
import java.io.InputStreamReader

object NcToolchainResolver {
    data class WhichResult(
        val schema: String? = null,
        val version: String? = null,
        val state: String? = null,
        val bin: String? = null,
        val path: String? = null,
        val toolchain_id: String? = null,
        val source: String? = null
    )

    private val mapper = ObjectMapper()

    fun resolveNccBinary(ncupCmd: String = "ncup"): WhichResult {
        val command = if (System.getProperty("os.name").lowercase().contains("win")) {
            listOf("cmd.exe", "/d", "/c", "call", ncupCmd, "which", "ncc", "--json")
        } else {
            listOf(ncupCmd, "which", "ncc", "--json")
        }
        val process = ProcessBuilder(command)
            .redirectErrorStream(true)
            .start()
        val output = BufferedReader(InputStreamReader(process.inputStream)).readText()
        val code = process.waitFor()
        if (code != 0) {
            throw IllegalStateException("ncup which ncc failed: $output")
        }
        return mapper.readValue(output, WhichResult::class.java)
    }
}
