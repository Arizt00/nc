[CmdletBinding()]
param(
    [string[]]$Root = @('C:\'),
    [string]$Ncc = (Join-Path $env:LOCALAPPDATA 'nC\bin\ncc.exe'),
    [int]$MinLines = 500,
    [long]$MinBytes = 32768,
    [int]$CheckTimeoutSeconds = 30,
    [int]$BuildTimeoutSeconds = 90,
    [string]$Output = (Join-Path $env:TEMP 'ncc-long-source-analysis.json'),
    [string]$WorkDir = (Join-Path $env:TEMP ("ncc-long-source-analysis-{0}" -f [Guid]::NewGuid().ToString('N'))),
    [switch]$SkipBuild,
    [switch]$KeepWorkDir
)

$ErrorActionPreference = 'Stop'
$rg = (Get-Command rg -ErrorAction Stop).Source
if (-not (Test-Path -LiteralPath $Ncc -PathType Leaf)) {
    throw "ncc executable not found: $Ncc"
}
New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null

function Invoke-NcTimed {
    param(
        [string[]]$Arguments,
        [int]$TimeoutSeconds
    )

    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $Ncc
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $psi.WorkingDirectory = $WorkDir
    foreach ($argument in $Arguments) {
        $null = $psi.ArgumentList.Add($argument)
    }

    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $psi
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $null = $process.Start()
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $completed = $process.WaitForExit($TimeoutSeconds * 1000)
    if (-not $completed) {
        try { $process.Kill($true) } catch { }
        $process.WaitForExit()
        $status = 'timeout'
        $exitCode = $null
    }
    else {
        $status = if ($process.ExitCode -eq 0) { 'passed' } else { 'failed' }
        $exitCode = $process.ExitCode
    }
    $watch.Stop()
    [pscustomobject]@{
        status = $status
        exit_code = $exitCode
        elapsed_ms = $watch.ElapsedMilliseconds
        output = (($stdout.Result + "`n" + $stderr.Result).Trim())
    }
}

function Get-FirstDiagnostic([string]$Text) {
    $code = [regex]::Match($Text, 'E_[A-Z0-9_]+').Value
    if ($code) { return $code }
    return ($Text -split "`r?`n" | Where-Object { $_ } | Select-Object -First 1)
}

$patterns = @('-g', '*.n', '-g', '*.nm', '-g', '*.nb', '-g', '*.nbb', '-g', '*.ncp')
$paths = [Collections.Generic.List[string]]::new()
foreach ($scanRoot in $Root) {
    $arguments = @('--files', '-uu') + $patterns + @($scanRoot)
    & $rg @arguments 2>$null | ForEach-Object { $paths.Add($_) }
}

$rows = [Collections.Generic.List[object]]::new()
$offlineSkipped = 0
$readErrors = 0
foreach ($path in $paths) {
    try {
        $item = Get-Item -LiteralPath $path -Force -ErrorAction Stop
    }
    catch {
        $readErrors++
        continue
    }
    if (($item.Attributes -band [IO.FileAttributes]::Offline) -ne 0) {
        $offlineSkipped++
        continue
    }
    if ($item.Length -lt [Math]::Min($MinBytes, 8192)) { continue }
    try {
        $text = [IO.File]::ReadAllText($item.FullName)
    }
    catch {
        $readErrors++
        continue
    }
    $lines = 1 + [regex]::Matches($text, "`n").Count
    if ($lines -lt $MinLines -and $item.Length -lt $MinBytes) { continue }

    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $item.FullName).Hash.ToLowerInvariant()
    $isNc = [regex]::IsMatch(
        $text,
        '(?m)(use::|\[@(?:machine|runtime|system|primitive)?\]|primitive\[@\]|^\s*(?:stc|fn|fa|local|type)\s*:|^\s*(?:ref\s+main|to\s+main)\b)'
    )
    $hasEntry = [regex]::IsMatch(
        $text,
        '(?m)^\s*(?:ref\s+main\b|to\s+main\b|ref\s+mainentry\b)'
    )
    $interactive = [regex]::IsMatch(
        $text,
        '\b(readInt|readStr|readLine|readKey|input|windowCreate|canvasCreate|pollEvent|nextEvent)\s*\('
    ) -or [regex]::IsMatch($text, 'while\s*\(\s*true\s*\)')
    $rawRisk = [regex]::IsMatch(
        $text,
        '\b(alloc|calloc|realloc|free|ptrWrite|deref|fswrite|spawn|processSpawn|mapAnonymous|mapFile|socket|connect)\s*\('
    )

    $lower = $item.FullName.ToLowerInvariant()
    $class = 'active'
    if ($lower -match '\\\$recycle\.bin\\') {
        $class = 'recycle'
    }
    elseif ($lower -match '\\(\.staging|_archive|archive|old_examples|history|backups?|temp|tmp)\\') {
        $class = 'historical'
    }
    elseif ($lower -match '\\(dist|target|generated)\\') {
        $class = 'generated'
    }
    elseif ($lower -match '\\.vscode\\') {
        $class = 'editor-history'
    }
    $penalty = switch ($class) {
        'active' { 0 }
        'generated' { 40 }
        'historical' { 80 }
        'editor-history' { 90 }
        'recycle' { 100 }
    }
    $rows.Add([pscustomobject]@{
        path = $item.FullName
        extension = $item.Extension.ToLowerInvariant()
        bytes = $item.Length
        lines = $lines
        sha256 = $hash
        is_nc = $isNc
        has_entry = $hasEntry
        interactive = $interactive
        raw_risk = $rawRisk
        class = $class
        penalty = $penalty
    })
}

