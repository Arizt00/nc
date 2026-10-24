package dev.nc.jetbrains

import com.intellij.lexer.Lexer
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighterBase
import com.intellij.psi.tree.IElementType

class NcSyntaxHighlighter(private val mode: NcVisualMode = NcVisualMode.Source) : SyntaxHighlighterBase() {
    override fun getHighlightingLexer(): Lexer = NcSimpleLexer()

    override fun getTokenHighlights(tokenType: IElementType): Array<TextAttributesKey> {
        return when (tokenType) {
            NcTokenTypes.KEYWORD -> pack(NcTextAttributes.NC_KEYWORD)
            NcTokenTypes.IMPORT_KEYWORD -> pack(NcTextAttributes.NC_IMPORT_KEYWORD)
            NcTokenTypes.IMPORT_SELECTOR -> pack(NcTextAttributes.NC_RUNTIME_FUNCTION)
            NcTokenTypes.RETURN_KEYWORD -> pack(NcTextAttributes.NC_RETURN)
            NcTokenTypes.DEV_CALL -> pack(NcTextAttributes.NC_DEV)
            NcTokenTypes.MAIN_FAMILY -> pack(NcTextAttributes.NC_MAIN_FAMILY)
            NcTokenTypes.VISIBILITY -> pack(NcTextAttributes.NC_VISIBILITY)
            NcTokenTypes.MODE_RUNTIME -> pack(NcTextAttributes.NC_MODE_RUNTIME)
            NcTokenTypes.MODE_COMPILER -> pack(NcTextAttributes.NC_MODE_COMPILER)
            NcTokenTypes.MODE_BASAL -> pack(NcTextAttributes.NC_MODE_BASAL)
            NcTokenTypes.MODE_KERNEL -> pack(NcTextAttributes.NC_MODE_KERNEL)
            NcTokenTypes.MODE_MACHINE -> pack(NcTextAttributes.NC_MODE_MACHINE)
            NcTokenTypes.MODE_PRIMITIVE -> pack(if (mode == NcVisualMode.Hardware) NcTextAttributes.NC_MODE_HARDWARE else NcTextAttributes.NC_MODE_PRIMITIVE)
            NcTokenTypes.MODE_OTHER -> pack(NcTextAttributes.NC_KEYWORD)
            NcTokenTypes.SECTION -> pack(sectionKey())
            NcTokenTypes.DOMAIN_STATE -> when (mode) {
                NcVisualMode.Basal -> pack(NcTextAttributes.NC_BASAL_SECTION)
                NcVisualMode.Primitive -> pack(NcTextAttributes.NC_PRIMITIVE_SECTION)
                NcVisualMode.Hardware -> pack(NcTextAttributes.NC_HARDWARE_SECTION)
                else -> pack(NcTextAttributes.NC_BAD)
            }
            NcTokenTypes.CONTAINER -> pack(NcTextAttributes.NC_CONTAINER)
            NcTokenTypes.STC_SECTION -> pack(NcTextAttributes.NC_STC_SECTION)
            NcTokenTypes.FN_SECTION -> pack(NcTextAttributes.NC_FN_SECTION)
            NcTokenTypes.FA_SECTION -> pack(NcTextAttributes.NC_FA_SECTION)
            NcTokenTypes.TYPE -> pack(typeKey())
            NcTokenTypes.BASIC_TYPE -> pack(NcTextAttributes.NC_BASIC_TYPE)
            NcTokenTypes.ADVANCED_TYPE -> pack(NcTextAttributes.NC_ADVANCED_TYPE)
            NcTokenTypes.GENERIC_TYPE -> pack(NcTextAttributes.NC_GENERIC_TYPE)
            NcTokenTypes.ARRAY_TYPE -> pack(NcTextAttributes.NC_ARRAY_TYPE)
            NcTokenTypes.POINTER_TYPE -> pack(NcTextAttributes.NC_POINTER_TYPE)
            NcTokenTypes.MEM_REGION_TYPE -> pack(NcTextAttributes.NC_MEM_REGION_TYPE)
            NcTokenTypes.BOOL -> pack(NcTextAttributes.NC_BOOL)
            NcTokenTypes.RUNTIME_FAMILY -> pack(NcTextAttributes.NC_RUNTIME_FUNCTION)
            NcTokenTypes.PRINT_FUNCTION -> pack(NcTextAttributes.NC_PRINT_FUNCTION)
            NcTokenTypes.POINTER_FUNCTION -> pack(NcTextAttributes.NC_POINTER_FUNCTION)
            NcTokenTypes.COLOR_FUNCTION -> pack(NcTextAttributes.NC_COLOR_FUNCTION)
            NcTokenTypes.CONTROL_FLOW -> pack(NcTextAttributes.NC_CONTROL_FLOW)
            NcTokenTypes.LOOP_CONTROL -> pack(NcTextAttributes.NC_LOOP_CONTROL)
            NcTokenTypes.KERNEL_ADDRESS_KEYWORD -> pack(NcTextAttributes.NC_KERNEL_ADDRESS_KEYWORD)
            NcTokenTypes.KERNEL_MMIO_8 -> pack(NcTextAttributes.NC_KERNEL_MMIO_8)
            NcTokenTypes.KERNEL_MMIO_16 -> pack(NcTextAttributes.NC_KERNEL_MMIO_16)
            NcTokenTypes.KERNEL_MMIO_32 -> pack(NcTextAttributes.NC_KERNEL_MMIO_32)
            NcTokenTypes.KERNEL_MMIO_64 -> pack(NcTextAttributes.NC_KERNEL_MMIO_64)
            NcTokenTypes.KERNEL_MMIO_128 -> pack(NcTextAttributes.NC_KERNEL_MMIO_128)
            NcTokenTypes.KERNEL_MMIO_256 -> pack(NcTextAttributes.NC_KERNEL_MMIO_256)
            NcTokenTypes.KERNEL_MMIO_512 -> pack(NcTextAttributes.NC_KERNEL_MMIO_512)
            NcTokenTypes.PRIMITIVE_ENTITY -> when (mode) {
                NcVisualMode.Primitive -> pack(NcTextAttributes.NC_PRIMITIVE_ENTITY)
                NcVisualMode.Hardware -> pack(NcTextAttributes.NC_HARDWARE_ENTITY)
                else -> pack(NcTextAttributes.NC_FALLBACK)
            }
            NcTokenTypes.MACHINE_INSTRUCTION -> if (mode == NcVisualMode.Machine || mode == NcVisualMode.BackendObject) pack(NcTextAttributes.NC_MACHINE_INSTRUCTION) else pack(NcTextAttributes.NC_FALLBACK)
            NcTokenTypes.MACHINE_REGISTER -> if (mode == NcVisualMode.Machine || mode == NcVisualMode.BackendObject) pack(NcTextAttributes.NC_MACHINE_REGISTER) else pack(NcTextAttributes.NC_FALLBACK)
            NcTokenTypes.NUMBER -> pack(numberKey())
            NcTokenTypes.HEX_NUMBER -> pack(numberKey())
            NcTokenTypes.BINARY_NUMBER -> pack(numberKey())
            NcTokenTypes.STRING -> pack(NcTextAttributes.NC_STRING)
            NcTokenTypes.CHAR -> pack(NcTextAttributes.NC_CHAR)
            NcTokenTypes.COMMENT -> pack(NcTextAttributes.NC_COMMENT)
            NcTokenTypes.OPERATOR -> pack(NcTextAttributes.NC_OPERATOR)
            NcTokenTypes.MATERIAL_WRITE_OPERATOR -> pack(NcTextAttributes.NC_MATERIAL_WRITE)
            NcTokenTypes.REMOVED_OPERATOR -> pack(NcTextAttributes.NC_BAD)
            NcTokenTypes.LBRACE -> pack(NcTextAttributes.NC_OPERATOR)
            NcTokenTypes.RBRACE -> pack(NcTextAttributes.NC_OPERATOR)
            NcTokenTypes.LPAREN -> pack(NcTextAttributes.NC_OPERATOR)
            NcTokenTypes.RPAREN -> pack(NcTextAttributes.NC_OPERATOR)
            NcTokenTypes.LBRACKET -> pack(NcTextAttributes.NC_OPERATOR)
            NcTokenTypes.RBRACKET -> pack(NcTextAttributes.NC_OPERATOR)
            NcTokenTypes.AT_REFERENCE -> pack(NcTextAttributes.NC_AT_REFERENCE)
            NcTokenTypes.AT_GROUP_REFERENCE -> pack(NcTextAttributes.NC_AT_GROUP_REFERENCE)
            NcTokenTypes.PROJECTION_OPERATOR -> pack(NcTextAttributes.NC_PROJECTION)
            NcTokenTypes.VERIFICATION_OPERATOR -> pack(NcTextAttributes.NC_VERIFICATION)
            NcTokenTypes.DIAGNOSTIC_ERROR -> pack(NcTextAttributes.NC_DIAGNOSTIC_ERROR)
            NcTokenTypes.RAW_HAZARD -> pack(NcTextAttributes.NC_RAW_HAZARD)
            NcTokenTypes.CAPABILITY -> pack(NcTextAttributes.NC_CAPABILITY)
            NcTokenTypes.POLICY -> pack(NcTextAttributes.NC_POLICY)
            NcTokenTypes.FFI -> pack(NcTextAttributes.NC_FFI)
            NcTokenTypes.COPY -> pack(NcTextAttributes.NC_COPY)
            NcTokenTypes.CLONE -> pack(NcTextAttributes.NC_CLONE)
            NcTokenTypes.IDENTIFIER -> pack(NcTextAttributes.NC_FALLBACK)
            NcTokenTypes.BAD_CHARACTER -> pack(NcTextAttributes.NC_BAD)
            else -> EMPTY_KEYS
        }
    }

