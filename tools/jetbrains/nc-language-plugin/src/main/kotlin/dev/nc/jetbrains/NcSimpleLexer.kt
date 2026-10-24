package dev.nc.jetbrains

import com.intellij.lexer.LexerBase
import com.intellij.psi.tree.IElementType

class NcSimpleLexer : LexerBase() {
    private var buffer: CharSequence = ""
    private var startOffset: Int = 0
    private var endOffset: Int = 0
    private var state: Int = 0
    private var tokenStart: Int = 0
    private var tokenEnd: Int = 0
    private var tokenType: IElementType? = null
    private var compactKernelSurface: Boolean = false

    override fun start(buffer: CharSequence, startOffset: Int, endOffset: Int, initialState: Int) {
        this.buffer = buffer
        this.startOffset = startOffset
        this.endOffset = endOffset
        this.state = initialState
        this.tokenStart = startOffset
        this.tokenEnd = startOffset
        this.tokenType = null
        this.compactKernelSurface = NcKernelSurfaceDetector.detect(buffer)
        advance()
    }

    override fun getState(): Int = state

    override fun getTokenType(): IElementType? = tokenType

    override fun getTokenStart(): Int = tokenStart

    override fun getTokenEnd(): Int = tokenEnd

    override fun getBufferSequence(): CharSequence = buffer

    override fun getBufferEnd(): Int = endOffset

    override fun advance() {
        if (tokenEnd >= endOffset) {
            tokenStart = endOffset
            tokenEnd = endOffset
            tokenType = null
            return
        }

        tokenStart = tokenEnd
        val c = buffer[tokenStart]

        if (c.isWhitespace()) {
            var i = tokenStart + 1
            while (i < endOffset && buffer[i].isWhitespace()) i += 1
            tokenEnd = i
            tokenType = NcTokenTypes.WHITE_SPACE
            return
        }

        if (c == '/' && tokenStart + 1 < endOffset && buffer[tokenStart + 1] == '/') {
            var i = tokenStart + 2
            while (i < endOffset && buffer[i] != '\n') i += 1
            tokenEnd = i
            tokenType = NcTokenTypes.COMMENT
            return
        }

        if (c == '/' && tokenStart + 1 < endOffset && buffer[tokenStart + 1] == '*') {
            var i = tokenStart + 2
            while (i + 1 < endOffset && !(buffer[i] == '*' && buffer[i + 1] == '/')) i += 1
            tokenEnd = if (i + 1 < endOffset) i + 2 else endOffset
            tokenType = NcTokenTypes.COMMENT
            return
        }

        if (c == '"') {
            var i = tokenStart + 1
            while (i < endOffset) {
                val ch = buffer[i]
                if (ch == '\\') {
                    i += 2
                    continue
                }
                if (ch == '"') {
                    i += 1
                    break
                }
                i += 1
            }
            tokenEnd = minOf(i, endOffset)
            tokenType = NcTokenTypes.STRING
            return
        }

        if (c == '\'') {
            var i = tokenStart + 1
            while (i < endOffset) {
                val ch = buffer[i]
                if (ch == '\\') {
                    i += 2
                    continue
                }
                if (ch == '\'') {
                    i += 1
                    break
                }
                i += 1
            }
            tokenEnd = minOf(i, endOffset)
            tokenType = NcTokenTypes.CHAR
            return
        }

        if (c == '[' && tokenStart + 1 < endOffset && buffer[tokenStart + 1] == '@') {
            var i = tokenStart + 2
            while (i < endOffset && buffer[i] != ']') i += 1
            if (i < endOffset) i += 1
            tokenEnd = i
            tokenType = classifyMode(buffer.subSequence(tokenStart, tokenEnd).toString())
            return
        }

        if (c == '@' && tokenStart + 1 < endOffset && buffer[tokenStart + 1] == '(') {
            val groupEnd = lexAtGroupReference()
            if (groupEnd != null) {
                tokenEnd = groupEnd
                tokenType = NcTokenTypes.AT_GROUP_REFERENCE
                return
            }
        }

        if (compactKernelSurface && c == '*') {
            val mmio = lexStarredKernelMmioWidth()
            if (mmio != null) {
                tokenEnd = mmio.second
                tokenType = mmio.first
                return
            }
        }

        if (c.isDigit()) {
            tokenType = lexNumber()
            if (compactKernelSurface && nextNonWhitespace(tokenEnd) == '(') {
                val width = buffer.subSequence(tokenStart, tokenEnd).toString()
                tokenType = kernelMmioWidthToken(width) ?: tokenType
            }
            return
        }

        if (c == '<') {
            if (previousWord(tokenStart)?.equals("use", ignoreCase = true) == true) {
                var i = tokenStart + 1
                while (i < endOffset && buffer[i] != '>') i += 1
                tokenEnd = if (i < endOffset) i + 1 else endOffset
                tokenType = NcTokenTypes.IMPORT_SELECTOR
                return
            }
            val genericEnd = lexGenericType()
            if (genericEnd != null) {
                tokenEnd = genericEnd
                tokenType = NcTokenTypes.GENERIC_TYPE
                return
            }
        }

        if (isWordStart(c)) {
            var i = tokenStart + 1
            while (i < endOffset && isWordChar(buffer[i])) i += 1
            tokenEnd = i
            val word = buffer.subSequence(tokenStart, tokenEnd).toString()
            tokenType = classifyWord(word, nextNonWhitespace(tokenEnd))
            return
        }

        val op = matchOperator(tokenStart)
        if (op != null) {
            tokenEnd = tokenStart + op.length
            tokenType = when (op) {
                ">>" -> NcTokenTypes.PROJECTION_OPERATOR
                "!>" -> NcTokenTypes.VERIFICATION_OPERATOR
                "<-" -> NcTokenTypes.MATERIAL_WRITE_OPERATOR
                ":=" -> NcTokenTypes.REMOVED_OPERATOR
                "{" -> NcTokenTypes.LBRACE
                "}" -> NcTokenTypes.RBRACE
                "(" -> NcTokenTypes.LPAREN
                ")" -> NcTokenTypes.RPAREN
                "[" -> NcTokenTypes.LBRACKET
                "]" -> NcTokenTypes.RBRACKET
                else -> NcTokenTypes.OPERATOR
            }
            return
        }

        tokenEnd = tokenStart + 1
        tokenType = NcTokenTypes.BAD_CHARACTER
    }

