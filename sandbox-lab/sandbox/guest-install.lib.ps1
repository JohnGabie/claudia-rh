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

function Get-LabGuestInternetPermitida($Cfg) {
  # From guest.config.json: internet is allowed only when the file says so with a real boolean (internetBloqueada = false).
  # Missing file, missing field or any other value = NOT allowed (fail closed).
  if ($null -eq $Cfg -or $Cfg -isnot [System.Management.Automation.PSCustomObject]) { return $false }
  $p = $Cfg.PSObject.Properties['internetBloqueada']
  if (-not $p -or $p.Value -isnot [bool]) { return $false }
  return (-not $p.Value)
}

function Get-LabAppStartRefusal {
  # guest-run.ps1: why the app must NOT be opened, or $null. Where the ambiente keeps the internet blocked, the app opens only
  # when C:\labwork\run\firewall.txt (written by guest-firewall.ps1 AFTER it applied the rule and tested it) carries the block
  # mark: the 3 profiles on, exactly 1 lab-block-internet-out rule, both internet probes blocked, none reachable.
  param([bool]$InternetPermitida, [string]$FirewallText)
  if ($InternetPermitida) { return $null }
  if ([string]::IsNullOrWhiteSpace($FirewallText)) { return 'firewall.txt is missing or empty: guest-firewall.ps1 did not finish, the internet may be open' }
  $lines = @($FirewallText -split "`r?`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ })
  if (@($lines | Where-Object { $_ -match 'ACESSIVEL' }).Count) { return 'firewall.txt says the internet is reachable' }
  foreach ($prof in 'Domain', 'Private', 'Public') {
    if (@($lines | Where-Object { $_ -ceq "$prof enabled=True" }).Count -ne 1) { return "firewall.txt does not show the $prof profile enabled" }
  }
  if (@($lines | Where-Object { $_ -ceq 'regra lab-block-internet-out: 1' }).Count -ne 1) { return 'firewall.txt does not show exactly one lab-block-internet-out rule' }
  if (@($lines | Where-Object { $_ -clike 'INTERNET: BLOQUEADA (*' }).Count -ne 1) { return 'firewall.txt does not carry the INTERNET: BLOQUEADA mark' }
  if (@($lines | Where-Object { $_ -ceq 'storage.googleapis.com: BLOQUEADO' }).Count -ne 1) { return 'firewall.txt does not show storage.googleapis.com blocked' }
  return $null
}

function Get-LabVsLayoutRefusal {
  # layout-msvc.ps1: did vs_layout.exe (run directly, stdout/stderr in files) finish the layout? $null = yes.
  # Seen live in the Sandbox (2026-10-08): with no console, vs_layout finishes the layout, logs "Completed 'Layout' operation",
  # and only then dies in a finalizer (System.Console.ControlCHooker.Unhook, IOException), so its exit code is not 0.
  # That single exit-time crash is tolerated; any other exception, a missing completion line or a null code refuses.
  # Because the completion line is the only success signal, it is not enough alone (limites.md item 14):
  #   * the stdout must carry no package-failure marker. Chosen against the real success log (6925 lines): it has ZERO
  #     matches for every marker below. NOT markers: "error" alone (package Microsoft.VisualStudio.ErrorList) and
  #     "corrupt" (every run logs "Cache: Skipping layout with missing or corrupt payload <id>" while it fills an empty layout).
  #   * the layout folder must hold the bootstrapper (vs_buildtools.exe or vs_setup.exe) and Catalog.json.
  # $LayoutFiles = names relative to the layout folder (top level) that exist there; $null = not listed = refuse.
  param($ExitCode, [string]$StdOut, [string]$StdErr, [string[]]$LayoutFiles = $null)
  if ($null -eq $ExitCode) { return 'vs_layout exit code unknown' }
  if ("$StdOut" -notmatch "(?m)Completed 'Layout' operation\s*$") { return "vs_layout did not log Completed 'Layout' operation (exit $ExitCode)" }
  $markers = [ordered]@{
    'failed/failure'            = '(?i)\bfail(ed|ure|ures)?\b'
    'error-level log line (e:)' = '(?m)^\[[0-9a-fA-F]{4}:[0-9a-fA-F]{4}\]\[[^\]\r\n]*\]e:'
    'exception (any *Exception)'  = '(?i)exception'
    'error: / error 0x'         = '(?i)\berror(:|\s+0x)'
    'HRESULT 0x8xxxxxxx'        = '(?i)\b0x8[0-9a-f]{7}\b'
  }
  foreach ($k in $markers.Keys) { if ("$StdOut" -match $markers[$k]) { return "vs_layout log has a failure marker ($k) although it logged completion (exit $ExitCode)" } }
  if ($null -eq $LayoutFiles) { return 'layout files not listed' }
  $have = @($LayoutFiles | ForEach-Object { "$_".ToLowerInvariant() })
  if ($have -notcontains 'catalog.json') { return 'layout folder has no Catalog.json' }
  if ($have -notcontains 'vs_buildtools.exe' -and $have -notcontains 'vs_setup.exe') { return 'layout folder has neither vs_buildtools.exe nor vs_setup.exe' }
  if ([int]$ExitCode -eq 0) { return $null }
  $crashes = [regex]::Matches("$StdErr", 'Unhandled Exception').Count
  if ($crashes -ne 1) { return "vs_layout exit $ExitCode with $crashes unhandled exceptions" }
  if ("$StdErr" -notmatch 'System\.Console\.ControlCHooker\.Unhook\(\)' -or "$StdErr" -notmatch 'ControlCHooker\.Finalize\(\)') { return "vs_layout exit $ExitCode with an exception other than the exit-time console finalizer" }
  return $null
}

function Get-LabGuestScenarioRefusal([string]$Path) {
  # guest-run.ps1 -Cenario: a scenario folder created by the scenario scripts in the guest (D1), nothing else.
  if ([string]::IsNullOrWhiteSpace($Path)) { return 'scenario folder is required' }
  if ($Path -notmatch '^[Zz]:\\[Cc][Ll][Aa][Uu][Dd][Ii][Aa]-[Tt][Ee][Ss][Tt][Ee]-[Ss][Aa][Nn][Dd][Bb][Oo][Xx]\\[^\\.][^\\]*$') { return 'scenario folder must be Z:\claudia-teste-sandbox\<name>' }
  return $null
}
