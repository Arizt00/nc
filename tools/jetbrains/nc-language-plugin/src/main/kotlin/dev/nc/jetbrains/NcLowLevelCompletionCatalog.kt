package dev.nc.jetbrains

data class NcLowLevelSuggestion(
    val label: String,
    val insertText: String = label,
    val kind: String,
    val detail: String,
    val priority: Double = 300.0,
)

object NcLowLevelCompletionCatalog {
    private val registers = mapOf(
        "x86_64" to listOf(
            "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rsp", "rbp", "rip", "rflags",
            "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
            "eax", "ebx", "ecx", "edx", "xmm0", "xmm1", "ymm0", "ymm1", "zmm0", "zmm1",
            "k0", "k1", "cr0", "cr2", "cr3", "cr4", "cr8", "dr0", "dr6", "dr7",
        ),
        "i386" to listOf(
            "eax", "ebx", "ecx", "edx", "esi", "edi", "esp", "ebp", "eip", "eflags",
            "ax", "bx", "cx", "dx", "si", "di", "sp", "bp", "al", "ah", "bl", "bh", "cl", "ch", "dl", "dh",
            "xmm0", "xmm1", "mm0", "mm1", "st0", "st1", "cr0", "cr2", "cr3", "cr4",
        ),
        "aarch64" to (
            (0..30).map { "x$it" } + (0..30).map { "w$it" } + (0..31).map { "v$it" } +
                (0..31).map { "z$it" } + (0..15).map { "p$it" } +
                listOf("sp", "fp", "lr", "pc", "xzr", "wzr", "nzcv", "fpcr", "fpsr", "ffr")
            ),
        "armv7" to ((0..15).map { "r$it" } + (0..31).map { "s$it" } + (0..31).map { "d$it" } + (0..15).map { "q$it" } + listOf("sp", "lr", "pc", "cpsr", "spsr")),
        "riscv32" to ((0..31).map { "x$it" } + listOf("zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4", "t5", "t6")),
        "riscv64" to ((0..31).map { "x$it" } + (0..31).map { "f$it" } + listOf("zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4", "t5", "t6")),
        "wasm32" to listOf("local0", "local1", "global0"),
    )

