# Pure tests for lib/travas.ps1: invented strings only, no disk. Runs on any OS:
#   pwsh -NoProfile -File sandbox-lab/scripts/test/travas.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../lib/travas.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) {
  try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" }
}
function Should-BeNull($v) { if ($null -ne $v) { throw "expected null, got '$v'" } }
function Should-Match($v, [string]$re) { if ($null -eq $v -or "$v" -notmatch $re) { throw "expected /$re/, got '$v'" } }

$R = 'Z:\proj x\claudia-rh'
$D = 'Z:\proj x\claudia-rh\.sandbox-work\deploy'
$L = 'Z:\proj x\claudia-rh\sandbox-lab'
$O = 'Z:\proj x\claudia-rh\sandbox-lab\shared\out'

Check 'clean: absent deploy at the exact path is allowed' { Should-BeNull (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $false -HasMarker $false) }
Check 'clean: existing deploy with the marker is allowed' { Should-BeNull (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $true) }
Check 'clean: letter case does not matter' { Should-BeNull (Get-LabCleanRefusal -Deploy $D.ToUpper() -RepoRoot $R -Exists $true -HasMarker $true) }
Check 'clean: existing deploy without the marker is refused' { Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $false) 'marker' }
Check 'clean: any other path is refused' {
  foreach ($p in @("$R\.sandbox-work", "$R\.sandbox-work\deploy2", "$D\app", "$D\..\..", "$D.", ($D -replace '\\', '/'), 'Z:\', 'Z:\outro\.sandbox-work\deploy', 'deploy')) {
    Should-Match (Get-LabCleanRefusal -Deploy $p -RepoRoot $R -Exists $true -HasMarker $true) 'not exactly'
  }
}
Check 'clean: a link wins over the marker' { Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot $R -Exists $true -HasMarker $true -Links @("$R\.sandbox-work")) 'link' }
Check 'clean: empty, relative, UNC, drive-root and dotted roots are refused' {
  Should-Match (Get-LabCleanRefusal -Deploy $D -RepoRoot '' -Exists $false -HasMarker $false) 'required'
  Should-Match (Get-LabCleanRefusal -Deploy 'claudia-rh\.sandbox-work\deploy' -RepoRoot 'claudia-rh' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy '\\srv\s\r\.sandbox-work\deploy' -RepoRoot '\\srv\s\r' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy 'Z:\.sandbox-work\deploy' -RepoRoot 'Z:\' -Exists $false -HasMarker $false) 'repo root'
  Should-Match (Get-LabCleanRefusal -Deploy 'Z:\a\..\b\.sandbox-work\deploy' -RepoRoot 'Z:\a\..\b' -Exists $false -HasMarker $false) 'repo root'
}
Check 'out: the exact shared\out is allowed' { Should-BeNull (Get-LabOutCleanRefusal -OutDir $O -LabDir $L) }
Check 'out: any other path is refused' {
  foreach ($p in @("$L\shared", "$O\sub", $L, 'Z:\', ($O -replace '\\', '/'))) { Should-Match (Get-LabOutCleanRefusal -OutDir $p -LabDir $L) 'not exactly' }
}
Check 'out: a link is refused' { Should-Match (Get-LabOutCleanRefusal -OutDir $O -LabDir $L -Links @("$O\x")) 'link' }
Check 'out: bad lab dirs are refused' {
  Should-Match (Get-LabOutCleanRefusal -OutDir 'Z:\shared\out' -LabDir 'Z:\') 'lab dir'
  Should-Match (Get-LabOutCleanRefusal -OutDir $O -LabDir '') 'required'
}

$H = 'a' * 64
$M = "$H 10 chrome/chrome-win64/chrome.exe`n$H 3 uv/uv.exe`n"
Check 'tools: everything present with the right size' {
  $r = Get-LabMissingTools -ManifestText $M -FoundSizes @{ 'chrome/chrome-win64/chrome.exe' = 10; 'uv/uv.exe' = 3; 'extra.txt' = 1 }
  if ($r.Count) { throw "unexpected: $($r -join '; ')" }
}
Check 'tools: missing and size differences are listed' {
  $r = Get-LabMissingTools -ManifestText $M -FoundSizes @{ 'uv/uv.exe' = 4 }
  if ($r.Count -ne 2 -or $r[0] -notmatch '^missing: chrome/' -or $r[1] -notmatch '^size differs: uv/uv\.exe') { throw "got: $($r -join '; ')" }
}
Check 'tools: letter case does not matter' {
  $r = Get-LabMissingTools -ManifestText $M -FoundSizes @{ 'Chrome/Chrome-Win64/Chrome.exe' = 10; 'UV/uv.exe' = 3 }
  if ($r.Count) { throw "unexpected: $($r -join '; ')" }
}
Check 'tools: an empty or malformed manifest is a problem, never a pass' {
  Should-Match ((Get-LabMissingTools -ManifestText '' -FoundSizes @{}) -join ';') 'empty'
  Should-Match ((Get-LabMissingTools -ManifestText "lixo`n" -FoundSizes @{}) -join ';') 'malformed'
}

Check 'sandbox: needs BOTH the WDAGUtilityAccount identity and the guest bootstrap folder' {
  if (-not (Test-LabSandboxDecision -Identity 'PC-NOME\WDAGUtilityAccount' -HasBootstrap $true)) { throw 'both factors should pass' }
  if (-not (Test-LabSandboxDecision -Identity 'pc\wdagutilityaccount' -HasBootstrap $true)) { throw 'case must not matter' }
  if (Test-LabSandboxDecision -Identity 'PC-NOME\WDAGUtilityAccount' -HasBootstrap $false) { throw 'identity alone must fail' }
  if (Test-LabSandboxDecision -Identity 'PC-NOME\fulano' -HasBootstrap $true) { throw 'bootstrap alone must fail' }
  foreach ($i in @('', $null, 'WDAGUtilityAccount', 'PC\WDAGUtilityAccount2', 'PC\xWDAGUtilityAccount', 'PC\WDAGUtilityAccount\x')) {
    if (Test-LabSandboxDecision -Identity $i -HasBootstrap $true) { throw "must fail for '$i'" }
  }
}
Check 'scan: only the root may be missing; an unreadable child refuses' {
  if (-not (Test-LabScanEntryOk -IsRoot $true -Found $false)) { throw 'absent root is fine' }
  if (-not (Test-LabScanEntryOk -IsRoot $false -Found $true)) { throw 'readable child is fine' }
  if (-not (Test-LabScanEntryOk -IsRoot $true -Found $true)) { throw 'readable root is fine' }
  if (Test-LabScanEntryOk -IsRoot $false -Found $false) { throw 'unreadable child must refuse' }
}
Check 'source: ordinary folders and files are allowed' {
  Should-BeNull (Get-LabSourceRefusal -Source 'Z:\build\app x.exe' -Deploy $D)
  Should-BeNull (Get-LabSourceRefusal -Source 'Z:\dados\roaming\app' -Deploy $D)
  Should-BeNull (Get-LabSourceRefusal -Source "$R\src-tauri\target" -Deploy $D)
}
Check 'source: drive roots, empty and relative values are refused' {
  foreach ($p in @('Z:\', 'Z:', '', '   ', 'build\app', '\\srv\s\app', 'Z:\a\..\b')) { if (-not (Get-LabSourceRefusal -Source $p -Deploy $D)) { throw "must refuse '$p'" } }
}
Check 'source: the deploy, its parents and anything inside it are refused' {
  foreach ($p in @($D, "$D\app", $D.ToUpper(), "$R\.sandbox-work", $R, 'Z:\proj x')) {
    Should-Match (Get-LabSourceRefusal -Source $p -Deploy $D) 'deploy'
  }
}
Check 'source: a sibling whose name only starts like the deploy is allowed' {
  Should-BeNull (Get-LabSourceRefusal -Source "$D-copia" -Deploy $D)
  Should-BeNull (Get-LabSourceRefusal -Source "$R\.sandbox-work2" -Deploy $D)
}
Check 'project: ordinary project folders are allowed' {
  Should-BeNull (Get-LabProjectRefusal -Project $R -UserProfile 'Z:\Perfis\fulano')
  Should-BeNull (Get-LabProjectRefusal -Project 'Z:\Perfis\fulano\proj' -UserProfile 'Z:\Perfis\fulano')
}
Check 'project: drive roots, the user folder and its parent are refused' {
  foreach ($p in @('Z:\', 'Z:', 'Z:\Perfis\fulano', 'Z:\Perfis\fulano\', 'z:\perfis\FULANO', 'Z:\Perfis')) {
    if (-not (Get-LabProjectRefusal -Project $p -UserProfile 'Z:\Perfis\fulano')) { throw "must refuse '$p'" }
  }
}
Check 'project: empty, relative project and unknown user folder are refused' {
  foreach ($p in @('', 'proj', 'Z:\a\..\b')) { if (-not (Get-LabProjectRefusal -Project $p -UserProfile 'Z:\Perfis\fulano')) { throw "must refuse '$p'" } }
  Should-Match (Get-LabProjectRefusal -Project $R -UserProfile '') 'user'
}
Check 'down: only exit code 0 lets Clear-LabOut run' {
  Should-BeNull (Get-LabDownRefusal -ExitCode 0)
  foreach ($c in @(1, 2, -1, $null, '')) { if (-not (Get-LabDownRefusal -ExitCode $c)) { throw "must refuse for '$c'" } }
}

if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
