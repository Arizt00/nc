param(
    [string]$Workspace = (Resolve-Path (Join-Path $PSScriptRoot "../..")),
    [switch]$Force
)

$ErrorActionPreference = "Stop"

function Get-DiagnosticPhase([string]$Code) {
    if ($Code.StartsWith("E_HIGH_")) { return "high-level" }
    if ($Code.StartsWith("E_MACHINE_")) { return "machine" }
    if ($Code.StartsWith("E_PRIMITIVE_")) { return "primitive" }
    if ($Code.StartsWith("E_BASAL_")) { return "basal" }
    if ($Code.StartsWith("E_RUNTIME_")) { return "runtime" }
    if ($Code -match '^[EWNI]_MCT_') { return "mct" }
    if ($Code.StartsWith("E_PARSE_") -or $Code.StartsWith("E_PARSER_")) { return "parser" }
    if ($Code.StartsWith("E_LEXER_")) { return "lexer" }
    if ($Code.StartsWith("E_USE_")) { return "imports" }
    if ($Code.StartsWith("E_PROJECT_")) { return "project" }
    if ($Code -match '^[EWNI]_GHOR_') { return "ghor" }
    if ($Code.StartsWith("E_TYPE_")) { return "sema.type" }
    if ($Code.StartsWith("E_LAYOUT_")) { return "sema.layout" }
    if ($Code.StartsWith("E_STATIC_")) { return "sema.const" }
    if ($Code.StartsWith("E_PTR_")) { return "sema.pointer" }
    if ($Code.StartsWith("E_FFI_")) { return "ffi" }
    return "compiler"
}
$inputPath = Join-Path $Workspace "docs/generated/language/nc-language-spec.json"
$outputPath = Join-Path $Workspace "diagnostics/registry.json"

if ((Test-Path -LiteralPath $outputPath) -and -not $Force) {
    throw "Canonical registry already exists at '$outputPath'. Refusing to replace its authority without -Force."
}

$language = Get-Content -LiteralPath $inputPath -Raw | ConvertFrom-Json
$help = @{
    E_PARSER_EXPECTED_SEMICOLON_001 = "add ';' after the preceding declaration or statement"
    E_PARSER_EXPRESSION_001 = "write a value, call, place, literal or grouped expression"
    E_TYPE_CONTRACT_SECTION_001 = "declare a field or use a supported type-contract clause"
    E_LEXER_ESCAPE_001 = "use a documented escape or write the byte explicitly"
}
$overrides = @{
    E_TYPE_CONTRACT_SECTION_001 = @{
        phase = "parser.type"
        summary = "invalid type contract section"
    }
    E_PARSER_EXPECTED_SEMICOLON_001 = @{
        phase = "parser"
        summary = "expected semicolon after the preceding declaration or statement"
    }
    E_PARSER_EXPRESSION_001 = @{
        phase = "parser"
        summary = "invalid expression"
    }
    E_LEXER_ESCAPE_001 = @{
        phase = "lexer"
        summary = "invalid string escape"
    }
}

$entries = @($language.diagnostics | ForEach-Object {
    $code = [string]$_.code
    $override = if ($overrides.ContainsKey($code)) { $overrides[$code] } else { $null }
    [pscustomobject][ordered]@{
        code = $code
        phase = if ($null -ne $override) {
            $override.phase
        } elseif ($code -match '^[EWNI]_GHOR_') {
            "ghor"
        } else {
            [string]$_.phase
        }
        summary = if ($null -ne $override) { $override.summary } else { [string]$_.summary }
        help = if ($help.ContainsKey($code)) { $help[$code] } else { "" }
    }
})

