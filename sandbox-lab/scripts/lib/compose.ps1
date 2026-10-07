# Pure helpers for sandbox-lab/compose.json: object and string checks only. No disk, no [IO.Path], no environment
# variables (the tests run on any OS). Objects come from ConvertFrom-Json (PSCustomObject), never hashtables.
# A refusal returns $null when allowed, otherwise the reason.

$script:LabInstallOrder = @('msvc', 'rust', 'node', 'claude-code')
$script:LabMountTargets = [ordered]@{
  projeto  = @{ sandbox = 'C:\labproject'; modo = 'ro' }
  trabalho = @{ sandbox = 'C:\labwork'; modo = 'rw' }
  deploy   = @{ sandbox = 'C:\appdeploy'; modo = 'ro' }
  # Output of the MSVC layout built in the guest (ambiente cache-msvc, spec addendum 8.2).
  'cache-saida' = @{ sandbox = 'C:\labcachesaida'; modo = 'rw' }
}
$script:LabPackageTypes = @{ 'msvc' = 'vs-layout'; 'rust' = 'msi'; 'node' = 'zip'; 'claude-code' = 'exe' }
# shared\ is mapped to C:\lab in the guest, so the cache needs no mount of its own.
$script:LabCacheGuestDir = 'C:\lab\cache'

function Get-LabProp($Object, [string]$Name) {
  if ($null -eq $Object -or $Object -isnot [System.Management.Automation.PSCustomObject]) { return $null }
  $p = $Object.PSObject.Properties[$Name]
  if ($p) { return $p.Value }
  return $null
}

function Get-LabPropNames($Object) {
  if ($null -eq $Object -or $Object -isnot [System.Management.Automation.PSCustomObject]) { return @() }
  return @($Object.PSObject.Properties | ForEach-Object { $_.Name })
}

function Get-LabList($Value) {
  # JSON arrays come as object[]; a missing key is $null and must become an empty list, not @($null).
  return @(@($Value) | Where-Object { $null -ne $_ })
}

function Get-LabComposeStrings($Node, [string]$Where = '') {
  # Every string value and property name under $Node, with where it was found.
  $out = New-Object System.Collections.Generic.List[object]
  if ($null -eq $Node) { return ,$out.ToArray() }
  if ($Node -is [string]) { $out.Add([pscustomobject]@{ Where = $Where; Value = $Node }); return ,$out.ToArray() }
  if ($Node -is [System.Management.Automation.PSCustomObject]) {
    foreach ($p in $Node.PSObject.Properties) {
      $out.Add([pscustomobject]@{ Where = "$Where.$($p.Name)"; Value = $p.Name })
      foreach ($s in (Get-LabComposeStrings $p.Value "$Where.$($p.Name)")) { $out.Add($s) }
    }
    return ,$out.ToArray()
  }
  if ($Node -is [System.Collections.IEnumerable]) {
    $i = 0
    foreach ($e in $Node) { foreach ($s in (Get-LabComposeStrings $e "$Where[$i]")) { $out.Add($s) }; $i++ }
  }
  return ,$out.ToArray()
}

function Get-LabComposeStringProblem([string]$Value) {
  # compose.json holds names, lab-relative paths, guest paths (C:\lab...) and https URLs. Nothing that points at the host.
  if ([string]::IsNullOrEmpty($Value)) { return $null }
  if ($Value -match '%[A-Za-z_][A-Za-z0-9_()]*%') { return "environment variable in '$Value'" }
  if ($Value -match '\$env:|\$\{|\$\(') { return "PowerShell variable in '$Value'" }
  if ($Value -match '^\s*~') { return "home shortcut in '$Value'" }
  if ($Value -match '^(\\\\|//)') { return "UNC path '$Value'" }
  if ($Value -match '(^|[\\/])\.\.([\\/]|$)') { return "'..' in '$Value'" }
  if ($Value -match '^[A-Za-z]:[\\/]' -and $Value -notmatch '^[Cc]:\\[Ll][Aa][Bb]') { return "host absolute path '$Value'" }
  if ($Value -match '^[A-Za-z]:[^\\/]') { return "drive-relative path '$Value'" }
  return $null
}

