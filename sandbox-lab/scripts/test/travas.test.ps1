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
  foreach ($p in @('Z:\', 'Q:\', 'Z:', '', '   ', 'build\app', '\\srv\s\app', 'Z:\a\..\b')) { if (-not (Get-LabSourceRefusal -Source $p -Deploy $D)) { throw "must refuse '$p'" } }
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
Check 'source: a link anywhere in the path is refused' {
  Should-Match (Get-LabSourceRefusal -Source 'Z:\build\app x.exe' -Deploy $D -Links @('Z:\build')) 'link'
}
Check 'source: 8.3 short name segments are refused' {
  foreach ($p in @('Z:\PROGRA~1\app', 'Z:\build\APP~2.exe', 'Z:\a~1')) { Should-Match (Get-LabSourceRefusal -Source $p -Deploy $D) '8\.3' }
  Should-BeNull (Get-LabSourceRefusal -Source 'Z:\build\app~beta.exe' -Deploy $D)
}
Check 'project: a link anywhere in the path is refused' {
  Should-Match (Get-LabProjectRefusal -Project $R -UserProfile 'Z:\Perfis\fulano' -Links @('Z:\proj x')) 'link'
}
Check 'project: 8.3 short name segments are refused' {
  Should-Match (Get-LabProjectRefusal -Project 'Z:\JOAOGA~1\proj' -UserProfile 'Z:\Perfis\fulano') '8\.3'
}
Check 'guest marker literal is pinned (the Node side pins the same literal)' {
  if ($script:LabGuestMarker -cne 'C:\labsandbox\bootstrap.ps1') { throw "marker changed: '$script:LabGuestMarker'" }
}
Check 'down: only exit code 0 lets Clear-LabOut run' {
  Should-BeNull (Get-LabDownRefusal -ExitCode 0)
  foreach ($c in @(1, 2, -1, $null, '')) { if (-not (Get-LabDownRefusal -ExitCode $c)) { throw "must refuse for '$c'" } }
}

$UP = 'Y:\Perfis\fulano'
$RR = 'Y:\proj x\claudia-rh'
$CD = 'Y:\proj x\claudia-rh\.sandbox-work\credencial'
$CF = "$CD\.credentials.json"
$G32 = '0123456789abcdef0123456789abcdef'

Check 'guest test root literal is pinned (the Node side pins the same literal)' {
  if ($script:LabGuestTestRoot -cne 'Z:\claudia-teste-sandbox') { throw "root changed: '$script:LabGuestTestRoot'" }
}
Check 'guest test root: the root and anything under it are allowed' {
  foreach ($p in @('Z:\claudia-teste-sandbox', 'z:\CLAUDIA-TESTE-SANDBOX\cs test a 1', 'Z:\claudia-teste-sandbox\a\b', 'Z:\claudia-teste-sandbox\')) { Should-BeNull (Get-LabGuestTestRootRefusal $p) }
}
Check 'guest test root: other drives, siblings, dotted, relative and empty are refused' {
  foreach ($p in @('C:\labdata\x', 'Z:\', 'Z:\tmp\x', 'Z:\claudia-teste-sandbox-irmao\x', 'Z:\claudia-teste-sandbox\..\x', 'Z:\claudia-teste-sandbox\.\x', '', 'claudia-teste-sandbox\x', '\\srv\s\claudia-teste-sandbox')) {
    if (-not (Get-LabGuestTestRootRefusal $p)) { throw "must refuse '$p'" }
  }
}
Check 'host drives: a Z: drive on the host refuses test-up' {
  Should-BeNull (Get-LabHostDriveRefusal -DriveNames @('C:\', 'D:\', 'G:\'))
  Should-BeNull (Get-LabHostDriveRefusal -DriveNames @())
  Should-Match (Get-LabHostDriveRefusal -DriveNames @('C:\', 'Z:\')) 'Z:'
  Should-Match (Get-LabHostDriveRefusal -DriveNames @('z:\')) 'Z:'
}
Check 'credential source: only the default login file of the user folder' {
  Should-BeNull (Get-LabCredentialSourceRefusal -Source "$UP\.claude\.credentials.json" -UserProfile $UP -ConfigDir '')
  Should-BeNull (Get-LabCredentialSourceRefusal -Source "Y:\PERFIS\FULANO\.claude\.credentials.json" -UserProfile "$UP\" -ConfigDir $null)
}
Check 'credential source: CLAUDE_CONFIG_DIR set is refused' {
  Should-Match (Get-LabCredentialSourceRefusal -Source "$UP\.claude\.credentials.json" -UserProfile $UP -ConfigDir 'Y:\outra') 'CLAUDE_CONFIG_DIR'
}
Check 'credential source: other files, empty values, drive-root profile, 8.3 names and links are refused' {
  foreach ($s in @("$UP\.claude\settings.json", "$UP\.claude", "$UP\.claude-work\.credentials.json", 'Y:\outro\.claude\.credentials.json', '')) {
    if (-not (Get-LabCredentialSourceRefusal -Source $s -UserProfile $UP -ConfigDir '')) { throw "must refuse '$s'" }
  }
  Should-Match (Get-LabCredentialSourceRefusal -Source 'Y:\.claude\.credentials.json' -UserProfile 'Y:\' -ConfigDir '') 'user folder'
  Should-Match (Get-LabCredentialSourceRefusal -Source 'Y:\Perfis\FULANO~1\.claude\.credentials.json' -UserProfile 'Y:\Perfis\FULANO~1' -ConfigDir '') '8\.3'
  Should-Match (Get-LabCredentialSourceRefusal -Source "$UP\.claude\.credentials.json" -UserProfile $UP -ConfigDir '' -Links @("$UP\.claude")) 'link'
}
Check 'credential paths: derived only from the repo root' {
  $p = Get-LabCredentialPaths "$RR\"
  if ($p.Dir -cne $CD -or $p.File -cne $CF) { throw "got $($p.Dir) / $($p.File)" }
}
Check 'credential clean: only the exact copy or its .tmp sibling' {
  Should-BeNull (Get-LabCredentialCleanRefusal -File $CF -RepoRoot $RR)
  Should-BeNull (Get-LabCredentialCleanRefusal -File $CF.ToUpper() -RepoRoot $RR)
  Should-BeNull (Get-LabCredentialCleanRefusal -File "$CF.$G32.tmp" -RepoRoot $RR)
  foreach ($f in @($CD, "$CD\x.json", "$CF.abc.tmp", "$CF.$G32.tmp.x", "$RR\.sandbox-work\deploy\.credentials.json", 'Y:\', $RR, "$CF\..", ($CF -replace '\\', '/'), "$UP\.claude\.credentials.json")) {
    if (-not (Get-LabCredentialCleanRefusal -File $f -RepoRoot $RR)) { throw "must refuse '$f'" }
  }
  Should-Match (Get-LabCredentialCleanRefusal -File $CF -RepoRoot '') 'required'
  Should-Match (Get-LabCredentialCleanRefusal -File 'Y:\.sandbox-work\credencial\.credentials.json' -RepoRoot 'Y:\') 'repo root'
  Should-Match (Get-LabCredentialCleanRefusal -File $CF -RepoRoot $RR -Links @("$RR\.sandbox-work")) 'link'
}
Check 'cache dir: only the exact shared\cache, no links' {
  $Lab = "$RR\sandbox-lab"
  Should-BeNull (Get-LabCacheDirRefusal -CacheDir "$Lab\shared\cache" -LabDir $Lab)
  foreach ($c in @("$Lab\shared", "$Lab\shared\cache\x", 'Y:\', "$Lab\shared\out", ("$Lab\shared\cache" -replace '\\', '/'))) {
    Should-Match (Get-LabCacheDirRefusal -CacheDir $c -LabDir $Lab) 'not exactly'
  }
  Should-Match (Get-LabCacheDirRefusal -CacheDir "$Lab\shared\cache" -LabDir $Lab -Links @("$Lab\shared")) 'link'
  Should-Match (Get-LabCacheDirRefusal -CacheDir 'Y:\shared\cache' -LabDir 'Y:\') 'lab dir'
}
Check 'cache plan: missing -> baixar, same hash -> ok, other hash or wrong kind -> conflito' {
  $h1 = 'a' * 64; $h2 = 'b' * 64
  $pk = @(
    [pscustomobject]@{ nome = 'rust'; arquivo = 'rust.msi'; sha256 = $h1 },
    [pscustomobject]@{ nome = 'node'; arquivo = 'node.zip'; sha256 = $h1 },
    [pscustomobject]@{ nome = 'claude-code'; arquivo = 'claude.exe'; sha256 = $h1 },
    [pscustomobject]@{ nome = 'msvc'; arquivo = 'msvc-17'; sha256 = '' }
  )
  $plan = @(Get-LabCachePlan -Pacotes $pk -Existentes @{ 'RUST.msi' = $h1; 'node.zip' = $h2; 'msvc-17' = 'dir' })
  $acts = @($plan | ForEach-Object { "$($_.nome)=$($_.acao)" }) -join ','
  if ($acts -ne 'rust=ok,node=conflito,claude-code=baixar,msvc=ok') { throw "got $acts" }
  $plan2 = @(Get-LabCachePlan -Pacotes @($pk[3], $pk[0]) -Existentes @{ 'msvc-17' = $h1; 'rust.msi' = 'dir' })
  if ((@($plan2 | ForEach-Object { $_.acao }) -join ',') -ne 'conflito,conflito') { throw 'wrong kind must be conflito' }
}
$RO = 'Y:\Perfis\fulano\AppData\Roaming'
Check 'real data: only the app folder under Roaming (spec addendum 8.1)' {
  Should-BeNull (Get-LabRealDataSourceRefusal -Source "$RO\io.github.johngabie.claudia-rh" -RoamingAppData $RO)
  Should-BeNull (Get-LabRealDataSourceRefusal -Source "$RO\IO.GITHUB.JOHNGABIE.CLAUDIA-RH" -RoamingAppData "$RO\")
}
Check 'real data: LOCALAPPDATA, other folders, the Roaming root, empty, 8.3 and links are refused' {
  foreach ($s in @('Y:\Perfis\fulano\AppData\Local\io.github.johngabie.claudia-rh', $RO, "$RO\outro", "$RO\io.github.johngabie.claudia-rh\x", "$RO\io.github.johngabie.claudia-rh2", '')) {
    if (-not (Get-LabRealDataSourceRefusal -Source $s -RoamingAppData $RO)) { throw "must refuse '$s'" }
  }
  Should-Match (Get-LabRealDataSourceRefusal -Source 'Y:\P\AppData\Local\io.github.johngabie.claudia-rh' -RoamingAppData 'Y:\P\AppData\Local') 'LOCALAPPDATA'
  Should-Match (Get-LabRealDataSourceRefusal -Source 'Y:\io.github.johngabie.claudia-rh' -RoamingAppData 'Y:\') 'roaming'
  Should-Match (Get-LabRealDataSourceRefusal -Source 'Y:\PERFIS~1\io.github.johngabie.claudia-rh' -RoamingAppData 'Y:\PERFIS~1') '8\.3'
  Should-Match (Get-LabRealDataSourceRefusal -Source "$RO\io.github.johngabie.claudia-rh" -RoamingAppData $RO -Links @($RO)) 'link'
}
Check 'real data: a link INSIDE the app folder refuses (no link inside is fine)' {
  $src = "$RO\io.github.johngabie.claudia-rh"
  Should-BeNull (Get-LabRealDataSourceRefusal -Source $src -RoamingAppData $RO -LinksInside @())
  Should-BeNull (Get-LabRealDataSourceRefusal -Source $src -RoamingAppData $RO -LinksInside @($null, ''))
  Should-Match (Get-LabRealDataSourceRefusal -Source $src -RoamingAppData $RO -LinksInside @("$src\diagnostics\latest")) 'inside the app data folder'
  Should-Match (Get-LabRealDataSourceRefusal -Source $src -RoamingAppData $RO -Links @() -LinksInside @("$src\x", "$src\y")) 'link'
}
Check 'layout import: only <repo>\.sandbox-work\cache-saida\<8 hex>\msvc-<versao>' {
  $ok = "$RR\.sandbox-work\cache-saida\0a1b2c3d\msvc-17.14.1"
  Should-BeNull (Get-LabLayoutImportRefusal -Source $ok -RepoRoot $RR -Versao '17.14.1')
  foreach ($s in @("$RR\.sandbox-work\cache-saida\0a1b2c3d", "$RR\.sandbox-work\cache-saida\0A1B2C3X\msvc-17.14.1", "$RR\.sandbox-work\cache-saida\0a1b2c3d4e\msvc-17.14.1", "$RR\.sandbox-work\cache-saida\0a1b2c\msvc-17.14.1","$RR\.sandbox-work\cache-saida\0a1b2c3d\msvc-17.14.2",
                   "$RR\.sandbox-work\cache-saida\0a1b2c3d\msvc-17.14.1\layout", "$RR\sandbox-lab\shared\cache\msvc-17.14.1", 'Y:\', '')) {
    if (-not (Get-LabLayoutImportRefusal -Source $s -RepoRoot $RR -Versao '17.14.1')) { throw "must refuse '$s'" }
  }
  Should-Match (Get-LabLayoutImportRefusal -Source $ok -RepoRoot $RR -Versao 'latest') 'versao'
  Should-Match (Get-LabLayoutImportRefusal -Source $ok -RepoRoot $RR -Versao '17.14.1' -Links @("$RR\.sandbox-work")) 'link'
}
Check 'layout manifest: same files, sizes and hashes pass; missing, extra, different or malformed fail' {
  $a = 'a' * 64; $b = 'b' * 64
  $man = "$a 10 vs_setup.exe`n$b 20 Archives/x.cab`n"
  if (@(Get-LabLayoutManifestProblems -ManifestText $man -Found @{ 'vs_setup.exe' = "$a 10"; 'archives/X.cab' = "$b 20" }).Count) { throw 'identical must pass' }
  $p = @(Get-LabLayoutManifestProblems -ManifestText $man -Found @{ 'vs_setup.exe' = "$a 11"; 'extra.txt' = "$a 1" })
  $txt = $p -join ';'
  if ($txt -notmatch 'differs: vs_setup\.exe' -or $txt -notmatch 'missing: Archives/x\.cab' -or $txt -notmatch 'not in the manifest: extra\.txt') { throw "got $txt" }
  Should-Match ((Get-LabLayoutManifestProblems -ManifestText '' -Found @{}) -join ';') 'empty'
  Should-Match ((Get-LabLayoutManifestProblems -ManifestText "lixo`n" -Found @{}) -join ';') 'malformed'
}

if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
