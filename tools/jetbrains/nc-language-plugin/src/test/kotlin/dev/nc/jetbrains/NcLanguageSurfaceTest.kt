package dev.nc.jetbrains

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class NcLanguageSurfaceTest {
    private fun lex(source: String): List<Pair<String, String>> {
        val lexer = NcSimpleLexer()
        val tokens = mutableListOf<Pair<String, String>>()
        lexer.start(source)
        while (lexer.tokenType != null) {
            tokens += source.substring(lexer.tokenStart, lexer.tokenEnd) to lexer.tokenType.toString()
            lexer.advance()
        }
        return tokens
    }

    @Test
    fun extensionsSelectIndependentVisualDomains() {
        assertEquals(NcVisualMode.Source, NcModeFileType.modeForExtension("n"))
        assertEquals(NcVisualMode.Basal, NcModeFileType.modeForExtension("nb"))
        assertEquals(NcVisualMode.Primitive, NcModeFileType.modeForExtension("nbb"))
        assertEquals(NcVisualMode.Machine, NcModeFileType.modeForExtension("nm"))
        assertEquals(NcVisualMode.BackendObject, NcModeFileType.modeForExtension("nbo"))
        assertEquals(NcVisualMode.Hardware, NcModeFileType.modeForExtension("ncp"))
    }

    @Test
    fun stateHasASeparateDomainToken() {
        assertTrue(lex("state: sealed;").contains("state" to "DOMAIN_STATE"))
    }

    @Test
    fun everyLowLevelExtensionUsesTheSystemLanguageServer() {
        for (extension in listOf("nb", "nbb", "nm", "ncp")) {
            assertTrue(NcLspModeRouter.isLowLevel("sample.$extension", ""))
            assertEquals("nc-sys-lsp", NcLspModeRouter.chooseBackend("sample.$extension", ""))
        }
    }

    @Test
    fun backendPreludeSymbolsNeverRequestSourceImports() {
        for (label in listOf("strLen", "alloc", "pRead")) {
            val row = assertNotNull(NcIntellisenseCatalog.lookup(label))
            assertTrue(NcIntellisenseCatalog.isBackendPrelude(row), label)
            assertEquals("", NcIntellisenseCatalog.preferredImport(row), label)
            assertTrue(NcIntellisenseCatalog.hasImport("", row), label)
        }
        val println = assertNotNull(NcIntellisenseCatalog.lookup("println"))
        assertTrue(!NcIntellisenseCatalog.isBackendPrelude(println))
        assertEquals("use <io:output>;", NcIntellisenseCatalog.preferredImport(println))
    }

	@Test
	fun lowLevelFilesAreManualWhileIsaRecognitionRemainsSemantic() {
		val machine = """
			demo :: machine {
			    target:
			        arch: x86_64;
			    symbol:
			        helper:
			    proc:
			        main:
			            instr:
			}
		""".trimIndent()
		for ((extension, text) in listOf(
			"nm" to machine,
			"nb" to "root :: basal {}",
			"nbb" to "primitive[@] target riscv32:",
			"ncp" to "primitive[@] target aarch64:",
		)) {
			assertTrue(assertNotNull(NcLowLevelCompletionCatalog.rows(extension, text, "")).isEmpty(), extension)
		}
		assertTrue(NcLowLevelCompletionCatalog.isMachineInstruction("serialize"))
		assertTrue(NcLowLevelCompletionCatalog.isMachineInstruction("cbo.clean"))
		assertTrue(NcLowLevelCompletionCatalog.isMachineInstruction("memory.copy"))
		for (instruction in listOf("endbr64", "wbnoinvd", "getsec", "rdpmc", "rdpid", "cld", "esb", "tsb", "dgh", "xaflag", "axflag", "wfet", "wfit", "dbg", "cpsid", "cpsie", "hvc", "smc", "hfence.vvma", "hfence.gvma", "hinval.vvma", "hinval.gvma", "hlv.w", "hlvx.hu", "hsv.w", "sfence.w.inval", "sfence.inval.ir", "sinval.vma", "memory.init", "data.drop", "table.get", "table.set", "table.copy", "table.init", "elem.drop", "ref.null", "ref.func")) {
			assertTrue(NcLowLevelCompletionCatalog.isMachineInstruction(instruction), instruction)
		}
		assertTrue(NcLowLevelCompletionCatalog.isMachineRegister("z3"))
	}

    @Test
    fun compactImportsRolesAndRetainedSectionsAreLexedDirectly() {
        val tokens = lex(
            """
            use <memory:region>;
            compact {
                const:
                    LIMIT >> int <- 1;
                stc:
                    value >> int <- 0;
                to main { value <- LIMIT; }
                fa:
                    return true;
            }
            """.trimIndent(),
        )
        assertTrue(tokens.contains("use" to "IMPORT_KEYWORD"))
        assertTrue(tokens.contains("<memory:region>" to "IMPORT_SELECTOR"))
        assertTrue(tokens.contains("compact" to "CONTAINER"))
        assertTrue(tokens.none { it.first == "program" })
        assertTrue(tokens.contains("stc" to "STC_SECTION"))
        assertTrue(tokens.contains("fa" to "FA_SECTION"))
        assertTrue(tokens.contains("to" to "MAIN_FAMILY"))
        assertTrue(tokens.contains("main" to "MAIN_FAMILY"))
    }

    @Test
    fun hybridSectionsRemainVisibleToNavigationAndTypeHints() {
        val source = """
            sample {
                type:
                    Packet { tag >> int; }
                stc:
                    packet >> Packet;
                local:
                    count >> int <- 0;
                fn:
                    public:
                        run(value >> int) -> bool { return value == count; }
                ref main ->> run;
                fa:
                    return true;
            }
        """.trimIndent()
        val declarations = NcSourceIndex.declarations(source)
        assertTrue(declarations.any { it.kind == "container" && it.name == "sample" })
        assertTrue(declarations.any { it.kind == "type" && it.name == "Packet" })
        assertTrue(declarations.any { it.kind == "variable" && it.name == "count" && it.typeName == "int" })
        assertTrue(declarations.any { it.kind == "field" && it.name == "tag" && it.owner == "Packet" })
        val run = assertNotNull(NcSourceIndex.signatures(source).find { it.name == "run" })
        assertEquals("int", run.params.single().typeName)
        assertEquals("bool", run.returnType)
    }

    @Test
    fun compactKernelDebugAndHardeningSymbolsNeedNoImports() {
        for (label in listOf(
            "dbgRegs", "dbgDump", "dbgTrace", "dbgSym", "rspSum", "rspFrame",
            "brkSet", "brkDel", "crashWrite", "kaslr", "canarySet", "canaryOk",
            "sanPoison", "sanClear", "sanOk"
        )) {
            val row = assertNotNull(NcIntellisenseCatalog.lookup(label), label)
            assertEquals("", NcIntellisenseCatalog.preferredImport(row), label)
            assertTrue(row.signature.startsWith("$label("), label)
        }
    }

    @Test
    fun compactKernelAddressAndEveryMmioWidthHaveIndependentTokens() {
        val tokens = lex(
            """
            kernelColors :: kernel {
                touch :: in() -> bool {
                    lapic at 0xFEE00000;
                    value = load32(lapic);
                    *8(lapic, value);
                    *16(lapic, value);
                    *32(lapic, value);
                    *64(lapic, value);
                    *128(lapic, value);
                    *256(lapic, value);
                    *512(lapic, value);
                    return true;
                }
                ref main ->> touch;
            }
            """.trimIndent()
        )
        assertTrue(tokens.contains("at" to "KERNEL_ADDRESS_KEYWORD"))
        for (width in listOf(8, 16, 32, 64, 128, 256, 512)) {
            assertTrue(
                tokens.contains("*$width" to "KERNEL_MMIO_$width"),
                "missing independent MMIO token for *$width: $tokens"
            )
        }
        assertTrue(tokens.contains("load32" to "KERNEL_MMIO_32"))

        val ordinary = lex("ordinary { fn: public: run() -> bool { value = at; product = value * 32; return true; } }")
        assertTrue(ordinary.none { it.second.startsWith("KERNEL_") })

        val hiddenMarkers = lex(
            """ordinary { fn: public: run() -> bool { text = "*32(p, 1)"; /* lapic at 1; */ return true; } }""",
        )
        assertTrue(hiddenMarkers.none { it.second.startsWith("KERNEL_") })

        val kernelNamedLibrary = lex(
            """kernelHelpers { fn: internal: probe() -> bool { value = at; return true; } }""",
        )
        assertTrue(kernelNamedLibrary.none { it.second.startsWith("KERNEL_") })
    }
}
