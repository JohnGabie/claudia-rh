# bootstrap.ps1 - roda DENTRO do Windows Sandbox (LogonCommand), PowerShell 5.1.
# Mapeamentos: C:\lab = shared (RO), C:\lab\out = shared\out (RW), C:\labproject = projeto (RO),
#              C:\labsandbox = sandbox\ (RO). Banco de trabalho: C:\labdata\app.sqlite.
# Saida: C:\lab\out\status.json + C:\lab\out\bootstrap.log
$ErrorActionPreference = 'Continue'
$Out      = 'C:\lab\out'
$LogFile  = Join-Path $Out 'bootstrap.log'
$StatusF  = Join-Path $Out 'status.json'
$DataDir  = 'C:\labdata'
$utf8     = New-Object System.Text.UTF8Encoding($false)

function Now { (Get-Date).ToString('o') }
function Log([string]$m) {
  $line = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss.fff'), $m
  try { [IO.File]::AppendAllText($LogFile, $line + "`r`n", $utf8) } catch { }
}

$status = [ordered]@{
  bootstrap        = 'running'
  phase            = 'starting'
  computerName     = $env:COMPUTERNAME
  user             = $env:USERNAME
  outWritable      = $false
  dbHashExpected   = $null
  dbHashBackup     = $null
  dbHashCopy       = $null
  dbHashMatch      = $false
  dbVerifyRows     = @()
  dbVerifyMatch    = $false
  projectMounted   = $false
  projectReadOnly  = $false
  testServer       = $null
  chromeStarted    = $false
  chromeExe        = $null
  chromeCdp        = $null
  chromePageTitle  = $null
  extensionMarker  = $false
  mcp              = [ordered]@{ script = 'C:\labsandbox\bootstrap-mcp.ps1'; present = $false; result = 'skipped' }
  timestamps       = [ordered]@{ start = (Now) }
  errors           = @()
}
function AddErr([string]$m) { $script:status.errors += $m; Log "ERRO: $m" }
function Save-Status {
  try { [IO.File]::WriteAllText($StatusF, ($script:status | ConvertTo-Json -Depth 8), $utf8) } catch { Log "falha ao gravar status: $_" }
}

# --- 0. config do guest (gerada pelo test-up.ps1) ---
$cfg = $null
try { $cfg = Get-Content 'C:\labsandbox\guest.config.json' -Raw | ConvertFrom-Json } catch { }
if (-not $cfg) {
  $cfg = [pscustomobject]@{ startUrl = 'http://localhost:8080/test-page.html'; testServerPort = 8080; remoteDebuggingPort = 9222;
                            extensions = @('C:\lab\extensions\lab-ext'); chromeExe = 'C:\lab\tools\chrome\chrome-win64\chrome.exe'; mcpPort = 8000 }
}

# --- 1. pasta de saida gravavel ---
try { [IO.File]::WriteAllText((Join-Path $Out '.write-test'), 'ok', $utf8); Remove-Item (Join-Path $Out '.write-test') -Force; $status.outWritable = $true }
catch { $status.errors += "C:\lab\out nao gravavel: $_" }
Log "bootstrap iniciado como $env:USERNAME em $env:COMPUTERNAME"
Save-Status

