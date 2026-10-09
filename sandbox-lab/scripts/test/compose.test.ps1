# Pure tests for lib/compose.ps1 and the versioned compose.json: invented values and the repo file only, no other disk.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/compose.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../lib/compose.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) {
  try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" }
}
function Should-BeNull($v) { if ($null -ne $v) { throw "expected null, got '$v'" } }
function Should-Match($v, [string]$re) { if ($null -eq $v -or "$v" -notmatch $re) { throw "expected /$re/, got '$v'" } }

$H = 'a' * 64
function New-Compose {
  $json = @"
{
  "comum": { "dbBackup": "shared/db/backup.sqlite", "vGPU": "Enable" },
  "ambientes": {
    "testes": { "rede": true, "memoriaMB": 8192, "conta": true, "instalar": ["claude-code", "node", "rust", "msvc"], "montar": ["projeto:ro", "trabalho:rw"] },
    "host":   { "rede": true, "memoriaMB": 8192, "conta": true, "instalar": ["claude-code"], "hostTools": ["rust", "node", "msvc", "winkits"], "montar": ["projeto:ro", "trabalho:rw"] },
    "app":    { "rede": false, "memoriaMB": 6144, "conta": false, "instalar": [], "montar": ["projeto:ro", "deploy:ro", "trabalho:rw"] }
  },
  "pacotes": {
    "msvc": { "tipo": "vs-layout", "versao": "17.14.1", "arquivo": "msvc-17.14.1", "componentes": ["Microsoft.VisualStudio.Workload.VCTools"], "bootstrapper": { "arquivo": "vs_buildtools-17.14.1.exe", "url": "https://example.invalid/vs_buildtools.exe", "sha256": "$H", "bytes": 10 } },
    "rust": { "tipo": "msi", "versao": "1.95.0", "arquivo": "rust.msi", "url": "https://example.invalid/rust.msi", "sha256": "$H", "bytes": 10 },
    "node": { "tipo": "zip", "versao": "22.23.3", "arquivo": "node.zip", "url": "https://example.invalid/node.zip", "sha256": "$H", "bytes": 10 },
    "claude-code": { "tipo": "exe", "versao": "2.1.285", "arquivo": "claude.exe", "url": "https://example.invalid/claude.exe", "sha256": "$H", "bytes": 10 }
  }
}
"@
  return ($json | ConvertFrom-Json)
}

