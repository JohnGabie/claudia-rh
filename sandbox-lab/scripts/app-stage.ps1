<#
.SYNOPSIS
  Publica so o app em <raiz do repo>\.sandbox-work\deploy (mapeado read-only no guest em C:\appdeploy):
    app\           <- exe (ou pasta inteira do build)
  App data never goes through here (D1): it is created in the guest and installed by guest-run.ps1.
  Copia apenas; nunca grava nas origens.
.PARAMETER App           exe ou pasta do build
.PARAMETER WorkDir       optional; must be exactly <repo root>\.sandbox-work (the default)
.PARAMETER Clean         wipes the previous deploy before publishing (only a deploy carrying the .lab-deploy marker)
#>
param(
  [Parameter(Mandatory)][string]$App,
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
# Every source is normalized ONCE (explicit base = repo root, never the CWD) and resolved to its real path; that same
# value is what the guard checks and what Test-Path / Copy-Tree / Copy-Item use below. Links anywhere in the path refuse.
function Resolve-Source([string]$src) {
  $full = Resolve-LabRealPath (ConvertTo-LabFullPath $src $RepoRoot)
  $srcRefusal = Get-LabSourceRefusal -Source $full -Deploy $Deploy -Links (Get-LabLinksAboveRoot $full)
  if ($srcRefusal) { throw "Refusing source ${src}: $srcRefusal" }
  return $full
}
# Every source is checked before anything is deleted or copied.
$App = Resolve-Source $App
if ($Clean -and $exists) { Remove-Item -LiteralPath $Deploy -Recurse -Force }
New-Item -ItemType Directory -Force "$Deploy\app", (Join-Path $WorkDir 'guest-rw') | Out-Null
[IO.File]::WriteAllText((Join-Path $Deploy $script:LabDeployMarker), '')

if (Test-Path -LiteralPath $App -PathType Container) {
  Copy-Tree $App "$Deploy\app"
} elseif (Test-Path -LiteralPath $App -PathType Leaf) {
  Copy-Item -LiteralPath $App "$Deploy\app\" -Force
} else { throw "App nao encontrado: $App" }

Get-ChildItem $Deploy -Recurse -File | Measure-Object Length -Sum | ForEach-Object { "[app-stage] {0} arquivos, {1:N1} MB em {2}" -f $_.Count, ($_.Sum / 1MB), $Deploy }
exit 0