    private fun lexNumber(): IElementType {
        var i = tokenStart
        if (buffer[i] == '0' && i + 1 < endOffset) {
            val next = buffer[i + 1]
            if (next == 'x' || next == 'X') {
                i += 2
                while (i < endOffset && buffer[i].isHexDigit()) i += 1
                tokenEnd = i
                return NcTokenTypes.HEX_NUMBER
            }
            if (next == 'b' || next == 'B') {
                i += 2
                while (i < endOffset && (buffer[i] == '0' || buffer[i] == '1')) i += 1
                tokenEnd = i
                return NcTokenTypes.BINARY_NUMBER
            }
        }
        i += 1
        while (i < endOffset && buffer[i].isDigit()) i += 1
        if (i < endOffset && buffer[i] == '.') {
            var j = i + 1
            while (j < endOffset && buffer[j].isDigit()) j += 1
            i = j
        }
        tokenEnd = i
        return NcTokenTypes.NUMBER
    }

    private fun lexAtGroupReference(): Int? {
        var i = tokenStart + 2
        while (i < endOffset && buffer[i].isWhitespace()) i += 1
        if (i >= endOffset || !(buffer[i] == '_' || buffer[i].isLetter())) return null
        i += 1
        while (i < endOffset && (buffer[i] == '_' || buffer[i] == '.' || buffer[i].isLetterOrDigit())) i += 1
        while (i < endOffset && buffer[i].isWhitespace()) i += 1
        return if (i < endOffset && buffer[i] == ')') i + 1 else null
    }

    private fun lexGenericType(): Int? {
        var i = tokenStart + 1
        var sawWord = false
        while (i < endOffset) {
            val ch = buffer[i]
            when {
                ch == '>' -> return if (sawWord) i + 1 else null
                ch.isWhitespace() || ch == ',' -> i += 1
                ch == '_' || ch.isLetterOrDigit() -> {
                    sawWord = true
                    i += 1
                }
                else -> return null
            }
        }
        return null
    }

    private fun classifyMode(marker: String): IElementType {
        return when (marker) {
            "[@runtime]" -> NcTokenTypes.MODE_RUNTIME
            "[@compiler]" -> NcTokenTypes.MODE_COMPILER
            "[@kernel]" -> NcTokenTypes.MODE_KERNEL
            "[@machine]" -> NcTokenTypes.MODE_MACHINE
            "[@]" -> NcTokenTypes.MODE_BASAL
            else -> NcTokenTypes.MODE_OTHER
        }
    }

