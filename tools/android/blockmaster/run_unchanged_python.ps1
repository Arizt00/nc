param(
    [Parameter(Mandatory = $true)]
    [string]$Script,
    [string]$Serial = ""
)

$ErrorActionPreference = "Stop"
$adbCandidates = @(
    "$env:LOCALAPPDATA\Android\Sdk\platform-tools\adb.exe",
    "adb.exe",
    "adb"
)
$adb = $adbCandidates | Where-Object {
    if ([IO.Path]::IsPathRooted($_)) { Test-Path -LiteralPath $_ }
    else { $null -ne (Get-Command $_ -ErrorAction SilentlyContinue) }
} | Select-Object -First 1
if (-not $adb) { throw "No se encontro adb. Instala Android platform-tools." }

$scriptItem = Get-Item -LiteralPath $Script -ErrorAction Stop
if ($scriptItem.PSIsContainer) {
    $scripts = @(Get-ChildItem -LiteralPath $scriptItem.FullName -File -Filter "*.py")
    if ($scripts.Count -ne 1) {
        throw "La carpeta debe contener exactamente un archivo .py; encontrados: $($scripts.Count)."
    }
    $scriptPath = $scripts[0].FullName
} else {
    $scriptPath = $scriptItem.FullName
}

$adbArgs = @()
if ($Serial) { $adbArgs += @("-s", $Serial) }
$deviceState = (& $adb @adbArgs get-state 2>&1 | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $deviceState -ne "device") {
    throw "ADB no autorizado para '$Serial'. Desbloquea el movil, acepta 'Permitir depuracion USB' y vuelve a ejecutar. Estado: $deviceState"
}
& $adb @adbArgs forward tcp:8080 tcp:8080 | Out-Null
if ($LASTEXITCODE -ne 0) { throw "No se pudo crear adb forward tcp:8080." }

$source = [IO.File]::ReadAllText($scriptPath)
if (-not $source.Contains('192.168.1.XX')) {
    throw "El script no contiene el marcador literal 192.168.1.XX esperado."
}
$runtimeSource = $source.Replace('192.168.1.XX', '127.0.0.1')
$payload = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($runtimeSource))

$python = Get-Command py.exe -ErrorAction SilentlyContinue
if ($python) {
    & $python.Source -3 -c "import base64;exec(compile(base64.b64decode('$payload'),'<script_pc_controlador.py>','exec'))"
} else {
    $python = Get-Command python.exe -ErrorAction Stop
    & $python.Source -c "import base64;exec(compile(base64.b64decode('$payload'),'<script_pc_controlador.py>','exec'))"
}
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
