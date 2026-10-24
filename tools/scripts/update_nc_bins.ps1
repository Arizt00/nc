[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SourceDir,

    [string]$BinDir = (Join-Path $env:LOCALAPPDATA "nC\bin"),

    [string]$ShimDir = (Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps"),

    [int]$CallerPid = 0
)

$ErrorActionPreference = "Stop"

function Get-NcSha256([string]$Path) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        return [System.BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '')
    }
    finally {
        $stream.Dispose()
        $sha.Dispose()
    }
}

$bins = @(
    "ncc",
    "nclsp",
    "nc-sys-lsp",
    "ghor",
    "nfmt",
    "nlint",
    "nbuild",
    "nide",
    "npkg",
    "ncup",
    "ncup-init",
    "nwal"
)
$companions = @("ghor-stage1")
$artifacts = @($bins) + $companions

$source = (Resolve-Path -LiteralPath $SourceDir).Path
$targetSpecCandidates = @(
    (Join-Path $source "targets"),
    (Join-Path $source "..\..\targets"),
    (Join-Path $PSScriptRoot "..\..\targets")
)
$targetSpecSource = $targetSpecCandidates |
    Where-Object { Test-Path -LiteralPath $_ -PathType Container } |
    Select-Object -First 1
if (-not $targetSpecSource) {
    throw "Missing Primitive target registry; checked: $($targetSpecCandidates -join ', ')"
}
$targetSpecSource = (Resolve-Path -LiteralPath $targetSpecSource).Path
$targetSpecFiles = @(Get-ChildItem -LiteralPath $targetSpecSource -Filter "*.nctarget" -File)
if ($targetSpecFiles.Count -eq 0) {
    throw "Primitive target registry is empty: $targetSpecSource"
}
$linkSource = (Resolve-Path -LiteralPath (Join-Path $targetSpecSource "..\link")).Path
$linkFiles = @(Get-ChildItem -LiteralPath $linkSource -File)
if ($linkFiles.Count -eq 0) {
    throw "Primitive linker registry is empty: $linkSource"
}
foreach ($bin in $artifacts) {
    $candidate = Join-Path $source "$bin.exe"
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "Missing Windows release binary: $candidate"
    }
    if ((Get-Item -LiteralPath $candidate).Length -eq 0) {
        throw "Empty Windows release binary: $candidate"
    }
}

& (Join-Path $source "ghor-stage1.exe") | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Native nC ghor-stage1 self-test failed with exit code $LASTEXITCODE"
}

New-Item -ItemType Directory -Force -Path $BinDir, $ShimDir | Out-Null
$transaction = "{0}-{1}" -f ([DateTimeOffset]::UtcNow.ToUnixTimeSeconds()), ([Guid]::NewGuid().ToString("N"))
$stage = Join-Path $BinDir ".ncup-stage-$transaction"
$backup = Join-Path $BinDir ".ncup-backup-$transaction"
New-Item -ItemType Directory -Force -Path $stage, $backup | Out-Null
$targetStage = Join-Path $stage "targets"
$targetBackup = Join-Path $backup "targets"
$linkStage = Join-Path $stage "link"
$linkBackup = Join-Path $backup "link"
$installPrefix = Split-Path -Parent $BinDir
$targetDestination = Join-Path $installPrefix "share\nC\targets"
$linkDestination = Join-Path $installPrefix "share\nC\link"
New-Item -ItemType Directory -Force -Path $targetStage, $targetBackup, $targetDestination, $linkStage, $linkBackup, $linkDestination | Out-Null

trap {
    Remove-Item -LiteralPath $stage, $backup -Recurse -Force -ErrorAction SilentlyContinue
    break
}

foreach ($bin in $artifacts) {
    $src = Join-Path $source "$bin.exe"
    $staged = Join-Path $stage "$bin.exe"
    Copy-Item -LiteralPath $src -Destination $staged -Force
    $sourceHash = Get-NcSha256 $src
    $stageHash = Get-NcSha256 $staged
    if ($sourceHash -ne $stageHash) {
        throw "Staging checksum mismatch for $bin.exe"
    }

    $destination = Join-Path $BinDir "$bin.exe"
    if (Test-Path -LiteralPath $destination -PathType Leaf) {
        Copy-Item -LiteralPath $destination -Destination (Join-Path $backup "$bin.exe") -Force
    }
    else {
        New-Item -ItemType File -Path (Join-Path $backup "$bin.absent") | Out-Null
    }
}

foreach ($targetSpec in $targetSpecFiles) {
    $staged = Join-Path $targetStage $targetSpec.Name
    Copy-Item -LiteralPath $targetSpec.FullName -Destination $staged -Force
    if ((Get-NcSha256 $targetSpec.FullName) -ne (Get-NcSha256 $staged)) {
        throw "Staging checksum mismatch for $($targetSpec.Name)"
    }
    $destination = Join-Path $targetDestination $targetSpec.Name
    if (Test-Path -LiteralPath $destination -PathType Leaf) {
        Copy-Item -LiteralPath $destination -Destination (Join-Path $targetBackup $targetSpec.Name) -Force
    }
    else {
        New-Item -ItemType File -Path (Join-Path $targetBackup "$($targetSpec.Name).absent") | Out-Null
    }
}

