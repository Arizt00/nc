[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$PluginRoot,

    [switch]$SkipTests,
    [switch]$Build,
    [switch]$RefreshDependencies
)

$ErrorActionPreference = "Stop"
$BundleRoot = Split-Path $PSScriptRoot -Parent
$PackagePath = "dev\nc\jetbrains"
$MainSource = Join-Path $BundleRoot "src\main\kotlin\$PackagePath"
$TestSource = Join-Path $BundleRoot "src\test\kotlin\$PackagePath"
$MainTarget = Join-Path $PluginRoot "src\main\kotlin\$PackagePath"
$TestTarget = Join-Path $PluginRoot "src\test\kotlin\$PackagePath"
$PluginXml = Join-Path $PluginRoot "src\main\resources\META-INF\plugin.xml"
$Timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$Backup = Join-Path $PluginRoot "navigation-backup-$Timestamp"

if (-not (Test-Path $PluginRoot)) {
    throw "No existe PluginRoot: $PluginRoot"
}
if (-not (Test-Path $MainSource)) {
    throw "No se encontró el código fuente del paquete: $MainSource"
}
if (-not (Test-Path $PluginXml)) {
    throw "No se encontró plugin.xml: $PluginXml"
}

New-Item $Backup -ItemType Directory -Force | Out-Null
if (Test-Path $MainTarget) {
    Copy-Item $MainTarget (Join-Path $Backup "main") -Recurse -Force
}
Copy-Item $PluginXml (Join-Path $Backup "plugin.xml") -Force
$BuildFile = Join-Path $PluginRoot "build.gradle.kts"
if (Test-Path $BuildFile) { Copy-Item $BuildFile (Join-Path $Backup "build.gradle.kts") -Force }

New-Item $MainTarget -ItemType Directory -Force | Out-Null
Copy-Item (Join-Path $MainSource "*.kt") $MainTarget -Force

if (-not $SkipTests) {
    New-Item $TestTarget -ItemType Directory -Force | Out-Null
    Copy-Item (Join-Path $TestSource "*.kt") $TestTarget -Force

    $FixTests = Join-Path $PSScriptRoot "Fix-NcPluginTestFramework.ps1"
    & $FixTests -PluginRoot $PluginRoot
}

$XmlText = [System.IO.File]::ReadAllText($PluginXml)
$HandlerPattern = '<gotoDeclarationHandler\s+implementation="[^"]+"\s*/>'
$HandlerReplacement = '<gotoDeclarationHandler implementation="dev.nc.jetbrains.NcGotoDeclarationHandler"/>'

if ([regex]::IsMatch($XmlText, $HandlerPattern)) {
    $XmlText = [regex]::Replace($XmlText, $HandlerPattern, $HandlerReplacement, 1)
} elseif ($XmlText -match '<extensions\s+defaultExtensionNs="com\.intellij"[^>]*>') {
    $XmlText = [regex]::Replace(
        $XmlText,
        '(<extensions\s+defaultExtensionNs="com\.intellij"[^>]*>)',
        "`$1`r`n        $HandlerReplacement",
        1
    )
} else {
    throw "plugin.xml no contiene <extensions defaultExtensionNs=`"com.intellij`">. Añade manualmente el fragmento incluido."
}

$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($PluginXml, $XmlText, $Utf8NoBom)

Write-Host "Navegación instalada. Copia de seguridad: $Backup"

if ($Build) {
    $Gradle = Join-Path $PluginRoot "gradlew.bat"
    if (-not (Test-Path $Gradle)) {
        throw "No se encontró gradlew.bat en $PluginRoot"
    }

    Push-Location $PluginRoot
    try {
        $GradleArgs = @("clean")
        if (-not $SkipTests) { $GradleArgs += "test" }
        $GradleArgs += "buildPlugin"
        if ($RefreshDependencies) { $GradleArgs += "--refresh-dependencies" }

        & $Gradle @GradleArgs
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }

        Get-ChildItem (Join-Path $PluginRoot "build\distributions\*.zip") |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1 |
            Format-List FullName, Length, LastWriteTime
    }
    finally {
        Pop-Location
    }
}
