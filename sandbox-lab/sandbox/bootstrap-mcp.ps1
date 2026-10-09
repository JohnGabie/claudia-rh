<#
.SYNOPSIS
  Sobe o Windows-MCP (CursorTouch) dentro do Windows Sandbox.

.DESCRIPTION
  Chamado por C:\labsandbox\bootstrap.ps1 (infra). Passos:
    1. le C:\lab\mcp.json (porta, arquivo da auth key, versao)
    2. copia o Python 3.14 pre-baixado (C:\lab\tools\python) para C:\labdata\python
       (C:\lab e read-only no guest)
    3. cria venv em C:\labdata\mcp-venv e instala windows-mcp OFFLINE do wheelhouse
       C:\lab\tools\wheels (fallback: download pela rede)
    4. regra de firewall de entrada para a porta
    5. inicia: windows-mcp serve --transport streamable-http --host 0.0.0.0 --port <port>
       (auth key via env WINDOWS_MCP_AUTH_KEY -> exige "Authorization: Bearer <key>")
    6. espera a porta escutar + self-test (POST initialize com o header)
    7. grava C:\lab\out\mcp-status.json e C:\lab\out\bootstrap-mcp.log

  -DryRun: faz o setup (passos 1-3) e roda --help, mas NAO abre firewall nem inicia o servidor.
  So roda dentro do guest (trava Test-LabInGuest, tambem em -DryRun); -LabRoot/-DataRoot/-OutDir mudam as pastas la dentro.

  Compativel com Windows PowerShell 5.1 (o guest nao tem pwsh). Arquivo ASCII de proposito.
