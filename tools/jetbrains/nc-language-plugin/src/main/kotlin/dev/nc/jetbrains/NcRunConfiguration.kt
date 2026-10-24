package dev.nc.jetbrains

import com.intellij.execution.ExecutionException
import com.intellij.execution.Executor
import com.intellij.execution.configurations.CommandLineState
import com.intellij.execution.configurations.ConfigurationFactory
import com.intellij.execution.configurations.ConfigurationTypeBase
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.configurations.LocatableConfigurationBase
import com.intellij.execution.configurations.RunProfileState
import com.intellij.execution.configurations.RuntimeConfigurationError
import com.intellij.execution.configurations.RuntimeConfigurationException
import com.intellij.execution.executors.DefaultDebugExecutor
import com.intellij.execution.process.ColoredProcessHandler
import com.intellij.execution.process.ProcessHandler
import com.intellij.execution.runners.ExecutionEnvironment
import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.NlsContexts
import com.intellij.openapi.util.WriteExternalException
import org.jdom.Element
import java.awt.GridBagConstraints
import java.awt.GridBagLayout
import javax.swing.JCheckBox
import javax.swing.JComboBox
import javax.swing.JComponent
import javax.swing.JLabel
import javax.swing.JPanel
import javax.swing.JTextField

class NcRunConfigurationType : ConfigurationTypeBase(
    ID,
    "nC",
    "Run or debug an nC source file through ncc",
    NcIcons.Plugin
) {
    init {
        addFactory(NcRunConfigurationFactory(this))
    }

    companion object {
        const val ID = "NcRunConfigurationType"
    }
}

class NcRunConfigurationFactory(type: NcRunConfigurationType) : ConfigurationFactory(type) {
    override fun createTemplateConfiguration(project: Project) = NcRunConfiguration(project, this, "nC Run")

    override fun getId(): String = "NcRunConfigurationFactory"
}

class NcRunConfiguration(
    project: Project,
    factory: ConfigurationFactory,
    name: String
) : LocatableConfigurationBase<RunProfileState>(project, factory, name) {
    var filePath: String = ""
    var programArguments: String = ""
    var profile: String = "debug"
    var targetTriple: String = ""
    var ccProfile: String = ""
    var entrySymbol: String = ""
    var linkerPath: String = ""
    var showReturn: Boolean = false
    var debugRun: Boolean = false

    override fun getConfigurationEditor(): SettingsEditor<out com.intellij.execution.configurations.RunConfiguration> {
        return NcRunSettingsEditor()
    }

    @Throws(RuntimeConfigurationException::class)
    override fun checkConfiguration() {
        if (!NcRunSupport.isRunnablePath(filePath)) {
            throw RuntimeConfigurationError("Select a runnable nC file: .n, .nb, .nbb or .ncp. Start .nm from a terminal.")
        }
    }

    override fun getState(executor: Executor, environment: ExecutionEnvironment): RunProfileState {
        return NcCommandLineState(environment, this, executor.id == DefaultDebugExecutor.EXECUTOR_ID || debugRun)
    }

    override fun readExternal(element: Element) {
        super.readExternal(element)
        filePath = element.getAttributeValue("filePath") ?: ""
        programArguments = element.getAttributeValue("programArguments") ?: ""
        profile = element.getAttributeValue("profile") ?: "debug"
        targetTriple = element.getAttributeValue("targetTriple") ?: ""
        ccProfile = element.getAttributeValue("ccProfile") ?: ""
        entrySymbol = element.getAttributeValue("entrySymbol") ?: ""
        linkerPath = element.getAttributeValue("linkerPath") ?: ""
        showReturn = element.getAttributeValue("showReturn") == "true"
        debugRun = element.getAttributeValue("debugRun") == "true"
    }

    @Throws(WriteExternalException::class)
    override fun writeExternal(element: Element) {
        super.writeExternal(element)
        element.setAttribute("filePath", filePath)
        element.setAttribute("programArguments", programArguments)
        element.setAttribute("profile", profile)
        element.setAttribute("targetTriple", targetTriple)
        element.setAttribute("ccProfile", ccProfile)
        element.setAttribute("entrySymbol", entrySymbol)
        element.setAttribute("linkerPath", linkerPath)
        element.setAttribute("showReturn", showReturn.toString())
        element.setAttribute("debugRun", debugRun.toString())
    }
}