    private fun classifyWord(word: String, nextNonWs: Char?): IElementType {
        if (word.startsWith("@")) return NcTokenTypes.AT_REFERENCE
        val cleanWord = word.trimEnd('@')
        val lower = cleanWord.lowercase()
        roleToken(lower)?.let { return it }
        if (compactKernelSurface && lower == "at") return NcTokenTypes.KERNEL_ADDRESS_KEYWORD
        if (compactKernelSurface && nextNonWs == '(') {
            val width = lower.removePrefix("load").takeIf { lower.startsWith("load") }
                ?: lower.removePrefix("store").takeIf { lower.startsWith("store") }
            if (width != null) {
                val widthToken = kernelMmioWidthToken(width)
                if (widthToken != null) return widthToken
            }
        }
        if (word == "primitive[@]") return NcTokenTypes.MODE_PRIMITIVE
        if (cleanWord.startsWith("nC.runtime.")) return NcTokenTypes.RUNTIME_FAMILY
        if (cleanWord.startsWith("reg.")) {
            val reg = cleanWord.substringAfter("reg.")
            if (NcLowLevelCompletionCatalog.isMachineRegister(reg)) {
                return NcTokenTypes.MACHINE_REGISTER
            }
        }
        if (NcLowLevelCompletionCatalog.isMachineRegister(lower)) return NcTokenTypes.MACHINE_REGISTER

        if (isMainFamilyWord(cleanWord)) return NcTokenTypes.MAIN_FAMILY
        if (isContainerNameAtCurrentWord(cleanWord)) return NcTokenTypes.CONTAINER
        if (lower == "state") return NcTokenTypes.DOMAIN_STATE
        if (nextNonWs == ':' && SECTIONS.contains(lower)) {
            return when (lower) {
                "stc" -> NcTokenTypes.STC_SECTION
                "fn" -> NcTokenTypes.FN_SECTION
                "fa" -> NcTokenTypes.FA_SECTION
                else -> NcTokenTypes.SECTION
            }
        }
        if (lower == "use" || lower == "import") return NcTokenTypes.IMPORT_KEYWORD
        if (lower == "return") return NcTokenTypes.RETURN_KEYWORD
        if (lower == "dev" && nextNonWs == '(') return NcTokenTypes.DEV_CALL
        if (lower == "true" || lower == "false" || lower == "bool") return NcTokenTypes.BOOL
        if (PRINT_FUNCTIONS.contains(cleanWord) && nextNonWs == '(') return NcTokenTypes.PRINT_FUNCTION
        if (COLOR_FUNCTIONS.contains(cleanWord) && nextNonWs == '(') return NcTokenTypes.COLOR_FUNCTION
        if (POINTER_FUNCTIONS.contains(cleanWord) && (nextNonWs == '(' || nextNonWs == '<')) return NcTokenTypes.POINTER_FUNCTION
        if (CONTROL_FLOW_KEYWORDS.contains(lower)) return NcTokenTypes.CONTROL_FLOW
        if (LOOP_CONTROL_KEYWORDS.contains(lower)) return NcTokenTypes.LOOP_CONTROL
        if (VISIBILITY_KEYWORDS.contains(lower)) return NcTokenTypes.VISIBILITY
        if (KEYWORDS_SET.contains(lower)) return NcTokenTypes.KEYWORD
        if (lower == "memregion") return NcTokenTypes.MEM_REGION_TYPE
        if (POINTER_TYPES.contains(lower)) return NcTokenTypes.POINTER_TYPE
        if (RAW_HAZARDS.contains(lower)) return NcTokenTypes.RAW_HAZARD
        if (ARRAY_TYPES.contains(cleanWord) || ARRAY_TYPES.contains(lower)) return NcTokenTypes.ARRAY_TYPE
        if (ADVANCED_TYPES.contains(cleanWord) || ADVANCED_TYPES.contains(lower)) return NcTokenTypes.ADVANCED_TYPE
        if (BASIC_TYPES.contains(cleanWord) || BASIC_TYPES.contains(lower)) return NcTokenTypes.BASIC_TYPE
        if (TYPES_SET.contains(cleanWord) || TYPES_SET.contains(lower)) return NcTokenTypes.TYPE
        if (PRIMITIVE_SET.contains(cleanWord) || PRIMITIVE_SET.contains(lower)) return NcTokenTypes.PRIMITIVE_ENTITY
        if (NcLowLevelCompletionCatalog.isMachineInstruction(lower) || MACHINE_SET.contains(lower)) {
            return NcTokenTypes.MACHINE_INSTRUCTION
        }
        if (FFI_SET.contains(cleanWord) || FFI_SET.contains(lower)) return NcTokenTypes.FFI
        if (cleanWord.startsWith("capability.") || cleanWord.contains(".capability")) return NcTokenTypes.CAPABILITY
        if (cleanWord.startsWith("policy.") || cleanWord.contains(".policy")) return NcTokenTypes.POLICY
        if (cleanWord.startsWith("E_")) return NcTokenTypes.DIAGNOSTIC_ERROR
        if (lower == "copy" || lower.startsWith("copy.")) return NcTokenTypes.COPY
        if (lower == "clone" || lower.startsWith("clone.")) return NcTokenTypes.CLONE
        return NcTokenTypes.IDENTIFIER
    }

