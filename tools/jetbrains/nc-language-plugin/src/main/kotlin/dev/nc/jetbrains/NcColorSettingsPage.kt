package dev.nc.jetbrains

import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighter
import com.intellij.openapi.options.colors.AttributesDescriptor
import com.intellij.openapi.options.colors.ColorDescriptor
import com.intellij.openapi.options.colors.ColorSettingsPage
import javax.swing.Icon

class NcColorSettingsPage : ColorSettingsPage {
    override fun getDisplayName(): String = "nC"

    override fun getIcon(): Icon? = NcIcons.Plugin

    override fun getAttributeDescriptors(): Array<AttributesDescriptor> = DESCRIPTORS

    override fun getColorDescriptors(): Array<ColorDescriptor> = ColorDescriptor.EMPTY_ARRAY

    override fun getHighlighter(): SyntaxHighlighter = NcSyntaxHighlighter()

    override fun getDemoText(): String = """
use <io:input, output, render>;

runtimeDemo {
    const:
        LIMIT >> int <- 2;

    stc:
        value >> int <- 0;
        ok >> bool <- false;
        region >> memRegion<dword>;
        cursor >> p<dword>;

    local:
        scratch >> int <- 0;

    fn:
        internal:
            inspectValue(input >> int) -> bool {
                inspect(input);
                return input < LIMIT;
            }

        public:
            run() -> bool {
                destructure.static(readInt());
                constructure.static(readInt());
                copy.value(value);
                clone.deep(value);
                return inspectValue(value);
            }

    ref main ->> run;

    fa:
        return true;
}

kernelDemo :: kernel {
    fn:
        public:
            touch() -> bool {
                lapic at 0xFEE00000;
                value = load32(lapic);
                *32(lapic, value);
                *64(lapic, value);
                *128(lapic, value);
                *256(lapic, value);
                return true;
            }

    ref main ->> touch;

    fa:
        return true;
}

machineDemo :: machine {
    target:
        arch: x86_64;
        abi: sysv64;
    instr:
        mov rax rdi;
        add rax rsi;
        ret;
}

rootDemo :: basal {
    const:
        width >> dword <- 32;
    routine:
        sum(src >> addr, dst >> addr) -> bool {
            raw {
                mem[@dst] >> dword : mem[@src];
            }
            return true;
        }
}

primitiveDemo :: primitive {
    capability boot_serial_io:
        allow port.read;
        deny dma.map;
}
""".trimIndent()

    override fun getAdditionalHighlightingTagToDescriptorMap(): MutableMap<String, TextAttributesKey>? = null

