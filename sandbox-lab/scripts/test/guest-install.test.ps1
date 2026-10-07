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
    [pscustomobject]@{ nome = 'msvc'; binDir = $null; vcvars = 'C:\labtools\msvc\VC\Auxiliary\Build\vcvars64.bat' },
    [pscustomobject]@{ nome = 'rust'; binDir = 'C:\Program Files\Rust stable MSVC 1.95\bin'; vcvars = $null },
    [pscustomobject]@{ nome = 'node'; binDir = 'C:\labtools\node\node-v22.23.3-win-x64'; vcvars = $null }) }
  $e = Get-LabGuestToolEnv $inst
  if (($e.paths -join '|') -ne 'C:\Program Files\Rust stable MSVC 1.95\bin|C:\labtools\node\node-v22.23.3-win-x64') { throw "paths $($e.paths -join '|')" }
  if ($e.vcvars -ne 'C:\labtools\msvc\VC\Auxiliary\Build\vcvars64.bat') { throw 'vcvars' }
  if ($null -ne (Get-LabGuestToolEnv ([pscustomobject]@{ ok = $false; pacotes = @() }))) { throw 'not ok -> null' }
  if ($null -ne (Get-LabGuestToolEnv $null)) { throw 'missing -> null' }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
