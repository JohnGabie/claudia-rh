<#
.SYNOPSIS
  Sobe o lab a partir de um ambiente de compose.json: valida (Get-LabComposeRefusal), recusa se o PC tiver Z:, gera
  sandbox\sandbox.generated.wsb e sandbox\guest.config.json, limpa shared\out, copia a credencial do Claude quando o
  ambiente pede (conta: true), inicia o Sandbox pelo wsb CLI e espera status.json, disco-z.json e instalar.json.
.PARAMETER Ambiente     Nome do ambiente em compose.json (padrao: exemplo).
.PARAMETER Project      Pasta do projeto montada read-only em C:\labproject (absoluta). Padrao: sandbox-lab\sample-project.
.PARAMETER TimeoutSec   Espera maxima pelo status.json (padrao: bootTimeoutSec do ambiente).
.PARAMETER GenerateOnly So gera os arquivos (validate-config.ps1); nao copia credencial nem sobe nada.
.PARAMETER Force        Se ja houver um Sandbox rodando, derruba antes (test-down.ps1).
.PARAMETER NoWait       Nao espera nada depois de iniciar.
.PARAMETER WaitForMcp   Espera tambem a fase MCP terminar.
.PARAMETER DadosReais   Monta SO LEITURA a pasta de dados do app no PC (%APPDATA%\io.github.johngabie.claudia-rh, achada
                        pela API) em C:\labdadosreais, para o cenario copia no guest. So com autorizacao do usuario NAQUELA
                        rodada. Nunca %LOCALAPPDATA%. So nos ambientes app e app-winapp; qualquer link dentro da
                        pasta de origem recusa.
#>
[CmdletBinding()]
param([int]$TimeoutSec = 0, [switch]$GenerateOnly, [switch]$Force, [switch]$NoWait, [switch]$WaitForMcp, [string]$Ambiente = 'exemplo', [string]$Project, [switch]$DadosReais)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\compose.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
$Root = Split-Path $PSScriptRoot -Parent
$RepoRoot = Split-Path $Root -Parent
$rootProblem = Get-LabRootProblem $RepoRoot
if ($rootProblem) { throw "[test-up] repo root $rootProblem" }

# --- compose.json: pure checks before anything is written ---
$compose = Get-Content -LiteralPath (Join-Path $Root 'compose.json') -Raw | ConvertFrom-Json
$refusal = Get-LabComposeRefusal -Compose $compose -Ambiente $Ambiente
if ($refusal) { throw "[test-up] compose.json refused: $refusal" }
if ($DadosReais) {
  # Before anything is generated, mounted or copied: the real data is only for the ambientes that run the app.
  $ambRefusal = Get-LabRealDataAmbienteRefusal $Ambiente
  if ($ambRefusal) { throw "[test-up] $ambRefusal" }
}
$driveRefusal = Get-LabHostDriveRefusal -DriveNames @([IO.DriveInfo]::GetDrives() | ForEach-Object { $_.Name })
if ($driveRefusal) { throw "[test-up] $driveRefusal" }
$amb = $compose.ambientes.PSObject.Properties[$Ambiente].Value
$cfg = $compose.comum
$conta = [bool]$amb.conta
$install = @(ConvertTo-LabGuestInstall -Compose $compose -Ambiente $Ambiente)
$mounts = @(Get-LabComposeMounts -Compose $compose -Ambiente $Ambiente)
Write-Host "[test-up] ambiente: $Ambiente (pacotes: $((@($install | ForEach-Object { $_.nome })) -join ', '); conta: $conta)"

function Resolve-LabPath([string]$p) {
  # compose.json holds only lab-relative paths (Get-LabComposeRefusal refuses absolute ones and variables).
  if ([IO.Path]::IsPathRooted($p)) { throw "[test-up] absolute path in compose.json: $p" }
  return [IO.Path]::GetFullPath((Join-Path $Root $p))
}
# host path inside shared\ -> guest path (C:\lab\...)
$SharedDir = Resolve-LabPath 'shared'
function To-Guest([string]$hostPath) {
  $full = Resolve-LabPath $hostPath
  if (-not $full.StartsWith($SharedDir, [StringComparison]::OrdinalIgnoreCase)) { throw "Caminho fora de shared\ nao e visivel no guest: $hostPath" }
  return 'C:\lab' + $full.Substring($SharedDir.Length)
}
function XmlEsc([string]$s) { [Security.SecurityElement]::Escape($s) }