    private fun lexStarredKernelMmioWidth(): Pair<IElementType, Int>? {
        var i = tokenStart + 1
        while (i < endOffset && buffer[i].isWhitespace()) i += 1
        val widthStart = i
        while (i < endOffset && buffer[i].isDigit()) i += 1
        if (i == widthStart) return null
        val width = buffer.subSequence(widthStart, i).toString()
        val widthToken = kernelMmioWidthToken(width) ?: return null
        if (nextNonWhitespace(i) != '(') return null
        return widthToken to i
    }

    private fun kernelMmioWidthToken(width: String): IElementType? = when (width) {
        "8" -> NcTokenTypes.KERNEL_MMIO_8
        "16" -> NcTokenTypes.KERNEL_MMIO_16
        "32" -> NcTokenTypes.KERNEL_MMIO_32
        "64" -> NcTokenTypes.KERNEL_MMIO_64
        "128" -> NcTokenTypes.KERNEL_MMIO_128
        "256" -> NcTokenTypes.KERNEL_MMIO_256
        "512" -> NcTokenTypes.KERNEL_MMIO_512
        else -> null
    }

    private fun nextNonWhitespace(offset: Int): Char? {
        var i = offset
        while (i < endOffset && buffer[i].isWhitespace()) i += 1
        return if (i < endOffset) buffer[i] else null
    }

    private fun nextWord(offset: Int): String? {
        var i = offset
        while (i < endOffset && buffer[i].isWhitespace()) i += 1
        if (i >= endOffset || !isWordStart(buffer[i])) return null
        val start = i
        i += 1
        while (i < endOffset && isWordChar(buffer[i])) i += 1
        return buffer.subSequence(start, i).toString().trimEnd('@')
    }

    private fun previousWord(offset: Int): String? {
        var i = offset - 1
        while (i >= startOffset && buffer[i].isWhitespace()) i -= 1
        if (i < startOffset || !isWordChar(buffer[i])) return null
        val end = i + 1
        while (i >= startOffset && isWordChar(buffer[i])) i -= 1
        val start = i + 1
        return buffer.subSequence(start, end).toString().trimEnd('@')
    }

    private fun isContainerNameAtCurrentWord(word: String): Boolean {
        val lower = word.lowercase()
        if (KEYWORDS_SET.contains(lower) || SECTIONS.contains(lower) || TYPES_SET.contains(word) || TYPES_SET.contains(lower)) {
            return false
        }
        var before = tokenStart - 1
        while (before >= startOffset && buffer[before].isWhitespace() && buffer[before] != '\n') before -= 1
        if (before >= startOffset && buffer[before] != '\n' && buffer[before] != '\r') return false

        var after = tokenEnd
        while (after < endOffset && buffer[after].isWhitespace()) after += 1
        if (after + 1 < endOffset && buffer[after] == ':' && buffer[after + 1] == ':') {
            val tail = buffer.subSequence(after, endOffset).toString()
            return Regex("""^::\s*(?:program|runtime|network|system|kernel|tooling|compiler|security|inspect|ai|machine|primitive|basal)\s*\{""", RegexOption.IGNORE_CASE)
                .containsMatchIn(tail)
        }
        if (after < endOffset && buffer[after] == '[') {
            after += 1
            while (after < endOffset && buffer[after] != ']') after += 1
            if (after < endOffset) after += 1
            while (after < endOffset && buffer[after].isWhitespace()) after += 1
        }
        return after < endOffset && buffer[after] == '{'
    }

