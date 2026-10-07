# guest-build.ps1 - runs INSIDE the Sandbox (PowerShell 5.1). Environment "testes" of compose.json.
# Builds the project inside the guest with what instalar.ps1 installed (C:\lab\out\instalar.json): MSVC, Rust, Node.
# Fonte: C:\labproject (RO, projectPath do perfil) -> copiada para C:\labdata\project.
# Saida: C:\labwork\build\ (pasta gravavel mapeada pelo perfil): guest-build.log, result.json.
# O target do cargo fica no disco do guest (C:\labdata\cargo-target): em pasta mapeada os build scripts falham (os error 1392).
# Uso (host): winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-build.ps1 `
#               [-Install 'npm.cmd ci'] [-Build 'npm.cmd run build'] [-Artifacts 'C:\labdata\cargo-target\release\*.exe'] [-Precisa todos|node|cargo]
param(
  [string]$Install = 'npm.cmd ci --no-audit --no-fund',
  [string]$Build = 'npm.cmd run build',
  [string]$Artifacts = 'C:\labdata\cargo-target\release\*.exe',
  # What this run needs from instalar.ps1: 'node' = the Sandbox-only tests (MSVC/Rust may have failed), 'cargo' = a Rust
  # build (MSVC and Rust ok), 'todos' = every package of the ambiente.
  [ValidateSet('todos', 'node', 'cargo')][string]$Precisa = 'todos'
)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '..\guest-install.lib.ps1')
if (-not (Test-LabInGuest)) { Write-Host 'not inside the lab guest (needs the guest identity and the bootstrap marker)'; exit 1 }
$Out = 'C:\labwork\build'
New-Item -ItemType Directory -Force $Out | Out-Null
$Log = Join-Path $Out 'guest-build.log'
$utf8 = New-Object System.Text.UTF8Encoding($false)
function Log([string]$m) { $l = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $m; [IO.File]::AppendAllText($Log, $l + "`r`n", $utf8); Write-Host $l }
$res = [ordered]@{ start = (Get-Date).ToString('o'); steps = [ordered]@{} }
function Save { [IO.File]::WriteAllText((Join-Path $Out 'result.json'), ($res | ConvertTo-Json -Depth 6), $utf8) }
function Step([string]$name, [string]$cmd) {
  Log "== $name : $cmd"
  $t = Get-Date
  $global:LASTEXITCODE = 0
  cmd.exe /d /c "$cmd 2>&1" | ForEach-Object { [IO.File]::AppendAllText($Log, "$_`r`n", $utf8) }
  $ec = $global:LASTEXITCODE
  $sec = [math]::Round(((Get-Date) - $t).TotalSeconds, 1)
  $res.steps[$name] = [ordered]@{ cmd = $cmd; exit = $ec; seconds = $sec }
  Log "== $name -> exit=$ec em ${sec}s"
  Save
  return ($ec -eq 0)
}

# --- environment: what instalar.ps1 installed in this guest (C:\lab\out\instalar.json, ambiente 'testes') ---
$inst = $null
try { $inst = Get-Content -LiteralPath 'C:\lab\out\instalar.json' -Raw | ConvertFrom-Json } catch { }
$toolEnv = Get-LabGuestToolEnv $inst -Precisa $Precisa
if (-not $toolEnv) {
  Log "instalar.json missing, unfinished, or the packages '$Precisa' needs did not install ok: start the lab with -Ambiente testes and wait for test-up to finish"
  # The host waits for a "build" step carrying its nonce: report the failure there instead of letting it time out.
  $res.steps['build'] = [ordered]@{ cmd = $Build; exit = 1; seconds = 0 }
  $res.end = (Get-Date).ToString('o'); Save
  exit 1
}
if ($toolEnv.vcvars) {
  # Import the MSVC environment (PATH, INCLUDE, LIB) the way a developer prompt does.
  foreach ($line in (cmd.exe /d /c "`"$($toolEnv.vcvars)`" >nul 2>&1 && set")) { if ($line -match '^([^=]+)=(.*)$') { Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2] } }
}
$env:PATH = ((@($toolEnv.paths) + $env:PATH) -join ';')
$env:DISABLE_AUTOUPDATER = '1'
$env:CARGO_HOME = 'C:\labdata\cargo-home'
$env:CARGO_TARGET_DIR = 'C:\labdata\cargo-target'
$env:npm_config_cache = 'C:\labdata\npm-cache'
$res.env = [ordered]@{ pacotes = @(@($inst.pacotes) | ForEach-Object { "$($_.nome) $($_.versao)" }); cargoTarget = $env:CARGO_TARGET_DIR }
Log "node=$(cmd /c node -v 2>&1) cargo=$(cmd /c cargo -V 2>&1) cl=$(cmd /c where cl.exe 2>&1 | Select-Object -First 1)"
Save

$Src = 'C:\labdata\project'
# Full paths in /XD exclude only these folders of the project root, not any folder with the same name.
robocopy C:\labproject $Src /E /XD node_modules target dist .git C:\labproject\sandbox-lab\shared C:\labproject\.sandbox-work C:\labproject\.claude\worktrees /NFL /NDL /NP /R:1 /W:1 | Out-Null
Set-Location $Src
if (-not $Install -or (Step 'install' $Install)) {
  if ($Build) { Step 'build' $Build | Out-Null }
}
$res.artifacts = @(Get-ChildItem $Artifacts -File -ErrorAction SilentlyContinue | ForEach-Object { [ordered]@{ path = $_.FullName; sizeMB = [math]::Round($_.Length / 1MB, 1) } })
$res.end = (Get-Date).ToString('o')
$res.guestFreeGB = [math]::Round((Get-PSDrive C).Free / 1GB, 1)
Save
Log "fim"
