# guest-run.ps1 - roda DENTRO do Sandbox (PowerShell 5.1), na sessao do usuario do guest.
# Instala e inicia o app publicado por scripts\app-stage.ps1 (C:\appdeploy):
#   1. registra o estado do runtime WebView2 no guest
#   2. com -Cenario Z:\claudia-teste-sandbox\<nome> (criado no guest): copia appdata\* -> %APPDATA% e localappdata\* -> %LOCALAPPDATA%
#   3. copia C:\appdeploy\app -> C:\labdata\app e inicia o exe (destacado)
# Saida: C:\labwork\run\guest-run.json
# Uso (host): wsb exec --id <id> -r ExistingLogin -c "powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-run.ps1 [-Exe nome.exe] [-Cenario Z:\claudia-teste-sandbox\<nome>]"
#   (via 'winapp target exec' o app e encerrado quando o comando termina; use -NoStart la so para instalar os dados)
param([string]$Exe = '', [string]$Cenario = '', [string]$Deploy = 'C:\appdeploy', [int]$WaitSec = 8, [switch]$NoStart)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '..\guest-install.lib.ps1')
if (-not (Test-LabInGuest)) { Write-Host 'not inside the lab guest (needs the guest identity and the bootstrap marker)'; exit 1 }
$Out = 'C:\labwork\run'; New-Item -ItemType Directory -Force $Out | Out-Null
$utf8 = New-Object System.Text.UTF8Encoding($false)
$r = [ordered]@{ at = (Get-Date).ToString('o'); user = $env:USERNAME; appdata = $env:APPDATA; localappdata = $env:LOCALAPPDATA; errors = @() }

# 1. WebView2
$wv = $null
foreach ($k in 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
               'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}') {
  try { $wv = (Get-ItemProperty $k -ErrorAction Stop).pv; break } catch { }
}
$wvDir = Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory -ErrorAction SilentlyContinue | Where-Object Name -match '^\d' | Select-Object -Expand Name
$r.webview2 = [ordered]@{ registryPv = $wv; evergreenDirs = @($wvDir) }

# 2. app data from a scenario created in the guest (D1); the deploy carries only the app
$r.dataFiles = [ordered]@{}
if ($Cenario) {
  $why = Get-LabGuestScenarioRefusal $Cenario
  $item = Get-Item -LiteralPath $Cenario -Force -ErrorAction SilentlyContinue
  if (-not $why -and -not $item) { $why = 'scenario folder not found' }
  if (-not $why -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { $why = 'scenario folder is a link' }
  if (-not $why) { $why = Get-LabGuestScenarioRefusal $item.FullName }   # the real path, too
  if (-not $why) {
    # Only a finished scenario is installed: cenario.json is the last file the scenario scripts write. A partial copy
    # (no or invalid cenario.json) is refused, not deleted (the guest is discarded).
    $cj = Join-Path $item.FullName 'cenario.json'
    $cjItem = Get-Item -LiteralPath $cj -Force -ErrorAction SilentlyContinue
    $json = $null
    if ($cjItem -and -not $cjItem.PSIsContainer -and -not ($cjItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) { try { $json = Get-Content -LiteralPath $cj -Raw -ErrorAction Stop } catch { $json = $null } }
    $why = Get-LabGuestScenarioJsonRefusal $json
  }
  if ($why) { $r.errors += "cenario: $why" }
  else {
    foreach ($pair in @(@('appdata', $env:APPDATA), @('localappdata', $env:LOCALAPPDATA))) {
      $src = Join-Path $item.FullName $pair[0]
      if (-not (Test-Path -LiteralPath $src)) { continue }
      try {
        robocopy $src $pair[1] /E /XJ /NFL /NDL /NP /R:1 /W:1 | Out-Null
        $files = @(Get-ChildItem -LiteralPath $src -Recurse -File)
        foreach ($f in $files) { $t = Join-Path $pair[1] $f.FullName.Substring($src.Length + 1); if (Test-Path -LiteralPath $t) { (Get-Item -LiteralPath $t).IsReadOnly = $false } }
        $r.dataFiles[$pair[0]] = $files.Count
      } catch { $r.errors += "$($pair[0]): $_" }
    }
  }
}

# 3. app
$appDir = 'C:\labdata\app'
try {
  New-Item -ItemType Directory -Force $appDir | Out-Null
  robocopy (Join-Path $Deploy 'app') $appDir /MIR /NFL /NDL /NP /R:1 /W:1 | Out-Null
  $exes = @(Get-ChildItem $appDir -Filter *.exe -File)
  $exePath = $(if ($Exe) { Join-Path $appDir $Exe } elseif ($exes.Count -eq 1) { $exes[0].FullName } else { $null })
  if (-not $exePath) { throw "informe -Exe (exes em ${appDir}: $($exes.Name -join ', '))" }
  $r.exe = $exePath
  # Where the ambiente keeps the internet blocked, the app opens only with the block mark in firewall.txt (guest-firewall.ps1
  # exits 1 without applying the rule when its guard refuses or it fails). Unreadable config or file = refuse.
  $refusal = $null
  if (-not $NoStart) {
    $cfgG = $null; try { $cfgG = Get-Content -LiteralPath 'C:\labsandbox\guest.config.json' -Raw -ErrorAction Stop | ConvertFrom-Json } catch { }
    $fwText = $null; try { $fwText = Get-Content -LiteralPath (Join-Path $Out 'firewall.txt') -Raw -ErrorAction Stop } catch { }
    $refusal = Get-LabAppStartRefusal -InternetPermitida (Get-LabGuestInternetPermitida $cfgG) -FirewallText $fwText
    if ($refusal) { $r.errors += "app not started: $refusal"; $r.refused = $true }
  }
  if (-not $NoStart -and -not $refusal) {
    $name = [IO.Path]::GetFileNameWithoutExtension($exePath)
    Get-Process $name -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    $p = Start-Process $exePath -PassThru
    Start-Sleep -Seconds $WaitSec
    $r.pid = $p.Id
    $r.alive = -not $p.HasExited
    if ($p.HasExited) { $r.exitCode = $p.ExitCode }
    $r.windows = @(Get-Process -Id $p.Id -ErrorAction SilentlyContinue | Where-Object MainWindowTitle | ForEach-Object MainWindowTitle)
  }
} catch { $r.errors += "app: $_" }
[IO.File]::WriteAllText((Join-Path $Out 'guest-run.json'), ($r | ConvertTo-Json -Depth 5), $utf8)
if ($r.Contains('refused')) { $r | ConvertTo-Json -Depth 5; exit 1 }
$r | ConvertTo-Json -Depth 5
