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
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
