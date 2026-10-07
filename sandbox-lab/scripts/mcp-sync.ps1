<#
.SYNOPSIS
  Atualiza .mcp.json (raiz do lab) com o IP atual do Sandbox.

.DESCRIPTION
  O IP do guest muda a cada boot. O guest (sandbox\bootstrap-mcp.ps1) grava
  shared\out\mcp-status.json com {ok, ip, port, url}. Este script le esse arquivo e
  reescreve .mcp.json:

    "windows-sandbox": {
      "type": "http",
      "url": "http://<ip>:<port>/mcp",
      "headers": { "Authorization": "Bearer ${LAB_MCP_AUTH_KEY}" }
    }

  A chave NAO vai para o .mcp.json: o Claude Code expande ${LAB_MCP_AUTH_KEY} do ambiente.
  Antes de abrir o `claude` no lab, defina a variavel (na mesma sessao):
     $env:LAB_MCP_AUTH_KEY = (Get-Content shared\secrets\mcp-auth.key -Raw).Trim()

.PARAMETER Wait     Segundos para esperar o mcp-status.json aparecer com ok=true (default 0).
.PARAMETER Test     Faz um POST initialize + tools/list no servidor e mostra quantas tools existem.
#>
[CmdletBinding()]
param(
  [int]$Wait = 0,
  [switch]$Test
)
$ErrorActionPreference = 'Stop'
$lab = Split-Path -Parent $PSScriptRoot
$statusFile = Join-Path $lab 'shared\out\mcp-status.json'
$cfg = Get-Content (Join-Path $lab 'shared\mcp.json') -Raw | ConvertFrom-Json
$keyFile = Join-Path (Join-Path $lab 'shared') $cfg.authKeyFile
$envName = if ($cfg.authKeyEnv) { $cfg.authKeyEnv } else { 'LAB_MCP_AUTH_KEY' }

$deadline = (Get-Date).AddSeconds($Wait)
do {
  $st = $null
  if (Test-Path $statusFile) { try { $st = Get-Content $statusFile -Raw | ConvertFrom-Json } catch { } }
  if ($st -and $st.ok -and -not $st.dryRun -and $st.ip) { break }
  if ((Get-Date) -ge $deadline) { break }
  Start-Sleep -Seconds 2
} while ($true)

if (-not $st) { throw "Nao achei $statusFile. O Sandbox esta rodando (scripts\test-up.ps1)?" }
if (-not $st.ok -or $st.dryRun -or -not $st.ip) {
  throw "mcp-status.json nao esta ok (ok=$($st.ok), dryRun=$($st.dryRun), ip=$($st.ip)). Erros: $($st.errors -join ' | ')"
}

$port = if ($st.port) { $st.port } else { $cfg.port }
$path = if ($cfg.path) { $cfg.path } else { '/mcp' }
$url = "http://$($st.ip):$port$path"
$key = (Get-Content $keyFile -Raw).Trim()
$auth = "Bearer `${$envName}"   # the key itself never goes to .mcp.json

$mcp = [ordered]@{
  mcpServers = [ordered]@{
    'windows-sandbox' = [ordered]@{
      type    = 'http'
      url     = $url
      headers = [ordered]@{ Authorization = $auth }
    }
  }
}
$json = $mcp | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $lab '.mcp.json'), $json + "`n", (New-Object Text.UTF8Encoding($false)))
Write-Host ".mcp.json -> $url (auth: `${$envName})"
if (-not [Environment]::GetEnvironmentVariable($envName)) {
  Write-Warning "`$env:$envName nao esta definida nesta sessao. Rode antes do claude:`n  `$env:$envName = (Get-Content '$keyFile' -Raw).Trim()"
}

if ($Test) {
  $h = @{ Authorization = "Bearer $key"; Accept = 'application/json, text/event-stream' }
  $init = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"mcp-sync","version":"1"}}}'
  $r = Invoke-WebRequest -Uri $url -Method Post -Body $init -ContentType 'application/json' -Headers $h -UseBasicParsing -TimeoutSec 20
  $sid = $r.Headers['Mcp-Session-Id']; if ($sid -is [array]) { $sid = $sid[0] }
  if ($sid) { $h['Mcp-Session-Id'] = $sid }
  Invoke-WebRequest -Uri $url -Method Post -Body '{"jsonrpc":"2.0","method":"notifications/initialized"}' -ContentType 'application/json' -Headers $h -UseBasicParsing -TimeoutSec 20 | Out-Null
  $r2 = Invoke-WebRequest -Uri $url -Method Post -Body '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' -ContentType 'application/json' -Headers $h -UseBasicParsing -TimeoutSec 30
  $content = if ($r2.Content -is [byte[]]) { [Text.Encoding]::UTF8.GetString($r2.Content) } else { [string]$r2.Content }
  $data = ($content -split "`n" | Where-Object { $_ -like 'data:*' } | Select-Object -Last 1)
  $payload = if ($data) { $data.Substring(5) } else { $content }
  $tools = ($payload | ConvertFrom-Json).result.tools
  Write-Host "MCP OK: initialize HTTP $($r.StatusCode); tools/list -> $($tools.Count) tools: $(($tools.name) -join ', ')"
}
