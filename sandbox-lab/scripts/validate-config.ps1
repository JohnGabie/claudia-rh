<#
.SYNOPSIS
  Validacao estatica (O1) - nao sobe o Sandbox. Sai com 0 se nao houver FAIL.
  .mcp.json e .claude/skills sao opcionais aqui (WARN se ausentes; FAIL se presentes e invalidos).
#>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Continue'
$Root = Split-Path $PSScriptRoot -Parent
. (Join-Path $PSScriptRoot 'lib\travas.ps1')
. (Join-Path $PSScriptRoot 'lib\compose.ps1')
. (Join-Path $PSScriptRoot 'lib\disco.ps1')
$results = New-Object System.Collections.Generic.List[object]
function Add-Result([string]$level, [string]$check, [string]$detail = '') {
  $results.Add([pscustomobject]@{ Level = $level; Check = $check; Detail = $detail })
}
function P([string]$p) { if ([IO.Path]::IsPathRooted($p)) { $p } else { Join-Path $Root $p } }

# 1. compose.json (the old per-profile files are ambientes now)
$compose = $null
try { $compose = Get-Content (Join-Path $Root 'compose.json') -Raw | ConvertFrom-Json; Add-Result PASS 'compose.json e JSON valido' }
catch { Add-Result FAIL 'compose.json e JSON valido' $_.Exception.Message }
$cfg = $null
if ($compose) {
  $cfg = $compose.comum
  foreach ($n in @($compose.ambientes.PSObject.Properties.Name)) {
    $why = Get-LabComposeRefusal -Compose $compose -Ambiente $n
    if ($why) { Add-Result FAIL "compose.json: ambiente $n" $why } else { Add-Result PASS "compose.json: ambiente $n" }
  }
}

if ($cfg) {
  foreach ($k in 'extensionsDir','dbBackup','chrome','mcp') {
    if ($null -eq $cfg.$k) { Add-Result FAIL "comum.$k presente" } }
  $paths = [ordered]@{
    projectPath = 'sample-project'; extensionsDir = $cfg.extensionsDir; dbBackup = $cfg.dbBackup
    dbHashFile = $cfg.dbHashFile; dbVerifySql = $cfg.dbVerifySql; dbVerifyExpected = $cfg.dbVerifyExpected
    'chrome.exe' = $cfg.chrome.exe; 'sqlite3.exe' = 'shared/tools/sqlite/sqlite3.exe'
    'test-page.html' = 'shared/test-page.html'; 'sandbox.template.wsb' = 'sandbox/sandbox.template.wsb'
    'bootstrap.ps1' = 'sandbox/bootstrap.ps1'; 'serve-lab.ps1' = 'sandbox/serve-lab.ps1'
    'test-down.ps1' = 'scripts/test-down.ps1'
  }
  foreach ($k in $paths.Keys) {
    if (-not $paths[$k]) { Add-Result FAIL "caminho $k definido"; continue }
    if (Test-Path (P $paths[$k])) { Add-Result PASS "caminho $k existe" $paths[$k] } else { Add-Result FAIL "caminho $k existe" $paths[$k] }
  }
  # extensoes
  foreach ($e in @($cfg.extensions)) {
    $mf = Join-Path (P $cfg.extensionsDir) "$e\manifest.json"
    try { $m = Get-Content $mf -Raw | ConvertFrom-Json
      if ($m.manifest_version -eq 3) { Add-Result PASS "extensao ${e}: manifest MV3 valido" } else { Add-Result FAIL "extensao ${e}: manifest_version=3" "$($m.manifest_version)" }
      foreach ($cs in @($m.content_scripts)) { foreach ($js in @($cs.js)) { if (-not (Test-Path (Join-Path (Split-Path $mf) $js))) { Add-Result FAIL "extensao ${e}: $js existe" } } }
    } catch { Add-Result FAIL "extensao ${e}: manifest.json" $_.Exception.Message }
  }
  # hash do banco
  try {
    $exp = ((Get-Content (P $cfg.dbHashFile) -Raw).Trim() -split '\s+')[0]
    $act = (Get-FileHash (P $cfg.dbBackup) -Algorithm SHA256).Hash
    if ($exp -ieq $act) { Add-Result PASS 'backup.sqlite bate com backup.sha256' $act } else { Add-Result FAIL 'backup.sqlite bate com backup.sha256' "esperado $exp, atual $act" }
  } catch { Add-Result FAIL 'hash do backup' $_.Exception.Message }
  if ($cfg.mcp.port -is [int] -or $cfg.mcp.port -is [long]) { Add-Result PASS 'mcp.port numerico' "$($cfg.mcp.port)" } else { Add-Result FAIL 'mcp.port numerico' }
  if ($cfg.mcp.configFile -and (Test-Path (P $cfg.mcp.configFile))) {
    try { $gm = Get-Content (P $cfg.mcp.configFile) -Raw | ConvertFrom-Json
      if ([int]$gm.port -eq [int]$cfg.mcp.port) { Add-Result PASS 'mcp.port bate com shared/mcp.json' } else { Add-Result FAIL 'mcp.port bate com shared/mcp.json' "$($cfg.mcp.port) vs $($gm.port)" }
    } catch { Add-Result FAIL 'shared/mcp.json e JSON valido' $_.Exception.Message }
  } elseif ($cfg.mcp.configFile) { Add-Result WARN 'mcp.configFile existe' "$($cfg.mcp.configFile) (gerado pelo setup do MCP)" }
  if ($cfg.mcp.authKeyFile -and -not (Test-Path (P $cfg.mcp.authKeyFile))) { Add-Result WARN 'mcp.authKeyFile existe' "$($cfg.mcp.authKeyFile) (gerado pelo setup do MCP)" }
}

