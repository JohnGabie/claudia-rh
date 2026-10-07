<#
.SYNOPSIS
  Publica o app em <raiz do repo>\.sandbox-work\deploy (mapeado read-only no guest em C:\appdeploy pelos perfis lab.config.app*.json):
    app\           <- exe (ou pasta inteira do build)
    appdata\       <- pastas copiadas para %APPDATA% do guest (uma subpasta por pasta informada)
    localappdata\  <- pastas copiadas para %LOCALAPPDATA% do guest
  Copia apenas; nunca grava nas origens. Os dados publicados devem ser copias ja preparadas para teste
  (sem automacoes que gerem efeito externo ligadas).
.PARAMETER App           exe ou pasta do build
.PARAMETER AppData       pastas a publicar em appdata\ (o nome da pasta e mantido)
.PARAMETER LocalAppData  pastas a publicar em localappdata\
.PARAMETER WorkDir       optional; must be exactly <repo root>\.sandbox-work (the default)
.PARAMETER Clean         wipes the previous deploy before publishing (only a deploy carrying the .lab-deploy marker)
#>
param(
  [Parameter(Mandatory)][string]$App,
  [string[]]$AppData = @(),
  [string[]]$LocalAppData = @(),
  [string]$WorkDir,
  [switch]$Clean
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
function Copy-Tree([string]$from, [string]$to) {
  # /XJ: never follow junctions inside the source (they could lead /MIR copying anywhere).
  robocopy $from $to /MIR /XJ /NFL /NDL /NP /R:1 /W:1 | Out-Null
  if ($LASTEXITCODE -ge 8) { throw "robocopy falhou ($LASTEXITCODE): $from -> $to" }
}
# The lab always lives in <repo root>\sandbox-lab, so the repo root is two levels above this script.
$RepoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$Expected = Join-Path $RepoRoot '.sandbox-work'
if ($WorkDir -and ((ConvertTo-LabPathKey ([IO.Path]::GetFullPath($WorkDir))) -ne (ConvertTo-LabPathKey $Expected))) {
  throw "WorkDir must be exactly $Expected (got $WorkDir)."
}
$WorkDir = $Expected
$Deploy = Join-Path $WorkDir 'deploy'
# robocopy /MIR below also deletes inside deploy, so the guard runs with or without -Clean.
$exists = Test-Path -LiteralPath $Deploy
$links = @(Get-LabLinksInPath $Deploy $RepoRoot) + @(Get-LabReparsePoints $Deploy)
$refusal = Get-LabCleanRefusal -Deploy $Deploy -RepoRoot $RepoRoot -Exists $exists `
  -HasMarker ($exists -and (Test-Path -LiteralPath (Join-Path $Deploy $script:LabDeployMarker))) -Links $links
if ($refusal) { throw "Refusing to touch ${Deploy}: $refusal" }
# Every source is checked before anything is deleted or copied.
foreach ($src in (@($App) + @($AppData) + @($LocalAppData))) {
  if (-not $src) { continue }
  $srcRefusal = Get-LabSourceRefusal -Source ([IO.Path]::GetFullPath($src)) -Deploy $Deploy
  if ($srcRefusal) { throw "Refusing source ${src}: $srcRefusal" }
}
if ($Clean -and $exists) { Remove-Item -LiteralPath $Deploy -Recurse -Force }
New-Item -ItemType Directory -Force "$Deploy\app", "$Deploy\appdata", "$Deploy\localappdata", (Join-Path $WorkDir 'guest-rw') | Out-Null
[IO.File]::WriteAllText((Join-Path $Deploy $script:LabDeployMarker), '')

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