    private fun isMainFamilyWord(word: String): Boolean {
        val lower = word.lowercase()
        val next = nextWord(tokenEnd)?.lowercase()
        val previous = previousWord(tokenStart)?.lowercase()
        if ((lower == "to" || lower == "ref") && next == "main") return true
        if (lower == "main" && (previous == "to" || previous == "ref")) return true
        return false
    }

    private fun roleToken(role: String): IElementType? {
        var index = tokenStart - 1
        while (index >= startOffset && buffer[index].isWhitespace()) index -= 1
        if (index < startOffset || buffer[index] != ':') return null
        index -= 1
        if (index < startOffset || buffer[index] != ':') return null
        return when (role) {
            "runtime", "network" -> NcTokenTypes.MODE_RUNTIME
            "compiler", "tooling", "security", "inspect", "ai" -> NcTokenTypes.MODE_COMPILER
            "system", "kernel" -> NcTokenTypes.MODE_KERNEL
            "machine" -> NcTokenTypes.MODE_MACHINE
            "primitive" -> NcTokenTypes.MODE_PRIMITIVE
            "basal" -> NcTokenTypes.MODE_BASAL
            "program" -> NcTokenTypes.MODE_OTHER
            else -> null
        }
    }

    private fun matchOperator(offset: Int): String? {
        for (op in OPERATORS) {
            if (offset + op.length <= endOffset && buffer.subSequence(offset, offset + op.length).toString() == op) {
                return op
            }
        }
        return null
    }

    private fun isWordStart(c: Char): Boolean = c == '@' || c == '_' || c.isLetter()

    private fun isWordChar(c: Char): Boolean = c == '@' || c == '_' || c == '.' || c == '[' || c == ']' || c.isLetterOrDigit()

    private fun Char.isHexDigit(): Boolean =
        (this in '0'..'9') || (this in 'a'..'f') || (this in 'A'..'F')

