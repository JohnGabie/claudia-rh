<#
.SYNOPSIS
  Derruba o Windows Sandbox do lab. Usa 'wsb stop' se existir; senao fecha/encerra apenas
  processos do Windows Sandbox (nunca toca em vmmem de WSL/Docker/Hyper-V).
#>
[CmdletBinding()]
param([int]$TimeoutSec = 30)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')

$wsb = Get-Command wsb -ErrorAction SilentlyContinue
if ($wsb) {
  $list = & $wsb.Source list 2>$null
  foreach ($id in ($list | Select-String -Pattern '[0-9a-fA-F-]{36}' -AllMatches | ForEach-Object { $_.Matches.Value } | Select-Object -Unique)) {
    Write-Host "[test-down] wsb stop --id $id"; & $wsb.Source stop --id $id
  }
}

# apenas processos do Windows Sandbox
$names = 'WindowsSandboxClient','WindowsSandboxRemoteSession','WindowsSandbox'
$procs = Get-Process -Name $names -ErrorAction SilentlyContinue
if ($procs) {
  # fechamento gracioso primeiro (equivale a fechar a janela do Sandbox)
  $procs | ForEach-Object { try { [void]$_.CloseMainWindow() } catch { } }
  $deadline = (Get-Date).AddSeconds($TimeoutSec)
  while ((Get-Date) -lt $deadline -and (Get-Process -Name $names -ErrorAction SilentlyContinue)) { Start-Sleep -Seconds 1 }
  $left = Get-Process -Name $names -ErrorAction SilentlyContinue
  if ($left) { Write-Host "[test-down] forcando: $($left.Name -join ', ')"; $left | Stop-Process -Force -ErrorAction SilentlyContinue }
}

# o VM em si (vmmemWindowsSandbox / WindowsSandboxServer) encerra sozinho quando o client fecha
$deadline = (Get-Date).AddSeconds($TimeoutSec)
while ((Get-Date) -lt $deadline -and (Get-Process -Name 'vmmemWindowsSandbox','WindowsSandboxServer' -ErrorAction SilentlyContinue)) { Start-Sleep -Seconds 1 }
$still = Get-Process -Name 'WindowsSandboxServer','vmmemWindowsSandbox','WindowsSandboxRemoteSession','WindowsSandboxClient' -ErrorAction SilentlyContinue
if ($still) {
  $still | Where-Object Name -eq 'WindowsSandboxServer' | Stop-Process -Force -ErrorAction SilentlyContinue
  Start-Sleep -Seconds 3
  $still = Get-Process -Name 'WindowsSandboxServer','vmmemWindowsSandbox','WindowsSandboxRemoteSession','WindowsSandboxClient' -ErrorAction SilentlyContinue
}
$code = 0
if ($still) { Write-Warning "[test-down] ainda ativos: $($still | ForEach-Object { "$($_.Name)($($_.Id))" })"; $code = 1 }
else { Write-Host "[test-down] Sandbox encerrado." -ForegroundColor Green }
# The copy of the Claude login (spec A4) never outlives the Sandbox: only that file, through the pure guard.
$RepoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
try { Remove-LabCredential -RepoRoot $RepoRoot } catch { Write-Warning "[test-down] login copy not removed: $($_.Exception.Message)"; $code = 1 }
exit $code
