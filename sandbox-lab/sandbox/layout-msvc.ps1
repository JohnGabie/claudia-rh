# layout-msvc.ps1 - runs INSIDE the Sandbox as System (wsb exec -r System), ambiente cache-msvc (spec addendum 8.2).
# Builds the offline layout of the VS Build Tools on the guest disk, lists every file with size and SHA-256
# (layout.sha256), and copies it into a NEW folder C:\labcachesaida\<id>\msvc-<version> (= <repo>\.sandbox-work\cache-saida
# on the host). The host checks and imports it with cache-up.ps1 -ImportarMsvc <id>. Output: C:\lab\out\layout-msvc.json
$ErrorActionPreference = 'Continue'   # PS 5.1: native stderr must not become a terminating error
. (Join-Path $PSScriptRoot 'guest-install.lib.ps1')
# Refusal outside the guest: print and leave, BEFORE anything else runs. Nothing is written to the disk (no Save-Result, no layout-msvc.json).
if (-not (Test-LabInGuest)) { Write-Host 'not inside the lab guest (needs the guest identity and the bootstrap marker)'; exit 1 }
$OutFile = 'C:\lab\out\layout-msvc.json'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$sw = [Diagnostics.Stopwatch]::StartNew()
$res = [ordered]@{ ok = $false; id = $null; versao = $null; arquivos = 0; bytes = 0; bootstrapperExit = $null; vsLayoutExit = $null; robocopy = [ordered]@{}; segundos = $null; erro = $null }
function Save-Result { $res.segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1); [IO.File]::WriteAllText($OutFile, ($res | ConvertTo-Json), $utf8) }
function Stop-With([string]$Why) { $res.erro = $Why; Save-Result; exit 1 }
# Every robocopy goes through here: its exit code is kept in $res.robocopy (latest per name); 8 or more means a copy failed.
function Copy-LabTree([string]$Name, [string]$From, [string]$To, [int]$Retries) {
  robocopy $From $To /E /XJ /NFL /NDL /NP /R:$Retries /W:$Retries | Out-Null   # new folder: /E only, never a mirror
  $res.robocopy[$Name] = $LASTEXITCODE
  return $res.robocopy[$Name]
}
$stageLimitMinutes = 15   # both stages: the bootstrapper (1) and vs_layout.exe (2)

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
# Why two stages (verified live 2026-10-07/08): in the Sandbox, 'vs_buildtools --layout' starts setup.exe, which starts
# vs_layout.exe with a console handle that does not work; vs_layout dies in Console.WriteLine (IOException, exit
# -532462766) right after loading the catalog. Redirecting the bootstrapper's output, a scheduled task in the user's
# desktop session and dropping --quiet all failed the same way. vs_layout.exe run DIRECTLY with stdout/stderr in files
# builds the whole layout.
# Stage 1: the bootstrapper, only to get what it extracts and downloads into the temp folder (vs_layout.exe with its
# installer files, the catalog vs_setup_bootstrapper.json and vs_installer.opc). They are COPIED into a new folder
# while it runs (it removes its temp folders when it fails). Nothing is deleted here.
$keep = 'C:\labdata\vs-boot'
if (Test-Path -LiteralPath $keep) { Stop-With "$keep already exists: run on a fresh guest" }
New-Item -ItemType Directory -Path "$keep\json", "$keep\opc", "$keep\inst" | Out-Null
$temps = @('C:\Windows\SystemTemp', [IO.Path]::GetTempPath().TrimEnd('\')) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -Unique
$before = @($temps | ForEach-Object { Get-ChildItem -LiteralPath $_ -Directory -Force -ErrorAction SilentlyContinue } | ForEach-Object FullName)
$argv = @('--layout', $work, '--lang', 'en-US', '--quiet', '--wait') + @($m.componentes | ForEach-Object { '--add'; $_ })
$p = Start-Process -FilePath $boot -ArgumentList $argv -PassThru -RedirectStandardOutput 'C:\lab\out\layout-msvc.boot.stdout.txt' -RedirectStandardError 'C:\lab\out\layout-msvc.boot.stderr.txt' -NoNewWindow
$null = $p.Handle   # keeps the exit code readable after the process ends
$got = @{}
$until = (Get-Date).AddMinutes($stageLimitMinutes)
while (-not $p.HasExited -and (Get-Date) -lt $until) {
  Start-Sleep -Milliseconds 300
  foreach ($d in @($temps | ForEach-Object { Get-ChildItem -LiteralPath $_ -Directory -Force -ErrorAction SilentlyContinue } | Where-Object { $before -notcontains $_.FullName })) {
    if (-not $got.json -and (Test-Path -LiteralPath (Join-Path $d.FullName 'vs_bootstrapper_d15\vs_setup_bootstrapper.json'))) {
      Start-Sleep -Milliseconds 500; if ((Copy-LabTree 'stage1-json' $d.FullName "$keep\json" 0) -lt 8) { $got.json = $true }   # on failure the next tick copies again
    }
    if (-not $got.opc -and (Test-Path -LiteralPath (Join-Path $d.FullName 'vs_installer.opc'))) {
      Start-Sleep -Milliseconds 800; [IO.File]::Copy((Join-Path $d.FullName 'vs_installer.opc'), "$keep\opc\vs_installer.opc", $false); $got.opc = $true
    }
    if (-not $got.inst -and (Test-Path -LiteralPath (Join-Path $d.FullName 'vs_layout.exe'))) {
      Start-Sleep -Milliseconds 300; if ((Copy-LabTree 'stage1-inst' $d.FullName "$keep\inst" 0) -lt 8) { $got.inst = $true }
    }
  }
}
if (-not $p.HasExited) { Stop-With "the bootstrapper did not finish in $stageLimitMinutes minutes" }
$res.bootstrapperExit = $p.ExitCode
foreach ($k in @($res.robocopy.Keys)) { if ($res.robocopy[$k] -ge 8) { Stop-With "robocopy $k exit $($res.robocopy[$k]) (copy of what the bootstrapper extracts failed)" } }
$json = Get-ChildItem -LiteralPath "$keep\json" -Recurse -File -Filter 'vs_setup_bootstrapper.json' | Select-Object -First 1
$opc = "$keep\opc\vs_installer.opc"
$vsLayout = "$keep\inst\vs_layout.exe"
if (-not $json -or -not (Test-Path -LiteralPath $opc) -or -not (Test-Path -LiteralPath $vsLayout)) {
  Stop-With "stage 1 did not leave vs_layout.exe, vs_setup_bootstrapper.json and vs_installer.opc (bootstrapper exit $($p.ExitCode))"
}
# Stage 2: vs_layout.exe directly, same arguments the bootstrapper passes, output in files next to layout-msvc.json.
$stdout = 'C:\lab\out\layout-msvc.stdout.txt'
$stderr = 'C:\lab\out\layout-msvc.stderr.txt'
$la = @('layout', '--bootstrapperPath', "`"$boot`"", '--layout', "`"$work`"", '--in', "`"$($json.FullName)`"", '--lang', 'en-US', '--quiet') +
  @($m.componentes | ForEach-Object { '--add'; $_ }) + @('--locale', 'en-US', '--InstallerPath', "`"$opc`"")
$q = Start-Process -FilePath $vsLayout -ArgumentList ($la -join ' ') -PassThru -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$null = $q.Handle   # keeps the exit code readable after the process ends
if (-not $q.WaitForExit($stageLimitMinutes * 60 * 1000)) {
  # Kill only the process this script started (never by name, never a tree); the guest is disposable.
  try { $q.Kill() } catch { }
  Stop-With "vs_layout.exe did not finish in $stageLimitMinutes minutes (killed; see layout-msvc.stdout.txt)"
}
$res.vsLayoutExit = $q.ExitCode
$why = Get-LabVsLayoutRefusal -ExitCode $q.ExitCode -StdOut ([IO.File]::ReadAllText($stdout)) -StdErr ([IO.File]::ReadAllText($stderr)) -LayoutFiles @(Get-ChildItem -LiteralPath $work -File -Force -Name)
if ($why) { Stop-With "$why (see layout-msvc.stdout.txt and layout-msvc.stderr.txt)" }
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
$rc = Copy-LabTree 'final' $work "$dest\layout" 1
if ($rc -ge 8) { Stop-With "robocopy final exit $rc" }
[IO.File]::WriteAllLines("$dest\layout.sha256", $lines.ToArray())
$res.id = $id
$res.ok = $true
Save-Result
exit 0