    private val instructions = mapOf(
        "x86_64" to listOf(
            "mov", "movzx", "movsx", "lea", "add", "adc", "adcx", "adox", "sub", "sbb", "imul", "mul", "idiv", "div",
            "and", "or", "xor", "not", "neg", "shl", "shr", "sar", "rol", "ror", "cmp", "test", "cmov", "setcc",
            "jmp", "je", "jne", "call", "ret", "push", "pop", "nop", "pause", "hlt", "mfence", "sfence", "lfence",
            "serialize", "cldemote", "clflush", "clflushopt", "clwb", "prefetchnta", "prefetcht0", "prefetcht1", "prefetcht2",
            "monitor", "mwait", "umonitor", "umwait", "tpause", "syscall", "sysret", "rdtsc", "rdtscp", "rdpmc", "rdpid", "cpuid", "getsec", "rdmsr", "wrmsr", "ud2", "endbr64", "endbr32", "rdpkru", "wrpkru",
            "rdrand", "rdseed", "aesenc", "pclmulqdq", "sha256rnds2", "vmxon", "vmptrld", "vmptrst", "vmread", "vmwrite", "invept", "invvpid", "vmrun", "vmload", "vmsave",
            "wbnoinvd", "clts", "clc", "stc", "cli", "sti", "cld", "std",
        ),
        "i386" to listOf(
            "mov", "movzx", "movsx", "lea", "add", "adc", "sub", "sbb", "imul", "mul", "idiv", "div", "and", "or", "xor",
            "cmp", "test", "jmp", "je", "jne", "call", "ret", "push", "pop", "pushad", "popad", "aaa", "aad", "aam", "aas", "daa", "das", "salc", "icebp", "into", "xlatb", "ud2", "in", "out", "hlt", "nop", "endbr32", "rdpkru", "wrpkru", "rdpmc", "rdpid", "getsec",
        ),
        "aarch64" to listOf(
            "mov", "movz", "movk", "adr", "adrp", "add", "adds", "adc", "sub", "subs", "sbc", "mul", "madd", "udiv", "sdiv",
            "and", "orr", "eor", "lsl", "lsr", "asr", "cmp", "csel", "b", "bl", "br", "blr", "ret", "eret",
            "ldr", "str", "ldp", "stp", "ldxr", "stxr", "cas", "swp", "dmb", "dsb", "isb", "sb", "ssbb", "pssbb", "csdb", "wfi", "wfe", "sev", "extr", "ubfm", "sbfm", "bfm",
            "paciasp", "autiasp", "xpaclri", "xpaci", "xpacd", "paciza", "autiza", "pacia", "autda",
            "whilelo", "ptrue", "pfalse", "ld1b", "ld1h", "ld1w", "ld1d", "st1b", "st1h", "st1w", "st1d",
            "cntb", "cnth", "cntw", "cntd", "incb", "inch", "incw", "incd", "cmpeq", "cmpne", "cmplt", "cmpgt", "cmpge", "cmple", "sel", "fmla", "faddv", "esb", "tsb", "dgh", "xaflag", "axflag", "wfet", "wfit",
        ),
        "armv7" to listOf(
            "mov", "movw", "movt", "add", "adc", "sub", "sbc", "mul", "mla", "and", "orr", "eor", "cmp", "tst", "b", "bl", "bx",
            "push", "pop", "ldr", "str", "ldrex", "strex", "dmb", "dsb", "isb", "yield", "wfi", "dbg", "setend", "qadd", "qsub", "ssat", "usat", "rbit", "rev", "hvc", "smc", "cpsid", "cpsie",
        ),
        "riscv32" to listOf(
            "lui", "auipc", "jal", "jalr", "beq", "bne", "blt", "bge", "lb", "lh", "lw", "sb", "sh", "sw", "addi", "add", "sub",
            "mul", "div", "rem", "and", "or", "xor", "sll", "srl", "sra", "fence", "fence.tso", "wrs.nto", "wrs.sto", "fence.i", "pause", "cbo.clean", "cbo.flush", "cbo.inval", "cbo.zero",
            "ecall", "ebreak", "mret", "sret", "sfence.vma", "sfence.w.inval", "sfence.inval.ir", "sinval.vma", "csrr", "csrw", "csrrw", "csrrs", "csrrc", "csrrwi", "csrrsi", "csrrci",
            "lr.w", "sc.w", "amoadd.w", "czero.eqz", "czero.nez", "clz", "ctz", "cpop", "sext.b", "sext.h", "zext.h", "orc.b", "rev8", "bset", "bclr", "binv", "bext", "bseti", "bclri", "binvi", "bexti",
            "hfence.vvma", "hfence.gvma", "hinval.vvma", "hinval.gvma", "hlv.b", "hlv.bu", "hlv.h", "hlv.hu", "hlv.w", "hlvx.hu", "hsv.b", "hsv.h", "hsv.w",
        ),
        "riscv64" to listOf(
            "lui", "auipc", "jal", "jalr", "beq", "bne", "blt", "bge", "lb", "lh", "lw", "ld", "sb", "sh", "sw", "sd", "addi", "add", "sub",
            "mul", "div", "rem", "and", "or", "xor", "sll", "srl", "sra", "fence", "fence.tso", "wrs.nto", "wrs.sto", "fence.i", "pause", "cbo.clean", "cbo.flush", "cbo.inval", "cbo.zero",
            "ecall", "ebreak", "mret", "sret", "sfence.vma", "sfence.w.inval", "sfence.inval.ir", "sinval.vma", "csrr", "csrw", "csrrw", "csrrs", "csrrc", "csrrwi", "csrrsi", "csrrci",
            "lr.d", "sc.d", "amoadd.d", "czero.eqz", "czero.nez", "clz", "ctz", "cpop", "clzw", "ctzw", "cpopw", "sext.b", "sext.h", "zext.h", "orc.b", "rev8", "bset", "bclr", "binv", "bext", "bseti", "bclri", "binvi", "bexti",
            "hfence.vvma", "hfence.gvma", "hinval.vvma", "hinval.gvma", "hlv.b", "hlv.bu", "hlv.h", "hlv.hu", "hlv.w", "hlv.wu", "hlv.d", "hlvx.hu", "hlvx.wu", "hsv.b", "hsv.h", "hsv.w", "hsv.d",
        ),
        "wasm32" to listOf(
            "local.get", "local.set", "local.tee", "global.get", "global.set", "i32.const", "i64.const", "i32.load", "i64.load", "i32.store", "i64.store",
            "i32.eqz", "i32.eq", "i32.ne", "i32.lt_s", "i32.lt_u", "i32.gt_s", "i32.gt_u", "i32.le_s", "i32.le_u", "i32.ge_s", "i32.ge_u",
            "i32.add", "i32.sub", "i32.mul", "i32.div_s", "i32.div_u", "i32.rem_s", "i32.rem_u", "i32.and", "i32.or", "i32.xor", "i32.shl", "i32.shr_s", "i32.shr_u", "i32.rotl", "i32.rotr",
            "i64.eqz", "i64.add", "i64.sub", "i64.mul", "i64.div_s", "i64.div_u", "i64.and", "i64.or", "i64.xor", "i64.shl", "i64.shr_s", "i64.shr_u", "i64.rotl", "i64.rotr",
            "call", "return", "block", "loop", "br", "br_if", "drop", "select", "memory.size", "memory.copy", "memory.fill", "memory.grow", "memory.init", "data.drop",
            "table.get", "table.set", "table.size", "table.grow", "table.fill", "table.copy", "table.init", "elem.drop", "ref.null", "ref.is_null", "ref.func",
        ),
    )