$OutDir     = Join-Path $SharedDir 'out'
$SandboxDir = Resolve-LabPath 'sandbox'
$ProjectDir = $(if ($Project) {
  if (-not [IO.Path]::IsPathRooted($Project)) { throw "-Project must be an absolute path: $Project" }
  [IO.Path]::GetFullPath($Project)
} else { Resolve-LabPath 'sample-project' })
# Real path first, then the reparse points in it and above it (junction, symlink, subst), then the pure decision.
$ProjectDir = Resolve-LabRealPath $ProjectDir
$projectRefusal = Get-LabProjectRefusal -Project $ProjectDir -UserProfile ([Environment]::GetFolderPath('UserProfile')) -Links (Get-LabLinksAboveRoot $ProjectDir)
if ($projectRefusal) { throw "[test-up] project folder refused ($ProjectDir): $projectRefusal" }
$ExtDir     = Resolve-LabPath $cfg.extensionsDir
foreach ($d in @($SharedDir, $SandboxDir, $ProjectDir, $ExtDir)) { if (-not (Test-Path -LiteralPath $d -PathType Container)) { throw "Pasta nao encontrada: $d" } }
New-Item -ItemType Directory -Force $OutDir | Out-Null

# --- host folders behind the mount names (never written in compose.json) ---
$Work = Join-Path $RepoRoot '.sandbox-work'
$HostOf = @{ trabalho = (Join-Path $Work 'guest-rw'); deploy = (Join-Path $Work 'deploy'); 'cache-saida' = (Join-Path $Work 'cache-saida') }
foreach ($m in $mounts) {
  $h = $HostOf[$m.nome]
  if ($m.nome -in @('trabalho', 'cache-saida') -and -not $GenerateOnly) { New-Item -ItemType Directory -Force -Path $h | Out-Null }   # creates only
  $links = @(Get-LabLinksInPath $h $RepoRoot)
  if ($links.Count) { throw "[test-up] link (junction/symlink) in the path of mount '$($m.nome)': $($links[0])" }
  if (-not (Test-Path -LiteralPath $h -PathType Container)) { throw "[test-up] folder for mount '$($m.nome)' not found$(if ($m.nome -eq 'deploy') { ': publish first (publicar.mjs)' })" }
}
$cred = Get-LabCredentialPaths $RepoRoot

# --- real app data for the copia scenario (spec addendum 8.1): read-only, found by the API, only with -DadosReais ---
$RealData = $null
if ($DadosReais) {
  $roaming = [Environment]::GetFolderPath('ApplicationData')
  $src = Join-Path $roaming $script:LabAppId
  # Links above the source AND any link inside it: a link in the data folder refuses the whole mount.
  $inside = Get-LabReparsePoints $src
  # Only the file link diagnostics\latest (Claude Code's debug logger) is tolerated: skipped, never followed.
  $why = Get-LabRealDataSourceRefusal -Source $src -RoamingAppData $roaming -Links (Get-LabLinksAboveRoot $src) -LinksInside $inside -FileLinksInside (Get-LabFileLinks $inside) -LinkTargets (Get-LabLinkTargets $inside)
  if ($why) { throw "[test-up] -DadosReais refused: $why" }
  if (-not (Test-Path -LiteralPath $src -PathType Container)) { throw '[test-up] -DadosReais: the app has no data folder on this PC (%APPDATA%\io.github.johngabie.claudia-rh)' }
  $RealData = $src
  Write-Host '[test-up] dados reais do app: montados SO LEITURA em C:\labdadosreais (so com autorizacao do usuario nesta rodada)'
}
$msvcLayout = $(if (@($mounts | Where-Object { $_.nome -eq 'cache-saida' }).Count) { ConvertTo-LabGuestMsvcLayout -Compose $compose } else { $null })

