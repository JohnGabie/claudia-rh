# guest-chrome-probe.ps1 - roda DENTRO do Sandbox (PowerShell 5.1).
# Encontra instancias do Chrome/Chromium com DevTools ligado (--remote-debugging-port na linha de comando;
# porta 0 -> le DevToolsActivePort do --user-data-dir), consulta /json/version e /json/list e, com -Navigate,
# abre UMA aba na URL informada via PUT /json/new.
# Saida: C:\labwork\run\chrome-probe.json
# Uso (host): winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-chrome-probe.ps1 [-Port N] [-Navigate <url>]
param([int]$Port = 0, [string]$Navigate = '', [string]$ExeName = 'chrome.exe')
$ErrorActionPreference = 'Continue'
$Out = 'C:\labwork\run'; New-Item -ItemType Directory -Force $Out | Out-Null
$r = [ordered]@{ at = (Get-Date).ToString('o'); instances = @(); errors = @() }

$cands = @()
if ($Port) { $cands += [ordered]@{ pid = $null; port = $Port; userDataDir = $null } }
else {
  foreach ($p in @(Get-CimInstance Win32_Process -Filter "Name='$ExeName'" | Where-Object { $_.CommandLine -match '--remote-debugging-port=(\d+)' -and $_.CommandLine -notmatch '--type=' })) {
    $null = $p.CommandLine -match '--remote-debugging-port=(\d+)'; $port = [int]$Matches[1]
    $udd = $(if ($p.CommandLine -match '--user-data-dir="?([^"]+?)"?(\s--|$)') { $Matches[1] } else { $null })
    if ($port -eq 0 -and $udd) { try { $port = [int](Get-Content (Join-Path $udd 'DevToolsActivePort') -ErrorAction Stop | Select-Object -First 1) } catch { $r.errors += "DevToolsActivePort: $_" } }
    $cands += [ordered]@{ pid = $p.ProcessId; port = $port; userDataDir = $udd }
  }
}
foreach ($c in $cands) {
  $i = [ordered]@{ pid = $c.pid; port = $c.port; userDataDir = $c.userDataDir; browser = $null; tabs = @(); navigated = $null }
  if ($c.port) {
    $base = "http://127.0.0.1:$($c.port)"
    try { $i.browser = (Invoke-RestMethod "$base/json/version" -TimeoutSec 5).Browser } catch { $r.errors += "${base}/json/version: $_" }
    if ($Navigate) { try { $i.navigated = (Invoke-RestMethod -Method Put "$base/json/new?$Navigate" -TimeoutSec 10).url; Start-Sleep -Seconds 3 } catch { $r.errors += "${base}/json/new: $_" } }
    try { $i.tabs = @(Invoke-RestMethod "$base/json/list" -TimeoutSec 5 | ForEach-Object { $_ } | ForEach-Object { [ordered]@{ type = $_.type; title = $_.title; url = $_.url } }) } catch { $r.errors += "${base}/json/list: $_" }
  }
  $r.instances += $i
}
$j = $r | ConvertTo-Json -Depth 6
[IO.File]::WriteAllText((Join-Path $Out 'chrome-probe.json'), $j, (New-Object System.Text.UTF8Encoding($false)))
$j
