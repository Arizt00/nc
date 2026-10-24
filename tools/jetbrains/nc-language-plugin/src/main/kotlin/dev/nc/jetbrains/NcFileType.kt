package dev.nc.jetbrains

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.lang.Language
import javax.swing.Icon

object NcLanguage : Language("nC")

enum class NcVisualMode {
    Source,
    Basal,
    Machine,
    BackendObject,
    Primitive,
    Hardware
}

open class NcModeFileType(
    private val displayName: String,
    private val description: String,
    private val extension: String,
    private val icon: Icon
) : LanguageFileType(NcLanguage) {
    override fun getName(): String = displayName
    override fun getDisplayName(): String = displayName
    override fun getDescription(): String = description
    override fun getDefaultExtension(): String = extension
    override fun getIcon(): Icon = icon

    companion object {
        fun modeForExtension(extension: String?): NcVisualMode {
            return when ((extension ?: "").lowercase()) {
                "nb" -> NcVisualMode.Basal
                "nbb" -> NcVisualMode.Primitive
                "nm" -> NcVisualMode.Machine
                "nbo" -> NcVisualMode.BackendObject
                "ncp" -> NcVisualMode.Hardware
                else -> NcVisualMode.Source
            }
        }
    }
}

class NcFileType private constructor() : NcModeFileType(
    "nC Source",
    "nC high-level/runtime/compiler source",
    "n",
    NcIcons.SourceFile
) {
    companion object { @JvmField val INSTANCE = NcFileType() }
}

class NcRootFileType private constructor() : NcModeFileType(
    "nC Basal Root",
    "nC basal/root source",
    "nb",
    NcIcons.BasalFile
) {
    companion object { @JvmField val INSTANCE = NcRootFileType() }
}

class NcPrimitiveFileType private constructor() : NcModeFileType(
    "nC Primitive",
    "nC primitive low-level source",
    "nbb",
    NcIcons.PrimitiveFile
) {
    companion object { @JvmField val INSTANCE = NcPrimitiveFileType() }
}

class NcMachineFileType private constructor() : NcModeFileType(
    "nC Machine",
    "nC machine/ASM source",
    "nm",
    NcIcons.MachineFile
) {
    companion object { @JvmField val INSTANCE = NcMachineFileType() }
}

class NcBackendObjectFileType private constructor() : NcModeFileType(
    "nC Backend Object IR",
    "Retained textual machine BackendObjectIR evidence",
    "nbo",
    NcIcons.BackendObjectFile
) {
    companion object { @JvmField val INSTANCE = NcBackendObjectFileType() }
}

class NcHardwareFileType private constructor() : NcModeFileType(
    "nC Physical Hardware",
    "nC physical hardware capability source",
    "ncp",
    NcIcons.HardwareFile
) {
    companion object { @JvmField val INSTANCE = NcHardwareFileType() }
}
