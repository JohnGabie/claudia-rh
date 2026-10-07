<#
.SYNOPSIS
  Sobe o lab: le lab.config.json, gera sandbox\sandbox.generated.wsb + sandbox\guest.config.json,
  limpa shared\out, inicia o Windows Sandbox e espera shared\out\status.json.
.PARAMETER TimeoutSec   Tempo maximo de espera pelo status.json final (padrao: bootTimeoutSec do config).
.PARAMETER GenerateOnly So gera os arquivos (usado pelo validate-config.ps1); nao sobe nada.
.PARAMETER Force        Se ja houver um Sandbox rodando, derruba antes (via test-down.ps1).
.PARAMETER NoWait       Nao espera o status.json.
.PARAMETER WaitForMcp   Espera tambem a fase MCP terminar (phase=complete), nao so o core.
.PARAMETER Config       Perfil do lab (padrao: lab.config.json). Ex.: -Config lab.config.app.json.
                        Caminho relativo a raiz do lab ou absoluto.
  Caminhos aceitam variaveis de ambiente (%USERPROFILE% etc.).
  Chave opcional no perfil: "extraMappedFolders": [ { "host": "D:\\x", "sandbox": "C:\\y", "readOnly": true } ]
  (pastas extras montadas no guest; o perfil padrao nao usa).
