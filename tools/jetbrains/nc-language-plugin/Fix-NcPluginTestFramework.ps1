[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$PluginRoot,

    [switch]$Build,
    [switch]$RefreshDependencies
)

$ErrorActionPreference = "Stop"

function Insert-AfterOpeningBrace {
    param(
        [Parameter(Mandatory)]
        [string]$Text,

        [Parameter(Mandatory)]
        [System.Text.RegularExpressions.Match]$BlockMatch,

        [Parameter(Mandatory)]
        [string]$Insertion
    )

    if (-not $BlockMatch.Success) {
        throw "No se encontró el bloque que debía modificarse."
    }

    $openBrace = $Text.IndexOf('{', $BlockMatch.Index)
    if ($openBrace -lt 0) {
        throw "El bloque localizado no contiene una llave de apertura."
    }

    return $Text.Substring(0, $openBrace + 1) + $Insertion + $Text.Substring($openBrace + 1)
}

$root = (Resolve-Path $PluginRoot).Path
$buildFile = Join-Path $root "build.gradle.kts"
$gradleWrapper = Join-Path $root "gradlew.bat"

if (-not (Test-Path $buildFile)) {
    throw "No se encontró build.gradle.kts en: $root"
}

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backup = "$buildFile.navigation-tests-$timestamp.bak"
Copy-Item $buildFile $backup -Force

$text = [System.IO.File]::ReadAllText($buildFile)
$newLine = if ($text.Contains("`r`n")) { "`r`n" } else { "`n" }

$requiredImport = "import org.jetbrains.intellij.platform.gradle.TestFrameworkType"
if ($text -notmatch '(?m)^\s*import\s+org\.jetbrains\.intellij\.platform\.gradle\.TestFrameworkType\s*$') {
    $text = $requiredImport + $newLine + $newLine + $text
}

$dependenciesMatch = [regex]::Match(
    $text,
    '(?m)^\s*dependencies\s*\{'
)

if (-not $dependenciesMatch.Success) {
    throw "build.gradle.kts no contiene un bloque dependencies { ... }."
}

$dependencyInsertion = ""

if ($text -notmatch 'testImplementation\s*\(\s*"junit:junit:4\.13\.2"\s*\)') {
    $dependencyInsertion += $newLine + '    testImplementation("junit:junit:4.13.2")'
}

if ($text -notmatch 'testImplementation\s*\(\s*"org\.opentest4j:opentest4j:1\.3\.0"\s*\)') {
    $dependencyInsertion += $newLine + '    testImplementation("org.opentest4j:opentest4j:1.3.0")'
}

if ($dependencyInsertion.Length -gt 0) {
    $dependencyInsertion += $newLine
    $text = Insert-AfterOpeningBrace `
        -Text $text `
        -BlockMatch $dependenciesMatch `
        -Insertion $dependencyInsertion
}

if ($text -notmatch 'testFramework\s*\(\s*TestFrameworkType\.Platform\s*\)') {
    # Se vuelve a buscar después de modificar el texto.
    $dependenciesMatch = [regex]::Match(
        $text,
        '(?m)^\s*dependencies\s*\{'
    )

    $searchStart = $dependenciesMatch.Index + $dependenciesMatch.Length
    $remaining = $text.Substring($searchStart)
    $relativePlatformMatch = [regex]::Match(
        $remaining,
        '(?m)^\s*intellijPlatform\s*\{'
    )

    if (-not $relativePlatformMatch.Success) {
        throw @"
No se encontró dependencies { intellijPlatform { ... } }.
Añade manualmente dentro de dependencies/intellijPlatform:
    testFramework(TestFrameworkType.Platform)
"@
    }

    # El overload anterior comienza desde cero. Ajustamos al bloque encontrado
    # dentro de dependencies mediante un Match sintético equivalente.
    $platformAbsoluteIndex = $searchStart + $relativePlatformMatch.Index
    $platformMatch = [regex]::Match(
        $text.Substring($platformAbsoluteIndex),
        '(?m)^\s*intellijPlatform\s*\{'
    )

    $absoluteMatch = [pscustomobject]@{
        Success = $platformMatch.Success
        Index = $platformAbsoluteIndex + $platformMatch.Index
    }

    if (-not $absoluteMatch.Success) {
        throw "No se pudo localizar el bloque intellijPlatform dentro de dependencies."
    }

    $openBrace = $text.IndexOf('{', $absoluteMatch.Index)
    if ($openBrace -lt 0) {
        throw "El bloque intellijPlatform no contiene una llave de apertura."
    }

    $insertion = (
        $newLine +
        "        testFramework(TestFrameworkType.Platform)" +
        $newLine
    )

    $text = $text.Substring(0, $openBrace + 1) + $insertion + $text.Substring($openBrace + 1)
}

$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($buildFile, $text, $utf8NoBom)

Write-Host "Configuración de pruebas aplicada." -ForegroundColor Green
Write-Host "Archivo: $buildFile"
Write-Host "Copia:   $backup"

Write-Host "`nEntradas verificadas:" -ForegroundColor Cyan
Select-String `
    -Path $buildFile `
    -Pattern 'TestFrameworkType|testFramework\(|junit:junit|opentest4j' |
    ForEach-Object {
        Write-Host ("  {0}: {1}" -f $_.LineNumber, $_.Line.Trim())
    }

if ($Build) {
    if (-not (Test-Path $gradleWrapper)) {
        throw "No se encontró gradlew.bat en: $root"
    }

    Push-Location $root
    try {
        $arguments = @("clean", "test", "buildPlugin")
        if ($RefreshDependencies) {
            $arguments += "--refresh-dependencies"
        }

        & $gradleWrapper @arguments
        if ($LASTEXITCODE -ne 0) {
            exit $LASTEXITCODE
        }

        $zip = Get-ChildItem (Join-Path $root "build\distributions\*.zip") |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1

        if ($null -eq $zip) {
            throw "La compilación terminó, pero no se encontró el ZIP de distribución."
        }

        Write-Host "`nPlugin generado:" -ForegroundColor Green
        $zip | Format-List FullName, Length, LastWriteTime
    }
    finally {
        Pop-Location
    }
}
