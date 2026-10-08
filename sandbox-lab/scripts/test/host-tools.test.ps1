# Pure tests for the host toolchains mounted read-only (ambiente testes, spec addendum 9, 2026-10-08): the folder decision
# (travas.ps1), the rustup answer parser, the names list in compose.ps1, and static checks (parser only, nothing is run) that
# test-up decides on the very object it mounts and that the finder reads and never writes. Invented paths only (Y:\inventado).
#   pwsh -NoProfile -File sandbox-lab/scripts/test/host-tools.test.ps1
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../lib/travas.ps1"
. "$PSScriptRoot/../lib/compose.ps1"
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }

$UP = 'Y:\inventado\usuario'
$good = [ordered]@{
  rust    = 'Y:\inventado\usuario\.rustup\toolchains\1.95.0-x86_64-pc-windows-msvc'
  node    = 'Y:\inventado\pf\nodejs'
  msvc    = 'Y:\inventado\pf86\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC'
  winkits = 'Y:\inventado\pf86\Windows Kits\10'
}
function Refuse([string]$Nome, [string]$Path, $Ro = $true, [string[]]$Links = @(), [int]$Marker = 1, [string]$User = $UP) {
  return Get-LabHostToolRefusal -Nome $Nome -HostPath $Path -ReadOnly $Ro -UserProfile $User -Links $Links -MarkerCount $Marker
}