if (-not $GenerateOnly) {
  if ($msvcLayout) {
    $boot = Join-Path $SharedDir ('cache\' + $msvcLayout.bootstrapper.Substring(($script:LabCacheGuestDir + '\').Length))
    if (-not (Test-Path -LiteralPath $boot -PathType Leaf)) { throw '[test-up] the MSVC bootstrapper is not in shared\cache: run scripts\cache-up.ps1 first. No Sandbox was started.' }
  }
  $missingTools = Test-LabTools -LabDir $Root
  if ($missingTools.Count) {
    throw ("[test-up] ferramentas faltando ou diferentes em shared\tools ({0}): {1}. Nada foi baixado e nenhum Sandbox subiu. Como obter: sandbox-lab\README.md, secao Ferramentas." -f $missingTools.Count, (($missingTools | Select-Object -First 10) -join '; '))
  }
  # --- cache present (size only; the guest checks SHA-256) ---
  $missing = @()
  foreach ($p in $install) {
    $name = $p.arquivo.Substring(($script:LabCacheGuestDir + '\').Length)
    $h = Join-Path $SharedDir "cache\$name"
    if ($p.tipo -eq 'vs-layout') { if (-not (Test-Path -LiteralPath $h -PathType Container)) { $missing += $name } }
    else { $f = Get-Item -LiteralPath $h -ErrorAction SilentlyContinue; if (-not $f -or $f.Length -ne [long]$p.bytes) { $missing += $name } }
  }
  if ($missing.Count) { throw "[test-up] cache incomplete in shared\cache ($($missing -join ', ')): run scripts\cache-up.ps1 first. No Sandbox was started." }
}

# --- guest.config.json (read by bootstrap.ps1, bootstrap-mcp.ps1, instalar.ps1). No host path, no secret. ---
$extList = @($cfg.extensions | ForEach-Object { To-Guest (Join-Path $cfg.extensionsDir $_) })
$guest = [ordered]@{
  generatedAt = (Get-Date).ToString('o'); ambiente = $Ambiente
  startUrl = $cfg.chrome.startUrl; testServerPort = [int]$cfg.chrome.testServerPort; remoteDebuggingPort = [int]$cfg.chrome.remoteDebuggingPort
  chromeExe = To-Guest $cfg.chrome.exe; extensions = $extList; dbBackup = To-Guest $cfg.dbBackup; workDb = 'C:\labdata\app.sqlite'
  mcpPort = [int]$cfg.mcp.port; mcpAuthKeyEnv = $cfg.mcp.authKeyEnv; mcpAuthKeyFile = $(if ($cfg.mcp.authKeyFile) { To-Guest $cfg.mcp.authKeyFile } else { $null })
  outDir = 'C:\lab\out'; conta = $conta; dadosReais = [bool]$RealData; internetBloqueada = [bool](Test-LabAmbienteInternetBlocked -Compose $compose -Ambiente $Ambiente); instalar = $install; msvc = $msvcLayout
}
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText((Join-Path $SandboxDir 'guest.config.json'), ($guest | ConvertTo-Json -Depth 6), $utf8)

# --- .wsb ---
$tpl = Get-Content (Join-Path $SandboxDir 'sandbox.template.wsb') -Raw
$map = @{
  '{{SHARED_DIR}}' = $SharedDir; '{{OUT_DIR}}' = $OutDir; '{{PROJECT_DIR}}' = $ProjectDir; '{{SANDBOX_DIR}}' = $SandboxDir
  '{{MEMORY_MB}}' = [string][int]$amb.memoriaMB; '{{VGPU}}' = [string]$cfg.vGPU
  '{{NETWORKING}}' = $(if ($amb.rede) { 'Enable' } else { 'Disable' }); '{{CLIPBOARD}}' = [string]$cfg.clipboardRedirection
}
foreach ($k in $map.Keys) { $tpl = $tpl.Replace($k, (XmlEsc $map[$k])) }
if ($tpl -match '\{\{[A-Z_]+\}\}') { throw "Placeholder nao substituido no template: $($Matches[0])" }
function New-LabMappedFolderXml([string]$HostDir, [string]$Sandbox, [bool]$ReadOnly) {
  return "    <MappedFolder>`r`n      <HostFolder>$(XmlEsc $HostDir)</HostFolder>`r`n      <SandboxFolder>$(XmlEsc $Sandbox)</SandboxFolder>`r`n      <ReadOnly>$(([string]$ReadOnly).ToLowerInvariant())</ReadOnly>`r`n    </MappedFolder>`r`n"
}
$extra = ''
foreach ($m in $mounts) { $extra += New-LabMappedFolderXml $HostOf[$m.nome] $m.sandbox $m.readOnly }
if ($conta) { $extra += New-LabMappedFolderXml $cred.Dir 'C:\labcred' $true }
if ($RealData) { $extra += New-LabMappedFolderXml $RealData 'C:\labdadosreais' $true }   # always read-only
if ($extra) { $tpl = $tpl.Replace('  </MappedFolders>', $extra + '  </MappedFolders>') }
$null = [xml]$tpl
$WsbPath = Join-Path $SandboxDir 'sandbox.generated.wsb'
[IO.File]::WriteAllText($WsbPath, $tpl, $utf8)
Write-Host "[test-up] gerado: $WsbPath"
if ($GenerateOnly) { return }

# --- already a Sandbox? (Windows allows only one) ---
# WindowsSandboxServer alone does NOT mean a VM: any 'wsb' call (e.g. 'wsb list') starts that idle broker.
# Count it as running only with a VM/client process or an instance listed by wsb.
$sbNames = 'WindowsSandboxRemoteSession','WindowsSandboxClient','WindowsSandbox','vmmemWindowsSandbox'
$running = @(Get-Process -Name $sbNames -ErrorAction SilentlyContinue)
$wsb = Get-Command wsb -ErrorAction SilentlyContinue
if (-not $wsb) { throw "[test-up] 'wsb' not found: the lab needs the Windows Sandbox CLI. No Sandbox was started." }
if (-not $running) {
  $ids = @((& $wsb.Source list 2>$null) | Select-String -Pattern '[0-9a-fA-F-]{36}' -AllMatches | ForEach-Object { $_.Matches.Value })
  if ($ids) { $running = @([pscustomobject]@{ Name = "wsb:$($ids -join ',')" }) }
}
if ($running) {
  if (-not $Force) { throw "Ja existe um Windows Sandbox rodando ($($running.Name -join ', ')). Rode scripts\test-down.ps1 ou use -Force." }
  $global:LASTEXITCODE = $null
  & (Join-Path $PSScriptRoot 'test-down.ps1')
  $downRefusal = Get-LabDownRefusal -ExitCode $global:LASTEXITCODE
  if ($downRefusal) { throw "[test-up] $downRefusal" }
}

# --- clean shared\out (only the exact <lab>\shared\out, no link in it or under it) ---
Clear-LabOut -OutDir $OutDir -LabDir $Root
$StatusFile = Join-Path $OutDir 'status.json'

# --- start (wsb CLI required: the System fallback below uses wsb exec) ---
$startGuest = {
  Write-Host '[test-up] iniciando via wsb CLI'
  $r = (& $wsb.Source start --config $tpl 2>&1) -join "`n"
  Write-Host $r
  # 'wsb start' boots the VM without a user session -> the LogonCommand only runs after 'wsb connect'.
  # 'connect' blocks while the window is open, so it runs detached.
  $sid = [regex]::Match($r, '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}').Value
  if (-not $sid) { throw "[test-up] nao consegui obter o Id do Sandbox da saida do wsb: $r" }
  Write-Host "[test-up] wsb connect --id $sid (abre a janela; dispara o LogonCommand)"
  Start-Process -FilePath $wsb.Source -ArgumentList @('connect', '--id', $sid) -WindowStyle Hidden
  return $sid
}
$id = Invoke-LabStartWithCredential -Conta $conta `
  -Copy { Copy-LabCredential -RepoRoot $RepoRoot -UserProfile ([Environment]::GetFolderPath('UserProfile')) -ConfigDir $env:CLAUDE_CONFIG_DIR } `
  -Start $startGuest `
  -Cleanup { Remove-LabCredential -RepoRoot $RepoRoot }
if ($conta) { Write-Host '[test-up] conta: login copiado para .sandbox-work\credencial (conteudo nunca exibido; test-down apaga)' }
if ($NoWait) { return }

if ($TimeoutSec -le 0) { $TimeoutSec = [int]$(if ($amb.bootTimeoutSec) { $amb.bootTimeoutSec } else { 240 }) }
$deadline = (Get-Date).AddSeconds($TimeoutSec)
$st = $null
$credGone = $false
Write-Host "[test-up] aguardando $StatusFile (ate ${TimeoutSec}s)..."
while ((Get-Date) -lt $deadline) {
  Start-Sleep -Seconds 3
  if (Test-Path -LiteralPath $StatusFile) {
    try { $st = Get-Content -LiteralPath $StatusFile -Raw | ConvertFrom-Json } catch { continue }
    # The guest copied the login into its own profile (bootstrap.ps1 reads C:\labcred only once, at step 2c): the copy on
    # this PC is not needed any more, so it goes now and not only at test-down.
    if (Test-LabCredentialRemovableNow -Conta $conta -AlreadyRemoved $credGone -Status $st) {
      Remove-LabCredential -RepoRoot $RepoRoot
      $credGone = $true
      Write-Host '[test-up] conta: o guest ja tem o login; a copia no PC foi apagada'
    }
    $done = $(if ($WaitForMcp) { $st.phase -eq 'complete' } else { $st.bootstrap -ne 'running' })
    if ($done) { break }
  }
}

function Wait-LabJson([string]$File, [int]$Sec, [scriptblock]$Done) {
  $deadline = (Get-Date).AddSeconds($Sec); $last = $null
  while ((Get-Date) -lt $deadline) {
    if (Test-Path -LiteralPath $File) { try { $last = Get-Content -LiteralPath $File -Raw | ConvertFrom-Json; if (& $Done $last) { return $last } } catch { } }
    Start-Sleep -Seconds 3
  }
  return $last
}
function Invoke-LabGuestSystem([string]$Script, [switch]$Detached) {
  $cmd = "powershell -NoProfile -ExecutionPolicy Bypass -File $Script"
  if ($Detached) { Start-Process -FilePath $wsb.Source -ArgumentList "exec --id $id -r System -c `"$cmd`"" -WindowStyle Hidden | Out-Null }
  else { $null = & $wsb.Source exec --id $id -r System -c $cmd 2>&1 }
}
if ($st -and $st.elevated -eq $false) {
  Write-Host '[test-up] logon is not elevated: running disco-z.ps1 (and instalar.ps1) as System'
  Invoke-LabGuestSystem 'C:\labsandbox\disco-z.ps1'
  if ($install.Count) { Invoke-LabGuestSystem 'C:\labsandbox\instalar.ps1' -Detached }
}
$dz = Wait-LabJson (Join-Path $OutDir 'disco-z.json') 180 { param($j) $null -ne $j.ok }
$inst = $null
if ($install.Count) {
  $instTimeout = [int]$(if ($amb.instalarTimeoutSec) { $amb.instalarTimeoutSec } else { 3600 })
  Write-Host "[test-up] aguardando instalar.json (ate ${instTimeout}s)..."
  $inst = Wait-LabJson (Join-Path $OutDir 'instalar.json') $instTimeout { param($j) [bool]$j.fim }
}
$v = Get-LabUpVerdict -Status $st -DiscoZ $dz -Instalar $inst -PrecisaInstalar ($install.Count -gt 0) -PrecisaConta $conta
if ($st) { $st | ConvertTo-Json -Depth 8 }
foreach ($p in @($inst.pacotes | Where-Object { $_ })) { Write-Host ("[test-up] instalar {0} {1}: exit={2} {3}s" -f $p.nome, $p.versao, $p.exit, $p.segundos) }
if ($v.exit -eq 0) { Write-Host "[test-up] OK: $($v.motivo)" -ForegroundColor Green } else { Write-Host "[test-up] $($v.motivo)" -ForegroundColor Yellow }
exit $v.exit