    private fun sectionKey(): TextAttributesKey {
        return when (mode) {
            NcVisualMode.Basal -> NcTextAttributes.NC_BASAL_SECTION
            NcVisualMode.Machine -> NcTextAttributes.NC_MACHINE_SECTION
            NcVisualMode.BackendObject -> NcTextAttributes.NC_MACHINE_SECTION
            NcVisualMode.Primitive -> NcTextAttributes.NC_PRIMITIVE_SECTION
            NcVisualMode.Hardware -> NcTextAttributes.NC_HARDWARE_SECTION
            NcVisualMode.Source -> NcTextAttributes.NC_SOURCE_SECTION
        }
    }

    private fun typeKey(): TextAttributesKey {
        return when (mode) {
            NcVisualMode.Basal -> NcTextAttributes.NC_BASAL_TYPE
            NcVisualMode.Machine -> NcTextAttributes.NC_MACHINE_TYPE
            NcVisualMode.BackendObject -> NcTextAttributes.NC_MACHINE_TYPE
            NcVisualMode.Primitive -> NcTextAttributes.NC_PRIMITIVE_TYPE
            NcVisualMode.Hardware -> NcTextAttributes.NC_HARDWARE_TYPE
            NcVisualMode.Source -> NcTextAttributes.NC_TYPE
        }
    }

    private fun numberKey(): TextAttributesKey {
        return when (mode) {
            NcVisualMode.Basal -> NcTextAttributes.NC_BASAL_NUMBER
            NcVisualMode.Machine -> NcTextAttributes.NC_MACHINE_NUMBER
            NcVisualMode.BackendObject -> NcTextAttributes.NC_MACHINE_NUMBER
            NcVisualMode.Primitive -> NcTextAttributes.NC_PRIMITIVE_NUMBER
            NcVisualMode.Hardware -> NcTextAttributes.NC_HARDWARE_NUMBER
            NcVisualMode.Source -> NcTextAttributes.NC_NUMBER
        }
    }

    companion object {
        private val EMPTY_KEYS = emptyArray<TextAttributesKey>()
    }
}
