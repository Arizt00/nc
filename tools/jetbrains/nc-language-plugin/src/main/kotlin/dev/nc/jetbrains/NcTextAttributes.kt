package dev.nc.jetbrains

import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.HighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.editor.markup.TextAttributes
import com.intellij.ui.JBColor
import java.awt.Color

object NcTextAttributes {
    private fun rgb(r: Int, g: Int, b: Int): TextAttributes {
        val c = JBColor(Color(r, g, b), Color(r, g, b))
        return TextAttributes(c, null, null, null, 0)
    }

    private fun pair(lightHex: Int, darkHex: Int): TextAttributes {
        val c = JBColor(Color(lightHex), Color(darkHex))
        return TextAttributes(c, null, null, null, 0)
    }

    val NC_KEYWORD: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KEYWORD",
        rgb(248, 152, 32)
    )
    val NC_IMPORT_KEYWORD: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_IMPORT_KEYWORD",
        pair(0x0284C7, 0x7DD3FC)
    )
    val NC_CONTAINER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_CONTAINER",
        pair(0x16202B, 0xD6DEE8)
    )
    val NC_STC_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_STC_SECTION",
        pair(0x008C6A, 0x38D9A9)
    )
    val NC_FN_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_FN_SECTION",
        pair(0x2563EB, 0x60A5FA)
    )
    val NC_FA_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_FA_SECTION",
        pair(0xB45309, 0xFBBF24)
    )
    val NC_MAIN_FAMILY: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MAIN_FAMILY",
        pair(0x7C3AED, 0xA78BFA)
    )
    val NC_VISIBILITY: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_VISIBILITY",
        pair(0x9F1239, 0xFDA4AF)
    )
    val NC_AT_REFERENCE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_AT_REFERENCE",
        pair(0x006DCC, 0x4DA3FF)
    )
    val NC_AT_GROUP_REFERENCE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_AT_GROUP_REFERENCE",
        pair(0x0F766E, 0x2DD4BF)
    )
    val NC_PROJECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_PROJECTION",
        pair(0xC2410C, 0xF59E0B)
    )
    val NC_VERIFICATION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_VERIFICATION",
        pair(0xBE185D, 0xFF5C8A)
    )
    val NC_DEV: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_DEV",
        pair(0xDC2626, 0xFF4D6D)
    )
    val NC_RETURN: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_RETURN",
        pair(0x6D28D9, 0xC084FC)
    )
    val NC_MODE_RUNTIME: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MODE_RUNTIME",
        rgb(83, 130, 161)
    )
    val NC_MODE_COMPILER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MODE_COMPILER",
        rgb(248, 152, 32)
    )
    val NC_MODE_BASAL: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MODE_BASAL",
        rgb(222, 165, 132)
    )
    val NC_MODE_KERNEL: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MODE_KERNEL",
        rgb(206, 65, 43)
    )
    val NC_MODE_MACHINE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MODE_MACHINE",
        rgb(0, 89, 156)
    )
    val NC_MODE_PRIMITIVE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MODE_PRIMITIVE",
        rgb(206, 65, 43)
    )
    val NC_MODE_HARDWARE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MODE_HARDWARE",
        pair(0x9F1239, 0xFDA4AF)
    )
    val NC_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_SECTION",
        rgb(101, 154, 210)
    )
    val NC_SOURCE_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_SOURCE_SECTION",
        rgb(55, 118, 171)
    )
    val NC_BASAL_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_BASAL_SECTION",
        rgb(222, 165, 132)
    )
    val NC_MACHINE_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MACHINE_SECTION",
        rgb(243, 114, 44)
    )
    val NC_PRIMITIVE_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_PRIMITIVE_SECTION",
        rgb(168, 185, 204)
    )
    val NC_HARDWARE_SECTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_HARDWARE_SECTION",
        pair(0x9F1239, 0xFB7185)
    )
    val NC_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_TYPE",
        rgb(255, 212, 59)
    )
    val NC_BASIC_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_BASIC_TYPE",
        pair(0x1D4ED8, 0x93C5FD)
    )
    val NC_ADVANCED_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_ADVANCED_TYPE",
        pair(0x0E7490, 0x67E8F9)
    )
    val NC_GENERIC_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_GENERIC_TYPE",
        pair(0x7E22CE, 0xC4B5FD)
    )
    val NC_ARRAY_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_ARRAY_TYPE",
        pair(0x0F766E, 0x2DD4BF)
    )
    val NC_BASAL_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_BASAL_TYPE",
        rgb(249, 199, 79)
    )
    val NC_MACHINE_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MACHINE_TYPE",
        rgb(87, 117, 144)
    )
    val NC_PRIMITIVE_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_PRIMITIVE_TYPE",
        rgb(101, 154, 210)
    )
    val NC_HARDWARE_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_HARDWARE_TYPE",
        pair(0xBE123C, 0xFDA4AF)
    )
    val NC_POINTER_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_POINTER_TYPE",
        pair(0xBE123C, 0xFB7185)
    )
    val NC_MEM_REGION_TYPE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MEM_REGION_TYPE",
        pair(0xEA580C, 0xF97316)
    )
    val NC_BOOL: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_BOOL",
        pair(0xA21CAF, 0xF0ABFC)
    )
    val NC_FUNCTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_FUNCTION",
        rgb(55, 118, 171)
    )
    val NC_RUNTIME_FUNCTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_RUNTIME_FUNCTION",
        rgb(83, 130, 161)
    )
    val NC_PRINT_FUNCTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_PRINT_FUNCTION",
        pair(0x0284C7, 0x22D3EE)
    )
    val NC_POINTER_FUNCTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_POINTER_FUNCTION",
        pair(0xBE123C, 0xFB7185)
    )
    val NC_COLOR_FUNCTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_COLOR_FUNCTION",
        pair(0x4D7C0F, 0xA3E635)
    )
    val NC_CONTROL_FLOW: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_CONTROL_FLOW",
        pair(0xC2410C, 0xF59E0B)
    )
    val NC_LOOP_CONTROL: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_LOOP_CONTROL",
        pair(0xA16207, 0xFBBF24)
    )
    val NC_KERNEL_ADDRESS_KEYWORD: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KERNEL_ADDRESS_KEYWORD",
        pair(0x0F766E, 0x2DD4BF)
    )
    val NC_KERNEL_MMIO_8: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KERNEL_MMIO_8",
        pair(0xA16207, 0xFDE68A)
    )
    val NC_KERNEL_MMIO_16: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KERNEL_MMIO_16",
        pair(0xC2410C, 0xFDBA74)
    )
    val NC_KERNEL_MMIO_32: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KERNEL_MMIO_32",
        pair(0xBE123C, 0xFB7185)
    )
    val NC_KERNEL_MMIO_64: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KERNEL_MMIO_64",
        pair(0x7E22CE, 0xC084FC)
    )
    val NC_KERNEL_MMIO_128: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KERNEL_MMIO_128",
        pair(0x1D4ED8, 0x60A5FA)
    )
    val NC_KERNEL_MMIO_256: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KERNEL_MMIO_256",
        pair(0x0E7490, 0x22D3EE)
    )
    val NC_KERNEL_MMIO_512: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_KERNEL_MMIO_512",
        pair(0x4D7C0F, 0xA3E635)
    )
    val NC_PRIMITIVE_ENTITY: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_PRIMITIVE_ENTITY",
        rgb(206, 65, 43)
    )
    val NC_HARDWARE_ENTITY: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_HARDWARE_ENTITY",
        pair(0x9F1239, 0xFB7185)
    )
    val NC_MACHINE_INSTRUCTION: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MACHINE_INSTRUCTION",
        rgb(243, 114, 44)
    )
    val NC_MACHINE_REGISTER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MACHINE_REGISTER",
        rgb(249, 199, 79)
    )
    val NC_NUMBER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_NUMBER",
        pair(0xA16207, 0xFDE68A)
    )
    val NC_BASAL_NUMBER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_BASAL_NUMBER",
        rgb(249, 199, 79)
    )
    val NC_MACHINE_NUMBER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MACHINE_NUMBER",
        rgb(249, 132, 74)
    )
    val NC_PRIMITIVE_NUMBER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_PRIMITIVE_NUMBER",
        rgb(249, 65, 68)
    )
    val NC_HARDWARE_NUMBER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_HARDWARE_NUMBER",
        pair(0xBE123C, 0xFDA4AF)
    )
    val NC_HEX_NUMBER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_HEX_NUMBER",
        rgb(222, 165, 132)
    )
    val NC_BINARY_NUMBER: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_BINARY_NUMBER",
        rgb(168, 185, 204)
    )
    val NC_STRING: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_STRING",
        pair(0x4D7C0F, 0xA3E635)
    )
    val NC_CHAR: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_CHAR",
        pair(0x4D7C0F, 0xA3E635)
    )
    val NC_COMMENT: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_COMMENT",
        DefaultLanguageHighlighterColors.BLOCK_COMMENT
    )
    val NC_OPERATOR: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_OPERATOR",
        rgb(248, 152, 32)
    )
    val NC_MATERIAL_WRITE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_MATERIAL_WRITE",
        pair(0x0369A1, 0x38BDF8)
    )
    val NC_DIAGNOSTIC_ERROR: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_DIAGNOSTIC_ERROR",
        TextAttributes(
            JBColor(Color(206, 65, 43), Color(206, 65, 43)),
            null,
            null,
            null,
            0
        )
    )
    val NC_DIAGNOSTIC_WARNING: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_DIAGNOSTIC_WARNING",
        rgb(248, 152, 32)
    )
    val NC_RAW_HAZARD: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_RAW_HAZARD",
        rgb(206, 65, 43)
    )
    val NC_CAPABILITY: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_CAPABILITY",
        rgb(222, 165, 132)
    )
    val NC_POLICY: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_POLICY",
        rgb(248, 152, 32)
    )
    val NC_FFI: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_FFI",
        rgb(168, 185, 204)
    )
    val NC_COPY: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_COPY",
        rgb(101, 154, 210)
    )
    val NC_CLONE: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_CLONE",
        rgb(83, 130, 161)
    )

    val NC_FALLBACK: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_FALLBACK",
        DefaultLanguageHighlighterColors.IDENTIFIER
    )

    val NC_BAD: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "NC_BAD",
        HighlighterColors.BAD_CHARACTER
    )
}
