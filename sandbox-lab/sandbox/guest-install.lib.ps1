# Pure decisions for the guest scripts (instalar, guest-build, guest-cenario, ...). Dot-sourced in the guest and by the
# pure test on the host. No disk, except Test-LabInGuest at the end, the one thin reader of the OS identity.

function ConvertTo-LabInstallExit($Exit) {
  # 3010 = success, reboot requested (the guest is never rebooted). No code at all is a failure.
  if ($null -eq $Exit) { return 1 }
  if ([int]$Exit -eq 3010) { return 0 }
  return [int]$Exit
}

function Test-LabInstallOk {
  # True only when every expected package ran, in the expected order, and exited 0.
  param([string[]]$Expected = @(), [object[]]$Results = @())
  $names = @(@($Results) | Where-Object { $_ } | ForEach-Object { [string]$_.nome })
  $exp = @(@($Expected) | Where-Object { $_ })
  if ($names.Count -ne $exp.Count) { return $false }
  for ($i = 0; $i -lt $exp.Count; $i++) { if ($names[$i] -ne $exp[$i]) { return $false } }
  return -not @(@($Results) | Where-Object { $_ -and $_.exit -ne 0 }).Count
}

function Get-LabInstallDone($Instalar) {
  # Names of the packages that finished with exit 0 in instalar.json. Failed packages never count.
  if ($null -eq $Instalar) { return @() }
  return @(@($Instalar.pacotes) | Where-Object { $_ -and $_.exit -eq 0 } | ForEach-Object { [string]$_.nome })
}

function Test-LabInstallNeed {
  # What a guest script needs from instalar.json ($Precisa): 'todos' = every package ok (Instalar.ok), 'node' = Node ok
  # (the Sandbox-only tests and the scenarios), 'cargo' = MSVC, Rust and Node ok (the Rust build links with MSVC; the documented command also runs npm). The partial
  # modes also need the install to have finished (fim), so a run never starts while a package is still being installed.
  param($Instalar, [string]$Precisa = 'todos')
  if ($null -eq $Instalar) { return $false }
  switch ($Precisa) {
    'todos' { return [bool]$Instalar.ok }
    'node' { if (-not $Instalar.fim) { return $false }; return ((Get-LabInstallDone $Instalar) -contains 'node') }
    'cargo' {
      if (-not $Instalar.fim) { return $false }
      $done = Get-LabInstallDone $Instalar
      return (($done -contains 'msvc') -and ($done -contains 'rust') -and ($done -contains 'node'))
    }
    default { return $false }
  }
}

function Get-LabGuestToolEnv {
  # From instalar.json: folders to put first on PATH and the vcvars64.bat to import, taken only from packages that
  # finished ok. $null when the install does not satisfy $Precisa (see Test-LabInstallNeed).
  param($Instalar, [string]$Precisa = 'todos')
  if (-not (Test-LabInstallNeed -Instalar $Instalar -Precisa $Precisa)) { return $null }
  $good = @(@($Instalar.pacotes) | Where-Object { $_ -and $_.exit -eq 0 })
  $paths = @($good | Where-Object { $_.binDir } | ForEach-Object { [string]$_.binDir })
  $vc = @($good | Where-Object { $_.vcvars } | ForEach-Object { [string]$_.vcvars }) | Select-Object -First 1
  return [ordered]@{ paths = $paths; vcvars = $vc }
}

function Test-LabGuestScriptDecision {
  # Two-factor guard of the guest scripts (instalar, disco-z, layout-msvc, guest-run, guest-cenario, guest-build). Both:
  #  1. the marker file mapped only in the guest (C:\labsandbox\bootstrap.ps1), and
  #  2. the process identity: WDAGUtilityAccount (DOMAIN\user from WindowsIdentity, never $env:USERNAME), or, for
  #     'wsb exec -r System', SYSTEM (decided by the identity's IsSystem flag, so the localized account name does not
  #     matter) together with the profile folder of WDAGUtilityAccount, which exists in the guest.
  # Inputs are injected by Test-LabInGuest; this function reads nothing.
  param([string]$Identity, [bool]$IsSystem, [bool]$HasBootstrap, [bool]$HasWdagProfile)
  if (-not $HasBootstrap) { return $false }
  if ($Identity -match '\\WDAGUtilityAccount$') { return $true }
  return ($IsSystem -and $HasWdagProfile)
}

function Test-LabInGuest {
  # The only impure part: reads the OS identity and two guest-only paths; the decision is Test-LabGuestScriptDecision.
  $id = [Security.Principal.WindowsIdentity]::GetCurrent()
  return Test-LabGuestScriptDecision -Identity $id.Name -IsSystem $id.IsSystem `
    -HasBootstrap (Test-Path -LiteralPath 'C:\labsandbox\bootstrap.ps1' -PathType Leaf) `
    -HasWdagProfile (Test-Path -LiteralPath 'C:\Users\WDAGUtilityAccount' -PathType Container)
}

function Get-LabGuestScenarioRefusal([string]$Path) {
  # guest-run.ps1 -Cenario: a scenario folder created by the scenario scripts in the guest (D1), nothing else.
  if ([string]::IsNullOrWhiteSpace($Path)) { return 'scenario folder is required' }
  if ($Path -notmatch '^[Zz]:\\[Cc][Ll][Aa][Uu][Dd][Ii][Aa]-[Tt][Ee][Ss][Tt][Ee]-[Ss][Aa][Nn][Dd][Bb][Oo][Xx]\\[^\\.][^\\]*$') { return 'scenario folder must be Z:\claudia-teste-sandbox\<name>' }
  return $null
}
