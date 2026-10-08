# Pure tests for sandbox/guest-install.lib.ps1. No disk.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/guest-install.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../../sandbox/guest-install.lib.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }
function Rs([string]$n, $e) { [pscustomobject]@{ nome = $n; exit = $e } }

Check 'exit: 3010 (reboot requested) is success, null is failure' {
  if ((ConvertTo-LabInstallExit 0) -ne 0 -or (ConvertTo-LabInstallExit 3010) -ne 0) { throw 'success codes' }
  if ((ConvertTo-LabInstallExit 1603) -ne 1603 -or (ConvertTo-LabInstallExit $null) -ne 1) { throw 'failure codes' }
}
Check 'ok: every expected package, in order, all exit 0' {
  $exp = @('msvc', 'rust', 'node', 'claude-code')
  if (-not (Test-LabInstallOk -Expected $exp -Results @((Rs 'msvc' 0), (Rs 'rust' 0), (Rs 'node' 0), (Rs 'claude-code' 0)))) { throw 'all ok' }
  if (Test-LabInstallOk -Expected $exp -Results @((Rs 'msvc' 0), (Rs 'rust' 1603))) { throw 'stopped early' }
  if (Test-LabInstallOk -Expected $exp -Results @((Rs 'msvc' 0), (Rs 'rust' 0), (Rs 'node' 0), (Rs 'claude-code' 2))) { throw 'last failed' }
  if (Test-LabInstallOk -Expected @('rust', 'msvc') -Results @((Rs 'msvc' 0), (Rs 'rust' 0))) { throw 'order' }
  if (-not (Test-LabInstallOk -Expected @() -Results @())) { throw 'nothing to install is ok' }
}
Check 'tool env: PATH folders and vcvars from instalar.json; nothing when not ok' {
  $inst = [pscustomobject]@{ ok = $true; pacotes = @(
    [pscustomobject]@{ nome = 'msvc'; exit = 0; binDir = $null; vcvars = 'C:\labtools\msvc\VC\Auxiliary\Build\vcvars64.bat' },
    [pscustomobject]@{ nome = 'rust'; exit = 0; binDir = 'C:\Program Files\Rust stable MSVC 1.95\bin'; vcvars = $null },
    [pscustomobject]@{ nome = 'node'; exit = 0; binDir = 'C:\labtools\node\node-v22.23.3-win-x64'; vcvars = $null }) }
  $e = Get-LabGuestToolEnv $inst
  if (($e.paths -join '|') -ne 'C:\Program Files\Rust stable MSVC 1.95\bin|C:\labtools\node\node-v22.23.3-win-x64') { throw "paths $($e.paths -join '|')" }
  if ($e.vcvars -ne 'C:\labtools\msvc\VC\Auxiliary\Build\vcvars64.bat') { throw 'vcvars' }
  if ($null -ne (Get-LabGuestToolEnv ([pscustomobject]@{ ok = $false; pacotes = @() }))) { throw 'not ok -> null' }
  if ($null -ne (Get-LabGuestToolEnv $null)) { throw 'missing -> null' }
}
function Pk([string]$n, $e, [string]$bin = '', [string]$vc = '') { [pscustomobject]@{ nome = $n; exit = $e; binDir = $(if ($bin) { $bin } else { $null }); vcvars = $(if ($vc) { $vc } else { $null }) } }
Check 'partial install (MSVC failed, Node ok): the Node environment comes out, cargo and todos do not' {
  $inst = [pscustomobject]@{ ok = $false; fim = 'x'; pacotes = @(
    (Pk 'msvc' 1603 '' 'C:\labtools\msvc\vcvars64.bat'), (Pk 'rust' 0 'C:\rust\bin'), (Pk 'node' 0 'C:\labtools\node'), (Pk 'claude-code' 0 'C:\labtools\claude')) }
  $e = Get-LabGuestToolEnv $inst -Precisa node
  if ($null -eq $e) { throw 'node must be available' }
  if (($e.paths -join '|') -ne 'C:\rust\bin|C:\labtools\node|C:\labtools\claude') { throw "paths $($e.paths -join '|')" }
  if ($e.vcvars) { throw 'the vcvars of a failed MSVC must not be imported' }
  if ($null -ne (Get-LabGuestToolEnv $inst -Precisa cargo)) { throw 'cargo needs MSVC ok' }
  if ($null -ne (Get-LabGuestToolEnv $inst)) { throw 'todos needs every package ok' }
  if ($null -ne (Get-LabGuestToolEnv $inst -Precisa nada)) { throw 'unknown need must fail closed' }
}
Check 'partial install: Node failed -> no node env; Rust failed -> no cargo env; unfinished install -> neither' {
  $nodeBad = [pscustomobject]@{ ok = $false; fim = 'x'; pacotes = @((Pk 'msvc' 0), (Pk 'rust' 0), (Pk 'node' 1)) }
  if ($null -ne (Get-LabGuestToolEnv $nodeBad -Precisa node)) { throw 'node failed' }
  if ($null -ne (Get-LabGuestToolEnv $nodeBad -Precisa cargo)) { throw 'cargo also needs node (the documented command runs npm)' }
  $allOk = [pscustomobject]@{ ok = $true; fim = 'x'; pacotes = @((Pk 'msvc' 0), (Pk 'rust' 0), (Pk 'node' 0)) }
  if ($null -eq (Get-LabGuestToolEnv $allOk -Precisa cargo)) { throw 'msvc, rust and node ok -> cargo env' }
  $rustBad = [pscustomobject]@{ ok = $false; fim = 'x'; pacotes = @((Pk 'msvc' 0), (Pk 'rust' 1603), (Pk 'node' 0)) }
  if ($null -ne (Get-LabGuestToolEnv $rustBad -Precisa cargo)) { throw 'rust failed' }
  $running = [pscustomobject]@{ ok = $false; fim = $null; pacotes = @((Pk 'msvc' 0), (Pk 'rust' 0), (Pk 'node' 0)) }
  if ($null -ne (Get-LabGuestToolEnv $running -Precisa node)) { throw 'still installing' }
  if ($null -ne (Get-LabGuestToolEnv $running -Precisa cargo)) { throw 'still installing (cargo)' }
  if ($null -ne (Get-LabGuestToolEnv $null -Precisa node)) { throw 'missing json' }
  if (Test-LabInstallOk -Expected @('msvc', 'rust', 'node') -Results @((Pk 'msvc' 1), (Pk 'rust' 0), (Pk 'node' 0))) { throw 'a failed package keeps the global ok false' }
}
Check 'guest scripts: BOTH factors (guest identity AND the bootstrap marker); SYSTEM only with the WDAG profile' {
  $d = { param($id, $sys, $boot, $prof) Test-LabGuestScriptDecision -Identity $id -IsSystem $sys -HasBootstrap $boot -HasWdagProfile $prof }
  if (-not (& $d 'PC\WDAGUtilityAccount' $false $true $false)) { throw 'WDAG + marker passes' }
  if (-not (& $d 'pc\wdagutilityaccount' $false $true $false)) { throw 'case must not matter' }
  if (& $d 'PC\WDAGUtilityAccount' $false $false $true) { throw 'identity without the marker fails' }
  if (& $d 'PC\fulano' $false $true $true) { throw 'marker without a guest identity fails' }
  if (-not (& $d 'NT AUTHORITY\SYSTEM' $true $true $true)) { throw 'SYSTEM + WDAG profile + marker passes (wsb exec -r System)' }
  if (-not (& $d 'NT-AUTORITAT\SYSTEM' $true $true $true)) { throw 'a localized SYSTEM name is decided by IsSystem' }
  if (& $d 'NT AUTHORITY\SYSTEM' $true $true $false) { throw 'SYSTEM without the WDAG profile fails (a host SYSTEM)' }
  if (& $d 'NT AUTHORITY\SYSTEM' $true $false $true) { throw 'SYSTEM without the marker fails' }
  if (& $d 'NT AUTHORITY\SYSTEM' $false $true $true) { throw 'the name alone is not SYSTEM: IsSystem is required' }
  foreach ($i in @('', $null, 'WDAGUtilityAccount', 'PC\WDAGUtilityAccount2', 'PC\xWDAGUtilityAccount')) {
    if (& $d $i $false $true $true) { throw "must fail for '$i'" }
  }
}
Check 'scenario folder for guest-run: only a direct child of Z:\claudia-teste-sandbox' {
  foreach ($ok in @('Z:\claudia-teste-sandbox\novo', 'z:\CLAUDIA-TESTE-SANDBOX\mock', 'Z:\claudia-teste-sandbox\cs test x 1')) {
    if (Get-LabGuestScenarioRefusal $ok) { throw "must allow '$ok'" }
  }
  foreach ($bad in @('', 'Z:\claudia-teste-sandbox', 'Z:\claudia-teste-sandbox\a\b', 'Z:\claudia-teste-sandbox\..\x', 'C:\labdata\novo', 'Z:\tmp\novo', 'Z:\claudia-teste-sandbox-irmao\novo', 'novo', '\\srv\s\novo')) {
    if (-not (Get-LabGuestScenarioRefusal $bad)) { throw "must refuse '$bad'" }
  }
}
Check 'scenario cenario.json for guest-run: only a finished scenario (valid cenario.json) is installed' {
  $good = '{"cenario":"copia","criadoEm":"2026-10-08T12:00:00.000Z","origem":"C:\\labdadosreais","arquivos":["appdata\\x\\a.db"]}'
  if (Get-LabGuestScenarioJsonRefusal $good) { throw 'a finished copia scenario must be accepted' }
  if (Get-LabGuestScenarioJsonRefusal '{"cenario":"novo","criadoEm":"2026-10-08T12:00:00.000Z","origem":null,"arquivos":[]}') { throw 'an empty arquivos list is a valid novo scenario' }
  foreach ($bad in @($null, '', '   ', 'not json', '[]', '"copia"', '{}', '{"cenario":"outro","criadoEm":"x","arquivos":[]}', '{"cenario":"copia","arquivos":[]}',
      '{"cenario":"copia","criadoEm":"x"}', '{"cenario":"copia","criadoEm":"x","arquivos":"a"}', '{"cenario":"copia","criadoEm":"x","arquivos":null}', '{"cenario":"copia","criadoEm":"x","arquivos":{}}', '{"cenario":"copia"')) {
    if (-not (Get-LabGuestScenarioJsonRefusal $bad)) { throw "must refuse '$bad'" }
  }
  if ((Get-LabGuestScenarioJsonRefusal $null) -notmatch 'partial copy') { throw 'the missing-file message must name the partial copy' }
}
Check 'guest copia: the PC fact about diagnostics\latest counts only as a literal true together with dadosReais' {
  $cfg = { param($json) $json | ConvertFrom-Json }
  if (-not (Test-LabGuestLatestLinkApproved (& $cfg '{"dadosReais":true,"dadosReaisLatestLink":true}'))) { throw 'true + true must be approved' }
  foreach ($bad in @('{"dadosReais":true}', '{"dadosReais":true,"dadosReaisLatestLink":false}', '{"dadosReais":false,"dadosReaisLatestLink":true}', '{"dadosReaisLatestLink":true}',
      '{"dadosReais":true,"dadosReaisLatestLink":"true"}', '{"dadosReais":true,"dadosReaisLatestLink":1}', '{"dadosReais":"true","dadosReaisLatestLink":true}', '{}')) {
    if (Test-LabGuestLatestLinkApproved (& $cfg $bad)) { throw "must NOT be approved: $bad" }
  }
  if (Test-LabGuestLatestLinkApproved $null) { throw 'no config must not be approved' }
}
$bootA = 'a1b2c3d4e5f60718293a4b5c6d7e8f90'   # the id of this boot (the old one below is from another Sandbox)
$bootOld = '0f9e8d7c6b5a49382716a5b4c3d2e1f0'
$fwOk = (@("BOOTID: $bootA", 'Domain enabled=True', 'Private enabled=True', 'Public enabled=True', 'regra lab-block-internet-out: 1',
  'INTERNET: BLOQUEADA (Unable to connect to the remote server)', 'storage.googleapis.com: BLOQUEADO') -join "`r`n") + "`r`n"
