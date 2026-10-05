$script = Join-Path $PSScriptRoot '..\print-janela.ps1'
$fail = 0
function Check([bool]$c, [string]$n) { if ($c) { "PASS $n" } else { "FAILED $n"; $script:fail++ } }
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("print test " + [guid]::NewGuid().ToString('N').Substring(0, 8) + '.png')

$out = @(& pwsh -NoProfile -File $script -TelaInteira -Saida $tmp)
Check ($LASTEXITCODE -eq 0) 'tela inteira exit 0'
$j = $out[-1] | ConvertFrom-Json
$bytes = [IO.File]::ReadAllBytes($tmp)
Check ($bytes[0] -eq 0x89 -and $bytes[1] -eq 0x50 -and $bytes[2] -eq 0x4E -and $bytes[3] -eq 0x47) 'arquivo e PNG'
Check ($j.largura -gt 0 -and $j.altura -gt 0) 'dimensoes positivas'
Remove-Item $tmp

# Regression: a bare -Saida name resolves against the current directory (absolute path in the JSON).
$dir = Join-Path ([IO.Path]::GetTempPath()) ("print bare " + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory $dir | Out-Null
Push-Location $dir
try {
  $out2 = @(& pwsh -NoProfile -File $script -TelaInteira -Saida shot.png)
  $code2 = $LASTEXITCODE
  $j2 = $out2[-1] | ConvertFrom-Json
} finally { Pop-Location }
Check ($code2 -eq 0) '-Saida sem pasta: exit 0'
Check (Test-Path (Join-Path $dir 'shot.png')) '-Saida sem pasta: arquivo criado na pasta atual'
Check ([IO.Path]::IsPathRooted($j2.arquivo)) '-Saida sem pasta: arquivo do JSON e absoluto'
Remove-Item $dir -Recurse -Force

$errFile = [IO.Path]::GetTempFileName()
& pwsh -NoProfile -File $script -Processo 'processo-que-nao-existe-xyz' -Saida $tmp 2>$errFile | Out-Null
Check ($LASTEXITCODE -eq 1 -and (Get-Content $errFile -Raw) -match 'TelaInteira') 'sem janela sai com 1 e sugere -TelaInteira'
Remove-Item $errFile

if ($fail) { exit 1 } else { 'OK'; exit 0 }
