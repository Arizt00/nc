package dev.nc.jetbrains

import com.intellij.testFramework.fixtures.BasePlatformTestCase

class NcGotoDeclarationHandlerTest : BasePlatformTestCase() {
    fun testLocalDeclaration() {
        val file = myFixture.configureByText(
            "local.n",
            """
                LocalNavigation :: runtime {
                    fn:
                        public:
                            run() -> int {
                                value >> int <- 41;
                                return val<caret>ue;
                            }
                    fa:
                        return true;
                }
            """.trimIndent()
        )

        val offset = myFixture.editor.caretModel.offset
        val source = file.findElementAt(offset) ?: file.findElementAt(offset - 1)
        val targets = NcGotoDeclarationHandler().getGotoDeclarationTargets(
            source,
            offset,
            myFixture.editor
        )

        assertNotNull(targets)
        assertTrue(targets!!.isNotEmpty())
        assertEquals("value", targets.first().text)
    }

    fun testQualifiedProjectDeclaration() {
        myFixture.addFileToProject(
            "src/model.n",
            """
                ModelRuntime :: runtime {
                    fn:
                        public:
                            openDoor() -> bool {
                                return true;
                            }
                    fa:
                        return true;
                }
            """.trimIndent()
        )

        val file = myFixture.configureByText(
            "main.n",
            """
                use <project:model>;

                MainRuntime :: runtime {
                    fn:
                        public:
                            run() -> bool {
                                return model.open<caret>Door();
                            }
                    fa:
                        return true;
                }
            """.trimIndent()
        )

        val offset = myFixture.editor.caretModel.offset
        val source = file.findElementAt(offset) ?: file.findElementAt(offset - 1)
        val targets = NcGotoDeclarationHandler().getGotoDeclarationTargets(
            source,
            offset,
            myFixture.editor
        )

        assertNotNull(targets)
        assertTrue(targets!!.isNotEmpty())
        assertEquals("openDoor", targets.first().text)
    }
}