Check 'guest internet: allowed only with a real boolean false in guest.config.json; anything else fails closed' {
  if (-not (Get-LabGuestInternetPermitida ([pscustomobject]@{ internetBloqueada = $false }))) { throw 'false -> allowed' }
  if (Get-LabGuestInternetPermitida ([pscustomobject]@{ internetBloqueada = $true })) { throw 'true -> blocked' }
  if (Get-LabGuestInternetPermitida $null) { throw 'no config -> blocked' }
  if (Get-LabGuestInternetPermitida ([pscustomobject]@{ outro = 1 })) { throw 'missing field -> blocked' }
  foreach ($v in @('false', 0, '', $null)) { if (Get-LabGuestInternetPermitida ([pscustomobject]@{ internetBloqueada = $v })) { throw "value '$v' must not allow the internet" } }
  if (Get-LabGuestInternetPermitida 'texto') { throw 'not an object -> blocked' }
}
Check 'app start: internet allowed -> no refusal; blocked -> only with the whole block mark in firewall.txt' {
  if (Get-LabAppStartRefusal -InternetPermitida $true -FirewallText $null) { throw 'internet allowed needs no firewall.txt' }
  if (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $fwOk -ExpectedBootId $bootA) { throw 'complete mark must pass' }
  if (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText ($fwOk -replace "`r`n", "`n") -ExpectedBootId $bootA) { throw 'LF line endings must pass' }
  foreach ($t in @($null, '', '   ', "`r`n")) { if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $t -ExpectedBootId $bootA)) { throw "missing/empty file '$t' must refuse" } }
}
Check 'app start: every missing or contradicting piece of the mark refuses' {
  $bad = @{
    'internet reachable'  = $fwOk.Replace('INTERNET: BLOQUEADA (Unable to connect to the remote server)', 'INTERNET: ACESSIVEL (FALHA)')
    'storage reachable'   = $fwOk.Replace('storage.googleapis.com: BLOQUEADO', 'storage.googleapis.com: ACESSIVEL (FALHA)')
    'no internet line'    = $fwOk.Replace('INTERNET: BLOQUEADA (Unable to connect to the remote server)', '')
    'no storage line'     = $fwOk.Replace('storage.googleapis.com: BLOQUEADO', '')
    'zero rules'          = $fwOk.Replace('regra lab-block-internet-out: 1', 'regra lab-block-internet-out: 0')
    'two rules'           = $fwOk.Replace('regra lab-block-internet-out: 1', 'regra lab-block-internet-out: 2')
    'domain profile off'  = $fwOk.Replace('Domain enabled=True', 'Domain enabled=False')
    'private profile off' = $fwOk.Replace('Private enabled=True', 'Private enabled=False')
    'public profile off'  = $fwOk.Replace('Public enabled=True', 'Public enabled=False')
    'no profile lines'    = "regra lab-block-internet-out: 1`r`nINTERNET: BLOQUEADA (x)`r`nstorage.googleapis.com: BLOQUEADO`r`n"
    'mark in other case'  = $fwOk.Replace('INTERNET: BLOQUEADA', 'internet: bloqueada')
    'mark AND a reachable line' = $fwOk + "INTERNET: ACESSIVEL (FALHA)`r`n"
    'junk file'           = 'qualquer coisa'
  }
  foreach ($k in $bad.Keys) { if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $bad[$k] -ExpectedBootId $bootA)) { throw "must refuse: $k" } }
}
# --- boot id: firewall.txt survives between Sandboxes, so it counts only with the id of THIS boot (pure; also run on mutated copies of the lib) ---
$bootCases = [ordered]@{
  'boot id: the firewall.txt of this boot passes' = { if (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $fwOk -ExpectedBootId $bootA) { throw 'must pass' } }
  'boot id: firewall.txt of ANOTHER boot (old id) is refused' = {
    $old = $fwOk.Replace("BOOTID: $bootA", "BOOTID: $bootOld")
    if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $old -ExpectedBootId $bootA)) { throw 'accepted a firewall.txt from another boot' } }
  'boot id: firewall.txt WITHOUT a BOOTID line (older format) is refused' = {
    $none = $fwOk.Replace("BOOTID: $bootA`r`n", '')
    if ($none -ceq $fwOk) { throw 'harness bug: nothing removed' }
    if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $none -ExpectedBootId $bootA)) { throw 'accepted a firewall.txt with no id' } }
  'boot id: two BOOTID lines (the right one and an old one) are refused' = {
    if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText ($fwOk + "BOOTID: $bootOld`r`n") -ExpectedBootId $bootA)) { throw 'accepted two ids' }
    if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText ("BOOTID: $bootOld`r`n" + $fwOk) -ExpectedBootId $bootA)) { throw 'accepted an old id before the right one' } }
  'boot id: a missing or malformed EXPECTED id refuses, even with a matching line' = {
    foreach ($e in @($null, '', '   ', 'abc', $bootA.ToUpper(), "$bootA`n", "$bootA ", ($bootA + 'f'), $bootA.Substring(1))) {
      $txt = $fwOk.Replace("BOOTID: $bootA", "BOOTID: $e")
      if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $txt -ExpectedBootId $e)) { throw "accepted the expected id '$e'" }
    } }
  'boot id: the line must be exactly "BOOTID: <id>" (no space, other case)' = {
    foreach ($l in @("BOOTID:$bootA", "bootid: $bootA", "BOOTID:  $bootA", "BOOTID: $bootA x")) {
      if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $fwOk.Replace("BOOTID: $bootA", $l) -ExpectedBootId $bootA)) { throw "accepted '$l'" }
    } }
  'boot id: internet allowed still needs no id and no firewall.txt' = { if (Get-LabAppStartRefusal -InternetPermitida $true -FirewallText $null -ExpectedBootId $null) { throw 'refused with the internet allowed' } }
  'boot id: Test-LabBootId takes only 32 lowercase hex digits; the writer line round-trips' = {
    if (-not (Test-LabBootId $bootA)) { throw 'valid id refused' }
    foreach ($b in @($null, '', 'abc', $bootA.ToUpper(), "$bootA`n", "$bootA`r`n", ($bootA + '0'), ('g' * 32))) { if (Test-LabBootId $b) { throw "accepted '$b'" } }
    if ((Format-LabFirewallBootLine $bootA) -cne "BOOTID: $bootA") { throw 'line format' } }
}
function Invoke-BootCases { $f = New-Object System.Collections.Generic.List[string]; foreach ($k in $bootCases.Keys) { try { & $bootCases[$k] } catch { $f.Add("$k - $($_.Exception.Message)") } }; return @($f) }
foreach ($k in $bootCases.Keys) { Check $k $bootCases[$k] }
$libText = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '../../sandbox/guest-install.lib.ps1'))
$bootMut = [ordered]@{
  'accept the firewall.txt of another subida (id not compared)' = @(, @('-or $idLines[0] -cne (Format-LabFirewallBootLine $ExpectedBootId)', ''))
  'accept an absent id (no BOOTID line)'                          = @(@('$idLines.Count -eq 0', '$false'), @('$idLines.Count -ne 1 -or $idLines[0] -cne (Format-LabFirewallBootLine $ExpectedBootId)', '$idLines.Count -gt 1'))
  'do not require the expected id of this guest'                  = @(, @('if (-not (Test-LabBootId $ExpectedBootId)) { return', 'if ($false) { return'))
  'Test-LabBootId accepts anything'                               = @(, @('\A[0-9a-f]{32}\z', '.*'))
  'Test-LabBootId with $ instead of \z (accepts a trailing newline)' = @(, @('\A[0-9a-f]{32}\z', '\A[0-9a-f]{32}$'))
}
foreach ($m in $bootMut.Keys) {
  Check "boot id mutation must be caught: $m" {
    $text = $libText
    foreach ($pair in $bootMut[$m]) { if (-not $text.Contains($pair[0])) { throw "harness bug: '$($pair[0])' not found in the lib" }; $text = $text.Replace($pair[0], $pair[1]) }
    $fails = @(& { . ([scriptblock]::Create($text)); Invoke-BootCases })
    if (-not $fails.Count) { throw 'NOT detected: the mutated lib passed every boot id case' }
    foreach ($x in $fails) { Write-Host "    mutation '$m' -> FAILED - $x" }
  }
}
Check 'vs_layout verdict: completed layout, no failure marker, layout files present; tolerating only the exit-time console finalizer crash' {
  # Texts as seen live in the Sandbox (2026-10-08), trimmed: the real success log, with lines that MUST NOT count as failure
  # ("Skipping layout with missing or corrupt payload" is logged on every run; ErrorList is a package name).
  $done = @"
[31e4:0001][2026-10-08T11:06:20] Cache: Skipping layout with missing or corrupt payload Microsoft.VisualStudio.VC.Icons at 'C:\labdata\msvc-layout'.
[31e4:0012][2026-10-08T11:06:21] BEGIN: Downloading package "Microsoft.VisualStudio.ErrorList,version=17.14.305.26414,productarch=neutral"
[31e4:0012][2026-10-08T11:06:22] Checking SHA256 for path: C:\labdata\msvc-layout\Microsoft.VisualStudio.ErrorList,version=17.14.305.26414,productarch=neutral\payload.vsix
[31e4:0012][2026-10-08T11:06:22] SHA256 verification for 'C:\labdata\msvc-layout\Microsoft.VisualStudio.ErrorList,version=17.14.305.26414,productarch=neutral\payload.vsix' succeeded. Hash: 6FD4A573BA7FC002A4EFE4C5567B78C58EE1B58BC8ED322E28944638F72A380C
Layout progress: 98%
[31e4:0012][2026-10-08T11:07:00] END: Downloading package "Microsoft.WebView2,version=131.0.2903.86,productarch=neutral,machinearch=ARM64"
[31e4:0001][2026-10-08T11:07:00] BEGIN: Deleting directory: C:\Windows\SystemTemp\ss1odrvu, recursive: True
[31e4:0001][2026-10-08T11:07:00] C:\Windows\SystemTemp\ss1odrvu directory deleted successfully.
[31e4:0001][2026-10-08T11:07:00] END: Deleting directory: C:\Windows\SystemTemp\ss1odrvu, recursive: True
[31e4:0001][2026-10-08T11:07:00] Completed 'Layout' operation
A instalacao foi concluida com exito.

"@ -replace "`r?`n", "`r`n"
  $finalizer = "`r`nUnhandled Exception: System.IO.IOException: Parametro incorreto.`r`n`r`n   at System.IO.__Error.WinIOError(Int32 errorCode, String maybeFullPath)`r`n   at System.Console.ControlCHooker.Unhook()`r`n   at System.Console.ControlCHooker.Finalize()`r`n"
  $writeLine = "`r`nUnhandled Exception: System.IO.IOException`r`n   at System.IO.__ConsoleStream.Write(Byte[], Int32, Int32)`r`n   at System.Console.WriteLine(System.String)`r`n"
  $files = @('Catalog.json', 'ChannelManifest.json', 'Layout.json', 'Response.json', 'vs_buildtools.exe', 'vs_installer.opc', 'vs_setup.exe')
  if ($null -ne (Get-LabVsLayoutRefusal -ExitCode 0 -StdOut $done -StdErr '' -LayoutFiles $files)) { throw 'exit 0 + completed + files must pass' }
  if ($null -ne (Get-LabVsLayoutRefusal -ExitCode -532462766 -StdOut $done -StdErr $finalizer -LayoutFiles $files)) { throw 'completed + finalizer crash must pass' }
  if ($null -ne (Get-LabVsLayoutRefusal -ExitCode 0 -StdOut $done -StdErr '' -LayoutFiles @('catalog.JSON', 'VS_SETUP.EXE'))) { throw 'names compare case-insensitively; vs_setup.exe alone is enough' }
  if ($null -ne (Get-LabVsLayoutRefusal -ExitCode 0 -StdOut $done -StdErr '' -LayoutFiles @('Catalog.json', 'vs_buildtools.exe'))) { throw 'vs_buildtools.exe alone is enough' }
  $bad = [ordered]@{
    'null exit' = @($null, $done, '', $files)
    'exit 0 without completion' = @(0, "[x] Catalog load operation correlation`r`n", '', $files)
    'finalizer crash without completion' = @(-532462766, "[x] Downloading package`r`n", $finalizer, $files)
    'WriteLine crash' = @(-532462766, $done, $writeLine, $files)
    'two crashes' = @(-532462766, $done, ($finalizer + $finalizer), $files)
    'finalizer crash in another frame' = @(-532462766, $done, $finalizer.Replace('ControlCHooker.Unhook()', 'ConsoleStream.Write(Byte[], Int32, Int32)'), $files)
    'other exit, empty stderr' = @(1, $done, '', $files)
    'completion only quoted mid-line' = @(0, "not Completed 'Layout' operation yet, retrying`r`n", '', $files)
    'package failed to download, then completion' = @(0, ("[31e4:0012][2026-10-08T11:06:30] Failed to download package Microsoft.VisualCpp.Redist.14`r`n" + $done), '', $files)
    'SHA256 verification failed, then completion' = @(0, ("[31e4:0012][2026-10-08T11:06:30] SHA256 verification for 'C:\labdata\msvc-layout\x\payload.vsix' failed. Hash: AB`r`n" + $done), '', $files)
    'error-level log line (e:), then completion' = @(0, ("[31e4:0012][2026-10-08T11:06:30]e: Package x could not be installed`r`n" + $done), '', $files)
    'exception in the log, then completion' = @(0, ("[31e4:0012][2026-10-08T11:06:30] System.Net.WebException: The remote server returned an error`r`n" + $done), '', $files)
    'HRESULT in the log, then completion' = @(0, ("[31e4:0012][2026-10-08T11:06:30] Operation returned 0x80070570`r`n" + $done), '', $files)
    'layout files not listed' = @(0, $done, '', $null)
    'layout folder empty' = @(0, $done, '', @())
    'no Catalog.json' = @(0, $done, '', @('vs_buildtools.exe', 'vs_setup.exe', 'Layout.json'))
    'no bootstrapper in the layout' = @(0, $done, '', @('Catalog.json', 'Layout.json', 'vs_installer.opc'))
    'finalizer crash tolerated only with a complete layout' = @(-532462766, $done, $finalizer, @('Layout.json'))
  }
  foreach ($k in $bad.Keys) { $v = $bad[$k]; if (-not (Get-LabVsLayoutRefusal -ExitCode $v[0] -StdOut $v[1] -StdErr $v[2] -LayoutFiles $v[3])) { throw "must refuse: $k" } }
}
Check 'version probe: only exit 0 is success; -1 is never accepted, even with a version line (the 2026-10-08 Node case)' {
  $p = ConvertTo-LabVersionProbe -Output @('v22.23.3') -ExitCode 0
  if ($p.exit -ne 0 -or $p.versao -ne 'v22.23.3') { throw "exit 0: $($p.exit) / $($p.versao)" }
  $p = ConvertTo-LabVersionProbe -Output @('v22.23.3') -ExitCode -1
  if ($p.exit -ne -1) { throw "exit -1 with a version line must stay -1, got $($p.exit)" }
  if ($p.versao -ne 'v22.23.3') { throw 'the version line is still reported for diagnosis' }
  $p = ConvertTo-LabVersionProbe -Output @('boom') -ExitCode 3
  if ($p.exit -ne 3) { throw 'other codes are kept' }
  $p = ConvertTo-LabVersionProbe -Output @('v22.23.3') -ExitCode $null
  if ($p.exit -ne 1) { throw 'no code at all is a failure (1)' }
}
Check 'version probe: the version is the first non-empty line of the whole output' {
  $p = ConvertTo-LabVersionProbe -Output @('', '  rustc 1.95.0 (abc 2026-01-01)  ', 'second line') -ExitCode 0
  if ($p.versao -ne 'rustc 1.95.0 (abc 2026-01-01)') { throw "got '$($p.versao)'" }
  $p = ConvertTo-LabVersionProbe -Output @() -ExitCode 0
  if ($p.exit -ne 0 -or $null -ne $p.versao) { throw 'empty output with exit 0: exit 0, no version' }
  $rec = New-Object System.Management.Automation.ErrorRecord ((New-Object System.Exception 'warn line')), 'x', 'NotSpecified', $null
  $p = ConvertTo-LabVersionProbe -Output @($rec, 'v1') -ExitCode 0
  if ($p.versao -ne 'warn line') { throw 'a stderr ErrorRecord is read as text' }
}