Check 'names: compose.ps1 and travas.ps1 list the same four tools, mounted at C:\hosttools\<name>' {
  if ((@($script:LabHostToolNames) -join ',') -ne 'rust,node,msvc,winkits') { throw "compose names: $($script:LabHostToolNames -join ',')" }
  if ((@($script:LabHostTools.Keys) -join ',') -ne 'rust,node,msvc,winkits') { throw "travas names: $(@($script:LabHostTools.Keys) -join ',')" }
  foreach ($n in $script:LabHostToolNames) { if ($script:LabHostTools[$n].sandbox -ne "C:\hosttools\$n") { throw "target of $n" } }
}
Check 'rustup answer: <toolchain>\bin\rustc.exe becomes the toolchain folder; anything else is null' {
  $tc = 'Y:\inventado\usuario\.rustup\toolchains\1.95.0-x86_64-pc-windows-msvc'
  if ((ConvertTo-LabRustToolchainDir "$tc\bin\rustc.exe") -ne $tc) { throw 'plain answer' }
  if ((ConvertTo-LabRustToolchainDir "  $tc\bin\rustc.exe`r`n") -ne $tc) { throw 'whitespace around the answer' }
  if ((ConvertTo-LabRustToolchainDir "$tc\BIN\RUSTC.EXE") -ne $tc) { throw 'case must not matter' }
  foreach ($bad in @($null, '', '   ', 'rustc.exe', '.\bin\rustc.exe', "$tc\bin\cargo.exe", "$tc\rustc.exe", "$tc\bin\rustc.exe.bak", '\\srv\share\bin\rustc.exe', "$tc\bin\rustc.exe`r`nY:\outro\bin\rustc.exe", 'bin\rustc.exe')) {
    if ($null -ne (ConvertTo-LabRustToolchainDir $bad)) { throw "must be null for '$bad'" }
  }
}
Check 'mount object: read-only and the C:\hosttools target; an unknown name has no target' {
  $m = New-LabHostToolMount -Nome 'node' -HostPath $good.node
  if ($m.readOnly -ne $true -or $m.sandbox -ne 'C:\hosttools\node' -or $m.host -ne $good.node -or $m.nome -ne 'node') { throw 'node mount' }
  if ($null -ne (New-LabHostToolMount -Nome 'perl' -HostPath 'Y:\x\perl').sandbox) { throw 'unknown name must have no target' }
}
Check 'allowed: each tool at its expected folder shape, read-only, no link, marker found' {
  foreach ($n in $good.Keys) {
    $why = Refuse $n $good[$n]
    if ($why) { throw "$n refused: $why" }
  }
}
Check 'refuse: empty, blank and relative paths' {
  foreach ($p in @($null, '', '   ', 'pf\nodejs', '.\nodejs', 'nodejs', '..\pf\nodejs', 'Y:nodejs', '\pf\nodejs')) {
    if (-not (Refuse 'node' $p)) { throw "must refuse '$p'" }
  }
}
Check 'refuse: a drive root, a UNC path and a path with . or .. segments' {
  foreach ($p in @('Y:\', 'Y:', 'y:\', '\\srv\share\nodejs', 'Y:\inventado\..\pf\nodejs', 'Y:\inventado\.\pf\nodejs')) {
    foreach ($n in $good.Keys) { if (-not (Refuse $n $p)) { throw "$n must refuse '$p'" } }
  }
}
Check 'refuse: the whole user folder and any folder that contains it' {
  foreach ($n in $good.Keys) {
    if ((Refuse $n $UP) -notmatch 'user folder') { throw "${n}: the user folder itself" }
    if ((Refuse $n "$UP\") -notmatch 'user folder') { throw "${n}: the user folder with a trailing slash" }
    if ((Refuse $n 'Y:\inventado') -notmatch 'user folder') { throw "${n}: the folder above the user folder" }
    if ((Refuse $n 'Y:\inventado\USUARIO') -notmatch 'user folder') { throw "${n}: case must not matter" }
  }
  if (-not (Refuse 'rust' $good.rust -User '')) { throw 'an unknown user folder must refuse' }
}
Check 'refuse: a link (junction, symlink, subst) in or above the path' {
  foreach ($n in $good.Keys) {
    if ((Refuse $n $good[$n] -Links @('Y:\inventado\pf')) -notmatch 'link') { throw "$n with a link above" }
    if ((Refuse $n $good[$n] -Links @($good[$n])) -notmatch 'link') { throw "$n with the folder itself a link" }
  }
  if (Refuse 'node' $good.node -Links @('', $null)) { throw 'empty link entries are not links' }
}
Check 'refuse: a mount that is not read-only (false, null, text, 1, missing)' {
  foreach ($n in $good.Keys) {
    foreach ($ro in @($false, $null, 'true', 'ReadOnly', 1, 0, '')) { if ((Refuse $n $good[$n] $ro) -notmatch 'read-only') { throw "$n with ReadOnly=$ro must refuse" } }
  }
  if ((Get-LabHostToolRefusal -Nome 'node' -HostPath $good.node -UserProfile $UP -MarkerCount 1) -notmatch 'read-only') { throw 'a missing ReadOnly argument must refuse (fail closed)' }
}
Check 'refuse: a rust toolchain without libstd (the stable of the PC was broken: libstd = 0)' {
  $why = Refuse 'rust' $good.rust -Marker 0
  if ($why -notmatch 'libstd') { throw "got '$why'" }
  if (Refuse 'rust' $good.rust -Marker 4) { throw '4 libstd files must pass' }
}
Check 'refuse: the other tools with none of their required files' {
  if ((Refuse 'node' $good.node -Marker 0) -notmatch 'node\.exe') { throw 'node' }
  if ((Refuse 'msvc' $good.msvc -Marker 0) -notmatch 'cl\.exe') { throw 'msvc' }
  if ((Refuse 'winkits' $good.winkits -Marker 0) -notmatch 'Include') { throw 'winkits' }
}
Check 'refuse: a folder that does not have the shape of the tool' {
  if (-not (Refuse 'rust' 'Y:\inventado\usuario\.rustup\outro\x')) { throw 'rust outside toolchains' }
  if (-not (Refuse 'rust' 'Y:\inventado\usuario\.rustup\toolchains')) { throw 'rust: the toolchains folder itself' }
  if (-not (Refuse 'node' 'Y:\inventado\pf\notnode')) { throw 'node' }
  if (-not (Refuse 'msvc' 'Y:\inventado\pf86\MSVC')) { throw 'msvc not under Tools' }
  if (-not (Refuse 'msvc' 'Y:\inventado\pf86\VC\Tools\MSVC\14.44.35207')) { throw 'msvc: a single version folder is not the MSVC folder' }
  if (-not (Refuse 'winkits' 'Y:\inventado\pf86\Windows Kits\11')) { throw 'winkits 11' }
  if (-not (Refuse 'winkits' 'Y:\inventado\pf86\Kits\10')) { throw 'winkits parent' }
}
Check 'refuse: an 8.3 short name hides the real folder' {
  if (-not (Refuse 'node' 'Y:\PROGRA~1\nodejs')) { throw 'PROGRA~1' }
  if (-not (Refuse 'rust' 'Y:\inventado\USUARI~1\.rustup\toolchains\x')) { throw 'USUARI~1' }
}
Check 'refuse: an unknown or empty tool name' {
  foreach ($n in @($null, '', ' ', 'perl', 'Rust', 'nodejs', 'python', 'cargo')) {
    if ((Get-LabHostToolRefusal -Nome $n -HostPath $good.node -ReadOnly $true -UserProfile $UP -MarkerCount 1) -notmatch 'unknown host tool') { throw "must refuse name '$n'" }
  }
}
Check 'messages never print the folder path' {
  $cases = @(
    (Refuse 'node' $good.node -Links @('Y:\inventado\pf')), (Refuse 'rust' $good.rust -Marker 0), (Refuse 'node' $UP), (Refuse 'winkits' 'Y:\inventado\pf86\Windows Kits\11'),
    (Refuse 'node' 'Y:\inventado\pf\nodejs' $false), (Refuse 'msvc' 'Y:\inventado\pf86\MSVC'), (Refuse 'node' 'Y:\PROGRA~1\nodejs'))
  foreach ($c in $cases) { if (-not $c) { throw 'a case that should refuse did not' }; if ($c -match 'inventado|PROGRA|usuario') { throw "the message prints a path: $c" } }
}

# --- static: parser only, nothing is executed ---
function Get-AstOf([string]$Text) {
  $tokens = $null; $errs = $null
  $ast = [System.Management.Automation.Language.Parser]::ParseInput($Text, [ref]$tokens, [ref]$errs)
  if ($errs.Count) { throw "parse errors: $($errs[0].Message)" }
  return $ast
}
function Get-FunctionCommands($Ast, [string]$Fn) {
  $f = @($Ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -ceq $Fn }, $true))
  if ($f.Count -ne 1) { throw "function $Fn not found exactly once" }
  return @($f[0].FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true) | ForEach-Object {
    $name = $_.GetCommandName()
    if ($name) { $name } else { '<invocation>' }
  })
}
$readOnlyCommands = @('Join-Path', 'Test-Path', 'Get-ChildItem', 'Get-Command', 'Select-Object', 'Where-Object', 'ForEach-Object', 'New-Object',
  'ConvertTo-LabRustToolchainDir', 'Get-LabRustcPathFromRustup', '<invocation>')