private class NcCommandLineState(
    environment: ExecutionEnvironment,
    private val configuration: NcRunConfiguration,
    private val debug: Boolean
) : CommandLineState(environment) {
    @Throws(ExecutionException::class)
    override fun startProcess(): ProcessHandler {
        val toolchain = try {
            NcRunSupport.resolveToolchain(environment.project)
        } catch (error: Exception) {
            throw ExecutionException(error.message ?: "Unable to resolve a compatible ncc toolchain", error)
        }
        val commandLine = GeneralCommandLine(toolchain.nccPath)
        commandLine.withWorkDirectory(NcRunSupport.projectRoot(environment.project))
        commandLine.withParentEnvironmentType(GeneralCommandLine.ParentEnvironmentType.CONSOLE)
        commandLine.withEnvironment("NCC_IDE", "jetbrains")
        commandLine.withEnvironment("NCC_IDE_TOOLCHAIN_SOURCE", toolchain.source)
        toolchain.toolchainId?.let { commandLine.withEnvironment("NCC_IDE_TOOLCHAIN_ID", it) }

        commandLine.addParameter("run")
        commandLine.addParameter(configuration.filePath)
        addProfile(commandLine, configuration.profile)
        if (configuration.ccProfile.isNotBlank()) {
            commandLine.addParameters("--cc", configuration.ccProfile.trim())
        }
        if (configuration.entrySymbol.isNotBlank()) {
            commandLine.addParameters("--entry", configuration.entrySymbol.trim())
        }
        if (configuration.linkerPath.isNotBlank()) {
            commandLine.addParameters("--linker", configuration.linkerPath.trim())
        }
        if (configuration.showReturn) {
            commandLine.addParameter("--show-return")
        }
        if (debug) {
            commandLine.withEnvironment("NCC_IDE_DEBUG", "1")
            commandLine.withEnvironment("NCC_DEBUGGER_FRONTEND", "jetbrains")
        }
        if (configuration.targetTriple.isNotBlank()) {
            commandLine.withEnvironment("NCC_TARGET_TRIPLE", configuration.targetTriple.trim())
        }
        splitArguments(configuration.programArguments).forEach { commandLine.addParameter(it) }
        return ColoredProcessHandler(commandLine)
    }

    private fun addProfile(commandLine: GeneralCommandLine, profile: String) {
        when (profile) {
            "interpreted" -> commandLine.addParameter("--interpreted")
            "real", "native-fast" -> commandLine.addParameter("--native-fast")
            "native-ffi" -> commandLine.addParameter("--native-ffi")
            "debug" -> {}
            else -> commandLine.addParameters("--profile", profile)
        }
    }
}

private fun splitArguments(text: String): List<String> {
    if (text.isBlank()) return emptyList()
    val result = mutableListOf<String>()
    val current = StringBuilder()
    var quote: Char? = null
    var escaping = false
    for (ch in text) {
        when {
            escaping -> {
                current.append(ch)
                escaping = false
            }
            ch == '\\' -> escaping = true
            quote != null && ch == quote -> quote = null
            quote == null && (ch == '\'' || ch == '"') -> quote = ch
            quote == null && ch.isWhitespace() -> {
                if (current.isNotEmpty()) {
                    result.add(current.toString())
                    current.clear()
                }
            }
            else -> current.append(ch)
        }
    }
    if (escaping) current.append('\\')
    if (current.isNotEmpty()) result.add(current.toString())
    return result
}

private class NcRunSettingsEditor : SettingsEditor<NcRunConfiguration>() {
    private val filePath = JTextField()
    private val args = JTextField()
    private val profile = JComboBox(
        arrayOf("debug", "interpreted", "real", "native-fast", "native-ffi", "release", "release-fast", "release-small", "unsafe-fast", "instrumented")
    )
    private val targetTriple = JTextField()
    private val ccProfile = JTextField()
    private val entrySymbol = JTextField()
    private val linkerPath = JTextField()
    private val showReturn = JCheckBox("Show return value")
    private val debugRun = JCheckBox("Enable nC debug environment")

    override fun resetEditorFrom(configuration: NcRunConfiguration) {
        filePath.text = configuration.filePath
        args.text = configuration.programArguments
        profile.selectedItem = configuration.profile
        targetTriple.text = configuration.targetTriple
        ccProfile.text = configuration.ccProfile
        entrySymbol.text = configuration.entrySymbol
        linkerPath.text = configuration.linkerPath
        showReturn.isSelected = configuration.showReturn
        debugRun.isSelected = configuration.debugRun
    }

    override fun applyEditorTo(configuration: NcRunConfiguration) {
        configuration.filePath = filePath.text.trim()
        configuration.programArguments = args.text.trim()
        configuration.profile = (profile.selectedItem as? String) ?: "debug"
        configuration.targetTriple = targetTriple.text.trim()
        configuration.ccProfile = ccProfile.text.trim()
        configuration.entrySymbol = entrySymbol.text.trim()
        configuration.linkerPath = linkerPath.text.trim()
        configuration.showReturn = showReturn.isSelected
        configuration.debugRun = debugRun.isSelected
    }

    override fun createEditor(): JComponent {
        val panel = JPanel(GridBagLayout())
        addRow(panel, 0, "File", filePath)
        addRow(panel, 1, "Arguments", args)
        addRow(panel, 2, "Profile", profile)
        addRow(panel, 3, "Target triple", targetTriple)
        addRow(panel, 4, "CC profile", ccProfile)
        addRow(panel, 5, "Entry symbol", entrySymbol)
        addRow(panel, 6, "Linker", linkerPath)
        addRow(panel, 7, "", showReturn)
        addRow(panel, 8, "", debugRun)
        return panel
    }

    private fun addRow(panel: JPanel, row: Int, @NlsContexts.Label label: String, component: JComponent) {
        val labelConstraints = GridBagConstraints()
        labelConstraints.gridx = 0
        labelConstraints.gridy = row
        labelConstraints.anchor = GridBagConstraints.WEST
        labelConstraints.insets.set(4, 4, 4, 8)
        panel.add(JLabel(label), labelConstraints)

        val fieldConstraints = GridBagConstraints()
        fieldConstraints.gridx = 1
        fieldConstraints.gridy = row
        fieldConstraints.weightx = 1.0
        fieldConstraints.fill = GridBagConstraints.HORIZONTAL
        fieldConstraints.insets.set(4, 0, 4, 4)
        panel.add(component, fieldConstraints)
    }
}
