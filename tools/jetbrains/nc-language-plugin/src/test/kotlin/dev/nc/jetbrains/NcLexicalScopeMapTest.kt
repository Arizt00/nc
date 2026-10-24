package dev.nc.jetbrains

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class NcLexicalScopeMapTest {
    @Test
    fun ignoresBracesInsideStringsAndComments() {
        val text = """
            demo :: runtime {
                value >> str <- "{not a scope}";
                // {not a scope}
                /* {still not a scope} */
                fn:
                    run() -> int {
                        return 0;
                    }
            }
        """.trimIndent()

        val map = buildNcLexicalScopeMap(text)
        assertEquals(3, map.scopes.size)
    }

    @Test
    fun detectsAncestorRelationships() {
        val text = "root { first { second { value; } } }"
        val map = buildNcLexicalScopeMap(text)
        val root = map.scopeAt(0)
        val first = map.scopeAt(text.indexOf("first"))
        val second = map.scopeAt(text.indexOf("value"))

        assertTrue(map.isAncestor(root.id, second.id))
        assertTrue(map.isAncestor(first.id, second.id))
        assertFalse(map.isAncestor(second.id, first.id))
    }
}
