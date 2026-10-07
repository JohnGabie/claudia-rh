# Pure decisions for instalar.ps1 and guest-build.ps1. Dot-sourced in the guest and by the pure test on the host. No disk.

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

function Get-LabGuestToolEnv {
  # From instalar.json: folders to put first on PATH and the vcvars64.bat to import. $null when the install is not ok.
  param($Instalar)
  if ($null -eq $Instalar -or -not $Instalar.ok) { return $null }
  $paths = @(@($Instalar.pacotes) | Where-Object { $_ -and $_.binDir } | ForEach-Object { [string]$_.binDir })
  $vc = @(@($Instalar.pacotes) | Where-Object { $_ -and $_.vcvars } | ForEach-Object { [string]$_.vcvars }) | Select-Object -First 1
  return [ordered]@{ paths = $paths; vcvars = $vc }
}

function Get-LabGuestScenarioRefusal([string]$Path) {
  # guest-run.ps1 -Cenario: a scenario folder created by the scenario scripts in the guest (D1), nothing else.
  if ([string]::IsNullOrWhiteSpace($Path)) { return 'scenario folder is required' }
  if ($Path -notmatch '^[Zz]:\\[Cc][Ll][Aa][Uu][Dd][Ii][Aa]-[Tt][Ee][Ss][Tt][Ee]-[Ss][Aa][Nn][Dd][Bb][Oo][Xx]\\[^\\.][^\\]*$') { return 'scenario folder must be Z:\claudia-teste-sandbox\<name>' }
  return $null
}