foreach ($linkFile in $linkFiles) {
    $staged = Join-Path $linkStage $linkFile.Name
    Copy-Item -LiteralPath $linkFile.FullName -Destination $staged -Force
    if ((Get-NcSha256 $linkFile.FullName) -ne (Get-NcSha256 $staged)) {
        throw "Staging checksum mismatch for $($linkFile.Name)"
    }
    $destination = Join-Path $linkDestination $linkFile.Name
    if (Test-Path -LiteralPath $destination -PathType Leaf) {
        Copy-Item -LiteralPath $destination -Destination (Join-Path $linkBackup $linkFile.Name) -Force
    }
    else {
        New-Item -ItemType File -Path (Join-Path $linkBackup "$($linkFile.Name).absent") | Out-Null
    }
}

function Write-NcShim([string]$Name) {
    $destination = Join-Path $BinDir "$Name.exe"
    $cmdTarget = $destination.Replace("%", "%%")
    $shim = @"
@echo off
setlocal
"$cmdTarget" %*
exit /b %ERRORLEVEL%
"@
    [System.IO.File]::WriteAllText(
        (Join-Path $ShimDir "$Name.cmd"),
        $shim.Replace("`n", "`r`n"),
        [System.Text.Encoding]::ASCII
    )
}

function Restore-NcTransaction {
    foreach ($bin in $artifacts) {
        $destination = Join-Path $BinDir "$bin.exe"
        $saved = Join-Path $backup "$bin.exe"
        $absent = Join-Path $backup "$bin.absent"
        if (Test-Path -LiteralPath $saved -PathType Leaf) {
            Copy-Item -LiteralPath $saved -Destination $destination -Force
        }
        elseif (Test-Path -LiteralPath $absent -PathType Leaf) {
            Remove-Item -LiteralPath $destination -Force -ErrorAction SilentlyContinue
        }
    }
    foreach ($targetSpec in $targetSpecFiles) {
        $destination = Join-Path $targetDestination $targetSpec.Name
        $saved = Join-Path $targetBackup $targetSpec.Name
        $absent = Join-Path $targetBackup "$($targetSpec.Name).absent"
        if (Test-Path -LiteralPath $saved -PathType Leaf) {
            Copy-Item -LiteralPath $saved -Destination $destination -Force
        }
        elseif (Test-Path -LiteralPath $absent -PathType Leaf) {
            Remove-Item -LiteralPath $destination -Force -ErrorAction SilentlyContinue
        }
    }
    foreach ($linkFile in $linkFiles) {
        $destination = Join-Path $linkDestination $linkFile.Name
        $saved = Join-Path $linkBackup $linkFile.Name
        $absent = Join-Path $linkBackup "$($linkFile.Name).absent"
        if (Test-Path -LiteralPath $saved -PathType Leaf) {
            Copy-Item -LiteralPath $saved -Destination $destination -Force
        }
        elseif (Test-Path -LiteralPath $absent -PathType Leaf) {
            Remove-Item -LiteralPath $destination -Force -ErrorAction SilentlyContinue
        }
    }
}

