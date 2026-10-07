# layout-msvc.ps1 - runs INSIDE the Sandbox as System (wsb exec -r System), ambiente cache-msvc (spec addendum 8.2).
# Builds the offline layout of the VS Build Tools on the guest disk, lists every file with size and SHA-256
# (layout.sha256), and copies it into a NEW folder C:\labcachesaida\<id>\msvc-<version> (= <repo>\.sandbox-work\cache-saida
# on the host). The host checks and imports it with cache-up.ps1 -ImportarMsvc <id>. Output: C:\lab\out\layout-msvc.json
$ErrorActionPreference = 'Continue'   # PS 5.1: native stderr must not become a terminating error
. (Join-Path $PSScriptRoot 'guest-install.lib.ps1')
$OutFile = 'C:\lab\out\layout-msvc.json'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$sw = [Diagnostics.Stopwatch]::StartNew()
$res = [ordered]@{ ok = $false; id = $null; versao = $null; arquivos = 0; bytes = 0; segundos = $null; erro = $null }
function Save-Result { $res.segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1); [IO.File]::WriteAllText($OutFile, ($res | ConvertTo-Json), $utf8) }
function Stop-With([string]$Why) { $res.erro = $Why; Save-Result; exit 1 }

if (-not (Test-LabInGuest)) { Stop-With 'not inside the lab guest (needs the guest identity and the bootstrap marker)' }
$cfg = Get-Content -LiteralPath 'C:\labsandbox\guest.config.json' -Raw | ConvertFrom-Json
$m = $cfg.msvc
if (-not $m) { Stop-With 'guest.config.json has no msvc entry: start the lab with -Ambiente cache-msvc' }
if (-not (Test-Path -LiteralPath 'C:\labcachesaida' -PathType Container)) { Stop-With 'C:\labcachesaida is not mounted' }
$res.versao = $m.versao
Save-Result
if ((Get-FileHash -LiteralPath $m.bootstrapper -Algorithm SHA256).Hash -ine $m.sha256) { Stop-With 'bootstrapper sha256 differs from compose.json' }
$work = 'C:\labdata\msvc-layout'   # short path on the guest disk (Microsoft asks for less than 80 characters)
if (Test-Path -LiteralPath $work) { Stop-With "$work already exists: run on a fresh guest" }
New-Item -ItemType Directory -Force 'C:\labdata' | Out-Null
$boot = 'C:\labdata\vs_buildtools.exe'
[IO.File]::Copy($m.bootstrapper, $boot, $false)
$argv = @('--layout', $work, '--lang', 'en-US', '--quiet', '--wait') + @($m.componentes | ForEach-Object { '--add'; $_ })
$p = Start-Process -FilePath $boot -ArgumentList $argv -Wait -PassThru
if ($p.ExitCode -ne 0) { Stop-With "vs_buildtools --layout exit $($p.ExitCode)" }
# '<sha256> <bytes> <relative/path>' per file, same shape as shared/tools.sha256.
$base = (Get-Item -LiteralPath $work).FullName.TrimEnd('\') + '\'
$lines = New-Object System.Collections.Generic.List[string]
foreach ($f in Get-ChildItem -LiteralPath $work -Recurse -File -Force) {
  $lines.Add(('{0} {1} {2}' -f (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $f.Length, $f.FullName.Substring($base.Length).Replace('\', '/')))
  $res.arquivos++; $res.bytes += $f.Length
}
$id = [guid]::NewGuid().ToString('N').Substring(0, 8)
$dest = "C:\labcachesaida\$id\msvc-$($m.versao)"
New-Item -ItemType Directory -Path "$dest\layout" | Out-Null
robocopy $work "$dest\layout" /E /XJ /NFL /NDL /NP /R:1 /W:1 | Out-Null   # new folder: /E only, never a mirror
if ($LASTEXITCODE -ge 8) { Stop-With "robocopy exit $LASTEXITCODE" }
[IO.File]::WriteAllLines("$dest\layout.sha256", $lines.ToArray())
$res.id = $id
$res.ok = $true
Save-Result
exit 0