#>
[CmdletBinding()]
param(
  [string]$LabRoot = 'C:\lab',
  [string]$DataRoot = 'C:\labdata',
  [string]$OutDir = 'C:\lab\out',
  [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# Guard (also in -DryRun: this script never runs outside the guest): bootstrap.ps1 starts this script as the
# guest logon user, so both factors hold there (WDAGUtilityAccount plus the bootstrap marker). Nothing is written on refusal.
. (Join-Path $PSScriptRoot 'guest-install.lib.ps1')
if (-not (Test-LabInGuest)) { Write-Host 'not inside the lab guest (needs the guest identity and the bootstrap marker)'; exit 1 }

$sw = [Diagnostics.Stopwatch]::StartNew()
$startedAt = (Get-Date).ToString('o')
$errors = New-Object System.Collections.Generic.List[string]
$steps = [ordered]@{}

New-Item -ItemType Directory -Force $OutDir, $DataRoot | Out-Null
$logFile = Join-Path $OutDir 'bootstrap-mcp.log'
$statusFile = Join-Path $OutDir 'mcp-status.json'

function Log([string]$msg) {
  $line = '[{0:HH:mm:ss.fff}] [{1,6:N1}s] {2}' -f (Get-Date), $sw.Elapsed.TotalSeconds, $msg
  Add-Content -Path $logFile -Value $line -Encoding UTF8
  Write-Host $line
}

function Step([string]$name, [scriptblock]$block) {
  $t = [Diagnostics.Stopwatch]::StartNew()
  Log ">> $name"
  try { & $block; $steps[$name] = [math]::Round($t.Elapsed.TotalSeconds, 1); Log "<< $name ok ($($steps[$name])s)"; return $true }
  catch { $steps[$name] = 'FAIL'; $errors.Add("${name}: $($_.Exception.Message)"); Log "!! $name FAILED: $($_.Exception.Message)"; return $false }
}

function Invoke-Native([string]$tag, [string]$exe, [string[]]$argv) {
  # PS 5.1 + EAP=Stop transforma stderr de exe nativo em erro terminante; isola aqui.
  $eap = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
  try { & $exe @argv 2>&1 | ForEach-Object { Log "${tag}: $_" } } finally { $ErrorActionPreference = $eap }
  return $LASTEXITCODE
}

function Get-GuestIPv4 {
  $cfg = Get-NetIPConfiguration -ErrorAction SilentlyContinue | Where-Object { $_.IPv4DefaultGateway -and $_.IPv4Address } | Select-Object -First 1
  if ($cfg) { return $cfg.IPv4Address[0].IPAddress }
  $ip = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
    Where-Object { $_.IPAddress -notmatch '^(127\.|169\.254\.)' } | Select-Object -First 1
  if ($ip) { return $ip.IPAddress }
  return $null
}

function Write-Status([bool]$ok, $port, $ip, $extra) {
  $url = if ($ip -and $port) { "http://${ip}:${port}/mcp" } else { $null }
  $o = [ordered]@{
    ok          = $ok
    dryRun      = [bool]$DryRun
    port        = $port
    url         = $url
    ip          = $ip
    startedAt   = $startedAt
    finishedAt  = (Get-Date).ToString('o')
    durationSec = [math]::Round($sw.Elapsed.TotalSeconds, 1)
    authHeader  = 'Authorization: Bearer <conteudo de shared\secrets\mcp-auth.key>'
    steps       = $steps
    errors      = @($errors)
  }
  if ($extra) { foreach ($k in $extra.Keys) { $o[$k] = $extra[$k] } }
  # UTF-8 sem BOM (ConvertFrom-Json/jq no host leem sem problema)
  [IO.File]::WriteAllText($statusFile, ($o | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding($false)))
  Log "status -> $statusFile (ok=$ok)"
}

Log "==== bootstrap-mcp start (DryRun=$DryRun, LabRoot=$LabRoot, DataRoot=$DataRoot) ===="

# ---------- 1. config ----------
$cfg = $null; $authKey = $null
$port = 8000
$okCfg = Step 'read-config' {
  $script:cfg = Get-Content (Join-Path $LabRoot 'mcp.json') -Raw | ConvertFrom-Json
  if ($script:cfg.port) { $script:port = [int]$script:cfg.port }
  if ($env:LAB_MCP_PORT -match '^\d+$') { $script:port = [int]$env:LAB_MCP_PORT }  # override vindo do bootstrap.ps1 (infra)
  $keyPath = Join-Path $LabRoot $script:cfg.authKeyFile
  $script:authKey = (Get-Content $keyPath -Raw).Trim()
  if ($script:authKey.Length -lt 16) { throw "auth key em $keyPath vazia/curta" }
  Log "port=$($script:port) version=$($script:cfg.windowsMcpVersion) keyFile=$keyPath"
}
if (-not $okCfg) { Write-Status $false $port (Get-GuestIPv4) $null; exit 1 }

$version   = if ($cfg.windowsMcpVersion) { [string]$cfg.windowsMcpVersion } else { '0.8.7' }
$pyVer     = if ($cfg.pythonVersion) { [string]$cfg.pythonVersion } else { '3.14' }
$uv        = Join-Path $LabRoot $cfg.uvExe
$pySrcRoot = Join-Path $LabRoot $cfg.pythonDir
$wheels    = Join-Path $LabRoot $cfg.wheelhouse
$pyDst     = Join-Path $DataRoot 'python'
$venv      = Join-Path $DataRoot 'mcp-venv'
$wmcpExe   = Join-Path $venv 'Scripts\windows-mcp.exe'

$env:UV_CACHE_DIR = Join-Path $DataRoot 'uv-cache'
$env:UV_PYTHON_INSTALL_DIR = Join-Path $DataRoot 'uv-python'
$env:UV_LINK_MODE = 'hardlink'   # cache e venv no mesmo volume (C:\labdata)
$env:UV_NO_CONFIG = '1'
$env:ANONYMIZED_TELEMETRY = 'false'
$env:PYTHONUTF8 = '1'
$installMode = 'offline'

# ---------- 2. python ----------
$pyExe = Join-Path $pyDst 'python.exe'
Step 'python' {
  if (Test-Path $pyExe) { Log "python ja existe em $pyDst"; return }
  $src = Get-ChildItem $pySrcRoot -Directory -Filter "cpython-$pyVer*" -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($src) {
    Log "copiando $($src.FullName) -> $pyDst"
    robocopy $src.FullName $pyDst /E /MT:16 /NFL /NDL /NJH /NJS /NP | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "robocopy falhou ($LASTEXITCODE)" }
  } else {
    Log "python pre-baixado nao encontrado; baixando via uv (rede)"
    $null = Invoke-Native uv $uv @('python', 'install', $pyVer)
    $found = Get-ChildItem $env:UV_PYTHON_INSTALL_DIR -Directory -Filter "cpython-$pyVer*" | Select-Object -First 1
    if (-not $found) { throw 'uv python install nao produziu interpretador' }
    $script:pyExe = Join-Path $found.FullName 'python.exe'
  }
  if (-not (Test-Path $script:pyExe)) { throw "python.exe ausente: $script:pyExe" }
} | Out-Null

# ---------- 3. venv + windows-mcp ----------
$okInstall = Step 'venv-install' {
  if (Test-Path $wmcpExe) { Log 'windows-mcp ja instalado'; return }
  $rc = Invoke-Native uv $uv @('venv', $venv, '--python', $pyExe, '--quiet')
  if ($rc -ne 0) { throw "uv venv falhou ($rc)" }
  $vpy = Join-Path $venv 'Scripts\python.exe'
  $rc = 1
  if (Test-Path $wheels) {
    $rc = Invoke-Native uv $uv @('pip', 'install', '--python', $vpy, '--offline', '--no-index', '--find-links', $wheels, "windows-mcp==$version")
  }
  if ($rc -ne 0) {
    Log 'instalacao offline falhou -> fallback online (PyPI)'
    $script:installMode = 'online-fallback'
    $rc = Invoke-Native uv $uv @('pip', 'install', '--python', $vpy, "windows-mcp==$version")
    if ($rc -ne 0) { throw "uv pip install falhou ($rc)" }
  }
  if (-not (Test-Path $wmcpExe)) { throw "windows-mcp.exe ausente apos install" }
}

$ip = Get-GuestIPv4
if (-not $okInstall) { Write-Status $false $port $ip @{ installMode = $installMode }; exit 1 }

if ($DryRun) {
  Step 'help' {
    $rc1 = Invoke-Native help $wmcpExe @('--help')
    $rc2 = Invoke-Native serve-help $wmcpExe @('serve', '--help')
    if ($rc1 -ne 0 -or $rc2 -ne 0) { throw "windows-mcp --help saiu com $rc1/$rc2" }
  } | Out-Null
  $cmd = "$venv\Scripts\pythonw.exe -m windows_mcp serve --transport streamable-http --host 0.0.0.0 --port $port   (env WINDOWS_MCP_AUTH_KEY=<key>)"
  Log "DryRun: NAO abrindo firewall nem iniciando servidor. Comando seria: $cmd"
  Write-Status ($errors.Count -eq 0) $port $ip @{ installMode = $installMode; command = $cmd }
  exit ([int]($errors.Count -ne 0))
}

# ---------- 4. firewall ----------
Step 'firewall' {
  $name = "lab-windows-mcp-$port"
  Get-NetFirewallRule -DisplayName $name -ErrorAction SilentlyContinue | Remove-NetFirewallRule
  New-NetFirewallRule -DisplayName $name -Direction Inbound -Action Allow -Protocol TCP -LocalPort $port -Profile Any | Out-Null
} | Out-Null

# ---------- 5. serve ----------
$serverLog = Join-Path $OutDir 'mcp-server.log'
$serverErr = Join-Path $OutDir 'mcp-server.err.log'
$proc = $null
Step 'start-server' {
  $env:WINDOWS_MCP_AUTH_KEY = $authKey          # nao vai para a linha de comando
  if ($cfg.excludeTools) { $env:WINDOWS_MCP_EXCLUDE_TOOLS = [string]$cfg.excludeTools }
  # pythonw.exe -m windows_mcp == windows-mcp.exe, mas sem janela de console no desktop do guest
  # (a janela apareceria nos screenshots/snapshots do proprio MCP).
  $pyw = Join-Path $venv 'Scripts\pythonw.exe'
  $args_ = @('-m', 'windows_mcp', 'serve', '--transport', 'streamable-http', '--host', '0.0.0.0', '--port', "$port")
  $script:proc = Start-Process -FilePath $pyw -ArgumentList $args_ -PassThru `
    -RedirectStandardOutput $serverLog -RedirectStandardError $serverErr -WorkingDirectory $DataRoot
  Remove-Item Env:\WINDOWS_MCP_AUTH_KEY
  Log "windows-mcp pid=$($script:proc.Id): $pyw $($args_ -join ' ')"
} | Out-Null

# ---------- 6. wait + self-test ----------
$timeout = if ($cfg.readyTimeoutSec) { [int]$cfg.readyTimeoutSec } else { 120 }
$listening = $false
Step 'wait-listen' {
  $deadline = (Get-Date).AddSeconds($timeout)
  while ((Get-Date) -lt $deadline) {
    if (Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue) { $script:listening = $true; break }
    if ($script:proc -and $script:proc.HasExited) { throw "windows-mcp saiu com codigo $($script:proc.ExitCode) (veja mcp-server.err.log)" }
    Start-Sleep -Milliseconds 500
  }
  if (-not $script:listening) { throw "porta $port nao escutou em ${timeout}s" }
} | Out-Null

$selfTest = $null
if ($listening) {
  Step 'self-test' {
    $body = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"lab-bootstrap","version":"1"}}}'
    $h = @{ Authorization = "Bearer $authKey"; Accept = 'application/json, text/event-stream' }
    $r = Invoke-WebRequest -Uri "http://127.0.0.1:$port/mcp" -Method Post -Body $body -ContentType 'application/json' -Headers $h -UseBasicParsing -TimeoutSec 30
    $script:selfTest = "HTTP $($r.StatusCode)"
    if ($r.Content -notmatch 'serverInfo') { throw "initialize sem serverInfo: $($r.Content.Substring(0, [math]::Min(200, $r.Content.Length)))" }
    try {
      Invoke-WebRequest -Uri "http://127.0.0.1:$port/mcp" -Method Post -Body $body -ContentType 'application/json' -Headers @{ Accept = 'application/json, text/event-stream' } -UseBasicParsing -TimeoutSec 10 | Out-Null
      throw 'request SEM auth foi aceito (esperado 401)'
    } catch [System.Net.WebException] {
      $script:selfTest += "; sem-auth -> $([int]$_.Exception.Response.StatusCode)"
    }
  } | Out-Null
}

Write-Status ($errors.Count -eq 0) $port $ip @{
  installMode = $installMode
  pid         = $(if ($proc) { $proc.Id } else { $null })
  selfTest    = $selfTest
  command     = "$venv\Scripts\pythonw.exe -m windows_mcp serve --transport streamable-http --host 0.0.0.0 --port $port (env WINDOWS_MCP_AUTH_KEY)"
}
exit ([int]($errors.Count -ne 0))
