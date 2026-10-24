[CmdletBinding()]
param(
    [switch]$Strict
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not (Get-Command rg -ErrorAction SilentlyContinue)) {
    throw 'ripgrep (rg) is required for the deterministic C provider audit'
}

Push-Location -LiteralPath $root
try {
    $discovered = & rg --files --hidden `
        -g '*.c' -g '*.h' -g '*.cc' -g '*.cpp' `
        -g '!target/**' -g '!.git/**'
    $rgExit = $LASTEXITCODE
} finally {
    Pop-Location
}
if ($rgExit -gt 1) {
    throw "ripgrep source inventory failed with exit code $rgExit"
}
$files = @($discovered | ForEach-Object { $_.Replace('\', '/') } | Sort-Object -Unique)

$knownProviders = @(
    'crates/ncc-cli/src/runtime_hosted_native.c',
    'crates/ncc-cli/src/runtime_json_native.c',
    'tools/android/blockmaster/jni/nc_android_jni.c',
    'tools/android/blockmaster/jni/nc_android_urgency_jni.c'
)

$externalOracles = @(
    $files | Where-Object {
        $_ -like '_archive/*' -or
        $_ -eq 'crates/ncc-cli/tests/raw_optimizer_oracle.c'
    }
)
$providers = @($files | Where-Object { $knownProviders -contains $_ })
$unexpected = @($files | Where-Object {
    $knownProviders -notcontains $_ -and
    $_ -notlike '_archive/*' -and
    $_ -ne 'crates/ncc-cli/tests/raw_optimizer_oracle.c'
})

$result = [ordered]@{
    schema = 'nc.no_c_language_providers.audit.v1'
    status = if ($unexpected.Count -gt 0) {
        'unexpected_internal_c_source'
    } elseif ($providers.Count -eq 0) {
        'closed'
    } else {
        'migration_active'
    }
    strict = [bool]$Strict
    source_count = $files.Count
    internal_provider_count = $providers.Count
    external_oracle_count = $externalOracles.Count
    unexpected_count = $unexpected.Count
    internal_providers = $providers
    external_oracles = $externalOracles
    unexpected = $unexpected
}

$result | ConvertTo-Json -Depth 6

if ($unexpected.Count -gt 0) {
    exit 2
}
if ($Strict -and $providers.Count -gt 0) {
    exit 3
}