function Get-WriteCommands([string[]]$Commands) { return @($Commands | Where-Object { $readOnlyCommands -cnotcontains $_ }) }

$disco = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '../lib/disco.ps1'))
$up = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '../test-up.ps1'))
Check 'finder: Get-LabHostToolCandidate and the rustup call only read (parser allowlist of commands)' {
  $ast = Get-AstOf $disco
  foreach ($fn in @('Get-LabHostToolCandidate', 'Get-LabRustcPathFromRustup')) {
    $bad = Get-WriteCommands (Get-FunctionCommands $ast $fn)
    if ($bad.Count) { throw "$fn uses commands outside the read-only list: $($bad -join ', ')" }
  }
  # The same check must catch a write: a copy of the finder with a delete added is refused.
  $mut = $disco.Replace('  $pf = [Environment]::GetFolderPath(''ProgramFiles'')', '  Remove-Item -LiteralPath $ProjectDir -Recurse' + "`r`n" + '  $pf = [Environment]::GetFolderPath(''ProgramFiles'')')
  if ($mut -ceq $disco) { throw 'mutation did not apply' }
  if ((Get-WriteCommands (Get-FunctionCommands (Get-AstOf $mut) 'Get-LabHostToolCandidate')) -cnotcontains 'Remove-Item') { throw 'the allowlist did not catch Remove-Item' }
}
Check 'test-up: the refusal runs on the same object that becomes the mount, read-only comes from that object' {
  $a = [regex]::Match($up, 'Get-LabHostToolRefusal\s+-Nome\s+\$n\s+-HostPath\s+\$toolMount\.host\s+-ReadOnly\s+\$toolMount\.readOnly\b')
  if (-not $a.Success) { throw 'the refusal call does not use $toolMount.host and $toolMount.readOnly' }
  $b = [regex]::Match($up, 'foreach \(\$t in \$hostToolMounts\) \{ \$extra \+= New-LabMappedFolderXml \$t\.host \$t\.sandbox \$t\.readOnly \}')
  if (-not $b.Success) { throw 'the .wsb mount is not built from the validated object' }
  if ($a.Index -gt $b.Index) { throw 'the .wsb mount comes before the refusal' }
  $c = [regex]::Match($up, '\$hostToolMounts \+= \$toolMount')
  if (-not $c.Success -or $c.Index -lt $a.Index -or $c.Index -gt $b.Index) { throw 'a mount is added before its refusal, or not at all' }
  if ($up -notmatch '\$toolMount = New-LabHostToolMount -Nome \$n -HostPath \$realTool') { throw 'the mount does not come from New-LabHostToolMount on the real path' }
  if ($up -notmatch 'if \(\$toolWhy\) \{ throw ') { throw 'a refusal does not stop test-up' }
}
Check 'test-up: guest.config.json gets only the NAMES of the host tools, and no host-tool path is written anywhere' {
  if ($up -notmatch 'hostTools = @\(\$hostToolMounts \| ForEach-Object \{ \[string\]\$_\.nome \}\)') { throw 'guest.config.json hostTools is not the list of names' }
  # No line of test-up prints or logs the folder of a host tool.
  foreach ($line in ($up -split "`r?`n")) {
    if ($line -match 'Write-Host|Write-Output|Write-Warning|Out-File|Add-Content|Set-Content|WriteAllText' -and $line -match '\$toolMount\.host|\$realTool|\$cand\.HostPath|\$t\.host') { throw "a line prints a host tool path: $($line.Trim())" }
  }
}
Check 'static check catches its mutations (read-only flag flipped, refusal skipped, path written to the config)' {
  $m1 = $up.Replace('-ReadOnly $toolMount.readOnly', '-ReadOnly $true')
  if ($m1 -ceq $up -or [regex]::IsMatch($m1, 'Get-LabHostToolRefusal\s+-Nome\s+\$n\s+-HostPath\s+\$toolMount\.host\s+-ReadOnly\s+\$toolMount\.readOnly\b')) { throw 'flipped ReadOnly is not detected' }
  $m2 = $up.Replace('New-LabMappedFolderXml $t.host $t.sandbox $t.readOnly', 'New-LabMappedFolderXml $t.host $t.sandbox $false')
  if ($m2 -ceq $up -or [regex]::IsMatch($m2, 'New-LabMappedFolderXml \$t\.host \$t\.sandbox \$t\.readOnly')) { throw 'a writable mount is not detected' }
  $m3 = $up.Replace('[string]$_.nome })', '[string]$_.host })')
  if ($m3 -ceq $up -or [regex]::IsMatch($m3, 'hostTools = @\(\$hostToolMounts \| ForEach-Object \{ \[string\]\$_\.nome \}\)')) { throw 'host paths in guest.config.json are not detected' }
}

