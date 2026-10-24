param(
    [Parameter(Mandatory)][string]$Repo,
    [Parameter(Mandatory)][ValidateSet('windows','linux')][string]$HostName,
    [Parameter(Mandatory)][string]$Candidate,
    [Parameter(Mandatory)][string]$Work,
    [string]$Python = 'C:/Users/arist/AppData/Local/Programs/Python/Python312/python.exe',
    [string]$Frozen = '',
    [Alias('Oracle')][switch]$OracleMode,
    [string]$Only = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Formats.Tar
$Repo = [IO.Path]::GetFullPath($Repo)
$Work = [IO.Path]::GetFullPath($Work)
$Candidate = [IO.Path]::GetFullPath($Candidate)
if ($OracleMode -and $Frozen) { throw 'Choose explicit oracle mode or native frozen replay, not both' }
if (-not $OracleMode -and -not $Frozen) { $Frozen=Join-Path $Repo ('bootstrap/existing43-primitive-oracle-'+$HostName+'.json') }
if (-not $Work.StartsWith($Repo + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -and -not $Work.Replace('\','/').StartsWith('C:/tmp/nc-primitive43-',[StringComparison]::OrdinalIgnoreCase)) { throw 'Fixture work must remain in repo or explicitly named C:/tmp/nc-primitive43-* test directory' }
$utf8 = [Text.UTF8Encoding]::new($false)
$fixedTime = [DateTime]::SpecifyKind([DateTime]'2020-01-02T03:04:05',[DateTimeKind]::Utc)
$required = @(
    'docs/spec/release/v2_0_0_primitive_release_evidence_spec.md',
    'docs/spec/release/v2_0_0_primitive_release_evidence_compatibility_spec.md',
    'docs/spec/release/v2_0_0_primitive_release_evidence_contract.json',
    'docs/status/STATUS_PRIMITIVE_release_evidence_v2_0_0.md',
    'tools/ncc/nc/src/verification/physical43_primitive.n',
    'tools/release/qa_primitive_release_evidence_v2_0_0.sh',
    'tools/scripts/run_primitive_qemu_smoke.sh',
    'targets/x86_64-unknown-none.nctarget','link/x86_64_baremetal.ld',
    'examples/primitive/boot_serial_hello.ncp'
)
$members = @('boot_serial_hello.sov','boot_serial_hello.elf','material_evidence.txt','qemu_serial.log','x86_64-unknown-none.nctarget','x86_64_baremetal.ld')
$cases = @(
    'positive','contract-negative','gate-string','gate-object','gate-null',
    'unicode-kv','duplicate-field-negative','mode-empty','mode-nul','mode-default',
    'serial-invalid-utf8','serial-nul','serial-letter-negative','serial-punctuation',
    'field-nul-negative','kv-invalid-utf8','smoke-failed','missing-artifact',
    'missing-binary','stale-unicode-member','readonly-source','archived-example',
    'command-nul','command-newlines'
)
if ($Only) { $cases = @($cases | Where-Object { $_ -eq $Only }); if (-not $cases.Count) { throw 'Unknown case selector' } }
$frozenRows = @{}
if ($Frozen) {
    $sealed = Get-Content -Raw -LiteralPath $Frozen | ConvertFrom-Json -DateKind String
    if (-not ($sealed.passed -is [bool] -and $sealed.passed) -or $sealed.host -cne $HostName) { throw 'Frozen oracle must be passed and host-specific' }
    foreach ($row in $sealed.cases) {
        if (-not ($row.passed_comparison -is [bool] -and $row.passed_comparison)) { throw 'Failed comparison cannot become a frozen oracle' }
        $frozenRows[$row.name]=$row
    }
}
function To-Wsl([string]$Path) {
    $Path = [IO.Path]::GetFullPath($Path).Replace('\','/')
    return '/mnt/' + $Path.Substring(0,1).ToLowerInvariant() + $Path.Substring(2)
}
function New-Directory([string]$Path) { [void][IO.Directory]::CreateDirectory($Path) }
function Write-Data([string]$Path,[byte[]]$Data) {
    New-Directory ([IO.Path]::GetDirectoryName($Path)); [IO.File]::WriteAllBytes($Path,$Data); [IO.File]::SetLastWriteTimeUtc($Path,$fixedTime)
}
function Prepare([string]$Root,[string]$Case,[bool]$Oracle) {
    if (Test-Path -LiteralPath $Root) { throw 'Each product pair needs a fresh root; stale reports cannot qualify' }
    foreach ($rel in $required) {
        $source = Join-Path $Repo $rel
        $destination = Join-Path $Root $rel
        if ($rel.StartsWith('examples/') -and (-not (Test-Path -LiteralPath $source))) { $source = Join-Path $Repo ('_archive/old_examples/' + $rel.Substring(9)) }
        if ($Case -eq 'archived-example' -and $rel.StartsWith('examples/')) { $destination = Join-Path $Root ('_archive/old_examples/' + $rel.Substring(9)) }
        if ($Case -eq 'missing-artifact' -and $rel.EndsWith('_spec.md') -and -not $rel.Contains('compatibility')) { continue }
        New-Directory ([IO.Path]::GetDirectoryName($destination)); Copy-Item -LiteralPath $source -Destination $destination
        [IO.File]::SetLastWriteTimeUtc($destination,$fixedTime)
    }
    Copy-Item -LiteralPath (Join-Path $Repo 'tools/tests/fixtures/existing43_primitive_smoke.sh') -Destination (Join-Path $Root 'tools/scripts/run_primitive_qemu_smoke.sh')
    if ($Oracle) {
        $destination = Join-Path $Root 'tools/release/verify_primitive_release_evidence_v2_0_0.py'
        Copy-Item -LiteralPath (Join-Path $Repo 'tools/release/verify_primitive_release_evidence_v2_0_0.py') -Destination $destination
    }
    $contractPath = Join-Path $Root 'docs/spec/release/v2_0_0_primitive_release_evidence_contract.json'
    $contract = Get-Content -Raw $contractPath | ConvertFrom-Json
    switch ($Case) {
        'contract-negative' { $contract.state = 'not-frozen' }
        'gate-string' { $contract.required_gates = 'prefix release_primitive_material_gate_tests suffix' }
        'gate-object' { $contract.required_gates = @{release_primitive_material_gate_tests=$false} }
        'gate-null' { $contract.required_gates = $null }
    }
    Write-Data $contractPath ($utf8.GetBytes(($contract | ConvertTo-Json -Depth 30)))
    $image = [byte[]]::new(512); $image[0]=186; $image[1]=238; $image[2]=236; $image[3]=168; $image[4]=244; $image[510]=85; $image[511]=170
    $elf = [byte[]]::new(64); $elf[0]=127; $elf[1]=69; $elf[2]=76; $elf[3]=70
    if ($Case -ne 'missing-binary') { Write-Data (Join-Path $Root 'test-input/boot_serial_hello.sov') $image }
    Write-Data (Join-Path $Root 'test-input/boot_serial_hello.elf') $elf
    $evidence = "primitive-image-size=512`nprimitive-boot-signature=55aa`nprimitive-opcodes=ba,ee,ec,a8,f4`nprimitive-elf-magic=7f454c46`nqemu-mode=controlled-native-fixture`n"
    switch ($Case) {
        'unicode-kv' { $space=[string][char]160; $line=[string][char]8232; $evidence=$space+'primitive-image-size'+$space+'='+$space+'512'+$space+$line+"primitive-boot-signature=55aa`vprimitive-opcodes=ba,ee,ec,a8,f4`fprimitive-elf-magic=7f454c46`nqemu-mode=controlled-native-fixture`n" }
        'duplicate-field-negative' { $evidence += "primitive-image-size=513`n" }
        'mode-empty' { $evidence = $evidence.Replace('qemu-mode=controlled-native-fixture','qemu-mode=') }
        'mode-nul' { $evidence = $evidence.Replace('controlled-native-fixture',"before`0after") }
        'mode-default' { $evidence = $evidence.Replace("qemu-mode=controlled-native-fixture`n",'') }
        'field-nul-negative' { $evidence = $evidence.Replace('primitive-image-size=512',"primitive-image-size=512`0not512") }
    }
    $evidenceBytes = $utf8.GetBytes($evidence)
    if ($Case -eq 'kv-invalid-utf8') { $evidenceBytes = [byte[]](@(255) + $evidenceBytes) }
    Write-Data (Join-Path $Root 'test-input/material_evidence.txt') $evidenceBytes
    $serial = $utf8.GetBytes("CONTROLLED TEST INPUT, NOT PHYSICAL EXECUTION`nn.C primitive alive`n")
    switch ($Case) {
        'serial-invalid-utf8' { $serial=[byte[]](@(255,192,128)+$serial) }
        'serial-nul' { $serial=$utf8.GetBytes("nCpri`0mitivealive") }
        'serial-letter-negative' { $serial=$utf8.GetBytes('nC'+[char]233+'primitivealive') }
        'serial-punctuation' { $serial=$utf8.GetBytes('nC'+[char]9731+'primitive'+[char]160+'alive') }
    }
    Write-Data (Join-Path $Root 'test-input/qemu_serial.log') $serial
    $staging = Join-Path $Root 'var/releases/v2_0_0/primitive_material_dist/staging/primitive-material-v2.0.0'
    New-Directory $staging
    foreach ($name in $members) { Write-Data (Join-Path $staging $name) ([byte[]]@(0)) }
    if ($Case -eq 'stale-unicode-member') {
        Write-Data (Join-Path $staging ('notes/caf'+[char]233+'.txt')) ($utf8.GetBytes('retained prior staging member'))
        [IO.Directory]::SetLastWriteTimeUtc((Join-Path $staging 'notes'),$fixedTime)
    }
    [IO.Directory]::SetLastWriteTimeUtc($staging,$fixedTime)
    if ($Case -eq 'readonly-source') {
        if ($HostName -eq 'windows') { [IO.File]::SetAttributes((Join-Path $Root 'link/x86_64_baremetal.ld'),[IO.FileAttributes]::ReadOnly) }
        else { & wsl.exe --distribution Ubuntu --exec chmod 0444 (To-Wsl (Join-Path $Root 'link/x86_64_baremetal.ld')); if ($LASTEXITCODE) { throw 'chmod fixture failed' } }
    }
}
function Invoke-Product([string]$Root,[string]$Case,[bool]$Oracle) {
    $info = [Diagnostics.ProcessStartInfo]::new(); $info.UseShellExecute=$false; $info.RedirectStandardOutput=$true; $info.RedirectStandardError=$true; $info.WorkingDirectory=$Repo
    $code = if ($Case -eq 'smoke-failed') { '89' } else { '0' }
    if ($HostName -eq 'windows') {
        $info.FileName = if ($Oracle) { $Python } else { $Candidate }
        $info.Environment['EXISTING43_PRIMITIVE_SMOKE_CODE']=$code
        $info.Environment['EXISTING43_PRIMITIVE_CASE']=$Case
        $info.Environment['PATH']='C:/msys64/mingw64/bin;C:/msys64/usr/bin;'+$env:PATH
        if ($Oracle) { $info.ArgumentList.Add((Join-Path $Root 'tools/release/verify_primitive_release_evidence_v2_0_0.py')) }
        else { $info.ArgumentList.Add('primitive-release-evidence'); $info.ArgumentList.Add('--repo'); $info.ArgumentList.Add($Root) }
    } else {
        $info.FileName='wsl.exe'
        foreach ($word in @('--distribution','Ubuntu','--cd',(To-Wsl $Repo),'--exec','env',('EXISTING43_PRIMITIVE_SMOKE_CODE='+$code),('EXISTING43_PRIMITIVE_CASE='+$Case))) { $info.ArgumentList.Add($word) }
        if ($Oracle) { $info.ArgumentList.Add('python3'); $info.ArgumentList.Add((To-Wsl (Join-Path $Root 'tools/release/verify_primitive_release_evidence_v2_0_0.py'))) }
        else { $info.ArgumentList.Add((To-Wsl $Candidate)); $info.ArgumentList.Add('primitive-release-evidence'); $info.ArgumentList.Add('--repo'); $info.ArgumentList.Add((To-Wsl $Root)) }
    }
    $proc=[Diagnostics.Process]::Start($info); $stdout=$proc.StandardOutput.ReadToEndAsync(); $stderr=$proc.StandardError.ReadToEndAsync()
    if (-not $proc.WaitForExit(300000)) { $proc.Kill($true); throw 'Primitive pair timed out' }
    return [ordered]@{code=$proc.ExitCode;stdout=$stdout.GetAwaiter().GetResult();stderr=$stderr.GetAwaiter().GetResult()}
}
function Canonical($Value) {
    if ($null -eq $Value) { return $null }
    if ($Value -is [double] -or $Value -is [single]) { return [decimal]::Parse($Value.ToString('R',[Globalization.CultureInfo]::InvariantCulture),[Globalization.NumberStyles]::Float,[Globalization.CultureInfo]::InvariantCulture) }
    $numberType=[Type]::GetTypeCode($Value.GetType())
    if ($numberType -ge 5 -and $numberType -le 15) { return [decimal]$Value }
    if ($Value -is [Collections.IDictionary]) { $ordered=[ordered]@{}; foreach ($key in @($Value.Keys | Sort-Object -CaseSensitive)) { $ordered[$key]=Canonical $Value[$key] }; return $ordered }
    if ($Value -is [pscustomobject]) { $ordered=[ordered]@{}; foreach ($property in @($Value.PSObject.Properties | Sort-Object Name -CaseSensitive)) { $ordered[$property.Name]=Canonical $property.Value }; return $ordered }
    if ($Value -is [array]) { return ,@($Value | ForEach-Object { Canonical $_ }) }
    return $Value
}
function Equal($Left,$Right) { return ((Canonical $Left | ConvertTo-Json -Depth 60 -Compress) -ceq (Canonical $Right | ConvertTo-Json -Depth 60 -Compress)) }
function Normalize-Report($Value,[string]$Root) {
    if ($Value.generated_at -notmatch '^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$') { throw 'Report timestamp missing or invalid' }
    $expected = if ($HostName -eq 'windows') { (Join-Path $Root 'var/releases/v2_0_0/primitive_material_dist').Replace('/','\') } else { (To-Wsl $Root)+'/var/releases/v2_0_0/primitive_material_dist' }
    if ($Value.dist_root -cne $expected) { throw "Wrong dist_root: $($Value.dist_root) expected $expected" }
    $Value.PSObject.Properties.Remove('generated_at'); $Value.dist_root='{ROOT}/var/releases/v2_0_0/primitive_material_dist'
    foreach ($command in $Value.commands) { $command.cmd=$command.cmd.Replace('C:\msys64\usr\bin\bash.exe','C:/msys64/usr/bin/bash.exe') }
    return $Value
}
function Check-ProductCli($Output,[string]$Root) {
    $rootText=if ($HostName -eq 'windows') { $Root.Replace('\','/') } else { To-Wsl $Root }
    $expected='OK: primitive release evidence report '+$rootText+'/docs/status/PRIMITIVE_release_evidence_report_v2_0_0.json'+"`n"
    if ($Output.stdout.Replace('\','/').Replace("`r`n","`n") -cne $expected -or $Output.stderr -cne '') { throw 'Product CLI report path/message differs' }
}
function Read-Archive([string]$Path) {
    $file=[IO.File]::OpenRead($Path); $gzip=[IO.Compression.GZipStream]::new($file,[IO.Compression.CompressionMode]::Decompress); $reader=[Formats.Tar.TarReader]::new($gzip)
    $entries=[Collections.Generic.List[object]]::new()
    try {
        while ($entry=$reader.GetNextEntry()) {
            $hash=''; if ($null -ne $entry.DataStream) { $sha=[Security.Cryptography.SHA256]::Create(); $hash=[Convert]::ToHexString($sha.ComputeHash($entry.DataStream)).ToLowerInvariant(); $sha.Dispose() }
            $entries.Add([ordered]@{name=$entry.Name.TrimEnd('/');type=$entry.EntryType.ToString();mode=[int]$entry.Mode;uid=$entry.Uid;gid=$entry.Gid;uname=$entry.UserName;gname=$entry.GroupName;mtime=[Math]::Round(($entry.ModificationTime.Ticks-[DateTimeOffset]::UnixEpoch.Ticks)/10000000.0,6);length=$entry.Length;linkname=$entry.LinkName;sha256=$hash})
        }
    } finally { $reader.Dispose(); $gzip.Dispose(); $file.Dispose() }
    return ,$entries.ToArray()
}
function Read-Sums([string]$Dist) {
    $map=[ordered]@{}; foreach ($line in [IO.File]::ReadAllLines((Join-Path $Dist 'integrity/SHA256SUMS'))) {
        if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { throw 'Malformed SHA256SUMS' }
        if ($map.Contains($Matches[2])) { throw 'Duplicate checksum member' }
        $hash=(Get-FileHash -LiteralPath (Join-Path $Dist $Matches[2])).Hash.ToLowerInvariant()
        if ($hash -cne $Matches[1]) { throw 'Checksum does not match its actual artifact' }
        $map[$Matches[2]]=$hash
    }
    if ($map.Count -ne 5) { throw 'Expected exactly five checksum members' }
    return $map
}
New-Directory $Work
$results=[Collections.Generic.List[object]]::new()
foreach ($case in $cases) {
    $oracleRoot=Join-Path $Work ($case+'/oracle'); $nativeRoot=Join-Path $Work ($case+'/native')
    if ($Frozen) {
        if (-not $frozenRows.ContainsKey($case)) { throw 'Frozen case missing' }
        $reference=$frozenRows[$case]; $oracle=$reference.original_output
    } else { Prepare $oracleRoot $case $true; $oracle=Invoke-Product $oracleRoot $case $true }
    Prepare $nativeRoot $case $false; $native=Invoke-Product $nativeRoot $case $false
    $oracleReport=Join-Path $oracleRoot 'docs/status/PRIMITIVE_release_evidence_report_v2_0_0.json'; $nativeReport=Join-Path $nativeRoot 'docs/status/PRIMITIVE_release_evidence_report_v2_0_0.json'
    $row=[ordered]@{name=$case;host=$HostName;original_exit=$oracle.code;native_exit=$native.code;passed=$false;passed_comparison=$false;checks_equal=$false;report_equal=$false;archive_metadata_equal=$false;archive_contents_equal=$false;original_output=$oracle;native_output=$native}
    try {
        $expectedExit=if ($case -in @('contract-negative','gate-null','duplicate-field-negative','serial-letter-negative','field-nul-negative','kv-invalid-utf8','smoke-failed','missing-artifact','missing-binary')) { 1 } else { 0 }
        if ($oracle.code -ne $expectedExit) { throw "Original did not meet intended fixture outcome: expected $expectedExit" }
        if ($oracle.code -ne $native.code) { throw 'Exit codes differ' }
        $hasOracle=if ($Frozen) { $null -ne $reference.original_report_projection } else { Test-Path -LiteralPath $oracleReport }; $hasNative=Test-Path -LiteralPath $nativeReport
        if ($hasOracle -ne $hasNative) { throw 'Report presence differs' }
        if ($hasOracle) {
            $originalCliRoot=if ($Frozen) { Join-Path (Split-Path $sealed.source_report.path -Parent) ($case+'/oracle') } else { $oracleRoot }
            Check-ProductCli $oracle $originalCliRoot; Check-ProductCli $native $nativeRoot
            $row.cli_product_output_equal=$true
            $before=if ($Frozen) { $reference.original_report_projection } else { Normalize-Report (Get-Content -Raw $oracleReport|ConvertFrom-Json -DateKind String) $oracleRoot }
            $after=Normalize-Report (Get-Content -Raw $nativeReport|ConvertFrom-Json -DateKind String) $nativeRoot
            if (@($before.checks.PSObject.Properties).Count -ne 8 -or @($after.checks.PSObject.Properties).Count -ne 8) { throw 'Eight checks not preserved' }
            $row.checks_equal=Equal $before.checks $after.checks
            $row.report_equal=Equal $before $after
            $row.original_report_projection=$before
            if (-not $row.checks_equal -or -not $row.report_equal) { throw 'Full report projection differs' }
            $originalDist=Join-Path $oracleRoot 'var/releases/v2_0_0/primitive_material_dist'; $nativeDist=Join-Path $nativeRoot 'var/releases/v2_0_0/primitive_material_dist'
            $originalArchive=if ($Frozen) { $reference.original_archive } else { Read-Archive (Join-Path $originalDist 'packages/primitive-material-x86_64-unknown-none-v2.0.0.tar.gz') }; $nativeArchive=Read-Archive (Join-Path $nativeDist 'packages/primitive-material-x86_64-unknown-none-v2.0.0.tar.gz')
            $row.original_archive=$originalArchive; $row.native_archive=$nativeArchive
            $row.archive_metadata_equal=Equal $originalArchive $nativeArchive; $row.archive_contents_equal=$row.archive_metadata_equal
            if (-not $row.archive_metadata_equal) { throw 'Archive members/content/metadata differ' }
            $originalManifest=if ($Frozen) { $reference.original_manifest } else { Get-Content -Raw (Join-Path $originalDist 'manifests/primitive-release-manifest.json')|ConvertFrom-Json }; $nativeManifest=Get-Content -Raw (Join-Path $nativeDist 'manifests/primitive-release-manifest.json')|ConvertFrom-Json
            if (-not (Equal $originalManifest $nativeManifest)) { throw 'Manifest differs' }
            $row.original_manifest=$originalManifest
            $originalSums=if ($Frozen) { Canonical $reference.original_sums } else { Read-Sums $originalDist }; $nativeSums=Read-Sums $nativeDist
            $row.original_sums=$originalSums
            foreach ($name in $originalSums.Keys) { if (-not $name.StartsWith('packages/') -and $originalSums[$name] -cne $nativeSums[$name]) { throw 'Non-package checksum differs' } }
            $row.original_report_sha256=if ($Frozen) { $reference.original_report_sha256 } else { (Get-FileHash $oracleReport).Hash.ToLower() }; $row.native_report_sha256=(Get-FileHash $nativeReport).Hash.ToLower()
        } else {
            if ($case -notin @('kv-invalid-utf8','missing-artifact','missing-binary')) { throw 'Expected substantive product report was not emitted' }
            if ($oracle.stdout -cne '' -or $native.stdout -cne '' -or $oracle.stderr -ceq '' -or $native.stderr -ceq '') { throw 'Expected failure without product report must emit only a diagnostic' }
            $row.cli_failure_behavior_valid=$true
            $row.expected_failure_without_product_report=$true
        }
        $row.passed=$true; $row.passed_comparison=$true
    } catch { $row.error=$_.Exception.Message }
    $results.Add($row); Write-Output "$HostName $case passed_comparison=$($row.passed_comparison) exits=$($oracle.code)/$($native.code) $($row.error)"
}
$passed=@($results | Where-Object {-not $_.passed}).Count -eq 0
$originalRecord=if ($Frozen) { $sealed.original } else { @{path='tools/release/verify_primitive_release_evidence_v2_0_0.py';sha256=(Get-FileHash (Join-Path $Repo 'tools/release/verify_primitive_release_evidence_v2_0_0.py')).Hash.ToLower()} }
$report=[ordered]@{schema='nc.existing43.primitive-pairs.v43';host=$HostName;passed=$passed;failures=@($results | Where-Object {-not $_.passed}).Count;cases=$results.ToArray();candidate=@{path=$Candidate;sha256=(Get-FileHash $Candidate).Hash.ToLower()};original=$originalRecord;python_executed=(-not [bool]$Frozen);frozen_oracle=if ($Frozen) { @{path=$Frozen;sha256=(Get-FileHash $Frozen).Hash.ToLower()} } else { $null };scope='Actual native product/CLI executions on controlled filesystem/smoke inputs; original executions only for initial oracle seal. Not QEMU/physical execution. No Python in native production implementation.';normalizations=@('generated_at timestamp only','fixture-root relocation in dist_root','MSYS bash executable slash spelling','archive directory trailing slash spelling');gzip_header_byte_identity_required=$false;archive_member_metadata_required=$true;originals_deleted=$false}
[IO.File]::WriteAllText((Join-Path $Work 'pairs.json'),($report|ConvertTo-Json -Depth 60)+"`n",$utf8)
if (-not $passed) { exit 1 }
