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
$fwOk = (@('Domain enabled=True', 'Private enabled=True', 'Public enabled=True', 'regra lab-block-internet-out: 1',
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
  if (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $fwOk) { throw 'complete mark must pass' }
  if (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText ($fwOk -replace "`r`n", "`n")) { throw 'LF line endings must pass' }
  foreach ($t in @($null, '', '   ', "`r`n")) { if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $t)) { throw "missing/empty file '$t' must refuse" } }
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
  foreach ($k in $bad.Keys) { if (-not (Get-LabAppStartRefusal -InternetPermitida $false -FirewallText $bad[$k])) { throw "must refuse: $k" } }
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
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