    companion object {
        private val DESCRIPTORS = arrayOf(
            AttributesDescriptor("Keyword", NcTextAttributes.NC_KEYWORD),
            AttributesDescriptor("Import/use keyword", NcTextAttributes.NC_IMPORT_KEYWORD),
            AttributesDescriptor("Container", NcTextAttributes.NC_CONTAINER),
            AttributesDescriptor("Section / stc", NcTextAttributes.NC_STC_SECTION),
            AttributesDescriptor("Section / fn", NcTextAttributes.NC_FN_SECTION),
            AttributesDescriptor("Section / fa", NcTextAttributes.NC_FA_SECTION),
            AttributesDescriptor("Main family", NcTextAttributes.NC_MAIN_FAMILY),
            AttributesDescriptor("Visibility / public internal private", NcTextAttributes.NC_VISIBILITY),
            AttributesDescriptor("@ reference", NcTextAttributes.NC_AT_REFERENCE),
            AttributesDescriptor("@(...) grouped reference", NcTextAttributes.NC_AT_GROUP_REFERENCE),
            AttributesDescriptor("Projection operator >>", NcTextAttributes.NC_PROJECTION),
            AttributesDescriptor("Verification operator !>", NcTextAttributes.NC_VERIFICATION),
            AttributesDescriptor("dev()", NcTextAttributes.NC_DEV),
            AttributesDescriptor("return", NcTextAttributes.NC_RETURN),
            AttributesDescriptor("Mode Runtime", NcTextAttributes.NC_MODE_RUNTIME),
            AttributesDescriptor("Mode Compiler", NcTextAttributes.NC_MODE_COMPILER),
            AttributesDescriptor("Mode Basal/Root", NcTextAttributes.NC_MODE_BASAL),
            AttributesDescriptor("Mode Kernel", NcTextAttributes.NC_MODE_KERNEL),
            AttributesDescriptor("Mode Machine", NcTextAttributes.NC_MODE_MACHINE),
            AttributesDescriptor("Mode Primitive", NcTextAttributes.NC_MODE_PRIMITIVE),
            AttributesDescriptor("Mode Physical Hardware", NcTextAttributes.NC_MODE_HARDWARE),
            AttributesDescriptor("Section", NcTextAttributes.NC_SECTION),
            AttributesDescriptor("Section / nC source", NcTextAttributes.NC_SOURCE_SECTION),
            AttributesDescriptor("Section / basal Rust-copper", NcTextAttributes.NC_BASAL_SECTION),
            AttributesDescriptor("Section / machine ASM", NcTextAttributes.NC_MACHINE_SECTION),
            AttributesDescriptor("Section / primitive C/C++", NcTextAttributes.NC_PRIMITIVE_SECTION),
            AttributesDescriptor("Section / physical hardware", NcTextAttributes.NC_HARDWARE_SECTION),
            AttributesDescriptor("Type", NcTextAttributes.NC_TYPE),
            AttributesDescriptor("Type / basal", NcTextAttributes.NC_BASAL_TYPE),
            AttributesDescriptor("Type / machine", NcTextAttributes.NC_MACHINE_TYPE),
            AttributesDescriptor("Type / primitive", NcTextAttributes.NC_PRIMITIVE_TYPE),
            AttributesDescriptor("Type / physical hardware", NcTextAttributes.NC_HARDWARE_TYPE),
            AttributesDescriptor("Type / basic", NcTextAttributes.NC_BASIC_TYPE),
            AttributesDescriptor("Type / advanced", NcTextAttributes.NC_ADVANCED_TYPE),
            AttributesDescriptor("Type / generic", NcTextAttributes.NC_GENERIC_TYPE),
            AttributesDescriptor("Type / arrays", NcTextAttributes.NC_ARRAY_TYPE),
            AttributesDescriptor("Pointer/Reference/Site Type", NcTextAttributes.NC_POINTER_TYPE),
            AttributesDescriptor("memRegion<T>", NcTextAttributes.NC_MEM_REGION_TYPE),
            AttributesDescriptor("Bool", NcTextAttributes.NC_BOOL),
            AttributesDescriptor("Function", NcTextAttributes.NC_FUNCTION),
            AttributesDescriptor("Runtime Function", NcTextAttributes.NC_RUNTIME_FUNCTION),
            AttributesDescriptor("print / println", NcTextAttributes.NC_PRINT_FUNCTION),
            AttributesDescriptor("Pointer operations", NcTextAttributes.NC_POINTER_FUNCTION),
            AttributesDescriptor("Color functions", NcTextAttributes.NC_COLOR_FUNCTION),
            AttributesDescriptor("Control flow", NcTextAttributes.NC_CONTROL_FLOW),
            AttributesDescriptor("break / continue", NcTextAttributes.NC_LOOP_CONTROL),
            AttributesDescriptor("Kernel address binding / at", NcTextAttributes.NC_KERNEL_ADDRESS_KEYWORD),
            AttributesDescriptor("Kernel MMIO / 8-bit", NcTextAttributes.NC_KERNEL_MMIO_8),
            AttributesDescriptor("Kernel MMIO / 16-bit", NcTextAttributes.NC_KERNEL_MMIO_16),
            AttributesDescriptor("Kernel MMIO / 32-bit", NcTextAttributes.NC_KERNEL_MMIO_32),
            AttributesDescriptor("Kernel MMIO / 64-bit", NcTextAttributes.NC_KERNEL_MMIO_64),
            AttributesDescriptor("Kernel MMIO / 128-bit", NcTextAttributes.NC_KERNEL_MMIO_128),
            AttributesDescriptor("Kernel MMIO / 256-bit", NcTextAttributes.NC_KERNEL_MMIO_256),
            AttributesDescriptor("Kernel MMIO / 512-bit", NcTextAttributes.NC_KERNEL_MMIO_512),
            AttributesDescriptor("Primitive Entity", NcTextAttributes.NC_PRIMITIVE_ENTITY),
            AttributesDescriptor("Physical Hardware Entity", NcTextAttributes.NC_HARDWARE_ENTITY),
            AttributesDescriptor("Machine Instruction", NcTextAttributes.NC_MACHINE_INSTRUCTION),
            AttributesDescriptor("Number", NcTextAttributes.NC_NUMBER),
            AttributesDescriptor("Number / basal", NcTextAttributes.NC_BASAL_NUMBER),
            AttributesDescriptor("Number / machine", NcTextAttributes.NC_MACHINE_NUMBER),
            AttributesDescriptor("Number / primitive", NcTextAttributes.NC_PRIMITIVE_NUMBER),
            AttributesDescriptor("Number / physical hardware", NcTextAttributes.NC_HARDWARE_NUMBER),
            AttributesDescriptor("Hex Number", NcTextAttributes.NC_HEX_NUMBER),
            AttributesDescriptor("Binary Number", NcTextAttributes.NC_BINARY_NUMBER),
            AttributesDescriptor("String", NcTextAttributes.NC_STRING),
            AttributesDescriptor("Char", NcTextAttributes.NC_CHAR),
            AttributesDescriptor("Comment", NcTextAttributes.NC_COMMENT),
            AttributesDescriptor("Operator", NcTextAttributes.NC_OPERATOR),
            AttributesDescriptor("Material write / <-", NcTextAttributes.NC_MATERIAL_WRITE),
            AttributesDescriptor("Diagnostic Error", NcTextAttributes.NC_DIAGNOSTIC_ERROR),
            AttributesDescriptor("Diagnostic Warning", NcTextAttributes.NC_DIAGNOSTIC_WARNING),
            AttributesDescriptor("Raw Hazard", NcTextAttributes.NC_RAW_HAZARD),
            AttributesDescriptor("Capability", NcTextAttributes.NC_CAPABILITY),
            AttributesDescriptor("Policy", NcTextAttributes.NC_POLICY),
            AttributesDescriptor("FFI", NcTextAttributes.NC_FFI),
            AttributesDescriptor("Copy", NcTextAttributes.NC_COPY),
            AttributesDescriptor("Clone", NcTextAttributes.NC_CLONE)
        )
    }
}