    companion object {
        private val SECTIONS = setOf(
            "stc", "local", "fn", "probe", "structure", "str", "type", "struct", "enum", "binary", "bind", "raw",
            "bridge", "fa", "final", "const", "routine", "verify", "report", "seal",
            "root", "target", "site", "meta", "evidence", "layout", "ffi",
            "section", "symbol", "entry", "proc", "frame", "instr", "operands", "registers", "stack",
            "capability", "bus", "port", "signal", "operation", "controller", "peripheral"
        )
        private val KEYWORDS_SET = setOf(
            "if", "else", "while", "for", "match", "guard", "unless", "return", "dev", "skip", "stop",
            "break", "continue", "use", "import", "public", "private", "internal", "protected", "strictly", "to", "ref", "main", "move", "const", "routine", "verify", "report", "seal"
        )
        private val POINTER_TYPES = setOf("p", "ptr")
        private val PRINT_FUNCTIONS = setOf("print", "println")
        private val COLOR_FUNCTIONS = setOf("color", "colorBold", "bgColor", "style")
        private val POINTER_FUNCTIONS = setOf(
            "rd", "wr", "off", "dist", "nil", "adr", "from", "asptr",
            "pRead", "pWrite", "pOffset", "pDiff", "pNull", "pRegion", "pAt", "pFromAddr", "pCast", "pAddr",
            "Read", "Write", "ptrRead", "ptrWrite", "ptrOffset", "ptrDiff", "ptrIsNull", "regionFromPtr", "regionPtrAt",
            "ptrFromAddr", "ptrCast", "ptrAddr"
        )
        private val CONTROL_FLOW_KEYWORDS = setOf("if", "elseif", "else", "while", "for", "loop", "same", "match", "guard", "unless")
        private val LOOP_CONTROL_KEYWORDS = setOf("break", "continue")
        private val VISIBILITY_KEYWORDS = setOf("public", "internal", "private")
        private val RAW_HAZARDS = setOf("raw", "risk", "hazard", "rawptr", "rawaddr", "rawsite", "unchecked")
        private val BASIC_TYPES = setOf(
            "int", "u64", "byte", "str", "text", "char", "ft64", "word", "dword", "qword",
            "sbyte", "sword", "sdword", "sqword", "void", "null"
        )
        private val ADVANCED_TYPES = setOf("addr", "ref", "site", "Result", "result", "Option", "option", "xy", "xyz")
        private val ARRAY_TYPES = setOf(
            "slice", "span", "bytes", "array", "list", "matrix", "tensor", "dinArray", "dinarray",
            "dinArray.binary", "dinarray.binary", "dinArray.unbinary", "dinarray.unbinary"
        )
        private val TYPES_SET = setOf(
            "int", "u64", "byte", "bool", "str", "text", "char", "ft64", "addr", "word", "dword", "qword",
            "sbyte", "sword", "sdword", "sqword", "void", "null", "p", "ptr", "ref", "site", "memRegion", "memregion", "slice", "span", "bytes", "array", "list", "Result", "Option", "xy", "xyz", "matrix",
            "tensor", "dinArray", "dinArray.binary", "dinArray.unbinary"
        )
        private val PRIMITIVE_SET = setOf(
            "allow", "deny", "read", "write", "rw", "route", "stable", "valid", "line_ready", "line_ack",
            "await", "barrier", "ioaddr", "timeout", "cycles", "capability", "operation", "port",
            "anchor", "slot", "pattern", "mask", "window", "signal", "bus", "state", "timing", "emit", "sample", "latch",
            "assert", "rupture", "seal", "voltage", "current", "clock", "edge", "latency", "metastability", "noise",
            "temperature", "power", "thermalState", "throttleState", "cacheState", "tlbState", "pipelineState",
            "branchPredictorState", "reorderState", "outOfOrder", "powerGate", "dmaChannel", "interruptLine", "watchdog",
            "firmwareRegion", "bootRom", "bootRomRegion", "biosRegion", "uefiHandoff", "resetVector", "bootWindow",
            "bootAnchor", "firmwareTable", "deviceTree", "acpiTable", "microcode", "microcodeState", "isa", "protocol",
            "controller", "peripheral", "phy", "lane", "channel", "registerWindow", "mmioWindow", "portWindow",
            "physicalEvent", "asyncHardwareEvent", "sideEffect", "physicalFault", "physicalFailure", "physicalViolation"
        )
        private val MACHINE_REGISTERS = setOf(
            "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rsp", "rbp", "rip", "rflags",
            "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
            "eax", "ebx", "ecx", "edx", "esi", "edi", "esp", "ebp",
            "ax", "bx", "cx", "dx", "si", "di", "sp", "bp",
            "al", "bl", "cl", "dl",
            "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
            "x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10",
            "x11", "x12", "x13", "x14", "x15", "x16", "x17", "x18", "x19", "x20",
            "x21", "x22", "x23", "x24", "x25", "x26", "x27", "x28", "x29", "x30",
            "w0", "w1", "w2", "w3", "w4", "w5", "w6", "w7", "sp", "xzr", "wzr"
        )
        private val MACHINE_SET = setOf(
            "mov", "add", "sub", "cmp", "test", "jmp", "jmp.eq", "jmp.ne", "call", "ret", "push", "pop", "in", "out", "hlt", "nop",
            "mfence", "sfence", "lfence", "volatile", "fence", "opcode", "modrm", "sib", "rex", "vex", "evex",
            "wbnoinvd", "getsec", "rdpmc", "rdpid", "clts", "clc", "stc", "cli", "sti", "cld", "std",
            "esb", "tsb", "dgh", "xaflag", "axflag", "wfet", "wfit", "hvc", "smc", "dbg", "cpsid", "cpsie",
            "hfence.vvma", "hfence.gvma", "hinval.vvma", "hinval.gvma", "hlv.b", "hlv.bu", "hlv.h", "hlv.hu", "hlv.w", "hlv.wu", "hlv.d", "hlvx.hu", "hlvx.wu", "hsv.b", "hsv.h", "hsv.w", "hsv.d",
            "sfence.w.inval", "sfence.inval.ir", "sinval.vma",
            "memory.init", "data.drop", "table.get", "table.set", "table.size", "table.grow", "table.fill", "table.copy", "table.init", "elem.drop",
            "ref.null", "ref.is_null", "ref.func"
        )
        private val FFI_SET = setOf("foreign.c", "foreign.cpp", "foreign.rust", "extern", "abi", "layout", "ownership", "nullability", "symbol", "repr")
        private val OPERATORS = listOf(
            "->>", "!>", "->", "::", ":=", "<-", "&&", "||", "==", "!=", "<=", ">=", "++", "--", "+=", "-=", "=+", "=-", "..=", "..",
            "@", ">>", "=", "+", "-", "*", "/", "%", "&", "|", "^", "~", "!", "<", ">",
            "{", "}", "(", ")", "[", "]", ",", ";", ":"
        )
    }
}