Check 'valid: every ambiente of the sample passes' {
  $c = New-Compose
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'app')
}
Check 'ambiente: missing or unknown name is refused' {
  $c = New-Compose
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente '') 'ambiente'
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'nope') 'unknown ambiente'
  Should-Match (Get-LabComposeRefusal -Compose $null -Ambiente 'testes') 'empty'
}
Check 'strings: host paths, variables, UNC, home and .. are refused anywhere' {
  foreach ($v in @('D:\dados\x', 'c:/dados/x', 'C:\Windows\x', '%USERPROFILE%\x', '%ProgramFiles(x86)%\y', '$env:APPDATA', '${HOME}', '\\srv\share\x', '~\x', 'shared\..\..\x', 'C:x')) {
    $c = New-Compose
    $c.comum | Add-Member -NotePropertyName 'extra' -NotePropertyValue $v
    Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'testes') 'comum'
  }
}
Check 'strings: a host path hidden in an array or a property name is refused too' {
  $c = New-Compose; $c.ambientes.testes.instalar = @('node', 'D:\x')
  if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')) { throw 'array value' }
  $c = New-Compose; $c.comum | Add-Member -NotePropertyName 'D:\x' -NotePropertyValue 'y'
  if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')) { throw 'property name' }
}
Check 'strings: guest paths under C:\lab, lab-relative paths and https URLs are allowed' {
  foreach ($v in @('C:\labwork\x', 'shared/db/backup.sqlite', 'https://example.invalid/a')) {
    $c = New-Compose; $c.comum | Add-Member -NotePropertyName 'extra' -NotePropertyValue $v
    Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')
  }
}
Check 'montar: unknown name, wrong mode, duplicates, bad shape and missing projeto are refused' {
  foreach ($m in @(@('projeto:ro', 'casa:rw'), @('projeto:rw'), @('projeto:ro', 'trabalho:ro'), @('projeto:ro', 'deploy:rw'), @('projeto:ro', 'projeto:ro'), @('projeto'), @('trabalho:rw'))) {
    $c = New-Compose; $c.ambientes.testes.montar = $m
    if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')) { throw "must refuse $($m -join ',')" }
  }
}
Check 'ambiente fields: rede bool, memoriaMB 2048..32768, conta bool' {
  foreach ($set in @(@('rede', 'sim'), @('memoriaMB', 1024), @('memoriaMB', 65536), @('memoriaMB', '8192'), @('conta', 'yes'))) {
    $c = New-Compose; $c.ambientes.testes.($set[0]) = $set[1]
    if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')) { throw "must refuse $($set[0])=$($set[1])" }
  }
}
Check 'internet: optional bool; false needs rede true; only rede true + internet false blocks the guest internet' {
  $c = New-Compose
  if (Test-LabAmbienteInternetBlocked -Compose $c -Ambiente 'testes') { throw 'absent -> internet allowed' }
  Add-Member -InputObject $c.ambientes.testes -NotePropertyName internet -NotePropertyValue $false
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'testes')
  if (-not (Test-LabAmbienteInternetBlocked -Compose $c -Ambiente 'testes')) { throw 'rede true + internet false -> blocked' }
  $c.ambientes.testes.internet = $true
  if (Test-LabAmbienteInternetBlocked -Compose $c -Ambiente 'testes') { throw 'internet true -> allowed' }
  $c.ambientes.testes.internet = 'nao'
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'testes') 'internet must be true or false'
  $c2 = New-Compose
  Add-Member -InputObject $c2.ambientes.app -NotePropertyName internet -NotePropertyValue $false
  Should-Match (Get-LabComposeRefusal -Compose $c2 -Ambiente 'app') 'rede true'
  if (Test-LabAmbienteInternetBlocked -Compose $c2 -Ambiente 'app') { throw 'no network -> nothing to block' }
  if (Test-LabAmbienteInternetBlocked -Compose $null -Ambiente 'app') { throw 'null compose' }
}
Check 'versioned compose.json: app-winapp declares internet false, the other ambientes do not block' {
  $real = Get-Content -LiteralPath (Join-Path $PSScriptRoot '../../compose.json') -Raw | ConvertFrom-Json
  if (-not (Test-LabAmbienteInternetBlocked -Compose $real -Ambiente 'app-winapp')) { throw 'app-winapp must block the internet' }
  foreach ($n in @('exemplo', 'app', 'cache-msvc', 'testes')) { if (Test-LabAmbienteInternetBlocked -Compose $real -Ambiente $n) { throw "$n must not require the firewall" } }
  Should-BeNull (Get-LabComposeRefusal -Compose $real -Ambiente 'app-winapp')
}
Check 'instalar: unknown or undeclared package is refused' {
  $c = New-Compose; $c.ambientes.testes.instalar = @('python')
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'testes') 'unknown package'
  $c = New-Compose; $c.pacotes.PSObject.Properties.Remove('node')
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'testes') 'not declared'
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'app')
}
Check 'pacotes: bad tipo, versao, arquivo, url, sha256 and bytes are refused' {
  $bad = @(
    @('tipo', 'zip'), @('versao', 'latest'), @('versao', '1.x'), @('arquivo', 'a\b.msi'), @('arquivo', 'a/b.msi'), @('arquivo', ' x'),
    @('url', 'http://example.invalid/r.msi'), @('url', 'ftp://example.invalid/r'), @('url', ''), @('sha256', ('A' * 64)), @('sha256', ('a' * 63)),
    @('bytes', 0), @('bytes', -1), @('bytes', '10')
  )
  foreach ($b in $bad) {
    $c = New-Compose; $c.pacotes.rust.($b[0]) = $b[1]
    Should-Match (Get-LabPackageRefusal -Nome 'rust' -Pacote $c.pacotes.rust) "rust"
  }
}
Check 'msvc: bootstrapper and componentes are required and checked' {
  $c = New-Compose; $c.pacotes.msvc.PSObject.Properties.Remove('bootstrapper')
  Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'bootstrapper'
  $c = New-Compose; $c.pacotes.msvc.componentes = @()
  Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'componentes'
  $c = New-Compose; $c.pacotes.msvc.componentes = @('Foo.Bar')
  Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'componentes'
  $c = New-Compose; $c.pacotes.msvc.bootstrapper.sha256 = 'x'
  Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'sha256'
  foreach ($a in @('vs\b.exe', 'vs_buildtools', '')) {
    $c = New-Compose; $c.pacotes.msvc.bootstrapper.arquivo = $a
    Should-Match (Get-LabPackageRefusal -Nome 'msvc' -Pacote $c.pacotes.msvc) 'bootstrapper.arquivo'
  }
}
Check 'install order is fixed (msvc, rust, node, claude-code) whatever the compose order, with guest cache paths' {
  $c = New-Compose
  $l = @(ConvertTo-LabGuestInstall -Compose $c -Ambiente 'testes')
  if ((@($l | ForEach-Object { $_.nome }) -join ',') -ne 'msvc,rust,node,claude-code') { throw "order: $(@($l | ForEach-Object { $_.nome }) -join ',')" }
  if ($l[1].arquivo -cne 'C:\lab\cache\rust.msi') { throw "rust path: $($l[1].arquivo)" }
  if ($l[0].arquivo -cne 'C:\lab\cache\msvc-17.14.1') { throw "msvc path: $($l[0].arquivo)" }
  if ($l[0].componentes[0] -ne 'Microsoft.VisualStudio.Workload.VCTools') { throw 'componentes not carried' }
  if ($l[0].sha256 -ne '' -or $l[1].sha256 -ne $H -or [long]$l[1].bytes -ne 10) { throw 'sha256/bytes not carried' }
  if (@(ConvertTo-LabGuestInstall -Compose $c -Ambiente 'app').Count -ne 0) { throw 'app installs nothing' }
}
Check 'mounts: extras only (projeto is in the template), compose order, guest folder and read-only flag' {
  $m = @(Get-LabComposeMounts -Compose (New-Compose) -Ambiente 'app')
  if ($m.Count -ne 2) { throw "count $($m.Count)" }
  if ($m[0].nome -ne 'deploy' -or $m[0].sandbox -cne 'C:\appdeploy' -or $m[0].readOnly -ne $true) { throw 'deploy' }
  if ($m[1].nome -ne 'trabalho' -or $m[1].sandbox -cne 'C:\labwork' -or $m[1].readOnly -ne $false) { throw 'trabalho' }
}
Check 'cache-saida: rw mount for the MSVC layout ambiente; ro is refused' {
  $c = New-Compose; $c.ambientes.app.montar = @('projeto:ro', 'cache-saida:rw')
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'app')
  $m = @(Get-LabComposeMounts -Compose $c -Ambiente 'app')
  if ($m[0].nome -ne 'cache-saida' -or $m[0].sandbox -cne 'C:\labcachesaida' -or $m[0].readOnly -ne $false) { throw 'cache-saida mount' }
  $c.ambientes.app.montar = @('projeto:ro', 'cache-saida:ro')
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'app') 'cache-saida'
}
Check 'msvc layout entry for the guest: version, bootstrapper guest path, hash, components, output folder' {
  $l = ConvertTo-LabGuestMsvcLayout -Compose (New-Compose)
  if ($l.versao -ne '17.14.1' -or $l.bootstrapper -cne 'C:\lab\cache\vs_buildtools-17.14.1.exe' -or $l.sha256 -ne $H -or $l.saida -cne 'C:\labcachesaida') { throw "got $($l | ConvertTo-Json -Compress)" }
  if ($l.componentes[0] -ne 'Microsoft.VisualStudio.Workload.VCTools') { throw 'componentes' }
  $i = @(ConvertTo-LabGuestInstall -Compose (New-Compose) -Ambiente 'testes')
  if ($i[0].bootstrapper -cne 'C:\lab\cache\vs_buildtools-17.14.1.exe' -or $i[1].bootstrapper -ne '') { throw 'install bootstrapper field' }
}
Check 'up verdict: status, Z: drive and install decide the exit code' {
  $ok = [pscustomobject]@{ bootstrap = 'ok' }
  $z = [pscustomobject]@{ ok = $true }
  $inst = [pscustomobject]@{ fim = 'x'; ok = $true; pacotes = @() }
  if ((Get-LabUpVerdict -Status $null -DiscoZ $z -Instalar $inst -PrecisaInstalar $true).exit -ne 2) { throw 'no status -> 2' }
  Should-Match (Get-LabUpVerdict -Status ([pscustomobject]@{ bootstrap = 'failed' }) -DiscoZ $z -Instalar $inst -PrecisaInstalar $true).motivo 'bootstrap'
  Should-Match (Get-LabUpVerdict -Status $ok -DiscoZ $null -Instalar $inst -PrecisaInstalar $true).motivo 'disco-z.json missing'
  Should-Match (Get-LabUpVerdict -Status $ok -DiscoZ ([pscustomobject]@{ ok = $false; erro = 'diskpart x' }) -Instalar $inst -PrecisaInstalar $true).motivo 'diskpart x'
  Should-Match (Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $null -PrecisaInstalar $true).motivo 'instalar'
  $bad = [pscustomobject]@{ fim = 'x'; ok = $false; pacotes = @([pscustomobject]@{ nome = 'rust'; exit = 1603 }) }
  Should-Match (Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $bad -PrecisaInstalar $true).motivo 'rust'
  if ((Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $inst -PrecisaInstalar $true).exit -ne 0) { throw 'all ok -> 0' }
  if ((Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $null -PrecisaInstalar $false).exit -ne 0) { throw 'nothing to install -> 0' }
  if ((Get-LabUpVerdict -Status $ok -DiscoZ $null -Instalar $null -PrecisaInstalar $false).exit -ne 1) { throw 'Z: is always required' }
}
Check 'early credential removal: only with an account, once, and only when the guest says conta=presente' {
  $p = [pscustomobject]@{ conta = 'presente' }
  if (-not (Test-LabCredentialRemovableNow -Conta $true -AlreadyRemoved $false -Status $p)) { throw 'presente -> remove' }
  if (Test-LabCredentialRemovableNow -Conta $false -AlreadyRemoved $false -Status $p) { throw 'no account in the ambiente -> never' }
  if (Test-LabCredentialRemovableNow -Conta $true -AlreadyRemoved $true -Status $p) { throw 'already removed -> never twice' }
  if (Test-LabCredentialRemovableNow -Conta $true -AlreadyRemoved $false -Status $null) { throw 'no status -> keep' }
  foreach ($v in @('ausente', '', 'Presente ', 'erro')) {
    if (Test-LabCredentialRemovableNow -Conta $true -AlreadyRemoved $false -Status ([pscustomobject]@{ conta = $v })) { throw "conta='$v' -> keep" }
  }
  if (Test-LabCredentialRemovableNow -Conta $true -AlreadyRemoved $false -Status ([pscustomobject]@{ bootstrap = 'ok' })) { throw 'no conta field -> keep' }
}
Check 'up verdict: a failed package stays visible (exit 1) and the reason lists what did install' {
  $ok = [pscustomobject]@{ bootstrap = 'ok' }; $z = [pscustomobject]@{ ok = $true }
  $inst = [pscustomobject]@{ fim = 'x'; ok = $false; pacotes = @(
    [pscustomobject]@{ nome = 'msvc'; exit = 1603 }, [pscustomobject]@{ nome = 'rust'; exit = 0 }, [pscustomobject]@{ nome = 'node'; exit = 0 }) }
  $v = Get-LabUpVerdict -Status $ok -DiscoZ $z -Instalar $inst -PrecisaInstalar $true
  if ($v.exit -ne 1) { throw 'msvc failed -> exit 1, never hidden' }
  Should-Match $v.motivo 'msvc \(exit 1603\)'
  Should-Match $v.motivo 'installed ok: rust, node'
}
Check 'up verdict: when the ambiente asks for the login, status.conta must be presente' {
  $z = [pscustomobject]@{ ok = $true }
  foreach ($c in @('ausente', 'falhou', $null, '')) {
    $v = Get-LabUpVerdict -Status ([pscustomobject]@{ bootstrap = 'ok'; conta = $c }) -DiscoZ $z -Instalar $null -PrecisaInstalar $false -PrecisaConta $true
    if ($v.exit -ne 1) { throw "conta '$c' must give exit 1" }
    Should-Match $v.motivo 'login'
  }
  $v = Get-LabUpVerdict -Status ([pscustomobject]@{ bootstrap = 'ok'; conta = 'presente' }) -DiscoZ $z -Instalar $null -PrecisaInstalar $false -PrecisaConta $true
  if ($v.exit -ne 0) { throw 'conta presente -> 0' }
  # An ambiente without conta does not care about status.conta.
  $v = Get-LabUpVerdict -Status ([pscustomobject]@{ bootstrap = 'ok' }) -DiscoZ $z -Instalar $null -PrecisaInstalar $false -PrecisaConta $false
  if ($v.exit -ne 0) { throw 'no conta needed -> 0' }
}
Check '-DadosReais: only the ambientes app and app-winapp; anything else is refused before mounting' {
  Should-BeNull (Get-LabRealDataAmbienteRefusal 'app')
  Should-BeNull (Get-LabRealDataAmbienteRefusal 'app-winapp')
  foreach ($a in @('testes', 'exemplo', 'cache-msvc', 'app2', 'xapp', '', $null)) {
    if (-not (Get-LabRealDataAmbienteRefusal $a)) { throw "must refuse '$a'" }
  }
  Should-Match (Get-LabRealDataAmbienteRefusal 'testes') 'only for the ambientes app, app-winapp'
}
Check 'start with credential: cleanup runs when the start throws, never when conta is false; a failed copy stops before start' {
  $log = New-Object System.Collections.Generic.List[string]
  $threw = $false
  try { Invoke-LabStartWithCredential -Conta $true -Copy { $log.Add('copy') } -Start { $log.Add('start'); throw 'boom' } -Cleanup { $log.Add('cleanup') } } catch { $threw = $true }
  if (-not $threw -or ($log -join ',') -ne 'copy,start,cleanup') { throw "throwing start: $($log -join ',')" }
  $log.Clear()
  $id = Invoke-LabStartWithCredential -Conta $true -Copy { $log.Add('copy') } -Start { 'id-1' } -Cleanup { $log.Add('cleanup') }
  if ($id -ne 'id-1' -or ($log -join ',') -ne 'copy') { throw "success: id=$id log=$($log -join ',')" }
  $log.Clear()
  try { Invoke-LabStartWithCredential -Conta $false -Copy { $log.Add('copy') } -Start { throw 'boom' } -Cleanup { $log.Add('cleanup') } } catch { }
  if ($log.Count) { throw "conta false: $($log -join ',')" }
  $log.Clear()
  try { Invoke-LabStartWithCredential -Conta $true -Copy { throw 'no login' } -Start { $log.Add('start') } -Cleanup { $log.Add('cleanup') } } catch { }
  if ($log -contains 'start') { throw 'start ran after a failed copy' }
}
Check 'the versioned compose.json passes for every ambiente and pins the researched versions' {
  $c = Get-Content -LiteralPath "$PSScriptRoot/../../compose.json" -Raw | ConvertFrom-Json
  $names = @($c.ambientes.PSObject.Properties | ForEach-Object { $_.Name })
  foreach ($n in @('exemplo', 'app', 'app-winapp', 'testes', 'cache-msvc')) { if ($names -notcontains $n) { throw "missing ambiente $n" } }
  foreach ($n in @('app', 'app-winapp')) { if ((@($c.ambientes.$n.instalar) -join ',') -ne 'node') { throw "$n must install node (scenarios run in the guest)" } }
  if ((@($c.ambientes.'cache-msvc'.montar) -join ',') -ne 'projeto:ro,cache-saida:rw' -or @($c.ambientes.'cache-msvc'.instalar).Count) { throw 'cache-msvc ambiente' }
  foreach ($n in $names) { Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente $n) }
  if ($c.pacotes.rust.versao -ne '1.95.0' -or $c.pacotes.rust.sha256 -ne '08d228850565cfe421da6faeac029fb1f0c8341219d0dd9490d42a8355aa0703') { throw 'rust pin' }
  if ($c.pacotes.node.versao -ne '22.23.3' -or $c.pacotes.node.sha256 -ne '2b0ff57b049cda1bbcea2240eec20467018713c1efe1f7360c2681859b90ed71') { throw 'node pin' }
  if ($c.pacotes.'claude-code'.versao -ne '2.1.285' -or $c.pacotes.'claude-code'.sha256 -ne '121fc8151ed40bd9c144d68aa1cea23427803628ffab65e23da1cceda155697e') { throw 'claude-code pin' }
  # Decision of the user (2026-10-08, spec addendum 9): testes uses the PC toolchains read-only; testes-limpo installs everything.
  if ((@($c.ambientes.testes.instalar) -join ',') -ne 'claude-code' -or $c.ambientes.testes.conta -ne $true) { throw 'testes ambiente installs only claude-code' }
  if ((@($c.ambientes.testes.hostTools) -join ',') -ne 'rust,node,msvc,winkits') { throw 'testes ambiente must declare the host tools by name' }
  if ((@($c.ambientes.'testes-limpo'.instalar) -join ',') -ne 'msvc,rust,node,claude-code' -or $c.ambientes.'testes-limpo'.conta -ne $true -or $null -ne $c.ambientes.'testes-limpo'.hostTools) { throw 'testes-limpo ambiente installs everything and mounts nothing' }
  if ([int]$c.ambientes.'testes-limpo'.instalarTimeoutSec -lt 11400) { throw 'testes-limpo waits at least the 3 h 08 min the MSVC install took' }
  if ((@($c.ambientes.testes.montar) -join ',') -ne (@($c.ambientes.'testes-limpo'.montar) -join ',')) { throw 'testes and testes-limpo mount the same names' }
}
Check 'hostTools: known names only, once, a list; msvc and winkits together; never also in instalar' {
  $c = New-Compose
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'host')
  foreach ($bad in @(@('rust', 'perl'), @('rust', 'rust'), @('Rust'), @('msvc'), @('winkits'), @('node', 'msvc'), @('C:\x'), @('%ProgramFiles%\nodejs'), @($null), @(5))) {
    $c = New-Compose; $c.ambientes.host.hostTools = $bad
    if (-not (Get-LabComposeRefusal -Compose $c -Ambiente 'host')) { throw "must refuse $($bad -join ',')" }
  }
  $c = New-Compose; $c.ambientes.host.hostTools = 'node'
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'host') 'must be a list'
  $c = New-Compose; $c.ambientes.host.instalar = @('claude-code', 'node')
  Should-Match (Get-LabComposeRefusal -Compose $c -Ambiente 'host') 'both hostTools and instalar'
  $c = New-Compose; $c.ambientes.host.hostTools = @('node', 'rust')
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'host')
  $c = New-Compose; $c.ambientes.host.hostTools = @()
  Should-BeNull (Get-LabComposeRefusal -Compose $c -Ambiente 'host')
  # Parsed from JSON, a one-element list must stay a list (a function returns it as the bare element).
  $one = '{"ambientes":{"x":{"rede":true,"memoriaMB":4096,"hostTools":["node"],"montar":["projeto:ro"]}}}' | ConvertFrom-Json
  Should-BeNull (Get-LabComposeRefusal -Compose $one -Ambiente 'x')
  if ((@(Get-LabComposeHostTools -Compose $one -Ambiente 'x') -join ',') -ne 'node') { throw 'one-element list' }
}
Check 'hostTools: no host path anywhere, even as a name (rule A5 keeps working)' {
  foreach ($v in @('D:\Program Files\nodejs', 'c:/rust', '%USERPROFILE%\.rustup', '\\srv\share')) {
    $c = New-Compose; $c.ambientes.host.hostTools = @('node', $v)
    $why = Get-LabComposeRefusal -Compose $c -Ambiente 'host'
    if (-not $why) { throw "must refuse $v" }
  }
}
Check 'hostTools: Get-LabComposeHostTools returns names in the fixed order; none for an ambiente without the key' {
  $c = New-Compose; $c.ambientes.host.hostTools = @('winkits', 'msvc', 'node', 'rust')
  if ((@(Get-LabComposeHostTools -Compose $c -Ambiente 'host') -join ',') -ne 'rust,node,msvc,winkits') { throw "order: $(@(Get-LabComposeHostTools -Compose $c -Ambiente 'host') -join ',')" }
  if (@(Get-LabComposeHostTools -Compose $c -Ambiente 'testes').Count) { throw 'testes of the sample has no hostTools' }
  if (@(Get-LabComposeHostTools -Compose $c -Ambiente 'nope').Count) { throw 'unknown ambiente has none' }
}

if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