$entries += @(
    [pscustomobject][ordered]@{ code = "NCC-CLI-0001"; phase = "tooling.cli"; summary = "A command invocation violates its declared interface."; help = "inspect the command help and provide the required arguments" }
    [pscustomobject][ordered]@{ code = "NCC-CLI-0002"; phase = "tooling.cli"; summary = "A command action is not available in the selected family."; help = "select an action listed by the command help" }
    [pscustomobject][ordered]@{ code = "NCC-CLI-0003"; phase = "tooling.cli"; summary = "A command option has an invalid value."; help = "use one of the values listed by the command help" }
    [pscustomobject][ordered]@{ code = "NCC-DEPRECATION-0001"; phase = "compiler.deprecation"; summary = "The source uses a removed or deprecated language surface."; help = "migrate to the canonical compact spelling" }
    [pscustomobject][ordered]@{ code = "NCC-DEPRECATION-0002"; phase = "compiler.deprecation"; summary = "The source uses a compatibility surface scheduled for removal."; help = "replace the compatibility spelling with its canonical form" }
    [pscustomobject][ordered]@{ code = "NCC-DX-0101"; phase = "tooling.dx"; summary = "Project discovery failed."; help = "open a valid nC project or provide its manifest path" }
    [pscustomobject][ordered]@{ code = "NCC-DX-0151"; phase = "tooling.dx"; summary = "Project configuration is incomplete."; help = "inspect bridge.json and the project metadata" }
    [pscustomobject][ordered]@{ code = "NCC-DX-0201"; phase = "tooling.dx"; summary = "Source analysis failed."; help = "inspect the canonical source diagnostic" }
    [pscustomobject][ordered]@{ code = "NCC-DX-0301"; phase = "tooling.dx"; summary = "Build orchestration failed."; help = "inspect the retained build artifacts and causes" }
    [pscustomobject][ordered]@{ code = "NCC-DX-0401"; phase = "tooling.dx"; summary = "Run orchestration failed."; help = "inspect the selected target, profile and executable" }
    [pscustomobject][ordered]@{ code = "NCC-DX-0501"; phase = "tooling.dx"; summary = "Debug orchestration failed."; help = "inspect the adapter, target and debug artifact" }
    [pscustomobject][ordered]@{ code = "NCC-DX-0601"; phase = "tooling.dx"; summary = "IDE integration failed."; help = "inspect the generated IDE configuration and toolchain path" }
)

# One-time exact-head promotion. Normal builds never scan Rust; after this file
# is committed, diagnostics/registry.json is the sole authority.
$known = @{}
foreach ($entry in $entries) { $known[$entry.code] = $true }
$codePattern = [regex]'\b[EWNI]_[A-Z][A-Z0-9_]*_[0-9]{3}\b'
Get-ChildItem -LiteralPath (Join-Path $Workspace "crates") -Recurse -File -Filter *.rs |
    ForEach-Object {
        $source = Get-Content -LiteralPath $_.FullName -Raw
        foreach ($match in $codePattern.Matches($source)) {
            $code = $match.Value
            if (-not $known.ContainsKey($code)) {
                $entries += [pscustomobject][ordered]@{
                    code = $code
                    phase = Get-DiagnosticPhase $code
                    summary = "Diagnostic promoted from the stage0 exact-head inventory."
                    help = ""
                }
                $known[$code] = $true
            }
        }
    }
$entries = @($entries | Sort-Object code)

$duplicates = @($entries | Group-Object code | Where-Object Count -ne 1)
if ($duplicates.Count -ne 0) {
    throw "Generated language inventory contains duplicate diagnostic codes."
}

$registry = [ordered]@{
    schema = "nc.diagnostic.registry.v1"
    version = 1
    source_authority = "diagnostics/registry.json"
    material_implementation = "runtime/nc/diagnostic_registry.n"
    stage0_bridge = "crates/ncc-diagnostics"
    entries = $entries
}

$directory = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Path $directory -Force | Out-Null
$json = $registry | ConvertTo-Json -Depth 8
[System.IO.File]::WriteAllText($outputPath, $json + "`n", [System.Text.UTF8Encoding]::new($false))
Write-Output "Wrote $($entries.Count) canonical diagnostics to $outputPath"