Check 'rustup call: the child process gets RUSTUP_AUTO_INSTALL=0 and no RUSTUP_TOOLCHAIN before it starts (so it only reads)' {
  $need = @('\$psi\.EnvironmentVariables\[''RUSTUP_AUTO_INSTALL''\]\s*=\s*''0''', '\$psi\.EnvironmentVariables\.Remove\(''RUSTUP_TOOLCHAIN''\)')
  function Get-RustupGaps([string]$Text) {
    $fn = @((Get-AstOf $Text).FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -ceq 'Get-LabRustcPathFromRustup' }, $true))
    if ($fn.Count -ne 1) { throw 'Get-LabRustcPathFromRustup not found exactly once' }
    $body = $fn[0].Extent.Text
    $start = $body.IndexOf('[System.Diagnostics.Process]::Start($psi)')
    if ($start -lt 0) { throw 'the Process::Start call is gone' }
    $before = $body.Substring(0, $start)
    return @($need | Where-Object { -not [regex]::IsMatch($before, $_) })
  }
  $gaps = Get-RustupGaps $disco
  if ($gaps.Count) { throw "missing before Process::Start: $($gaps -join ' ; ')" }
  $m1 = $disco.Replace('$psi.EnvironmentVariables[''RUSTUP_AUTO_INSTALL''] = ''0''', '')
  if ($m1 -ceq $disco -or (Get-RustupGaps $m1).Count -ne 1) { throw 'NOT detected: RUSTUP_AUTO_INSTALL line removed' }; Write-Host "    mutation 'RUSTUP_AUTO_INSTALL line removed' -> FAILED - missing before Process::Start: $((Get-RustupGaps $m1) -join ' ; ')"
  $m2 = $disco.Replace('$psi.EnvironmentVariables.Remove(''RUSTUP_TOOLCHAIN'')', '')
  if ($m2 -ceq $disco -or (Get-RustupGaps $m2).Count -ne 1) { throw 'NOT detected: RUSTUP_TOOLCHAIN removal removed' }; Write-Host "    mutation 'RUSTUP_TOOLCHAIN removal removed' -> FAILED - missing before Process::Start: $((Get-RustupGaps $m2) -join ' ; ')"
  $m3 = $disco.Replace('$psi.EnvironmentVariables[''RUSTUP_AUTO_INSTALL''] = ''0''', '$psi.EnvironmentVariables[''RUSTUP_AUTO_INSTALL''] = ''1''')
  if ($m3 -ceq $disco -or (Get-RustupGaps $m3).Count -ne 1) { throw 'NOT detected: RUSTUP_AUTO_INSTALL set to 1' }; Write-Host "    mutation 'RUSTUP_AUTO_INSTALL set to 1' -> FAILED - missing before Process::Start: $((Get-RustupGaps $m3) -join ' ; ')"
  # Setting them after the process started is as good as not setting them.
  $m4 = $disco.Replace('  $psi.EnvironmentVariables.Remove(''RUSTUP_TOOLCHAIN'')' + "`n", '').Replace('  $psi.EnvironmentVariables.Remove(''RUSTUP_TOOLCHAIN'')' + "`r`n", '').Replace('$errTask = $proc.StandardError', '$psi.EnvironmentVariables.Remove(''RUSTUP_TOOLCHAIN''); $errTask = $proc.StandardError')
  if ($m4 -ceq $disco -or (Get-RustupGaps $m4).Count -ne 1) { throw 'NOT detected: RUSTUP_TOOLCHAIN removed only after Start' }; Write-Host "    mutation 'RUSTUP_TOOLCHAIN removed only after Start' -> FAILED - missing before Process::Start: $((Get-RustupGaps $m4) -join ' ; ')"
}
Check 'test-up: the refusal call passes -Links (links above the REAL path), -MarkerCount from the finder, -UserProfile from the OS, and the path is the resolved one' {
  function Get-RefusalFacts([string]$Text) {
    $ast = Get-AstOf $Text
    $calls = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] -and $n.GetCommandName() -ceq 'Get-LabHostToolRefusal' }, $true))
    if ($calls.Count -ne 1) { throw "Get-LabHostToolRefusal is called $($calls.Count) times in test-up, expected 1" }
    $args = @{}
    $els = @($calls[0].CommandElements)
    for ($i = 0; $i -lt $els.Count; $i++) {
      if ($els[$i] -is [System.Management.Automation.Language.CommandParameterAst]) {
        $p = $els[$i].ParameterName
        $v = $(if ($els[$i].Argument) { $els[$i].Argument.Extent.Text } elseif ($i + 1 -lt $els.Count) { $els[$i + 1].Extent.Text } else { '' })
        $args[$p] = ($v -replace '\s+', ' ')
      }
    }
    $assign = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -ceq '$realTool' }, $true))
    $real = $(if ($assign.Count -eq 1) { ($assign[0].Right.Extent.Text -replace '\s+', ' ') } else { "<$($assign.Count) assignments>" })
    return @{ args = $args; real = $real }
  }
  $want = [ordered]@{
    Links       = '@(ConvertTo-LabLinkList (Get-LabLinksAboveRoot $realTool))'
    MarkerCount = '$cand.MarkerCount'
    UserProfile = '([Environment]::GetFolderPath(''UserProfile''))'
  }
  function Get-RefusalGaps([string]$Text) {
    $facts = Get-RefusalFacts $Text
    $gaps = @()
    foreach ($k in $want.Keys) { if ($facts.args[$k] -cne $want[$k]) { $gaps += "-$k is '$($facts.args[$k])'" } }
    if ($facts.real -cne 'Resolve-LabRealPath $cand.HostPath') { $gaps += "`$realTool comes from '$($facts.real)'" }
    return $gaps
  }
  $gaps = Get-RefusalGaps $up
  if ($gaps.Count) { throw ($gaps -join ' ; ') }
  $muts = [ordered]@{
    'no -Links'                      = @('-Links @(ConvertTo-LabLinkList (Get-LabLinksAboveRoot $realTool))', '')
    '-Links of the unresolved path'  = @('(Get-LabLinksAboveRoot $realTool)', '(Get-LabLinksAboveRoot $cand.HostPath)')
    '-Links without the list helper' = @('@(ConvertTo-LabLinkList (Get-LabLinksAboveRoot $realTool))', '@()')
    '-MarkerCount fixed'             = @('-MarkerCount $cand.MarkerCount', '-MarkerCount 1')
    '-UserProfile empty'             = @('([Environment]::GetFolderPath(''UserProfile''))', '''''')
    '$realTool not resolved'         = @('$realTool = Resolve-LabRealPath $cand.HostPath', '$realTool = $cand.HostPath')
  }
  foreach ($k in $muts.Keys) {
    $pair = $muts[$k]
    $m = $up.Replace($pair[0], $pair[1])
    if ($m -ceq $up) { throw "harness bug: mutation '$k' did not apply" }
    $g = Get-RefusalGaps $m
    if (-not $g.Count) { throw "NOT detected: $k" }
    Write-Host "    mutation '$k' -> FAILED - $($g -join ' ; ')"
  }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