# --- host toolchains mounted read-only at C:\hosttools (ambiente testes; spec addendum 9, decided by the user on 2026-10-08) ---
function Hl([string[]]$Nomes, [string[]]$Msvc = @('14.44.35207'), [string[]]$Sdk = @('10.0.19041.0', '10.0.26100.0')) {
  return ConvertTo-LabHostToolsLayout -Cfg ([pscustomobject]@{ hostTools = $Nomes }) -MsvcDirs $Msvc -SdkDirs $Sdk
}
$allHost = @('rust', 'node', 'msvc', 'winkits')
Check 'host layout: latest version folder compared as versions, names filtered, no config = no tools' {
  if ((Get-LabLatestVersionDir @('9.0.1', '10.0.2', '10.0.10', 'x64', 'Hostx64', '')) -ne '10.0.10') { throw 'numeric order' }
  if ((Get-LabLatestVersionDir @('10.0.19041.0', '10.0.26100.0')) -ne '10.0.26100.0') { throw 'sdk order' }
  if ($null -ne (Get-LabLatestVersionDir @('x64', 'arm64'))) { throw 'no version folder -> null' }
  if ($null -ne (Get-LabLatestVersionDir @())) { throw 'empty -> null' }
  $l = Hl @('node', 'perl', 'Rust', 'msvc', 5)
  if (($l.nomes -join ',') -ne 'node,msvc') { throw "names: $($l.nomes -join ',')" }
  $none = ConvertTo-LabHostToolsLayout -Cfg $null
  if ($none.nomes.Count -or $none.msvc -or $none.sdk) { throw 'null config' }
  $fromJson = ConvertTo-LabHostToolsLayout -Cfg ('{"hostTools":["rust"]}' | ConvertFrom-Json) -MsvcDirs @() -SdkDirs @()
  if (($fromJson.nomes -join ',') -ne 'rust') { throw 'a one-name list parsed from JSON' }
  if ((ConvertTo-LabHostToolsLayout -Cfg ([pscustomobject]@{ outro = 1 })).nomes.Count) { throw 'no hostTools key' }
}
Check 'host tools: node-only need is met by the mounted Node, even while claude-code is still installing' {
  $e = Get-LabGuestToolEnv ([pscustomobject]@{ ok = $false; fim = $null; pacotes = @() }) -Precisa node -HostLayout (Hl $allHost)
  if ($null -eq $e) { throw 'host node must be enough' }
  if ($e.paths -notcontains 'C:\hosttools\node') { throw "paths: $($e.paths -join '|')" }
  if ($null -eq (Get-LabGuestToolEnv $null -Precisa node -HostLayout (Hl @('node')))) { throw 'no instalar.json at all is fine for a host Node' }
  if ($null -ne (Get-LabGuestToolEnv $null -Precisa node -HostLayout (Hl @('rust')))) { throw 'rust alone is not node' }
  if ($null -ne (Get-LabGuestToolEnv $null -Precisa node -HostLayout (Hl @()))) { throw 'no tools -> nothing' }
}
Check 'host tools: the cargo environment is PATH + INCLUDE + LIB from C:\hosttools, the latest MSVC and SDK versions' {
  $e = Get-LabGuestToolEnv $null -Precisa cargo -HostLayout (Hl $allHost)
  if ($null -eq $e) { throw 'all four host tools -> cargo environment' }
  $want = 'C:\hosttools\node|C:\hosttools\rust\bin|C:\hosttools\msvc\14.44.35207\bin\Hostx64\x64|C:\hosttools\winkits\bin\10.0.26100.0\x64'
  if (($e.paths -join '|') -ne $want) { throw "paths: $($e.paths -join '|')" }
  $inc = 'C:\hosttools\msvc\14.44.35207\include;C:\hosttools\winkits\Include\10.0.26100.0\ucrt;C:\hosttools\winkits\Include\10.0.26100.0\um;C:\hosttools\winkits\Include\10.0.26100.0\shared;C:\hosttools\winkits\Include\10.0.26100.0\winrt;C:\hosttools\winkits\Include\10.0.26100.0\cppwinrt'
  $lib = 'C:\hosttools\msvc\14.44.35207\lib\x64;C:\hosttools\winkits\Lib\10.0.26100.0\ucrt\x64;C:\hosttools\winkits\Lib\10.0.26100.0\um\x64'
  if ($e.env['INCLUDE'] -ne $inc) { throw "INCLUDE: $($e.env['INCLUDE'])" }
  if ($e.env['LIB'] -ne $lib) { throw "LIB: $($e.env['LIB'])" }
  if ($e.env['VCINSTALLDIR'] -ne 'C:\hosttools\' -or $e.env['VSCMD_ARG_TGT_ARCH'] -ne 'x64' -or $e.env['WindowsSdkDir'] -ne 'C:\hosttools\winkits\' -or $e.env['WindowsSDKVersion'] -ne '10.0.26100.0\') { throw 'SDK variables' }
  if ($e.vcvars) { throw 'no vcvars for host tools' }
  if (($e.hostTools -join ',') -ne 'node,rust,msvc,winkits') { throw "hostTools: $($e.hostTools -join ',')" }
  foreach ($k in @('INCLUDE', 'LIB', 'VCINSTALLDIR', 'WindowsSdkDir')) { foreach ($part in ($e.env[$k] -split ';')) { if ($part -notmatch '^C:\\hosttools\\') { throw "${k} leaves C:\hosttools: $part" } } }
  foreach ($p in $e.paths) { if ($p -notmatch '^C:\\hosttools\\') { throw "PATH entry outside C:\hosttools: $p" } }
}
Check 'host tools: cargo needs rust, node, msvc AND winkits with a version folder of each; a missing piece fails closed' {
  $cases = [ordered]@{
    'no rust'          = (Hl @('node', 'msvc', 'winkits'))
    'no node'          = (Hl @('rust', 'msvc', 'winkits'))
    'no msvc'          = (Hl @('rust', 'node', 'winkits'))
    'no winkits'       = (Hl @('rust', 'node', 'msvc'))
    'no msvc version'  = (Hl $allHost -Msvc @('x64'))
    'no sdk version'   = (Hl $allHost -Sdk @())
    'no layout at all' = $null
  }
  foreach ($k in $cases.Keys) { if ($null -ne (Get-LabGuestToolEnv $null -Precisa cargo -HostLayout $cases[$k])) { throw "$k must not satisfy cargo" } }
  if ($null -eq (Get-LabGuestToolEnv $null -Precisa cargo -HostLayout (Hl $allHost))) { throw 'complete layout must satisfy cargo' }
}
Check 'host tools: todos needs instalar.json ok AND every declared host tool usable' {
  $ok = [pscustomobject]@{ ok = $true; fim = 'x'; pacotes = @((Pk 'claude-code' 0 'C:\labtools\claude')) }
  $e = Get-LabGuestToolEnv $ok -HostLayout (Hl $allHost)
  if ($null -eq $e) { throw 'claude-code ok + all host tools -> todos' }
  if (($e.paths -join '|') -notmatch '^C:\\labtools\\claude\|C:\\hosttools\\node\|') { throw "installed paths first, then host: $($e.paths -join '|')" }
  if ($null -ne (Get-LabGuestToolEnv ([pscustomobject]@{ ok = $false; fim = 'x'; pacotes = @((Pk 'claude-code' 1)) }) -HostLayout (Hl $allHost))) { throw 'a failed install keeps todos false' }
  if ($null -ne (Get-LabGuestToolEnv $ok -HostLayout (Hl $allHost -Sdk @()))) { throw 'a declared msvc without an SDK version keeps todos false' }
  if ($null -ne (Get-LabGuestToolEnv $ok -HostLayout (Hl @('rust', 'node', 'msvc')))) { throw 'a declared msvc without winkits keeps todos false' }
  if ($null -ne (Get-LabGuestToolEnv $ok -HostLayout (Hl @('rust', 'node', 'winkits')))) { throw 'a declared winkits without msvc keeps todos false' }
  if ($null -eq (Get-LabGuestToolEnv $ok -HostLayout (Hl @('rust', 'node')))) { throw 'rust and node alone satisfy todos' }
  if ($null -ne (Get-LabGuestToolEnv $null -HostLayout (Hl $allHost))) { throw 'no instalar.json: todos stays closed' }
}
Check 'host tools and installed packages: each tool comes from its own source (a mix works; a tool in neither fails)' {
  # node installed in the guest (finished ok), rust + msvc + winkits from the host
  $inst = [pscustomobject]@{ ok = $true; fim = 'x'; pacotes = @((Pk 'node' 0 'C:\labtools\node')) }
  $e = Get-LabGuestToolEnv $inst -Precisa cargo -HostLayout (Hl @('rust', 'msvc', 'winkits'))
  if ($null -eq $e) { throw 'installed node + host rust/msvc must satisfy cargo' }
  if (($e.paths -join '|') -notmatch '^C:\\labtools\\node\|C:\\hosttools\\rust\\bin\|') { throw "paths: $($e.paths -join '|')" }
  $running = [pscustomobject]@{ ok = $false; fim = $null; pacotes = @() }
  if ($null -ne (Get-LabGuestToolEnv $running -Precisa cargo -HostLayout (Hl @('rust', 'msvc', 'winkits')))) { throw 'an installed node that is still installing must not count' }
  $failed = [pscustomobject]@{ ok = $false; fim = 'x'; pacotes = @((Pk 'node' 1)) }
  if ($null -ne (Get-LabGuestToolEnv $failed -Precisa node -HostLayout (Hl @('rust')))) { throw 'node in neither source' }
}
Check 'host tools: without a layout the answers are the ones the installed packages gave before (no regression)' {
  $allOk = [pscustomobject]@{ ok = $true; fim = 'x'; pacotes = @((Pk 'msvc' 0 '' 'C:\labtools\msvc\vcvars64.bat'), (Pk 'rust' 0 'C:\rust\bin'), (Pk 'node' 0 'C:\labtools\node')) }
  $e = Get-LabGuestToolEnv $allOk -Precisa cargo
  if ($null -eq $e -or ($e.paths -join '|') -ne 'C:\rust\bin|C:\labtools\node' -or $e.vcvars -ne 'C:\labtools\msvc\vcvars64.bat' -or $e.env.Count -or $e.hostTools.Count) { throw 'installed cargo environment changed' }
  if ($null -eq (Get-LabGuestToolEnv $allOk -Precisa cargo -HostLayout (Hl @()))) { throw 'an empty host layout leaves the installed answer alone' }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
