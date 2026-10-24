param(
    [string]$Ncc,
    [string]$ExamplesRoot
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $Ncc) {
    $candidate = Join-Path $repoRoot "target\debug\ncc.exe"
    if (-not (Test-Path -LiteralPath $candidate)) {
        $candidate = Join-Path $repoRoot "target\release\ncc.exe"
    }
    $Ncc = $candidate
}
if (-not $ExamplesRoot) {
    $ExamplesRoot = Join-Path $repoRoot "examples"
}

$nccPath = (Resolve-Path -LiteralPath $Ncc).Path
$examplesPath = (Resolve-Path -LiteralPath $ExamplesRoot).Path
$stats = [ordered]@{
    total = 0
    positive_ok = 0
    positive_failed = 0
    negative_rejected = 0
    negative_accepted = 0
    projects_ok = 0
    projects_failed = 0
}
$failures = [System.Collections.Generic.List[string]]::new()

$files = Get-ChildItem -LiteralPath $examplesPath -Recurse -File -Filter "*.n" |
    Where-Object { $_.FullName -notmatch "[\\/]\.ncc[\\/]" } |
    Sort-Object FullName

foreach ($file in $files) {
    $stats.total++
    $relative = $file.FullName.Substring($examplesPath.Length + 1)
    $source = [IO.File]::ReadAllText($file.FullName)
    $negative =
        $relative -match "(^|[\\/])negative([\\/]|$)" -or
        $file.BaseName -match "(_fail|_error|_bad_type)$"
    $project = -not $negative -and $source -match "use\s*(?:::|<)\s*project"

    if ($project) {
        $output = & $nccPath project check $file.DirectoryName 2>&1
        if ($LASTEXITCODE -eq 0) {
            $stats.projects_ok++
        } else {
            $stats.projects_failed++
            $failures.Add("PROJECT_FAIL $relative :: $(($output | Select-Object -First 3) -join ' | ')")
        }
        continue
    }

    $output = & $nccPath check $file.FullName 2>&1
    if ($negative) {
        if ($LASTEXITCODE -ne 0) {
            $stats.negative_rejected++
        } else {
            $stats.negative_accepted++
            $failures.Add("NEGATIVE_ACCEPTED $relative")
        }
    } elseif ($LASTEXITCODE -eq 0) {
        $stats.positive_ok++
    } else {
        $stats.positive_failed++
        $failures.Add("POSITIVE_FAIL $relative :: $(($output | Select-Object -First 3) -join ' | ')")
    }
}

$stats.GetEnumerator() | ForEach-Object { "{0}={1}" -f $_.Key, $_.Value }
if ($failures.Count -gt 0) {
    "--- failures ---"
    $failures
}

if ($stats.positive_failed -gt 0 -or $stats.negative_accepted -gt 0 -or $stats.projects_failed -gt 0) {
    exit 1
}
