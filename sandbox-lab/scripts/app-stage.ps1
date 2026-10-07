<#
.SYNOPSIS
  Publica o app em %LAB_WORK_DIR%\deploy (mapeado read-only no guest em C:\appdeploy pelos perfis lab.config.app*.json):
    app\           <- exe (ou pasta inteira do build)
    appdata\       <- pastas copiadas para %APPDATA% do guest (uma subpasta por pasta informada)
    localappdata\  <- pastas copiadas para %LOCALAPPDATA% do guest
  Copia apenas; nunca grava nas origens. Os dados publicados devem ser copias ja preparadas para teste
  (sem automacoes que gerem efeito externo ligadas).
.PARAMETER App           exe ou pasta do build
.PARAMETER AppData       pastas a publicar em appdata\ (o nome da pasta e mantido)
.PARAMETER LocalAppData  pastas a publicar em localappdata\
.PARAMETER WorkDir       padrao: $env:LAB_WORK_DIR
.PARAMETER Clean         apaga o deploy anterior antes de publicar
#>
param(
  [Parameter(Mandatory)][string]$App,
  [string[]]$AppData = @(),
  [string[]]$LocalAppData = @(),
  [string]$WorkDir = $env:LAB_WORK_DIR,
  [switch]$Clean
)
$ErrorActionPreference = 'Stop'
function Copy-Tree([string]$from, [string]$to) {
  robocopy $from $to /MIR /NFL /NDL /NP /R:1 /W:1 | Out-Null
  if ($LASTEXITCODE -ge 8) { throw "robocopy falhou ($LASTEXITCODE): $from -> $to" }
}
if (-not $WorkDir) { throw 'Defina LAB_WORK_DIR ou passe -WorkDir.' }
$Deploy = Join-Path ([IO.Path]::GetFullPath($WorkDir)) 'deploy'
if ($Clean -and (Test-Path $Deploy)) { Remove-Item $Deploy -Recurse -Force }
New-Item -ItemType Directory -Force "$Deploy\app", "$Deploy\appdata", "$Deploy\localappdata", (Join-Path $WorkDir 'guest-rw') | Out-Null

if (Test-Path $App -PathType Container) {
  Copy-Tree $App "$Deploy\app"
} elseif (Test-Path $App -PathType Leaf) {
  Copy-Item $App "$Deploy\app\" -Force
} else { throw "App nao encontrado: $App" }

foreach ($pair in @(@($AppData, 'appdata'), @($LocalAppData, 'localappdata'))) {
  foreach ($src in @($pair[0])) {
    if (-not $src) { continue }
    if (-not (Test-Path $src -PathType Container)) { throw "Pasta nao encontrada: $src" }
    Copy-Tree $src (Join-Path "$Deploy\$($pair[1])" (Split-Path $src -Leaf))
  }
}
Get-ChildItem $Deploy -Recurse -File | Measure-Object Length -Sum | ForEach-Object { "[app-stage] {0} arquivos, {1:N1} MB em {2}" -f $_.Count, ($_.Sum / 1MB), $Deploy }
exit 0