# --- 2. banco: copia backup -> C:\labdata\app.sqlite, confere hash, roda verify ---
$status.phase = 'db'
try {
  New-Item -ItemType Directory -Force $DataDir | Out-Null
  $expected = ((Get-Content 'C:\lab\db\backup.sha256' -Raw).Trim() -split '\s+')[0].ToUpper()
  $status.dbHashExpected = $expected
  $status.dbHashBackup = (Get-FileHash 'C:\lab\db\backup.sqlite' -Algorithm SHA256).Hash
  Copy-Item 'C:\lab\db\backup.sqlite' (Join-Path $DataDir 'app.sqlite') -Force
  Set-ItemProperty (Join-Path $DataDir 'app.sqlite') -Name IsReadOnly -Value $false
  $status.dbHashCopy = (Get-FileHash (Join-Path $DataDir 'app.sqlite') -Algorithm SHA256).Hash
  $status.dbHashMatch = ($status.dbHashCopy -eq $expected) -and ($status.dbHashBackup -eq $expected)
  Log "db hash esperado=$expected backup=$($status.dbHashBackup) copia=$($status.dbHashCopy)"
  if (-not $status.dbHashMatch) { AddErr 'hash do banco diverge' }

  # hash ANTES de qualquer consulta (o verify e somente leitura, mas abrimos em modo ro mesmo assim)
  $sqlite = 'C:\lab\tools\sqlite\sqlite3.exe'
  if (Test-Path $sqlite) {
    $dbUri = 'file:' + ((Join-Path $DataDir 'app.sqlite') -replace '\\','/') + '?mode=ro'
    $rows = Get-Content 'C:\lab\db\verify.sql' -Raw | & $sqlite -batch -bail $dbUri 2>&1
    $status.dbVerifyRows = @($rows | ForEach-Object { "$_".Trim() } | Where-Object { $_ })
    $exp = @(Get-Content 'C:\lab\db\verify.expected.txt' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    $status.dbVerifyMatch = (($status.dbVerifyRows -join "`n") -eq ($exp -join "`n"))
    Log "verify rows: $($status.dbVerifyRows -join ' ; ') match=$($status.dbVerifyMatch)"
    if (-not $status.dbVerifyMatch) { AddErr 'consulta de verificacao nao retornou as linhas esperadas' }
  } else { AddErr "sqlite3.exe ausente em $sqlite" }
} catch { AddErr "db: $_" }
Save-Status

# --- 3. projeto montado e somente leitura ---
$status.phase = 'project'
$status.projectMounted = Test-Path 'C:\labproject'
if ($status.projectMounted) {
  $probe = 'C:\labproject\__lab_write_probe.tmp'
  try {
    [IO.File]::WriteAllText($probe, 'x')
    Remove-Item $probe -Force -ErrorAction SilentlyContinue
    $status.projectReadOnly = $false
    AddErr 'C:\labproject aceitou escrita (deveria ser read-only)'
  } catch {
    $status.projectReadOnly = $true
    Log "escrita em C:\labproject falhou como esperado: $($_.Exception.Message)"
  }
} else { AddErr 'C:\labproject nao existe' }
Save-Status

# --- 4. servidor HTTP local da pagina de teste ---
$status.phase = 'test-server'
try {
  $port = [int]$cfg.testServerPort
  Start-Process powershell.exe -WindowStyle Hidden -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File','C:\labsandbox\serve-lab.ps1','-Port',"$port",'-Root','C:\lab') | Out-Null
  $ok = $false
  for ($i = 0; $i -lt 20 -and -not $ok; $i++) {
    Start-Sleep -Milliseconds 500
    try { $r = Invoke-WebRequest "http://localhost:$port/test-page.html" -UseBasicParsing -TimeoutSec 3; $ok = ($r.StatusCode -eq 200) } catch { }
  }
  $status.testServer = $(if ($ok) { "http://localhost:$port/" } else { 'failed' })
  if (-not $ok) { AddErr 'servidor HTTP da pagina de teste nao respondeu' }
} catch { AddErr "test-server: $_" }
Save-Status

# --- 5. Chrome for Testing com a extensao ---
$status.phase = 'chrome'
function Test-Cdp($p) { try { Invoke-RestMethod "http://127.0.0.1:$p/json/version" -TimeoutSec 2 } catch { $null } }
function Start-LabChrome([string]$exe) {
  $extArg = (@($cfg.extensions) -join ',')
  $cargs = @(
    "--load-extension=$extArg",
    "--disable-extensions-except=$extArg",
    '--user-data-dir=C:\labdata\chrome-profile',
    '--no-first-run', '--no-default-browser-check', '--disable-search-engine-choice-screen',
    "--remote-debugging-port=$($cfg.remoteDebuggingPort)", '--remote-allow-origins=*',
    '--start-maximized',
    $cfg.startUrl
  )
  Log "iniciando chrome: $exe $($cargs -join ' ')"
  Start-Process -FilePath $exe -ArgumentList $cargs | Out-Null
  for ($i = 0; $i -lt 30; $i++) { Start-Sleep -Seconds 1; $v = Test-Cdp $cfg.remoteDebuggingPort; if ($v) { return $v } }
  return $null
}
try {
  $exe = $cfg.chromeExe
  $v = $null
  if (Test-Path $exe) { $v = Start-LabChrome $exe } else { AddErr "chrome.exe ausente: $exe" }
  if (-not $v -and (Test-Path $exe)) {
    # fallback: copiar o Chrome para disco local do guest (caso rodar da pasta mapeada falhe)
    Log 'CDP nao respondeu; copiando Chrome para C:\labdata\chrome e tentando de novo'
    Get-Process chrome -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Copy-Item (Split-Path $exe) 'C:\labdata\chrome' -Recurse -Force
    $exe = 'C:\labdata\chrome\chrome.exe'
    $v = Start-LabChrome $exe
  }
  $status.chromeExe = $exe
  if ($v) {
    $status.chromeStarted = $true
    $status.chromeCdp = [ordered]@{ port = $cfg.remoteDebuggingPort; browser = $v.Browser }
    # marcador da extensao: content script prefixa o titulo com "[LAB] "
    for ($i = 0; $i -lt 20; $i++) {
      Start-Sleep -Seconds 1
      try {
        # PS 5.1: Invoke-RestMethod emite o array JSON como UM objeto -> atribuir antes de filtrar (desenrola)
        $list = Invoke-RestMethod "http://127.0.0.1:$($cfg.remoteDebuggingPort)/json/list" -TimeoutSec 2
        $pages = @($list | ForEach-Object { $_ } | Where-Object { $_.type -eq 'page' })
        $p = $pages | Where-Object { $_.url -like '*test-page*' } | Select-Object -First 1
        if ($p) { $status.chromePageTitle = $p.title; if ($p.title -and $p.title.StartsWith("[LAB] ")) { $status.extensionMarker = $true; break } }
      } catch { }
    }
    Log "chrome ok: $($v.Browser) titulo='$($status.chromePageTitle)' marcador=$($status.extensionMarker)"
    if (-not $status.extensionMarker) { AddErr "marcador da extensao nao detectado (titulo: '$($status.chromePageTitle)')" }
  } else { AddErr 'Chrome nao respondeu no CDP' }
} catch { AddErr "chrome: $_" }

$status.timestamps.coreDone = (Now)
$coreOk = $status.outWritable -and $status.dbHashMatch -and $status.dbVerifyMatch -and $status.projectReadOnly -and $status.chromeStarted -and $status.extensionMarker
$status.bootstrap = $(if ($coreOk) { 'ok' } else { 'failed' })
Save-Status

# --- 6. hook do MCP (opcional) ---
# Contrato: C:\labsandbox\bootstrap-mcp.ps1 (sem parametros obrigatorios), le env LAB_GUEST_CONFIG / LAB_MCP_PORT,
# deve iniciar o servidor de forma destacada, retornar, e gravar C:\lab\out\mcp-status.json.
$status.phase = 'mcp'
$mcpScript = 'C:\labsandbox\bootstrap-mcp.ps1'
if (Test-Path $mcpScript) {
  $status.mcp.present = $true
  $status.mcp.result = 'running'
  $status.timestamps.mcpStart = (Now)
  Save-Status
  try {
    $env:LAB_GUEST_CONFIG = 'C:\labsandbox\guest.config.json'
    $env:LAB_MCP_PORT = "$($cfg.mcpPort)"
    $mcpLog = Join-Path $Out 'bootstrap-mcp.stdout.log'
    $p = Start-Process powershell.exe -PassThru -RedirectStandardOutput $mcpLog -RedirectStandardError (Join-Path $Out 'bootstrap-mcp.stderr.log') `
          -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',$mcpScript)
    $null = $p.Handle   # cacheia o handle; sem isso ExitCode volta $null no PS 5.1
    $timeout = 600
    $exited = $p.WaitForExit($timeout * 1000)
    if ($exited) { $p.WaitForExit(); $status.mcp.exitCode = $p.ExitCode }
    $ms = Join-Path $Out 'mcp-status.json'
    $mcpOk = $false
    if (Test-Path $ms) {
      try { $status.mcp.status = Get-Content $ms -Raw | ConvertFrom-Json; $mcpOk = [bool]$status.mcp.status.ok } catch { $status.mcp.status = 'mcp-status.json invalido' }
    } else { $status.mcp.status = 'mcp-status.json ausente' }
    # resultado: mcp-status.json.ok e a fonte de verdade; exit code != 0 tambem reprova
    if (-not $exited) { $status.mcp.result = 'timeout'; AddErr "bootstrap-mcp.ps1 excedeu ${timeout}s" }
    elseif ($mcpOk -and ($null -eq $status.mcp.exitCode -or $status.mcp.exitCode -eq 0)) { $status.mcp.result = 'ok' }
    else { $status.mcp.result = 'failed'; AddErr "bootstrap-mcp.ps1 falhou (exit=$($status.mcp.exitCode), ok=$mcpOk)" }
  } catch { $status.mcp.result = 'failed'; AddErr "bootstrap-mcp: $_" }
  $status.timestamps.mcpEnd = (Now)
} else {
  Log 'bootstrap-mcp.ps1 ausente; etapa MCP ignorada'
}

$status.phase = 'complete'
$status.timestamps.end = (Now)
Save-Status
Log "bootstrap finalizado: $($status.bootstrap) (erros: $($status.errors.Count))"