    private val machineKeywords = listOf(
        "target", "symbol", "entry", "proc", "instr", "frame", "operands", "registers", "verify", "seal",
        "section", "binding", "type", "artifact", "require",
    )
    private val basalKeywords = listOf(
        "root", "site", "bind", "routine", "verify", "report", "seal", "identity", "version", "profile", "authority",
        "binary", "evidence", "lowering", "kind", "size", "align", "permissions", "symbol", "status", "emit", "require",
        "hash", "memory_anchor", "evidence_anchor", "source_hash", "read", "write", "execute", "sealed", "optional",
    )
    private val primitiveKeywords = listOf(
        "type", "isa", "opcode", "encoding", "relocation", "abi", "capability", "bus", "port", "signal", "operation", "anchor", "entry", "bridge", "seal",
        "window", "state", "timing", "controller", "peripheral", "protocol", "phy", "lane", "channel", "registerWindow", "mmioWindow", "portWindow",
        "interruptLine", "dmaChannel", "watchdog", "firmwareRegion", "bootRom", "bootRomRegion", "biosRegion", "uefiHandoff", "resetVector", "bootWindow", "bootAnchor",
        "firmwareTable", "deviceTree", "acpiTable", "microcode", "microcodeState", "physicalEvent", "asyncHardwareEvent", "sideEffect", "physicalFault", "physicalFailure", "physicalViolation",
        "rupture", "allow", "deny", "barrier", "event", "read", "write", "trace", "haltLoop", "volatile", "ioaddr", "mmioaddr", "physaddr",
    )

    fun rows(extension: String?, text: String, linePrefix: String = ""): List<NcLowLevelSuggestion>? =
        when (extension?.lowercase()) {
            "nm", "nb", "nbb", "ncp" -> emptyList()
            else -> null
        }

    fun isMachineInstruction(word: String): Boolean =
        instructions.values.any { word.lowercase() in it }

    fun isMachineRegister(word: String): Boolean =
        registers.values.any { word.lowercase() in it }

