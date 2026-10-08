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

function ConvertTo-LabVersionProbe {
  # instalar.ps1: the exit and the version line of a '<tool> --version' probe. Pure.
  # The caller captures the WHOLE output first ($out = @(& $exe --version 2>&1)), reads $LASTEXITCODE right after, and only
  # then takes the first line. Seen live (Windows PowerShell 5.1, 2026-10-08): '... 2>&1 | Select-Object -First 1' stops the
  # native process early and $LASTEXITCODE becomes -1 although Node works (version v22.23.3, exit -1). The fix is not to
  # truncate the pipe; it is NOT to accept -1 either: only exit 0 is success, whatever the output looks like.
  # Rule: no code (null) = 1; any other code is kept as is; versao = first non-empty line, or $null.
  param([string[]]$Output = @(), $ExitCode = $null)
  $lines = @(@($Output) | ForEach-Object { "$_".Trim() } | Where-Object { $_ })
  $versao = $(if ($lines.Count) { $lines[0] } else { $null })
  $exit = $(if ($null -eq $ExitCode) { 1 } else { [int]$ExitCode })
  return [ordered]@{ exit = $exit; versao = $versao }
}

function Get-LabInstallDone($Instalar) {
  # Names of the packages that finished with exit 0 in instalar.json. Failed packages never count.
  if ($null -eq $Instalar) { return @() }
  return @(@($Instalar.pacotes) | Where-Object { $_ -and $_.exit -eq 0 } | ForEach-Object { [string]$_.nome })
}

# Host toolchains the PC mounted READ-ONLY at C:\hosttools (ambiente "testes", spec addendum 9, decided by the user on 2026-10-08).
# guest.config.json carries only the NAMES (hostTools); the versions are the folders found under C:\hosttools\msvc and
# C:\hosttools\winkits\Include. A tool comes from the host when the ambiente declares it, otherwise from instalar.json.
$script:LabGuestHostToolRoot = 'C:\hosttools'
$script:LabGuestHostToolNames = @('rust', 'node', 'msvc', 'winkits')

function Get-LabLatestVersionDir {
  # The highest folder name that is a plain version (digits and dots, 2 to 4 parts), compared as versions and not as text
  # ('9.0' < '10.0'). $null when none.
  param([string[]]$Dirs = @())
  $ok = @(@($Dirs) | Where-Object { $_ -match '^\d+(\.\d+){1,3}$' })
  if (-not $ok.Count) { return $null }
  return [string](@($ok | Sort-Object { [version]$_ })[-1])
}

function ConvertTo-LabHostToolsLayout {
  # What the guest knows about the host tools: the declared names (guest.config.json hostTools, known names only) and the
  # latest MSVC and SDK version folders. Pure; Get-LabHostToolsLayout (end of the file) is the thin reader of the folders.
  param($Cfg, [string[]]$MsvcDirs = @(), [string[]]$SdkDirs = @())
  $names = @()
  if ($Cfg -is [System.Management.Automation.PSCustomObject]) {
    $p = $Cfg.PSObject.Properties['hostTools']
    if ($p) { $names = @(@($p.Value) | Where-Object { $_ -is [string] -and $script:LabGuestHostToolNames -ccontains $_ }) }
  }
  return [ordered]@{ nomes = $names; msvc = (Get-LabLatestVersionDir $MsvcDirs); sdk = (Get-LabLatestVersionDir $SdkDirs) }
}

function Test-LabHostToolUsable {
  # Is this tool available from the host mounts? msvc also needs winkits declared and one version folder of each found.
  param($HostLayout, [string]$Tool)
  if ($null -eq $HostLayout) { return $false }
  $has = @($HostLayout.nomes)
  switch ($Tool) {
    'node' { return ($has -contains 'node') }
    'rust' { return ($has -contains 'rust') }
    'msvc' { return (($has -contains 'msvc') -and ($has -contains 'winkits') -and [bool]$HostLayout.msvc -and [bool]$HostLayout.sdk) }
  }
  return $false
}

function Test-LabInstallNeed {
  # What a guest script needs ($Precisa): 'todos' = every package of instalar.json ok (Instalar.ok) and every declared host tool
  # usable, 'node' = Node (the Sandbox-only tests and the scenarios), 'cargo' = MSVC, Rust and Node (the Rust build links with
  # MSVC; the documented command also runs npm). Each tool comes from the host mounts when the ambiente declares it
  # ($HostLayout, ConvertTo-LabHostToolsLayout), otherwise from instalar.json: there it must be ok AND the install must have
  # finished (fim), so a run never starts while a package is still being installed.
  param($Instalar, [string]$Precisa = 'todos', $HostLayout = $null)
  $tools = $null
  switch ($Precisa) {
    'node' { $tools = @('node') }
    'cargo' { $tools = @('msvc', 'rust', 'node') }
    'todos' {
      if ($null -eq $Instalar -or -not [bool]$Instalar.ok) { return $false }
      # rust and node are usable as soon as they are declared; msvc and winkits need each other and a version folder of each.
      $declared = @($HostLayout.nomes)
      if (($declared -contains 'msvc' -or $declared -contains 'winkits') -and -not (Test-LabHostToolUsable $HostLayout 'msvc')) { return $false }
      return $true
    }
    default { return $false }
  }
  $done = $null
  foreach ($t in $tools) {
    if (Test-LabHostToolUsable $HostLayout $t) { continue }
    if ($null -eq $Instalar -or -not $Instalar.fim) { return $false }
    if ($null -eq $done) { $done = Get-LabInstallDone $Instalar }
    if ($done -notcontains $t) { return $false }
  }
  return $true
}

