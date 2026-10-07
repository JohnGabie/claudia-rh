# instalar.ps1 - runs INSIDE the Sandbox as an administrator (started by bootstrap.ps1 when the logon is elevated,
# otherwise by test-up.ps1 through 'wsb exec -r System'). Installs, from the read-only cache C:\lab\cache, the packages
# guest.config.json lists (already in the order msvc, rust, node, claude-code; spec A3). No network needed.
# Output: C:\lab\out\instalar.json (+ C:\lab\out\instalar-<name>.log). Tools go to C:\labtools (guest disk).
# 'Continue' on purpose: in PowerShell 5.1 with 'Stop', any stderr line of a native exe (rustc, cmd, claude) becomes a
# terminating error. Failures are decided by exit codes and explicit throws below.
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot 'guest-install.lib.ps1')
$OutFile = 'C:\lab\out\instalar.json'
$Tools = 'C:\labtools'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$elevated = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$res = [ordered]@{ inicio = (Get-Date).ToString('o'); fim = $null; ok = $false; elevado = $elevated; erro = $null; pacotes = @() }
function Save-Result { [IO.File]::WriteAllText($OutFile, ($res | ConvertTo-Json -Depth 5), $utf8) }
function Stop-With([string]$Why) { $res.erro = $Why; $res.fim = (Get-Date).ToString('o'); Save-Result; exit 1 }

if (-not (Test-LabInGuest)) { Stop-With 'not inside the lab guest (needs the guest identity and the bootstrap marker)' }
if (-not $elevated) { Stop-With 'not elevated' }
$cfg = Get-Content -LiteralPath 'C:\labsandbox\guest.config.json' -Raw | ConvertFrom-Json
$wanted = @(@($cfg.instalar) | Where-Object { $_ })
Save-Result

function Test-Sha([string]$File, [string]$Sha) { return ((Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash -ieq $Sha) }
function Invoke-Package($p) {
  $r = [ordered]@{ nome = $p.nome; versao = $p.versao; exit = $null; segundos = $null; binDir = $null; vcvars = $null; versaoVista = $null; erro = $null }
  $sw = [Diagnostics.Stopwatch]::StartNew()
  try {
    switch ($p.tipo) {
      'vs-layout' {
        # Install from the offline layout without network (--noWeb); exe inside the layout, as Microsoft documents.
        # The layout came from the cache-msvc guest (spec addendum 8.2); the pinned bootstrapper is the fallback.
        $exe = Join-Path $p.arquivo 'layout\vs_buildtools.exe'
        if (-not (Test-Path -LiteralPath $exe)) { $exe = Join-Path $p.arquivo 'layout\vs_setup.exe' }
        if (-not (Test-Path -LiteralPath $exe)) { $exe = $p.bootstrapper }
        $argv = @('--noWeb', '--quiet', '--wait', '--norestart', '--nocache', '--installPath', "$Tools\msvc") + @($p.componentes | ForEach-Object { '--add'; $_ })
        $proc = Start-Process -FilePath $exe -ArgumentList $argv -Wait -PassThru
        $r.exit = ConvertTo-LabInstallExit $proc.ExitCode
        $vc = "$Tools\msvc\VC\Auxiliary\Build\vcvars64.bat"
        if ($r.exit -eq 0 -and -not (Test-Path -LiteralPath $vc)) { $r.exit = 1; $r.erro = 'vcvars64.bat not found after install' }
        if ($r.exit -eq 0) { $r.vcvars = $vc; $r.versaoVista = (cmd.exe /d /c "`"$vc`" >nul 2>&1 && where cl.exe" 2>&1 | Select-Object -First 1) }
      }
      'msi' {
        if (-not (Test-Sha $p.arquivo $p.sha256)) { throw 'sha256 differs from compose.json' }
        $log = "C:\lab\out\instalar-$($p.nome).log"
        $proc = Start-Process -FilePath msiexec.exe -ArgumentList @('/i', "`"$($p.arquivo)`"", '/qn', '/norestart', '/l*v', "`"$log`"") -Wait -PassThru
        $r.exit = ConvertTo-LabInstallExit $proc.ExitCode
        $rustc = Get-ChildItem -Path 'C:\Program Files\Rust*' -Recurse -Filter rustc.exe -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($r.exit -eq 0 -and -not $rustc) { $r.exit = 1; $r.erro = 'rustc.exe not found under C:\Program Files\Rust*' }
        if ($rustc) { $r.binDir = $rustc.DirectoryName; $r.versaoVista = (& $rustc.FullName --version 2>&1 | Select-Object -First 1) }
      }
      'zip' {
        if (-not (Test-Sha $p.arquivo $p.sha256)) { throw 'sha256 differs from compose.json' }
        Expand-Archive -LiteralPath $p.arquivo -DestinationPath "$Tools\node" -Force
        $node = Get-ChildItem -Path "$Tools\node" -Recurse -Filter node.exe | Select-Object -First 1
        if (-not $node) { throw 'node.exe not found after extract' }
        $r.binDir = $node.DirectoryName
        $r.versaoVista = (& $node.FullName --version 2>&1 | Select-Object -First 1)
        $r.exit = $(if ($LASTEXITCODE -eq 0) { 0 } else { $LASTEXITCODE })
      }
      'exe' {
        if (-not (Test-Sha $p.arquivo $p.sha256)) { throw 'sha256 differs from compose.json' }
        $d = "$Tools\claude"
        New-Item -ItemType Directory -Force $d | Out-Null
        Copy-Item -LiteralPath $p.arquivo -Destination "$d\claude.exe" -Force
        # Pinned version: no background update inside the guest (decision D2e).
        [Environment]::SetEnvironmentVariable('DISABLE_AUTOUPDATER', '1', 'Machine'); $env:DISABLE_AUTOUPDATER = '1'
        $r.binDir = $d
        $r.versaoVista = (& "$d\claude.exe" --version 2>&1 | Select-Object -First 1)
        $r.exit = $(if ($LASTEXITCODE -eq 0) { 0 } else { $LASTEXITCODE })
      }
      default { throw "unknown tipo '$($p.tipo)'" }
    }
  } catch { $r.exit = 1; $r.erro = $_.Exception.Message }
  $r.segundos = [math]::Round($sw.Elapsed.TotalSeconds, 1)
  return $r
}

foreach ($p in $wanted) {
  $r = Invoke-Package $p
  $res.pacotes += $r
  Save-Result
  # No 'break' on purpose (decision 2026-10-07): a failed package is recorded and the next ones still run, so Node can be
  # used for the Sandbox tests even when MSVC fails. Test-LabInstallOk stays false and Get-LabGuestToolEnv -Precisa
  # decides, per use, which packages the run needs.
}
# Interactive shells in the guest also find the tools.
$dirs = @($res.pacotes | Where-Object { $_.binDir -and $_.exit -eq 0 } | ForEach-Object { $_.binDir })
if ($dirs.Count) { [Environment]::SetEnvironmentVariable('Path', (($dirs + [Environment]::GetEnvironmentVariable('Path', 'Machine')) -join ';'), 'Machine') }
$res.ok = Test-LabInstallOk -Expected @($wanted | ForEach-Object { $_.nome }) -Results $res.pacotes
$res.fim = (Get-Date).ToString('o')
Save-Result
if ($res.ok) { exit 0 } else { exit 1 }