    private fun machineRows(text: String, linePrefix: String): List<NcLowLevelSuggestion> {
        requestedMachineField(linePrefix)?.let { field ->
            return machineFieldValues(field).map {
                NcLowLevelSuggestion(it, kind = "enum", detail = "$field value accepted by Machine sema", priority = 340.0)
            }
        }

        val arch = machineArch(text)
        val arches = if (arch.isBlank()) instructions.keys.toList() else listOf(arch)
        val rows = mutableListOf(
            NcLowLevelSuggestion(
                "machine container",
                "name :: machine {\n    target:\n        arch: x86_64;\n        abi: sysv64;\n        object: elf64;\n        endian: little;\n        pointerWidth: 64;\n        profile: userland;\n\n    symbol:\n        main:\n            section: .text;\n            binding: global;\n            type: routine;\n\n    entry:\n        main;\n\n    proc:\n        main:\n            instr:\n                ret;\n\n    verify:\n        target;\n        operands;\n        encoding;\n        object;\n\n    seal:\n        artifact: elf64;\n        require: verified;\n}",
                "snippet",
                "Complete typed Machine translation unit",
                360.0,
            ),
        )
        rows += machineKeywords.map { NcLowLevelSuggestion(it, "$it:", "keyword", "Machine structural field", 320.0) }
        rows += machineDeclaredNames(text).map { NcLowLevelSuggestion(it, kind = "reference", detail = "Symbol or procedure declared in this Machine unit", priority = 350.0) }
        rows += arches.flatMap { current ->
            registers[current].orEmpty().map { NcLowLevelSuggestion(it, kind = "register", detail = "$current register", priority = 340.0) }
        }
        rows += arches.flatMap { current ->
            instructions[current].orEmpty().map { NcLowLevelSuggestion(it, "$it ", "instruction", "$current instruction encoded by Machine", 330.0) }
        }
        return rows.distinctBy { "${it.kind}:${it.label}" }
    }

    private fun basalRows(text: String): List<NcLowLevelSuggestion> {
        val rows = mutableListOf(
            NcLowLevelSuggestion(
                "basal container",
                "name :: root {\n    root:\n        identity: \"nC.root.name\";\n        version: 1;\n        profile: basal;\n        authority: sealed;\n\n    site:\n        rootSlot:\n            kind: memory_anchor;\n            size: 8;\n            align: 8;\n            permissions: read | write;\n\n    verify:\n        true;\n\n    seal:\n        require source_hash;\n}",
                "snippet",
                "Complete editable basal/root contract",
                360.0,
            ),
            NcLowLevelSuggestion("site contract", "site:\n    slot:\n        kind: memory_anchor;\n        size: 8;\n        align: 8;\n        permissions: read | write;", "snippet", "Address-backed basal site", 350.0),
            NcLowLevelSuggestion("binding contract", "bind:\n    symbol name -> root.site slot;", "snippet", "Material symbol binding", 350.0),
        )
        rows += basalKeywords.map { NcLowLevelSuggestion(it, kind = "keyword", detail = "Basal/root contract token", priority = 320.0) }
        rows += basalDeclaredNames(text).map { NcLowLevelSuggestion(it, kind = "reference", detail = "Declaration in this basal unit", priority = 350.0) }
        return rows.distinctBy { "${it.kind}:${it.label}" }
    }

    private fun primitiveRows(text: String, hardware: Boolean): List<NcLowLevelSuggestion> {
        val rows = mutableListOf(
            NcLowLevelSuggestion(
                if (hardware) "hardware primitive target" else "primitive target",
                "primitive[@] target x86_64:\n    capability hardware:\n        allow port.write;\n\n    operation entry():\n        capability hardware;\n        barrier io;\n\n    bridge:\n        root.policy.bind;\n        machine.machineCore;\n\n    seal:\n        primitiveClosed;",
                "snippet",
                if (hardware) "Complete physical hardware unit" else "Complete Primitive unit",
                360.0,
            ),
            NcLowLevelSuggestion("capability block", "capability name:\n    allow port.read;\n    deny host.console.write;", "snippet", "Explicit physical capability", 350.0),
            NcLowLevelSuggestion("operation block", "operation name():\n    capability authority;\n    barrier io;", "snippet", "Primitive operation and barrier", 350.0),
        )
        val visibleKeywords = if (hardware) primitiveKeywords else primitiveKeywords.filterNot {
            it in setOf("voltage", "current", "temperature", "phy", "lane", "peripheral")
        }
        rows += visibleKeywords.map {
            NcLowLevelSuggestion(
                it,
                kind = "keyword",
                detail = if (hardware) "Physical hardware/firmware contract token" else "Primitive material contract token",
                priority = 320.0,
            )
        }
        rows += primitiveDeclaredNames(text).map { NcLowLevelSuggestion(it, kind = "reference", detail = "Declaration in this material unit", priority = 350.0) }
        return rows.distinctBy { "${it.kind}:${it.label}" }
    }