function Get-LabPackageRefusal {
  param([string]$Nome, $Pacote)
  if ($null -eq $Pacote) { return "package '$Nome' is not declared in pacotes" }
  $tipo = Get-LabProp $Pacote 'tipo'
  if ("$tipo" -ne $script:LabPackageTypes[$Nome]) { return "package '$Nome': tipo must be '$($script:LabPackageTypes[$Nome])'" }
  if ("$(Get-LabProp $Pacote 'versao')" -notmatch '^\d+(\.\d+){1,3}$') { return "package '$Nome': versao must be digits and dots" }
  if ("$(Get-LabProp $Pacote 'arquivo')" -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$') { return "package '$Nome': arquivo must be a plain file or folder name" }
  $src = $Pacote
  if ($tipo -eq 'vs-layout') {
    $src = Get-LabProp $Pacote 'bootstrapper'
    if ($null -eq $src) { return "package '$Nome': bootstrapper is required" }
    if ("$(Get-LabProp $src 'arquivo')" -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*\.exe$') { return "package '$Nome': bootstrapper.arquivo must be a plain .exe file name" }
    $comp = Get-LabList (Get-LabProp $Pacote 'componentes')
    if (-not $comp.Count) { return "package '$Nome': componentes is empty" }
    foreach ($c in $comp) { if ("$c" -notmatch '^Microsoft\.VisualStudio\.[A-Za-z0-9.]+$') { return "package '$Nome': bad componentes entry '$c'" } }
  }
  if ("$(Get-LabProp $src 'url')" -notmatch '^https://\S+$') { return "package '$Nome': url must be https" }
  if ("$(Get-LabProp $src 'sha256')" -cnotmatch '^[0-9a-f]{64}$') { return "package '$Nome': sha256 must be 64 lowercase hex characters" }
  $bytes = Get-LabProp $src 'bytes'
  if (-not ($bytes -is [int] -or $bytes -is [long]) -or $bytes -le 0) { return "package '$Nome': bytes must be a positive integer" }
  return $null
}

function Get-LabComposeRefusal {
  param($Compose, [string]$Ambiente)
  if ($null -eq $Compose) { return 'compose is empty' }
  foreach ($s in (Get-LabComposeStrings $Compose)) {
    $why = Get-LabComposeStringProblem $s.Value
    if ($why) { return "$($s.Where): $why" }
  }
  $ambientes = Get-LabProp $Compose 'ambientes'
  $names = Get-LabPropNames $ambientes
  if ([string]::IsNullOrWhiteSpace($Ambiente)) { return "ambiente is required (one of: $($names -join ', '))" }
  if ($names -notcontains $Ambiente) { return "unknown ambiente '$Ambiente' (one of: $($names -join ', '))" }
  $a = Get-LabProp $ambientes $Ambiente
  if ((Get-LabProp $a 'rede') -isnot [bool]) { return "ambiente ${Ambiente}: rede must be true or false" }
  $mem = Get-LabProp $a 'memoriaMB'
  if (-not ($mem -is [int] -or $mem -is [long]) -or $mem -lt 2048 -or $mem -gt 32768) { return "ambiente ${Ambiente}: memoriaMB must be an integer between 2048 and 32768" }
  $conta = Get-LabProp $a 'conta'
  if ($null -ne $conta -and $conta -isnot [bool]) { return "ambiente ${Ambiente}: conta must be true or false" }
  $seen = @{}
  foreach ($m in (Get-LabList (Get-LabProp $a 'montar'))) {
    if ($m -isnot [string] -or $m -notmatch '^([a-z][a-z-]*):(ro|rw)$') { return "ambiente ${Ambiente}: montar entry '$m' must be <name>:ro or <name>:rw" }
    $n = $Matches[1].ToLowerInvariant(); $modo = $Matches[2].ToLowerInvariant()
    if (-not $script:LabMountTargets.Contains($n)) { return "ambiente ${Ambiente}: unknown mount name '$n' (one of: $(@($script:LabMountTargets.Keys) -join ', '))" }
    if ($script:LabMountTargets[$n].modo -ne $modo) { return "ambiente ${Ambiente}: mount '$n' must be :$($script:LabMountTargets[$n].modo)" }
    if ($seen.ContainsKey($n)) { return "ambiente ${Ambiente}: mount '$n' listed twice" }
    $seen[$n] = $true
  }
  if (-not $seen.ContainsKey('projeto')) { return "ambiente ${Ambiente}: montar must include projeto:ro" }
  $pacotes = Get-LabProp $Compose 'pacotes'
  foreach ($nome in (Get-LabList (Get-LabProp $a 'instalar'))) {
    if ($script:LabInstallOrder -notcontains $nome) { return "ambiente ${Ambiente}: unknown package '$nome'" }
    $why = Get-LabPackageRefusal -Nome $nome -Pacote (Get-LabProp $pacotes $nome)
    if ($why) { return $why }
  }
  return $null
}

function ConvertTo-LabGuestInstall {
  # The guest.config.json "instalar" list: fixed order (MSVC first: Rust links with it), guest cache paths, no host path.
  param($Compose, [string]$Ambiente)
  $a = Get-LabProp (Get-LabProp $Compose 'ambientes') $Ambiente
  $wanted = Get-LabList (Get-LabProp $a 'instalar')
  $pacotes = Get-LabProp $Compose 'pacotes'
  $list = @()
  foreach ($nome in $script:LabInstallOrder) {
    if ($wanted -notcontains $nome) { continue }
    $p = Get-LabProp $pacotes $nome
    $tipo = [string](Get-LabProp $p 'tipo')
    $list += [ordered]@{
      nome        = $nome
      versao      = [string](Get-LabProp $p 'versao')
      tipo        = $tipo
      arquivo     = $script:LabCacheGuestDir + '\' + (Get-LabProp $p 'arquivo')
      sha256      = $(if ($tipo -eq 'vs-layout') { '' } else { [string](Get-LabProp $p 'sha256') })
      bytes       = $(if ($tipo -eq 'vs-layout') { 0 } else { [long](Get-LabProp $p 'bytes') })
      componentes = @(Get-LabList (Get-LabProp $p 'componentes'))
      bootstrapper = $(if ($tipo -eq 'vs-layout') { $script:LabCacheGuestDir + '\' + (Get-LabProp (Get-LabProp $p 'bootstrapper') 'arquivo') } else { '' })
    }
  }
  return $list
}

function ConvertTo-LabGuestMsvcLayout {
  # guest.config.json "msvc" entry for sandbox\layout-msvc.ps1 (ambiente cache-msvc, spec addendum 8.2). Guest paths only.
  param($Compose)
  $p = Get-LabProp (Get-LabProp $Compose 'pacotes') 'msvc'
  if ($null -eq $p) { return $null }
  $b = Get-LabProp $p 'bootstrapper'
  return [ordered]@{
    versao       = [string](Get-LabProp $p 'versao')
    bootstrapper = $script:LabCacheGuestDir + '\' + (Get-LabProp $b 'arquivo')
    sha256       = [string](Get-LabProp $b 'sha256')
    componentes  = @(Get-LabList (Get-LabProp $p 'componentes'))
    saida        = $script:LabMountTargets['cache-saida'].sandbox
  }
}

function Get-LabComposeMounts {
  # Mounts besides projeto (always in sandbox.template.wsb), in compose order.
  param($Compose, [string]$Ambiente)
  $a = Get-LabProp (Get-LabProp $Compose 'ambientes') $Ambiente
  $out = @()
  foreach ($m in (Get-LabList (Get-LabProp $a 'montar'))) {
    $n = ($m -split ':')[0].ToLowerInvariant()
    if ($n -eq 'projeto') { continue }
    $out += [ordered]@{ nome = $n; sandbox = $script:LabMountTargets[$n].sandbox; readOnly = ($script:LabMountTargets[$n].modo -eq 'ro') }
  }
  return $out
}

$script:LabRealDataAmbientes = @('app', 'app-winapp')

function Get-LabRealDataAmbienteRefusal([string]$Ambiente) {
  # test-up -DadosReais: the real app data is only for the ambientes that run the app (scenario copia). Anything else
  # is refused before a folder is mounted or copied.
  if ([string]::IsNullOrWhiteSpace($Ambiente)) { return '-DadosReais needs an ambiente' }
  if ($script:LabRealDataAmbientes -notcontains $Ambiente) { return "-DadosReais is only for the ambientes $($script:LabRealDataAmbientes -join ', '), not '$Ambiente'" }
  return $null
}

function Get-LabUpVerdict {
  # Final exit code of test-up.ps1: 0 ok, 1 something in the guest failed, 2 the guest never reported.
  # A failed package is never hidden (exit 1 with the reason), but the reason also says which packages did install, so
  # a Node-only use of the guest (Sandbox tests, scenarios) still works: guest scripts decide with -Precisa.
  param($Status, $DiscoZ, $Instalar, [bool]$PrecisaInstalar, [bool]$PrecisaConta = $false)
  if ($null -eq $Status) { return [ordered]@{ exit = 2; motivo = 'status.json did not appear' } }
  if ($Status.bootstrap -ne 'ok') { return [ordered]@{ exit = 1; motivo = "bootstrap=$($Status.bootstrap)" } }
  if ($PrecisaConta -and "$($Status.conta)" -ne 'presente') { return [ordered]@{ exit = 1; motivo = "Claude login not in the guest: conta=$($Status.conta)" } }
  if ($null -eq $DiscoZ) { return [ordered]@{ exit = 1; motivo = 'Z: drive not ready in the guest: disco-z.json missing' } }
  if (-not $DiscoZ.ok) { return [ordered]@{ exit = 1; motivo = "Z: drive not ready in the guest: $($DiscoZ.erro)" } }
  if ($PrecisaInstalar) {
    if ($null -eq $Instalar -or -not $Instalar.fim) { return [ordered]@{ exit = 1; motivo = 'instalar.json did not finish in time' } }
    if (-not $Instalar.ok) {
      $bad = @(@($Instalar.pacotes) | Where-Object { $_ -and $_.exit -ne 0 } | ForEach-Object { "$($_.nome) (exit $($_.exit))" })
      $good = @(@($Instalar.pacotes) | Where-Object { $_ -and $_.exit -eq 0 } | ForEach-Object { [string]$_.nome })
      $why = $(if ($bad.Count) { $bad -join ', ' } else { $Instalar.erro })
      return [ordered]@{ exit = 1; motivo = "instalar failed: $why; installed ok: $(if ($good.Count) { $good -join ', ' } else { 'none' })" }
    }
  }
  return [ordered]@{ exit = 0; motivo = 'bootstrap ok, Z: ready' + $(if ($PrecisaInstalar) { ', packages installed' } else { '' }) }
}

function Test-LabCredentialRemovableNow {
  # test-up: the PC copy of the login may go as soon as the guest reports conta=presente (bootstrap.ps1 reads C:\labcred
  # only once). Never when the ambiente has no account, never twice, never on a missing status or any other value.
  param([bool]$Conta, [bool]$AlreadyRemoved, $Status)
  if (-not $Conta -or $AlreadyRemoved -or $null -eq $Status) { return $false }
  return ("$($Status.conta)" -eq 'presente')
}

function Invoke-LabStartWithCredential {
  # Copy the credential (when the ambiente asks), start the guest, and remove the copy if the start throws.
  # The three steps are injected, so the ordering is tested without a disk.
  param([bool]$Conta, [scriptblock]$Copy, [scriptblock]$Start, [scriptblock]$Cleanup)
  if ($Conta) { & $Copy | Out-Null }
  try { return (& $Start) }
  catch { if ($Conta) { & $Cleanup | Out-Null }; throw }
}