$unique = [Collections.Generic.List[object]]::new()
foreach ($group in ($rows | Group-Object sha256)) {
    $representative = $group.Group |
        Sort-Object -Property penalty, @{ Expression = { $_.path.Length } } |
        Select-Object -First 1
    $unique.Add([pscustomobject]@{
        sha256 = $group.Name
        copies = $group.Count
        representative = $representative.path
        extension = $representative.extension
        bytes = $representative.bytes
        lines = $representative.lines
        is_nc = $representative.is_nc
        has_entry = $representative.has_entry
        interactive = $representative.interactive
        raw_risk = $representative.raw_risk
        class = $representative.class
        check_status = 'not_checked'
        check_ms = $null
        diagnostic = $null
        build_status = 'not_attempted'
        build_ms = $null
        build_diagnostic = $null
    })
}

$buildIndex = 0
foreach ($program in $unique) {
    if (-not $program.is_nc) {
        $program.check_status = 'extension_collision_not_nc'
        $program.diagnostic = 'Content does not match nC syntax markers.'
        continue
    }

    $check = Invoke-NcTimed -Arguments @('check', $program.representative) `
        -TimeoutSeconds $CheckTimeoutSeconds
    $program.check_status = $check.status
    $program.check_ms = $check.elapsed_ms
    if ($check.status -ne 'passed') {
        $program.diagnostic = Get-FirstDiagnostic $check.output
    }

    if ($SkipBuild) {
        $program.build_status = 'skipped_by_option'
    }
    elseif ($check.status -eq 'passed' -and $program.has_entry -and $program.class -eq 'active') {
        $buildIndex++
        $outDir = Join-Path $WorkDir "build-$buildIndex"
        $executable = Join-Path $outDir 'program.exe'
        New-Item -ItemType Directory -Path $outDir -Force | Out-Null
        $build = Invoke-NcTimed `
            -Arguments @('build', $program.representative, $executable, '--out-dir', $outDir, '--json') `
            -TimeoutSeconds $BuildTimeoutSeconds
        $program.build_status = $build.status
        $program.build_ms = $build.elapsed_ms
        if ($build.status -ne 'passed') {
            $program.build_diagnostic = Get-FirstDiagnostic $build.output
        }
    }
    elseif (-not $program.has_entry) {
        $program.build_status = 'module_or_fixture_no_entry'
    }
    elseif ($program.class -ne 'active') {
        $program.build_status = 'historical_copy_not_built'
    }
}

$resolvedWorkDir = [IO.Path]::GetFullPath($WorkDir)
$resolvedTemp = [IO.Path]::GetFullPath($env:TEMP).TrimEnd(
    [IO.Path]::DirectorySeparatorChar,
    [IO.Path]::AltDirectorySeparatorChar
)
$generatedWorkDir = $resolvedWorkDir.StartsWith(
    $resolvedTemp + [IO.Path]::DirectorySeparatorChar,
    [StringComparison]::OrdinalIgnoreCase
) -and (Split-Path -Leaf $resolvedWorkDir).StartsWith(
    'ncc-long-source-analysis-',
    [StringComparison]::OrdinalIgnoreCase
)
$cleanupWorkDir = -not $KeepWorkDir -and $generatedWorkDir

$payload = [ordered]@{
    schema = 'ncc.long_source_analysis.v1'
    generated_at_utc = [DateTime]::UtcNow.ToString('o')
    roots = $Root
    work_dir = $WorkDir
    work_dir_retained = -not $cleanupWorkDir
    thresholds = @{ lines = $MinLines; bytes = $MinBytes }
    inventory = [ordered]@{
        matching_paths = $paths.Count
        offline_skipped = $offlineSkipped
        read_errors = $readErrors
        long_paths = $rows.Count
        unique_contents = $unique.Count
        duplicate_paths = $rows.Count - $unique.Count
        nc_unique = ($unique | Where-Object is_nc | Measure-Object).Count
        extension_collisions = ($unique | Where-Object { -not $_.is_nc } | Measure-Object).Count
    }
    classification = @($unique | Group-Object class | ForEach-Object {
        [pscustomobject]@{ name = $_.Name; count = $_.Count }
    })
    check = @($unique | Group-Object check_status | ForEach-Object {
        [pscustomobject]@{ name = $_.Name; count = $_.Count }
    })
    build = @($unique | Group-Object build_status | ForEach-Object {
        [pscustomobject]@{ name = $_.Name; count = $_.Count }
    })
    programs = $unique
}

$outputParent = Split-Path -Parent $Output
if ($outputParent) { New-Item -ItemType Directory -Path $outputParent -Force | Out-Null }
[IO.File]::WriteAllText(
    $Output,
    ($payload | ConvertTo-Json -Depth 8),
    [Text.UTF8Encoding]::new($false)
)

if ($cleanupWorkDir -and (Test-Path -LiteralPath $resolvedWorkDir -PathType Container)) {
    Remove-Item -LiteralPath $resolvedWorkDir -Recurse -Force
}

[pscustomobject]@{
    report = $Output
    work_dir = if ($cleanupWorkDir) { $null } else { $WorkDir }
    work_dir_retained = -not $cleanupWorkDir
    inventory = $payload.inventory
    classification = $payload.classification
    check = $payload.check
    build = $payload.build
    slow_checks = @($unique | Where-Object check_ms | Sort-Object check_ms -Descending |
        Select-Object -First 10 representative, lines, check_ms, check_status, diagnostic)
    failed_builds = @($unique | Where-Object { $_.build_status -in @('failed', 'timeout') } |
        Select-Object representative, lines, build_ms, build_status, build_diagnostic)
} | ConvertTo-Json -Depth 6