    private fun machineArch(text: String): String {
        val raw = Regex("""(?mi)^\s*arch\s*:\s*([A-Za-z0-9_]+)\s*;""").find(text)?.groupValues?.get(1)?.lowercase() ?: return ""
        return when (raw) {
            "amd64" -> "x86_64"
            "x86", "i686" -> "i386"
            "arm64" -> "aarch64"
            "thumb", "thumbv7", "armv7-thumb" -> "armv7"
            "rv32" -> "riscv32"
            "rv64" -> "riscv64"
            else -> raw
        }
    }

    private fun requestedMachineField(prefix: String): String? =
        Regex("""(?i)(arch|abi|object|endian|pointerWidth|profile|binding|type)\s*:\s*[A-Za-z0-9_-]*$""")
            .find(prefix)?.groupValues?.get(1)

    private fun machineFieldValues(field: String): List<String> = when (field.lowercase()) {
        "arch" -> listOf("x86_64", "i386", "aarch64", "armv7", "riscv32", "riscv64", "wasm32")
        "abi" -> listOf("sysv64", "win64", "i386-sysv", "aapcs64", "aapcs32", "ilp32", "riscv64", "wasm")
        "object" -> listOf("elf64", "elf32", "coff", "macho64", "wasm")
        "endian" -> listOf("little", "big")
        "pointerwidth" -> listOf("16", "32", "64", "128")
        "profile" -> listOf("userland", "kernel", "firmware", "baremetal", "legacy", "webassembly")
        "binding" -> listOf("local", "global", "weak", "common")
        "type" -> listOf("routine", "object", "section", "tls", "absolute")
        else -> emptyList()
    }

    private fun machineDeclaredNames(text: String): List<String> {
        val reserved = setOf("target", "symbol", "entry", "proc", "instr", "verify", "seal", "frame", "operands", "registers")
        return Regex("""(?mi)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*$""").findAll(text)
            .map { it.groupValues[1] }.filter { it.lowercase() !in reserved }.distinct().sorted().toList()
    }

    private fun basalDeclaredNames(text: String): List<String> {
        val reserved = setOf("root", "site", "bind", "routine", "verify", "report", "seal")
        val names = mutableListOf<String>()
        names += Regex("""(?mi)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*$""").findAll(text).map { it.groupValues[1] }
        names += Regex("""(?mi)^\s*symbol\s+([A-Za-z_][A-Za-z0-9_]*)\s*->""").findAll(text).map { it.groupValues[1] }
        names += Regex("""(?mi)^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\([^)]*\)\s*->""").findAll(text).map { it.groupValues[1] }
        return names.filter { it.lowercase() !in reserved }.distinct().sorted()
    }

    private fun primitiveDeclaredNames(text: String): List<String> =
        Regex("""(?mi)^\s*(?:capability|bus|port|signal|operation|anchor|entry|window|state|controller|peripheral|protocol|phy|lane|channel|interruptLine|dmaChannel|watchdog|firmwareRegion|bootWindow|bootAnchor|firmwareTable|deviceTree|acpiTable|microcode|physicalFault)\s+([A-Za-z_][A-Za-z0-9_.]*)""")
            .findAll(text).map { it.groupValues[1] }.distinct().sorted().toList()
}
