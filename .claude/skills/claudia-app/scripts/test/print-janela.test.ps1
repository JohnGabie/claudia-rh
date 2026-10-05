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

$errFile = [IO.Path]::GetTempFileName()
& pwsh -NoProfile -File $script -Processo 'processo-que-nao-existe-xyz' -Saida $tmp 2>$errFile | Out-Null
Check ($LASTEXITCODE -eq 1 -and (Get-Content $errFile -Raw) -match 'TelaInteira') 'sem janela sai com 1 e sugere -TelaInteira'
Remove-Item $errFile

if ($fail) { exit 1 } else { 'OK'; exit 0 }