function Get-LabGuestToolEnv {
  # PATH folders, environment variables and the vcvars64.bat to import. Installed tools come from instalar.json (only
  # packages that finished ok); host tools come from C:\hosttools when $HostLayout declares them (INCLUDE/LIB as a developer
  # prompt would set them, from the mounted MSVC and Windows SDK). $null when the sources do not satisfy $Precisa (see
  # Test-LabInstallNeed). The cargo home and target stay on the guest disk, set by the caller, never in the mounted folders.
  param($Instalar, [string]$Precisa = 'todos', $HostLayout = $null)
  if (-not (Test-LabInstallNeed -Instalar $Instalar -Precisa $Precisa -HostLayout $HostLayout)) { return $null }
  $good = @(@($(if ($Instalar) { $Instalar.pacotes })) | Where-Object { $_ -and $_.exit -eq 0 })
  $paths = @($good | Where-Object { $_.binDir } | ForEach-Object { [string]$_.binDir })
  $vc = @($good | Where-Object { $_.vcvars } | ForEach-Object { [string]$_.vcvars }) | Select-Object -First 1
  $vars = [ordered]@{}
  $used = @()
  $root = $script:LabGuestHostToolRoot
  if (Test-LabHostToolUsable $HostLayout 'node') { $paths += "$root\node"; $used += 'node' }
  if (Test-LabHostToolUsable $HostLayout 'rust') { $paths += "$root\rust\bin"; $used += 'rust' }
  if (Test-LabHostToolUsable $HostLayout 'msvc') {
    $m = "$root\msvc\$($HostLayout.msvc)"; $k = "$root\winkits"; $sdk = [string]$HostLayout.sdk
    $paths += "$m\bin\Hostx64\x64", "$k\bin\$sdk\x64"
    $vars['INCLUDE'] = "$m\include;$k\Include\$sdk\ucrt;$k\Include\$sdk\um;$k\Include\$sdk\shared;$k\Include\$sdk\winrt;$k\Include\$sdk\cppwinrt"
    $vars['LIB'] = "$m\lib\x64;$k\Lib\$sdk\ucrt\x64;$k\Lib\$sdk\um\x64"
    $vars['VCINSTALLDIR'] = "$root\"   # cc-rs/rustc use this environment (PATH/INCLUDE/LIB) instead of looking for Visual Studio in the registry
    $vars['VSCMD_ARG_TGT_ARCH'] = 'x64'
    $vars['WindowsSdkDir'] = "$k\"
    $vars['WindowsSDKVersion'] = "$sdk\"
    $used += 'msvc', 'winkits'
  }
  return [ordered]@{ paths = $paths; vcvars = $vc; env = $vars; hostTools = $used }
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

function Get-LabHostToolsLayout {
  # Thin reader, like Test-LabInGuest: guest.config.json (the declared host tool names) and the version folder names under
  # C:\hosttools. Lists folders only; the decision is the pure ConvertTo-LabHostToolsLayout. A missing file or folder gives a
  # layout with no tools.
  param($Cfg = $null)
  if ($null -eq $Cfg) { try { $Cfg = Get-Content -LiteralPath 'C:\labsandbox\guest.config.json' -Raw -ErrorAction Stop | ConvertFrom-Json } catch { } }
  $r = $script:LabGuestHostToolRoot
  $msvc = @(Get-ChildItem -LiteralPath "$r\msvc" -Directory -ErrorAction SilentlyContinue | ForEach-Object { $_.Name })
  $sdk = @(Get-ChildItem -LiteralPath "$r\winkits\Include" -Directory -ErrorAction SilentlyContinue | ForEach-Object { $_.Name })
  return ConvertTo-LabHostToolsLayout -Cfg $Cfg -MsvcDirs $msvc -SdkDirs $sdk
}

function Get-LabGuestInternetPermitida($Cfg) {
  # From guest.config.json: internet is allowed only when the file says so with a real boolean (internetBloqueada = false).
  # Missing file, missing field or any other value = NOT allowed (fail closed).
  if ($null -eq $Cfg -or $Cfg -isnot [System.Management.Automation.PSCustomObject]) { return $false }
  $p = $Cfg.PSObject.Properties['internetBloqueada']
  if (-not $p -or $p.Value -isnot [bool]) { return $false }
  return (-not $p.Value)
}

function Test-LabBootId([string]$Id) {
  # The id of THIS boot of the guest: 32 lowercase hex digits (a GUID in the "N" format), made by bootstrap.ps1 and kept in
  # C:\labdata\boot-id.txt, a folder of the guest disk that does not survive the Sandbox.
  return ($Id -cmatch '\A[0-9a-f]{32}\z')
}

function Format-LabFirewallBootLine([string]$Id) {
  # The first line guest-firewall.ps1 writes in firewall.txt; Get-LabAppStartRefusal wants exactly this line.
  return "BOOTID: $Id"
}

function Get-LabAppStartRefusal {
  # guest-run.ps1: why the app must NOT be opened, or $null. Where the ambiente keeps the internet blocked, the app opens only
  # when C:\labwork\run\firewall.txt (written by guest-firewall.ps1 AFTER it applied the rule and tested it) carries the block
  # mark: the 3 profiles on, exactly 1 lab-block-internet-out rule, both internet probes blocked, none reachable.
  # C:\labwork\run is the PC folder .sandbox-work\guest-rw\run and survives between Sandboxes, so the file may be from an OLDER
  # guest. It counts only with the boot id of THIS guest (-ExpectedBootId, read from C:\labdata\boot-id.txt) on its BOOTID line.
  # The clock is not used. Missing or malformed expected id = refuse (the guest was not bootstrapped).
  param([bool]$InternetPermitida, [string]$FirewallText, [string]$ExpectedBootId = $null)
  if ($InternetPermitida) { return $null }
  if (-not (Test-LabBootId $ExpectedBootId)) { return 'the boot id of this guest is missing or malformed (C:\labdata\boot-id.txt): cannot tell whether firewall.txt is from this boot' }
  if ([string]::IsNullOrWhiteSpace($FirewallText)) { return 'firewall.txt is missing or empty: guest-firewall.ps1 did not finish, the internet may be open' }
  $lines = @($FirewallText -split "`r?`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ })
  $idLines = @($lines | Where-Object { $_ -clike 'BOOTID:*' })
  if ($idLines.Count -eq 0) { return 'firewall.txt has no BOOTID line: it is from an older guest, guest-firewall.ps1 did not run in this one' }
  if ($idLines.Count -ne 1 -or $idLines[0] -cne (Format-LabFirewallBootLine $ExpectedBootId)) { return 'firewall.txt is from another boot of the guest (BOOTID differs): guest-firewall.ps1 did not run in this one' }
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

function Get-LabGuestScenarioJsonRefusal([string]$Json) {
  # guest-run.ps1 -Cenario: a scenario folder counts only if its scenario script FINISHED. The scripts write cenario.json as
  # the very last step (after the copy, the database fold and the automation switch-off), so a missing, empty or malformed
  # file means a partial copy (for example fs.cpSync failed half way): refuse to install it. Nothing is deleted.
  if ([string]::IsNullOrWhiteSpace($Json)) { return 'cenario.json is missing or empty: the scenario did not finish (partial copy), not installed' }
  $o = $null
  try { $o = $Json | ConvertFrom-Json -ErrorAction Stop } catch { return 'cenario.json is not valid JSON: the scenario did not finish, not installed' }
  if ($o -isnot [System.Management.Automation.PSCustomObject]) { return 'cenario.json is not a JSON object: not installed' }
  if ("$($o.cenario)" -notin @('novo', 'mock', 'copia')) { return 'cenario.json has no valid "cenario" (novo, mock or copia): not installed' }
  if ($o.criadoEm -isnot [string] -and $o.criadoEm -isnot [datetime]) { return 'cenario.json has no "criadoEm": not installed' }
  $list = $o.PSObject.Properties['arquivos']
  if (-not $list -or $list.Value -isnot [System.Array]) { return 'cenario.json has no "arquivos" list: not installed' }
  return $null
}

function Test-LabGuestLatestLinkApproved($Config) {
  # copia scenario in the guest: the PC decided (test-up, Test-LabLatestLinkApproved) that diagnostics\latest is an approved
  # FILE link and wrote only that boolean to guest.config.json. Only a literal true counts, and only together with dadosReais.
  if ($null -eq $Config) { return $false }
  return [bool]($Config.dadosReais -is [bool] -and $Config.dadosReais -and $Config.dadosReaisLatestLink -is [bool] -and $Config.dadosReaisLatestLink)
}

function Get-LabGuestScenarioRefusal([string]$Path) {
  # guest-run.ps1 -Cenario: a scenario folder created by the scenario scripts in the guest (D1), nothing else.
  if ([string]::IsNullOrWhiteSpace($Path)) { return 'scenario folder is required' }
  if ($Path -notmatch '^[Zz]:\\[Cc][Ll][Aa][Uu][Dd][Ii][Aa]-[Tt][Ee][Ss][Tt][Ee]-[Ss][Aa][Nn][Dd][Bb][Oo][Xx]\\[^\\.][^\\]*$') { return 'scenario folder must be Z:\claudia-teste-sandbox\<name>' }
  return $null
}
