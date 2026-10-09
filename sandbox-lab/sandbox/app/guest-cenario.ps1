# guest-cenario.ps1 - runs INSIDE the Sandbox (wsb exec -r ExistingLogin). Creates a data scenario on Z: with the Node the
# guest installed (spec addendum 8.1: scenarios never run on the PC). copia reads only C:\labdadosreais (read-only mount,
# present only when test-up ran with -DadosReais). Output: C:\labwork\run\cenario.json {nonce, cenario, exit, saida, erro}
param(
  [ValidateSet('novo', 'mock', 'copia')][string]$Cenario,
  [ValidatePattern('^[a-f0-9]{8,64}$')][string]$Nonce,
  [switch]$Autorizado,
  [switch]$ManterAutomacao
)
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '..\guest-install.lib.ps1')
if (-not (Test-LabInGuest)) { Write-Host 'not inside the lab guest (needs the guest identity and the bootstrap marker)'; exit 1 }
$Out = 'C:\labwork\run'
New-Item -ItemType Directory -Force $Out | Out-Null
$utf8 = New-Object System.Text.UTF8Encoding($false)
$res = [ordered]@{ nonce = $Nonce; cenario = $Cenario; exit = $null; saida = "Z:\claudia-teste-sandbox\$Cenario"; erro = $null }
function Save-Result { [IO.File]::WriteAllText((Join-Path $Out 'cenario.json'), ($res | ConvertTo-Json), $utf8) }
function Stop-With([int]$Code, [string]$Why) { $res.exit = $Code; $res.erro = $Why; Save-Result; exit $Code }

$inst = $null
try { $inst = Get-Content -LiteralPath 'C:\lab\out\instalar.json' -Raw | ConvertFrom-Json } catch { }
$hostLayout = Get-LabHostToolsLayout
$toolEnv = Get-LabGuestToolEnv $inst -Precisa node -HostLayout $hostLayout   # only Node is needed: a failed MSVC/Rust does not block scenarios
if (-not $toolEnv) { Stop-With 1 'node is neither mounted from the PC (hostTools) nor installed ok (instalar.json missing or not finished): this ambiente must install or mount node (compose.json)' }
$node = @(@($toolEnv.paths) | ForEach-Object { Join-Path $_ 'node.exe' } | Where-Object { Test-Path -LiteralPath $_ }) | Select-Object -First 1
if (-not $node) { Stop-With 1 'node.exe not found in the installed or mounted tools' }
$script = "C:\labproject\.claude\skills\claudia-teste-sandbox\scripts\cenario-$Cenario.mjs"
$argv = @($script, '--saida', $res.saida)
if ($Cenario -eq 'copia') {
  if (-not (Test-Path -LiteralPath 'C:\labdadosreais' -PathType Container)) { Stop-With 2 'real data not mounted: start the lab with test-up -DadosReais (only with the user authorization for this round)' }
  $argv += @('--origem', 'C:\labdadosreais')
  # The PC's decision about diagnostics\latest (a boolean from test-up, no path): the guest skips that one name without
  # reading or following the link.
  $guestCfg = $null; try { $guestCfg = Get-Content -LiteralPath 'C:\labsandbox\guest.config.json' -Raw -ErrorAction Stop | ConvertFrom-Json } catch { }
  if (Test-LabGuestLatestLinkApproved $guestCfg) { $argv += '--latest-link-aprovado' }
  if ($Autorizado) { $argv += '--autorizado' }
  if ($ManterAutomacao) { $argv += '--manter-automacao' }
}
$lines = @(& $node @argv 2>&1 | ForEach-Object { "$_" })
$res.exit = $LASTEXITCODE
if ($res.exit -ne 0) { $res.erro = ($lines | Select-Object -Last 3) -join ' | ' }
Save-Result
exit $res.exit