# 1b. ferramentas (shared\tools) batem com o manifesto versionado
$missingTools = Test-LabTools -LabDir $Root
if ($missingTools.Count) { Add-Result FAIL 'shared/tools bate com shared/tools.sha256' ("{0}: {1}" -f $missingTools.Count, (($missingTools | Select-Object -First 5) -join '; ')) }
else { Add-Result PASS 'shared/tools bate com shared/tools.sha256' }

# 2. scripts PowerShell parseiam
foreach ($s in Get-ChildItem (Join-Path $Root 'scripts'), (Join-Path $Root 'scripts\lib'), (Join-Path $Root 'scripts\test'), (Join-Path $Root 'sandbox'), (Join-Path $Root 'sandbox\app') -Filter *.ps1) {
  $tok = $null; $err = $null
  [void][Management.Automation.Language.Parser]::ParseFile($s.FullName, [ref]$tok, [ref]$err)
  if ($err.Count) { Add-Result FAIL "sintaxe $($s.Name)" ($err | Select-Object -First 1).Message } else { Add-Result PASS "sintaxe $($s.Name)" }
}

# 3. .wsb gerado e XML valido
try {
  & (Join-Path $PSScriptRoot 'test-up.ps1') -GenerateOnly -Ambiente exemplo *> $null
  $wsbPath = Join-Path $Root 'sandbox\sandbox.generated.wsb'
  [xml]$x = Get-Content $wsbPath -Raw
  $mf = @($x.Configuration.MappedFolders.MappedFolder)
  $bad = $mf | Where-Object { -not (Test-Path $_.HostFolder) }
  if ($bad) { Add-Result FAIL '.wsb: HostFolders existem' ($bad.HostFolder -join '; ') }
  elseif ($mf.Count -lt 4) { Add-Result FAIL '.wsb: 4 MappedFolders' "$($mf.Count)" }
  else { Add-Result PASS '.wsb gerado e XML valido, HostFolders existem' $wsbPath }
  $ro = @{}; $mf | ForEach-Object { $ro[$_.SandboxFolder] = $_.ReadOnly }
  if ($ro['C:\lab'] -eq 'true' -and $ro['C:\lab\out'] -eq 'false' -and $ro['C:\labproject'] -eq 'true' -and $ro['C:\labsandbox'] -eq 'true') { Add-Result PASS '.wsb: flags ReadOnly corretas' }
  else { Add-Result FAIL '.wsb: flags ReadOnly corretas' ($ro.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ', ' }
  if ($x.Configuration.LogonCommand.Command -match 'bootstrap\.ps1' -and $x.Configuration.LogonCommand.Command -match 'Bypass') { Add-Result PASS '.wsb: LogonCommand roda bootstrap.ps1 (Bypass)' } else { Add-Result FAIL '.wsb: LogonCommand' }
} catch { Add-Result FAIL '.wsb gerado' $_.Exception.Message }

# 4. .mcp.json (opcional)
$mcpJson = Join-Path $Root '.mcp.json'
if (Test-Path $mcpJson) {
  try { $mj = Get-Content $mcpJson -Raw | ConvertFrom-Json
    if ($mj.mcpServers -and @($mj.mcpServers.PSObject.Properties).Count -gt 0) { Add-Result PASS '.mcp.json valido' (($mj.mcpServers.PSObject.Properties.Name) -join ', ') }
    else { Add-Result FAIL '.mcp.json tem mcpServers' }
  } catch { Add-Result FAIL '.mcp.json e JSON valido' $_.Exception.Message }
} else { Add-Result WARN '.mcp.json presente' 'ausente' }

# 5. skills (opcional)
$skillsDir = Join-Path (Split-Path $Root -Parent) '.claude\skills'   # skills live in the repo root
$skillFiles = @(Get-ChildItem $skillsDir -Recurse -Filter SKILL.md -ErrorAction SilentlyContinue)
if (-not $skillFiles.Count) { Add-Result WARN '.claude/skills com SKILL.md' 'ausente' }
foreach ($f in $skillFiles) {
  $t = Get-Content $f.FullName -Raw
  # guarda o frontmatter antes: cada -match sobrescreve $Matches
  $fm = $(if ($t -match '(?s)^\uFEFF?---\s*\r?\n(.*?)\r?\n---') { $Matches[1] } else { '' })
  if ($fm -match '(?m)^name:\s*\S' -and $fm -match '(?m)^description:\s*\S') { Add-Result PASS "skill $($f.Directory.Name): frontmatter ok" }
  else { Add-Result FAIL "skill $($f.Directory.Name): frontmatter name/description" $f.FullName }
}
if (Test-Path (Join-Path $Root 'sandbox\bootstrap-mcp.ps1')) { Add-Result PASS 'sandbox/bootstrap-mcp.ps1 presente' } else { Add-Result WARN 'sandbox/bootstrap-mcp.ps1 presente' 'ausente (etapa MCP sera ignorada no guest)' }

# saida
$color = @{ PASS = 'Green'; WARN = 'Yellow'; FAIL = 'Red' }
foreach ($r in $results) { Write-Host ("[{0}] {1}{2}" -f $r.Level, $r.Check, $(if ($r.Detail) { " - $($r.Detail)" } else { '' })) -ForegroundColor $color[$r.Level] }
$f = @($results | Where-Object Level -eq 'FAIL').Count; $w = @($results | Where-Object Level -eq 'WARN').Count
Write-Host ("`nResumo: {0} PASS, {1} WARN, {2} FAIL" -f @($results | Where-Object Level -eq 'PASS').Count, $w, $f)
if ($f) { Write-Host 'O1: FALHOU' -ForegroundColor Red; exit 1 } else { Write-Host 'O1: OK' -ForegroundColor Green; exit 0 }
