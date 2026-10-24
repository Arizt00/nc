package dev.nc.jetbrains

import com.intellij.openapi.util.IconLoader

object NcIcons {
    @JvmField
    val File = IconLoader.getIcon("/icons/nc-logo.svg", NcIcons::class.java)

    @JvmField
    val SourceFile = IconLoader.getIcon("/icons/nc-file-n.svg", NcIcons::class.java)

    @JvmField
    val BasalFile = IconLoader.getIcon("/icons/nc-file-nb.svg", NcIcons::class.java)

    @JvmField
    val PrimitiveFile = IconLoader.getIcon("/icons/nc-file-nbb.svg", NcIcons::class.java)

    @JvmField
    val MachineFile = IconLoader.getIcon("/icons/nc-file-nm.svg", NcIcons::class.java)

    @JvmField
    val BackendObjectFile = IconLoader.getIcon("/icons/nc-file-nbo.svg", NcIcons::class.java)

    @JvmField
    val HardwareFile = IconLoader.getIcon("/icons/nc-file-ncp.svg", NcIcons::class.java)

    @JvmField
    val Plugin = IconLoader.getIcon("/icons/nc-logo.svg", NcIcons::class.java)
}
