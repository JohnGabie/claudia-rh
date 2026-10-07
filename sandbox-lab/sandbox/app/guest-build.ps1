# guest-build.ps1 - roda DENTRO do Sandbox (PowerShell 5.1). Perfil: lab.config.app-build.json.
# Build do projeto dentro do guest, usando toolchains do host montados read-only (os que existirem):
#   C:\hosttools\nodejs  C:\hosttools\rust (toolchain msvc)  C:\hosttools\msvc (<versao>\...)  C:\hosttools\winkits
# Fonte: C:\labproject (RO, projectPath do perfil) -> copiada para C:\labdata\project.
# Saida: C:\labwork\build\ (pasta gravavel mapeada pelo perfil): guest-build.log, result.json.
# O target do cargo fica no disco do guest (C:\labdata\cargo-target): em pasta mapeada os build scripts falham (os error 1392).
# Uso (host): winapp target exec sandbox -- powershell -ExecutionPolicy Bypass -File C:\labsandbox\app\guest-build.ps1 `
#               [-Install 'npm.cmd ci'] [-Build 'npm.cmd run build'] [-Artifacts 'C:\labdata\cargo-target\release\*.exe']
param(
  [string]$Install = 'npm.cmd ci --no-audit --no-fund',
  [string]$Build = 'npm.cmd run build',
  [string]$Artifacts = 'C:\labdata\cargo-target\release\*.exe'
)
$ErrorActionPreference = 'Continue'
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

# --- ambiente: toolchains do host (sem instalar nada no guest) ---
$paths = @()
if (Test-Path 'C:\hosttools\nodejs') { $paths += 'C:\hosttools\nodejs' }
if (Test-Path 'C:\hosttools\rust\bin') { $paths += 'C:\hosttools\rust\bin' }
$msvcVer = Get-ChildItem 'C:\hosttools\msvc' -Directory -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -Last 1
$Kits = 'C:\hosttools\winkits'
$sdk = (Get-ChildItem "$Kits\Include" -Directory -ErrorAction SilentlyContinue | Where-Object Name -like '10.*' | Sort-Object Name | Select-Object -Last 1).Name
if ($msvcVer -and $sdk) {
  $Msvc = $msvcVer.FullName
  $paths += "$Msvc\bin\Hostx64\x64", "$Kits\bin\$sdk\x64"
  $env:INCLUDE = "$Msvc\include;$Kits\Include\$sdk\ucrt;$Kits\Include\$sdk\um;$Kits\Include\$sdk\shared;$Kits\Include\$sdk\winrt;$Kits\Include\$sdk\cppwinrt"
  $env:LIB = "$Msvc\lib\x64;$Kits\Lib\$sdk\ucrt\x64;$Kits\Lib\$sdk\um\x64"
  $env:VCINSTALLDIR = 'C:\hosttools\'   # cc-rs/rustc: usa o ambiente (PATH/INCLUDE/LIB) em vez de procurar o VS no registro
  $env:VSCMD_ARG_TGT_ARCH = 'x64'
  $env:WindowsSdkDir = "$Kits\"; $env:WindowsSDKVersion = "$sdk\"
}
$env:PATH = (($paths + $env:PATH) -join ';')
$env:CARGO_HOME = 'C:\labdata\cargo-home'
$env:CARGO_TARGET_DIR = 'C:\labdata\cargo-target'
$env:npm_config_cache = 'C:\labdata\npm-cache'
$res.env = [ordered]@{ msvc = $(if ($msvcVer) { $msvcVer.Name }); sdk = $sdk; cargoTarget = $env:CARGO_TARGET_DIR }
Log "node=$(cmd /c node -v 2>&1) cargo=$(cmd /c cargo -V 2>&1) msvc=$($res.env.msvc) sdk=$sdk"
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
