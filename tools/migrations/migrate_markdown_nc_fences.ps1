param(
    [string]$Root = "docs",
    [string]$Ncc = "target/debug/ncc.exe"
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$docs = (Resolve-Path (Join-Path $repo $Root)).Path
$compiler = (Resolve-Path (Join-Path $repo $Ncc)).Path
$tempRoot = Join-Path $repo "target/tmp/markdown-nfmt-migration"

New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

$seen = 0
$migrated = 0
$filesChanged = 0
$pattern = '(?ms)(?<open>^```(?:n|nc|nC)\s*\r?\n)(?<source>.*?)(?<close>^```\s*$)'
$excluded = '[\\/](?:archive|handoff|status|base_inputs)[\\/]'
$immutable = '[\\/]docs[\\/]spec[\\/]release[\\/].*immutable'

try {
    Get-ChildItem -LiteralPath $docs -Recurse -File -Filter '*.md' |
        Where-Object { $_.FullName -notmatch $excluded -and $_.FullName -notmatch $immutable } |
        ForEach-Object {
            $path = $_.FullName
            $text = [System.IO.File]::ReadAllText($path)
            $rewritten = [regex]::Replace($text, $pattern, {
                param($match)

                $script:seen += 1
                $inputPath = Join-Path $tempRoot 'snippet.n'
                $outputPath = Join-Path $tempRoot 'snippet.formatted.n'
                [System.IO.File]::WriteAllText($inputPath, $match.Groups['source'].Value)
                if (Test-Path -LiteralPath $outputPath) {
                    Remove-Item -LiteralPath $outputPath -Force
                }

                $oldPreference = $ErrorActionPreference
                $ErrorActionPreference = 'Continue'
                & $compiler nfmt $inputPath --out $outputPath 1>$null 2>$null
                $formatExit = $LASTEXITCODE
                $ErrorActionPreference = $oldPreference
                if ($formatExit -ne 0 -or !(Test-Path -LiteralPath $outputPath)) {
                    return $match.Value
                }

                $formatted = [System.IO.File]::ReadAllText($outputPath).TrimEnd("`r", "`n") + "`n"
                if ($formatted -eq $match.Groups['source'].Value) {
                    return $match.Value
                }

                $script:migrated += 1
                return $match.Groups['open'].Value + $formatted + $match.Groups['close'].Value
            })

            if ($rewritten -ne $text) {
                [System.IO.File]::WriteAllText($path, $rewritten)
                $script:filesChanged += 1
            }
        }
} finally {
    $resolvedTemp = (Resolve-Path $tempRoot).Path
    $allowedRoot = (Resolve-Path (Join-Path $repo 'target/tmp')).Path
    if ($resolvedTemp.StartsWith($allowedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }
}

Write-Output "markdown nC fences: seen=$seen migrated=$migrated files_changed=$filesChanged"