try {
    foreach ($bin in $artifacts | Where-Object { $_ -ne "ncup" }) {
        $staged = Join-Path $stage "$bin.exe"
        $destination = Join-Path $BinDir "$bin.exe"
        Copy-Item -LiteralPath $staged -Destination $destination -Force
        $stageHash = Get-NcSha256 $staged
        $installedHash = Get-NcSha256 $destination
        if ($stageHash -ne $installedHash) {
            throw "Installed checksum mismatch for $bin.exe"
        }
        if ($bins -contains $bin) {
            Write-NcShim $bin
        }
        Write-Output "updated: $destination sha256:$($installedHash.ToLowerInvariant())"
    }

    foreach ($targetSpec in $targetSpecFiles) {
        $staged = Join-Path $targetStage $targetSpec.Name
        $destination = Join-Path $targetDestination $targetSpec.Name
        Copy-Item -LiteralPath $staged -Destination $destination -Force
        if ((Get-NcSha256 $staged) -ne (Get-NcSha256 $destination)) {
            throw "Installed checksum mismatch for $($targetSpec.Name)"
        }
    }
    Write-Output "updated Primitive targets: $targetDestination count=$($targetSpecFiles.Count)"

    foreach ($linkFile in $linkFiles) {
        $staged = Join-Path $linkStage $linkFile.Name
        $destination = Join-Path $linkDestination $linkFile.Name
        Copy-Item -LiteralPath $staged -Destination $destination -Force
        if ((Get-NcSha256 $staged) -ne (Get-NcSha256 $destination)) {
            throw "Installed checksum mismatch for $($linkFile.Name)"
        }
    }
    Write-Output "updated Primitive link resources: $linkDestination count=$($linkFiles.Count)"

    Write-NcShim "ncup"
    $ncupSource = Join-Path $stage "ncup.exe"
    $ncupDestination = Join-Path $BinDir "ncup.exe"
    if ($CallerPid -gt 0) {
        $helper = Join-Path $env:TEMP "ncup-complete-$transaction.ps1"
        $quotedSource = $ncupSource.Replace("'", "''")
        $quotedDestination = $ncupDestination.Replace("'", "''")
        $quotedStage = $stage.Replace("'", "''")
        $quotedBackup = $backup.Replace("'", "''")
        $quotedBinDir = $BinDir.Replace("'", "''")
        $helperBins = ($artifacts | ForEach-Object { "'$($_.Replace("'", "''"))'" }) -join ", "
        $helperBody = @"
`$ErrorActionPreference = 'Stop'
`$caller = $CallerPid
function Get-NcSha256([string]`$Path) {
    `$sha = [System.Security.Cryptography.SHA256]::Create()
    `$stream = [System.IO.File]::OpenRead(`$Path)
    try {
        return [System.BitConverter]::ToString(`$sha.ComputeHash(`$stream)).Replace('-', '')
    }
    finally {
        `$stream.Dispose()
        `$sha.Dispose()
    }
}
while (Get-Process -Id `$caller -ErrorAction SilentlyContinue) {
    Start-Sleep -Milliseconds 100
}
try {
    Copy-Item -LiteralPath '$quotedSource' -Destination '$quotedDestination' -Force
    `$expected = Get-NcSha256 '$quotedSource'
    `$actual = Get-NcSha256 '$quotedDestination'
    if (`$expected -ne `$actual) { throw 'Installed checksum mismatch for ncup.exe' }
    `$manifest = [ordered]@{
        schema = 'ncup.complete_update.install.v1'
        status = 'ok'
        completed_at_utc = [DateTime]::UtcNow.ToString('o')
        bin_dir = '$quotedBinDir'
        managed_bin_count = $bins.Count
        companion_artifact_count = 1
        managed_artifact_count = $artifacts.Count
        ncup_sha256 = `$actual.ToLowerInvariant()
    }
    Remove-Item -LiteralPath (Join-Path '$quotedBinDir' '.ncup-update-error.log') -Force -ErrorAction SilentlyContinue
    `$manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path '$quotedBinDir' '.ncup-last-update.json') -Encoding UTF8
    Remove-Item -LiteralPath '$quotedStage' -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath '$quotedBackup' -Recurse -Force -ErrorAction SilentlyContinue
}
catch {
    foreach (`$bin in @($helperBins)) {
        `$destination = Join-Path '$quotedBinDir' "`$bin.exe"
        `$saved = Join-Path '$quotedBackup' "`$bin.exe"
        `$absent = Join-Path '$quotedBackup' "`$bin.absent"
        if (Test-Path -LiteralPath `$saved -PathType Leaf) {
            Copy-Item -LiteralPath `$saved -Destination `$destination -Force
        }
        elseif (Test-Path -LiteralPath `$absent -PathType Leaf) {
            Remove-Item -LiteralPath `$destination -Force -ErrorAction SilentlyContinue
        }
    }
    `$_.Exception.Message | Set-Content -LiteralPath (Join-Path '$quotedBinDir' '.ncup-update-error.log') -Encoding UTF8
}
Remove-Item -LiteralPath `$PSCommandPath -Force -ErrorAction SilentlyContinue
"@
        [System.IO.File]::WriteAllText($helper, $helperBody, [System.Text.Encoding]::UTF8)
        Start-Process -FilePath "powershell.exe" -WindowStyle Hidden -ArgumentList @(
            "-NoProfile",
            "-ExecutionPolicy", "Bypass",
            "-File", ('"{0}"' -f $helper)
        ) | Out-Null
        Write-Output "pending self-update: $ncupDestination after pid $CallerPid exits"
    }
    else {
        Copy-Item -LiteralPath $ncupSource -Destination $ncupDestination -Force
        $expected = Get-NcSha256 $ncupSource
        $actual = Get-NcSha256 $ncupDestination
        if ($expected -ne $actual) {
            throw "Installed checksum mismatch for ncup.exe"
        }
        $manifest = [ordered]@{
            schema = 'ncup.complete_update.install.v1'
            status = 'ok'
            completed_at_utc = [DateTime]::UtcNow.ToString('o')
            bin_dir = $BinDir
            managed_bin_count = $bins.Count
            companion_artifact_count = $companions.Count
            managed_artifact_count = $artifacts.Count
            ncup_sha256 = $actual.ToLowerInvariant()
        }
        Remove-Item -LiteralPath (Join-Path $BinDir '.ncup-update-error.log') -Force -ErrorAction SilentlyContinue
        $manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $BinDir '.ncup-last-update.json') -Encoding UTF8
        Write-Output "updated: $ncupDestination sha256:$($actual.ToLowerInvariant())"
        Remove-Item -LiteralPath $stage, $backup -Recurse -Force
    }
}
catch {
    Restore-NcTransaction
    throw
}

Write-Output "updated shims: $ShimDir"
