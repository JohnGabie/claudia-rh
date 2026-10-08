# guest-build.ps1 - runs INSIDE the Sandbox (PowerShell 5.1). Environment "testes" of compose.json.
# Builds the project inside the guest with the PC toolchains mounted read-only at C:\hosttools (ambiente testes) or with what
# instalar.ps1 installed (C:\lab\out\instalar.json, ambiente testes-limpo): MSVC, Rust, Node.
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
  # build (MSVC, Rust and Node ok), 'todos' = every package of the ambiente.
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

# --- environment: the PC toolchains mounted read-only at C:\hosttools (ambiente 'testes', guest.config.json hostTools) and/or
# what instalar.ps1 installed in this guest (C:\lab\out\instalar.json, ambiente 'testes-limpo') ---
$inst = $null
try { $inst = Get-Content -LiteralPath 'C:\lab\out\instalar.json' -Raw | ConvertFrom-Json } catch { }
$hostLayout = Get-LabHostToolsLayout
$toolEnv = Get-LabGuestToolEnv $inst -Precisa $Precisa -HostLayout $hostLayout
if (-not $toolEnv) {
  Log "the tools '$Precisa' needs are neither mounted from the PC (hostTools in guest.config.json, folders in C:\hosttools) nor installed ok (instalar.json missing or unfinished): start the lab with -Ambiente testes (or testes-limpo) and wait for test-up to finish"
  # The host waits for a "build" step carrying its nonce: report the failure there instead of letting it time out.
  $res.steps['build'] = [ordered]@{ cmd = $Build; exit = 1; seconds = 0 }
  $res.end = (Get-Date).ToString('o'); Save
  exit 1
}
if ($toolEnv.vcvars) {
  # Import the MSVC environment (PATH, INCLUDE, LIB) the way a developer prompt does.
  foreach ($line in (cmd.exe /d /c "`"$($toolEnv.vcvars)`" >nul 2>&1 && set")) { if ($line -match '^([^=]+)=(.*)$') { Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2] } }
}
foreach ($k in @($toolEnv.env.Keys)) { Set-Item -Path "Env:$k" -Value ([string]$toolEnv.env[$k]) }   # INCLUDE, LIB, ... from the host MSVC and SDK
$env:PATH = ((@($toolEnv.paths) + $env:PATH) -join ';')
$env:DISABLE_AUTOUPDATER = '1'
# Cargo's home and target stay on the guest disk, never inside a mounted folder (the host tools are read-only; in a mapped
# folder the build scripts fail with os error 1392).
$env:CARGO_HOME = 'C:\labdata\cargo-home'
$env:CARGO_TARGET_DIR = 'C:\labdata\cargo-target'
$env:npm_config_cache = 'C:\labdata\npm-cache'
$res.env = [ordered]@{ pacotes = @(@($inst.pacotes) | Where-Object { $_ } | ForEach-Object { "$($_.nome) $($_.versao)" }); hostTools = @($toolEnv.hostTools); cargoTarget = $env:CARGO_TARGET_DIR }
Log "node=$(cmd /c node -v 2>&1) cargo=$(cmd /c cargo -V 2>&1) cl=$(@(cmd /c where cl.exe 2>&1)[0])"
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