#>
[CmdletBinding()]
param(
  [int]$TimeoutSec = 0,
  [switch]$GenerateOnly,
  [switch]$Force,
  [switch]$NoWait,
  [switch]$WaitForMcp,
  [string]$Config = 'lab.config.json'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
$Root = Split-Path $PSScriptRoot -Parent
$cfgPath = $(if ([IO.Path]::IsPathRooted($Config)) { $Config } else { Join-Path $Root $Config })
if (-not (Test-Path $cfgPath -PathType Leaf)) { throw "Perfil nao encontrado: $cfgPath" }
Write-Host "[test-up] perfil: $cfgPath"
$cfg = Get-Content $cfgPath -Raw | ConvertFrom-Json

function Resolve-LabPath([string]$p) {
  $p = [Environment]::ExpandEnvironmentVariables($p)
  if ([IO.Path]::IsPathRooted($p)) { return [IO.Path]::GetFullPath($p) }
  return [IO.Path]::GetFullPath((Join-Path $Root $p))
}
# caminho do host dentro de shared\ -> caminho no guest (C:\lab\...)
$SharedDir = Resolve-LabPath 'shared'
function To-Guest([string]$hostPath) {
  $full = Resolve-LabPath $hostPath
  if (-not $full.StartsWith($SharedDir, [StringComparison]::OrdinalIgnoreCase)) { throw "Caminho fora de shared\ nao e visivel no guest: $hostPath" }
  return 'C:\lab' + $full.Substring($SharedDir.Length)
}
function XmlEsc([string]$s) { [Security.SecurityElement]::Escape($s) }

$OutDir     = Join-Path $SharedDir 'out'
$SandboxDir = Resolve-LabPath 'sandbox'
$ProjectDir = Resolve-LabPath $cfg.projectPath
$ExtDir     = Resolve-LabPath $cfg.extensionsDir
foreach ($d in @($SharedDir, $SandboxDir, $ProjectDir, $ExtDir)) { if (-not (Test-Path $d -PathType Container)) { throw "Pasta nao encontrada: $d" } }
New-Item -ItemType Directory -Force $OutDir | Out-Null

# --- guest.config.json (lido pelo bootstrap.ps1 e bootstrap-mcp.ps1 no guest) ---
$extList = @($cfg.extensions | ForEach-Object { To-Guest (Join-Path $cfg.extensionsDir $_) })
$guest = [ordered]@{
  generatedAt         = (Get-Date).ToString('o')
  startUrl            = $cfg.chrome.startUrl
  testServerPort      = [int]$cfg.chrome.testServerPort
  remoteDebuggingPort = [int]$cfg.chrome.remoteDebuggingPort
  chromeExe           = To-Guest $cfg.chrome.exe
  extensions          = $extList
  dbBackup            = To-Guest $cfg.dbBackup
  workDb              = 'C:\labdata\app.sqlite'
  mcpPort             = [int]$cfg.mcp.port
  mcpAuthKeyEnv       = $cfg.mcp.authKeyEnv
  mcpAuthKeyFile      = $(if ($cfg.mcp.authKeyFile) { To-Guest $cfg.mcp.authKeyFile } else { $null })
  outDir              = 'C:\lab\out'
}
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText((Join-Path $SandboxDir 'guest.config.json'), ($guest | ConvertTo-Json -Depth 5), $utf8)

# --- .wsb gerado ---
$tpl = Get-Content (Join-Path $SandboxDir 'sandbox.template.wsb') -Raw
$map = @{
  '{{SHARED_DIR}}'  = $SharedDir
  '{{OUT_DIR}}'     = $OutDir
  '{{PROJECT_DIR}}' = $ProjectDir
  '{{SANDBOX_DIR}}' = $SandboxDir
  '{{MEMORY_MB}}'   = [string][int]$cfg.memoryMB
  '{{VGPU}}'        = [string]$cfg.vGPU
  '{{NETWORKING}}'  = [string]$cfg.networking
  '{{CLIPBOARD}}'   = [string]$cfg.clipboardRedirection
}
foreach ($k in $map.Keys) { $tpl = $tpl.Replace($k, (XmlEsc $map[$k])) }
if ($tpl -match '\{\{[A-Z_]+\}\}') { throw "Placeholder nao substituido no template: $($Matches[0])" }
# pastas extras (opcional, por perfil) - inseridas antes de </MappedFolders>
if ($cfg.PSObject.Properties.Name -contains 'extraMappedFolders' -and $cfg.extraMappedFolders) {
  $extra = ''
  foreach ($m in @($cfg.extraMappedFolders)) {
    $h = Resolve-LabPath $m.host
    if (-not (Test-Path $h -PathType Container)) { throw "extraMappedFolders: pasta do host nao encontrada: $h" }
    $ro = $(if ($null -eq $m.readOnly -or [bool]$m.readOnly) { 'true' } else { 'false' })
    $extra += "    <MappedFolder>`r`n      <HostFolder>$(XmlEsc $h)</HostFolder>`r`n      <SandboxFolder>$(XmlEsc $m.sandbox)</SandboxFolder>`r`n      <ReadOnly>$ro</ReadOnly>`r`n    </MappedFolder>`r`n"
  }
  $tpl = $tpl.Replace('  </MappedFolders>', $extra + '  </MappedFolders>')
}
$null = [xml]$tpl   # valida XML
$WsbPath = Join-Path $SandboxDir 'sandbox.generated.wsb'
[IO.File]::WriteAllText($WsbPath, $tpl, $utf8)
Write-Host "[test-up] gerado: $WsbPath"
if ($GenerateOnly) { return }

# --- ja existe Sandbox? (Windows permite so um) ---
# WindowsSandboxServer sozinho NAO indica VM: qualquer chamada 'wsb' (ex.: 'wsb list') sobe esse broker ocioso.
# Considera "rodando" so se houver VM/cliente ou instancia listada pelo wsb.
$sbNames = 'WindowsSandboxRemoteSession','WindowsSandboxClient','WindowsSandbox','vmmemWindowsSandbox'
$running = @(Get-Process -Name $sbNames -ErrorAction SilentlyContinue)
$wsbCmd = Get-Command wsb -ErrorAction SilentlyContinue
if (-not $running -and $wsbCmd) {
  $ids = @((& $wsbCmd.Source list 2>$null) | Select-String -Pattern '[0-9a-fA-F-]{36}' -AllMatches | ForEach-Object { $_.Matches.Value })
  if ($ids) { $running = @([pscustomobject]@{ Name = "wsb:$($ids -join ',')" }) }
}
if ($running) {
  if (-not $Force) { throw "Ja existe um Windows Sandbox rodando ($($running.Name -join ', ')). Rode scripts\test-down.ps1 ou use -Force." }
  & (Join-Path $PSScriptRoot 'test-down.ps1')
}

# --- limpa shared\out (only the exact <lab>\shared\out, no link in it or under it) ---
Clear-LabOut -OutDir $OutDir -LabDir $Root
$StatusFile = Join-Path $OutDir 'status.json'

# --- inicia ---
$wsb = Get-Command wsb -ErrorAction SilentlyContinue
if ($wsb) {
  Write-Host "[test-up] iniciando via wsb CLI"
  $r = (& $wsb.Source start --config $tpl 2>&1) -join "`n"
  Write-Host $r
  # 'wsb start' sobe a VM sem sessao de usuario -> o LogonCommand so roda apos 'wsb connect'.
  # 'connect' bloqueia enquanto a janela estiver aberta, entao roda destacado.
  $id = [regex]::Match($r, '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}').Value
  if (-not $id) { throw "[test-up] nao consegui obter o Id do Sandbox da saida do wsb: $r" }
  Write-Host "[test-up] wsb connect --id $id (abre a janela; dispara o LogonCommand)"
  Start-Process -FilePath $wsb.Source -ArgumentList @('connect', '--id', $id) -WindowStyle Hidden
} else {
  Write-Host "[test-up] 'wsb' nao encontrado; iniciando via WindowsSandbox.exe"
  Start-Process -FilePath "$env:WINDIR\System32\WindowsSandbox.exe" -ArgumentList "`"$WsbPath`""
}
if ($NoWait) { return }

if ($TimeoutSec -le 0) { $TimeoutSec = [int]$(if ($cfg.bootTimeoutSec) { $cfg.bootTimeoutSec } else { 240 }) }
$deadline = (Get-Date).AddSeconds($TimeoutSec)
$st = $null
Write-Host "[test-up] aguardando $StatusFile (ate ${TimeoutSec}s)..."
while ((Get-Date) -lt $deadline) {
  Start-Sleep -Seconds 3
  if (Test-Path $StatusFile) {
    try { $st = Get-Content $StatusFile -Raw | ConvertFrom-Json } catch { continue }
    $done = $(if ($WaitForMcp) { $st.phase -eq 'complete' } else { $st.bootstrap -ne 'running' })
    if ($done) { break }
  }
}
if (-not $st) { Write-Error "[test-up] status.json nao apareceu em ${TimeoutSec}s"; exit 2 }
$st | ConvertTo-Json -Depth 8
if ($st.bootstrap -eq 'ok') { Write-Host "[test-up] OK: bootstrap=ok" -ForegroundColor Green; exit 0 }
Write-Host "[test-up] bootstrap=$($st.bootstrap) phase=$($st.phase)" -ForegroundColor Yellow
exit 1
