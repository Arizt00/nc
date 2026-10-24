package dev.nc.jetbrains

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NcNavigationPatternsTest {
    @Test
    fun parsesBothProjectImportForms() {
        val text = """
            use <project:model, users>;
            use <project: audit, door>;
        """.trimIndent()

        assertEquals(
            listOf("model", "users", "audit", "door"),
            ncImportedModules(text)
        )
    }

    @Test
    fun identifiesSelectedSegmentInQualifiedPath() {
        val text = "model.UserRuntime.registerUser"
        val offset = text.indexOf("UserRuntime") + 2
        val path = ncDottedPathAt(text, offset)

        assertNotNull(path)
        assertEquals("UserRuntime", path?.selectedSegment?.value)
        assertTrue(path?.segments?.size == 3)
    }
}
