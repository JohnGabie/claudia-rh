<#
.SYNOPSIS
  Cache de instaladores do guest em sandbox-lab\shared\cache\ (fora do git; spec A2 e adendo 8.2). So cria: nunca
  apaga, nunca sobrescreve. Um arquivo existente com outro hash e um conflito: nada e baixado.
  Sem parametros: baixa Rust, Node, Claude Code e o bootstrapper do MSVC. O bootstrapper NUNCA roda no PC: o layout do
  MSVC e montado num Sandbox (test-up -Ambiente cache-msvc + sandbox\layout-msvc.ps1 como System) e trazido com
  -ImportarMsvc, que confere SHA-256 e tamanho de cada arquivo no PC antes e depois de copiar.
.PARAMETER SoPlano       Mostra o plano e sai sem baixar nada.
.PARAMETER ImportarMsvc  Id (8 hex) gravado pelo layout-msvc.ps1 em shared\out\layout-msvc.json.
#>
[CmdletBinding()]
param([switch]$SoPlano, [string]$ImportarMsvc)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\compose.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
$Lab = Split-Path $PSScriptRoot -Parent
$RepoRoot = Split-Path $Lab -Parent
$Cache = Join-Path $Lab 'shared\cache'
$refusal = Get-LabCacheDirRefusal -CacheDir $Cache -LabDir $Lab -Links @(Get-LabLinksInPath $Cache $Lab)
if ($refusal) { throw "[cache-up] refusing to write in ${Cache}: $refusal" }
$compose = Get-Content -LiteralPath (Join-Path $Lab 'compose.json') -Raw | ConvertFrom-Json
$msvc = $compose.pacotes.PSObject.Properties['msvc'].Value
$utf8 = New-Object System.Text.UTF8Encoding($false)
$log = [ordered]@{ inicio = (Get-Date).ToString('o'); acao = $(if ($ImportarMsvc) { 'importar-msvc' } else { 'baixar' }); itens = @() }
function Save-RunLog {
  [IO.Directory]::CreateDirectory($Cache) | Out-Null   # creates only; -SoPlano never reaches this
  $log.fim = (Get-Date).ToString('o')
  [IO.File]::WriteAllText((Join-Path $Cache ('cache-up-{0}.json' -f (Get-Date -Format 'yyyyMMdd-HHmmss'))), ($log | ConvertTo-Json -Depth 4), $utf8)
}
function Get-LabShortId { return [guid]::NewGuid().ToString('N').Substring(0, 8) }
function Get-LabFileStamp([string]$File) { return "$((Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash.ToLowerInvariant()) $((Get-Item -LiteralPath $File).Length)" }
function Get-LabTreeStamps([string]$Dir) {
  # relative path with '/' -> "<sha256> <bytes>" for every file under $Dir (links were refused before)
  $base = (Get-Item -LiteralPath $Dir).FullName.TrimEnd('\') + '\'
  $found = @{}
  foreach ($f in Get-ChildItem -LiteralPath $Dir -Recurse -File -Force) { $found[$f.FullName.Substring($base.Length).Replace('\', '/')] = Get-LabFileStamp $f.FullName }
  return $found
}

if ($ImportarMsvc) {
  # --- import the layout built in the guest (adendo 8.2) ---
  if ($ImportarMsvc -notmatch '^[0-9a-f]{8}$') { throw '[cache-up] -ImportarMsvc takes the 8-hex id from shared\out\layout-msvc.json' }
  $why = Get-LabPackageRefusal -Nome 'msvc' -Pacote $msvc
  if ($why) { throw "[cache-up] compose.json: $why" }
  $src = Join-Path $RepoRoot ".sandbox-work\cache-saida\$ImportarMsvc\msvc-$($msvc.versao)"
  $links = @(Get-LabLinksInPath $src $RepoRoot) + @(Get-LabReparsePoints $src)
  $why = Get-LabLayoutImportRefusal -Source $src -RepoRoot $RepoRoot -Versao $msvc.versao -Links $links
  if ($why) { throw "[cache-up] refusing to import ${src}: $why" }
  if (-not (Test-Path -LiteralPath (Join-Path $src 'layout.sha256') -PathType Leaf)) { throw "[cache-up] $src has no layout.sha256: run layout-msvc.ps1 again" }
  $final = Join-Path $Cache $msvc.arquivo
  if (Test-Path -LiteralPath $final) { throw "[cache-up] $($msvc.arquivo) is already in the cache: nothing imported (nothing is ever overwritten)" }
  $manifest = Get-Content -LiteralPath (Join-Path $src 'layout.sha256') -Raw
  Write-Host '[cache-up] conferindo o layout vindo do guest (SHA-256 de cada arquivo)...'
  $problems = @(Get-LabLayoutManifestProblems -ManifestText $manifest -Found (Get-LabTreeStamps (Join-Path $src 'layout')))
  if ($problems.Count) { throw "[cache-up] the layout in cache-saida does not match its layout.sha256 ($($problems.Count)): $(($problems | Select-Object -First 5) -join '; '). Nothing copied." }
  $tmp = "$final.$(Get-LabShortId).tmp"
  [IO.Directory]::CreateDirectory($tmp) | Out-Null
  $srcLayout = (Get-Item -LiteralPath (Join-Path $src 'layout')).FullName.TrimEnd('\') + '\'
  foreach ($f in Get-ChildItem -LiteralPath $srcLayout -Recurse -File -Force) {
    $dst = Join-Path $tmp ('layout\' + $f.FullName.Substring($srcLayout.Length))
    [IO.Directory]::CreateDirectory((Split-Path $dst -Parent)) | Out-Null
    [IO.File]::Copy($f.FullName, $dst, $false)
  }
  [IO.File]::Copy((Join-Path $src 'layout.sha256'), (Join-Path $tmp 'layout.sha256'), $false)
  $problems = @(Get-LabLayoutManifestProblems -ManifestText $manifest -Found (Get-LabTreeStamps (Join-Path $tmp 'layout')))
  if ($problems.Count) { throw "[cache-up] the copy differs from layout.sha256 ($($problems.Count)); partial folder kept as $(Split-Path $tmp -Leaf), nothing renamed" }
  [IO.Directory]::Move($tmp, $final)   # throws if $final exists
  $log.itens += [ordered]@{ nome = 'msvc'; versao = $msvc.versao; id = $ImportarMsvc }
  Save-RunLog
  Write-Host "[cache-up] layout do MSVC $($msvc.versao) importado para shared\cache\$($msvc.arquivo)." -ForegroundColor Green
  exit 0
}

# --- downloads (Rust, Node, Claude Code, MSVC bootstrapper); the MSVC layout folder is only reported ---
$pkgs = @()
foreach ($nome in $script:LabInstallOrder) {
  $prop = $compose.pacotes.PSObject.Properties[$nome]
  if (-not $prop) { continue }
  $p = $prop.Value
  $why = Get-LabPackageRefusal -Nome $nome -Pacote $p
  if ($why) { throw "[cache-up] compose.json: $why" }
  if ($p.tipo -eq 'vs-layout') {
    $pkgs += [pscustomobject]@{ nome = 'msvc-bootstrapper'; arquivo = $p.bootstrapper.arquivo; sha256 = $p.bootstrapper.sha256; url = $p.bootstrapper.url }
    $pkgs += [pscustomobject]@{ nome = 'msvc'; arquivo = $p.arquivo; sha256 = ''; url = $null }
  } else {
    $pkgs += [pscustomobject]@{ nome = $nome; arquivo = $p.arquivo; sha256 = $p.sha256; url = $p.url }
  }
}
$existentes = @{}
foreach ($k in $pkgs) {
  $path = Join-Path $Cache $k.arquivo
  if (Test-Path -LiteralPath $path -PathType Container) { $existentes[$k.arquivo] = 'dir' }
  elseif (Test-Path -LiteralPath $path -PathType Leaf) { $existentes[$k.arquivo] = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
}
$plan = @(Get-LabCachePlan -Pacotes $pkgs -Existentes $existentes)
foreach ($p in $plan) { Write-Host ("[cache-up] {0,-18} {1,-9} {2}" -f $p.nome, $p.acao, $p.motivo) }
$conflicts = @($plan | Where-Object { $_.acao -eq 'conflito' })
if ($conflicts.Count) { throw "[cache-up] $($conflicts.Count) conflict(s): nothing was downloaded and the existing files were kept. Decide by hand what to do with them." }
if ($SoPlano) { exit 0 }

function Save-LabDownload([string]$Url, [string]$Final, [string]$Sha) {
  # curl.exe ships with Windows 10+. New .tmp next to the final name; rename only after the hash matches.
  $tmp = "$Final.$(Get-LabShortId).tmp"
  & curl.exe -fL --retry 3 --silent --show-error -o $tmp $Url
  if ($LASTEXITCODE -ne 0) { throw "download failed ($LASTEXITCODE): $Url (partial file kept as $(Split-Path $tmp -Leaf))" }
  $got = (Get-FileHash -LiteralPath $tmp -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($got -ne $Sha) { throw "sha256 differs for ${Url}: expected $Sha, got $got (file kept as $(Split-Path $tmp -Leaf); nothing renamed)" }
  [IO.File]::Move($tmp, $Final)   # throws if $Final exists: never overwrites
}
[IO.Directory]::CreateDirectory($Cache) | Out-Null
foreach ($item in @($plan | Where-Object { $_.acao -eq 'baixar' -and $_.nome -ne 'msvc' })) {
  $k = $pkgs | Where-Object { $_.nome -eq $item.nome }
  $sw = [Diagnostics.Stopwatch]::StartNew()
  Write-Host "[cache-up] baixando $($k.nome)..."
  Save-LabDownload -Url $k.url -Final (Join-Path $Cache $k.arquivo) -Sha $k.sha256
  $log.itens += [ordered]@{ nome = $k.nome; arquivo = $k.arquivo; segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1) }
}
Save-RunLog
if (@($plan | Where-Object { $_.nome -eq 'msvc' -and $_.acao -eq 'baixar' }).Count) {
  Write-Host '[cache-up] falta o layout do MSVC. Ele e montado num Sandbox (o PC nao executa o Visual Studio):' -ForegroundColor Yellow
  Write-Host '  1) scripts\test-up.ps1 -Ambiente cache-msvc -Project <projeto>   2) wsb exec --id <id> -r System -c "powershell -NoProfile -ExecutionPolicy Bypass -File C:\labsandbox\layout-msvc.ps1"'
  Write-Host '  3) ler o id em shared\out\layout-msvc.json   4) scripts\test-down.ps1   5) scripts\cache-up.ps1 -ImportarMsvc <id>'
}
exit 0
