# Pure static test: only READS and PARSES the guest scripts (System.Management.Automation.Language.Parser), never runs any of them.
# The guard is checked by an ALLOW-LIST over the syntax tree, not by searching for text:
#   * the guard is a top-level IfStatementAst (not inside a function, a scriptblock or another condition),
#   * its condition is exactly  -not (Test-LabInGuest)  and its body is only Write-Host/Write-Output lines plus  exit 1,
#   * before it only these top-level statements may appear: the param block, simple assignments (no command, no method call,
#     no scriptblock, no subexpression), function definitions, and the dot-source of a guest-*.lib.ps1 library,
#   * no function of the script is called Test-LabInGuest.
# A statement of any other kind before the guard fails by name, so a future effect placed before it cannot slip through.
# The mutation checks run the same analysis on in-memory mutated text (ParseInput): nothing is written, nothing runs.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/guest-travas.test.ps1
# -LabRoot points the 8 guard checks at another copy of sandbox-lab (used to see the FAILED line of a mutated copy; parse only).
param([string]$LabRoot = '')
$ErrorActionPreference = 'Stop'
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }

$lab = Join-Path $PSScriptRoot '../..'
$scan = $(if ($LabRoot) { $LabRoot } else { $lab })
$guarded = @(
  'sandbox/instalar.ps1', 'sandbox/disco-z.ps1', 'sandbox/layout-msvc.ps1', 'sandbox/bootstrap-mcp.ps1',
  'sandbox/app/guest-run.ps1', 'sandbox/app/guest-cenario.ps1', 'sandbox/app/guest-build.ps1', 'sandbox/app/guest-firewall.ps1'
)

function Test-HasForbiddenNode($Node) {
  # Anything that could DO something inside an expression: a command, a method call, a scriptblock, a subexpression.
  $hit = $Node.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] -or $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] -or
    $n -is [System.Management.Automation.Language.ScriptBlockExpressionAst] -or $n -is [System.Management.Automation.Language.SubExpressionAst] -or
    ($n -is [System.Management.Automation.Language.PipelineAst] -and $n.PipelineElements.Count -gt 1) }, $true)
  return (@($hit).Count -gt 0)
}

function Test-IsSimpleAssignment($S) {
  if ($S -isnot [System.Management.Automation.Language.AssignmentStatementAst]) { return $false }
  if ($S.Left -isnot [System.Management.Automation.Language.VariableExpressionAst]) { return $false }
  if ($S.Operator -ne [System.Management.Automation.Language.TokenKind]::Equals) { return $false }
  return -not (Test-HasForbiddenNode $S.Right)
}

function Test-IsLibDotSource($S) {
  # . (Join-Path $PSScriptRoot '<optional ..\>guest-<name>.lib.ps1')   and nothing else
  if ($S -isnot [System.Management.Automation.Language.PipelineAst] -or $S.PipelineElements.Count -ne 1) { return $false }
  $c = $S.PipelineElements[0]
  if ($c -isnot [System.Management.Automation.Language.CommandAst] -or $c.InvocationOperator -ne [System.Management.Automation.Language.TokenKind]::Dot) { return $false }
  if ($c.CommandElements.Count -ne 1 -or $c.Redirections.Count -ne 0) { return $false }
  $p = $c.CommandElements[0]
  if ($p -isnot [System.Management.Automation.Language.ParenExpressionAst]) { return $false }
  $inner = $p.Pipeline
  if ($inner -isnot [System.Management.Automation.Language.PipelineAst] -or $inner.PipelineElements.Count -ne 1) { return $false }
  $jp = $inner.PipelineElements[0]
  if ($jp -isnot [System.Management.Automation.Language.CommandAst] -or "$($jp.GetCommandName())" -ne 'Join-Path' -or $jp.CommandElements.Count -ne 3) { return $false }
  $root = $jp.CommandElements[1]; $leaf = $jp.CommandElements[2]
  if ($root -isnot [System.Management.Automation.Language.VariableExpressionAst] -or $root.VariablePath.UserPath -ne 'PSScriptRoot') { return $false }
  if ($leaf -isnot [System.Management.Automation.Language.StringConstantExpressionAst]) { return $false }
  return ($leaf.Value -cmatch '^(\.\.\\)?guest-[a-z]+\.lib\.ps1$')
}

function Test-IsNotInGuestCondition($Cond) {
  # exactly:  -not (Test-LabInGuest)
  if ($Cond -isnot [System.Management.Automation.Language.PipelineAst] -or $Cond.PipelineElements.Count -ne 1) { return $false }
  $ce = $Cond.PipelineElements[0]
  if ($ce -isnot [System.Management.Automation.Language.CommandExpressionAst]) { return $false }
  $u = $ce.Expression
  if ($u -isnot [System.Management.Automation.Language.UnaryExpressionAst] -or $u.TokenKind -ne [System.Management.Automation.Language.TokenKind]::Not) { return $false }
  if ($u.Child -isnot [System.Management.Automation.Language.ParenExpressionAst]) { return $false }
  $inner = $u.Child.Pipeline
  if ($inner -isnot [System.Management.Automation.Language.PipelineAst] -or $inner.PipelineElements.Count -ne 1) { return $false }
  $cmd = $inner.PipelineElements[0]
  return ($cmd -is [System.Management.Automation.Language.CommandAst] -and "$($cmd.GetCommandName())" -ceq 'Test-LabInGuest' -and $cmd.CommandElements.Count -eq 1 -and $cmd.Redirections.Count -eq 0)
}

function Get-GuardShapeProblem($If) {
  # $null when $If is the guard: -not (Test-LabInGuest) { Write-Host/Write-Output ...; exit 1 } with no else
  if ($If.Clauses.Count -ne 1 -or $If.ElseClause) { return 'if with elseif/else' }
  if (-not (Test-IsNotInGuestCondition $If.Clauses[0].Item1)) { return "condition is not exactly '-not (Test-LabInGuest)': $($If.Clauses[0].Item1.Extent.Text)" }
  $body = @($If.Clauses[0].Item2.Statements)
  if ($body.Count -lt 2) { return 'body must print and then exit 1' }
  for ($i = 0; $i -lt $body.Count - 1; $i++) {
    $s = $body[$i]
    $ok = $s -is [System.Management.Automation.Language.PipelineAst] -and $s.PipelineElements.Count -eq 1 -and $s.PipelineElements[0] -is [System.Management.Automation.Language.CommandAst]
    if ($ok) {
      $c = $s.PipelineElements[0]
      $name = "$($c.GetCommandName())"
      $ok = (($name -ceq 'Write-Host') -or ($name -ceq 'Write-Output')) -and $c.Redirections.Count -eq 0 -and $c.InvocationOperator -eq [System.Management.Automation.Language.TokenKind]::Unknown
      if ($ok) { $ok = -not (@($c.CommandElements | Select-Object -Skip 1 | Where-Object { Test-HasForbiddenNode $_ }).Count) }
    }
    if (-not $ok) { return "body statement $($i + 1) is not a plain Write-Host/Write-Output: $($s.Extent.Text)" }
  }
  $last = $body[$body.Count - 1]
  if ($last -isnot [System.Management.Automation.Language.ExitStatementAst]) { return "last body statement is not 'exit 1': $($last.Extent.Text)" }
  $v = $last.Pipeline
  if ($v -isnot [System.Management.Automation.Language.PipelineAst] -or $v.PipelineElements.Count -ne 1 -or $v.PipelineElements[0] -isnot [System.Management.Automation.Language.CommandExpressionAst]) { return 'exit without a plain value' }
  $e = $v.PipelineElements[0].Expression
  if ($e -isnot [System.Management.Automation.Language.ConstantExpressionAst] -or $e.Value -ne 1) { return "exit code is not 1: $($last.Extent.Text)" }
  return $null
}

function Get-LabGuardProblem($Ast) {
  # $null when the guard is first, alone and well formed; otherwise the reason (with the line).
  foreach ($blk in 'BeginBlock', 'ProcessBlock', 'DynamicParamBlock') { if ($Ast.$blk) { return "script uses a named block ($blk): the guard cannot be proven first" } }
  $fn = @($Ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -ieq 'Test-LabInGuest' }, $true))
  if ($fn.Count) { return "the script defines its own Test-LabInGuest (line $($fn[0].Extent.StartLineNumber)): it would replace the real guard" }
  $stmts = @()
  if ($Ast.EndBlock) { $stmts = @($Ast.EndBlock.Statements) }
  $dot = $false
  foreach ($s in $stmts) {
    if ($s -is [System.Management.Automation.Language.IfStatementAst] -and -not (Get-GuardShapeProblem $s)) {
      if (-not $dot) { return "the guest-install.lib.ps1 library is not dot-sourced before the guard (line $($s.Extent.StartLineNumber))" }
      return $null
    }
    if (Test-IsSimpleAssignment $s) { continue }
    if ($s -is [System.Management.Automation.Language.FunctionDefinitionAst]) { continue }
    if (Test-IsLibDotSource $s) { if ($s.Extent.Text -like '*guest-install.lib.ps1*') { $dot = $true }; continue }
    $why = $(if ($s -is [System.Management.Automation.Language.IfStatementAst]) { ": $(Get-GuardShapeProblem $s)" } else { '' })
    return "line $($s.Extent.StartLineNumber) is before the guard and is not allowed (only param, simple assignments, function definitions, library dot-source)$why -> $($s.Extent.Text.Split("`n")[0].Trim())"
  }
  return 'no top-level guard: if (-not (Test-LabInGuest)) { Write-Host ...; exit 1 }'
}

function Read-Ast([string]$Path) {
  $tokens = $null; $errs = $null
  $ast = [System.Management.Automation.Language.Parser]::ParseFile($Path, [ref]$tokens, [ref]$errs)
  if (@($errs).Count) { throw "$Path does not parse: $($errs[0].Message)" }
  return $ast
}
function Read-AstText([string]$Text) {
  $tokens = $null; $errs = $null
  $ast = [System.Management.Automation.Language.Parser]::ParseInput($Text, [ref]$tokens, [ref]$errs)
  if (@($errs).Count) { throw "mutated text does not parse: $($errs[0].Message)" }
  return $ast
}

foreach ($rel in $guarded) {
  Check "$rel : the guard is a top-level if, first, exactly -not (Test-LabInGuest) with print + exit 1" {
    $path = Join-Path $scan $rel
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "script not found: $rel" }
    $why = Get-LabGuardProblem (Read-Ast $path)
    if ($why) { throw $why }
  }
}

# --- Mutations (in memory, parser only): each one must be reported as a problem, by name. ---
$sample = Get-Content -LiteralPath (Join-Path $lab 'sandbox/app/guest-cenario.ps1') -Raw
$guardLine = "if (-not (Test-LabInGuest)) { Write-Host 'not inside the lab guest (needs the guest identity and the bootstrap marker)'; exit 1 }"
Check 'mutation harness: the real sample is clean and contains the guard line it mutates' {
  if ($sample.IndexOf($guardLine) -lt 0) { throw 'guard line of the sample not found (update the mutation harness)' }
  if (Get-LabGuardProblem (Read-AstText $sample)) { throw 'the unmutated sample must pass' }
}
$mutants = [ordered]@{
  'guard inside if ($false)'               = $sample.Replace($guardLine, "if (`$false) { $guardLine }")
  'guard inside an assigned scriptblock'   = $sample.Replace($guardLine, "`$g = { $guardLine }")
  'inverted logic, exit 1 only in a comment' = $sample.Replace($guardLine, "if (Test-LabInGuest) { Write-Host 'x' } # exit 1")
  'Remove-Item before the guard'           = $sample.Replace($guardLine, "Remove-Item -LiteralPath 'Y:\inventado\x'`n$guardLine")
  'script function call (Log) before the guard' = $sample.Replace($guardLine, "function Log([string]`$m) { }`nLog 'x'`n$guardLine")
  'own Test-LabInGuest redefined'          = $sample.Replace($guardLine, "function Test-LabInGuest { `$true }`n$guardLine")
  'guard condition without -not'           = $sample.Replace($guardLine, "if ((Test-LabInGuest)) { Write-Host 'x'; exit 1 }")
  'guard without exit'                     = $sample.Replace($guardLine, "if (-not (Test-LabInGuest)) { Write-Host 'x' }")
  'guard exits 0'                          = $sample.Replace($guardLine, "if (-not (Test-LabInGuest)) { Write-Host 'x'; exit 0 }")
  'guard with an extra effect in the body' = $sample.Replace($guardLine, "if (-not (Test-LabInGuest)) { New-Item -ItemType Directory 'Y:\inventado\x'; exit 1 }")
  'guard with else'                        = $sample.Replace($guardLine, "if (-not (Test-LabInGuest)) { Write-Host 'x'; exit 1 } else { Write-Host 'y' }")
  'method call assigned before the guard'  = $sample.Replace($guardLine, "`$t = [IO.Path]::GetTempPath()`n$guardLine")
  'guard missing'                          = $sample.Replace($guardLine, '')
  'guard only in a function'               = $sample.Replace($guardLine, "function Test-X { $guardLine }")
}
foreach ($k in $mutants.Keys) {
  Check "mutation must be caught: $k" {
    $text = $mutants[$k]
    if ($text -ceq $sample) { throw 'the mutation did not change the text (harness bug)' }
    $why = Get-LabGuardProblem (Read-AstText $text)
    if (-not $why) { throw 'NOT detected: the analysis accepted the mutated script' }
  }
}

# Wirings: a refactor that drops one of these would not fail any other pure test.
$js = Get-Content -LiteralPath (Join-Path $lab '../.claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs') -Raw
$up = Get-Content -LiteralPath (Join-Path $lab 'scripts/test-up.ps1') -Raw
$cen = Get-Content -LiteralPath (Join-Path $lab 'sandbox/app/guest-cenario.ps1') -Raw
Check 'testes-no-guest.mjs and guest-cenario.ps1 ask only for node (-Precisa node)' {
  if ($js -notmatch '-Precisa node') { throw 'testes-no-guest.mjs does not pass -Precisa node' }
  if ($cen -notmatch 'Get-LabGuestToolEnv\s+\$inst\s+-Precisa node') { throw 'guest-cenario.ps1 does not use -Precisa node' }
}
$bld = Get-Content -LiteralPath (Join-Path $lab 'sandbox/app/guest-build.ps1') -Raw
function Get-HostToolsWiringProblem([string]$Build, [string]$Scenario) {
  # The tools of guest-build/guest-cenario come from Get-LabGuestToolEnv with the host layout (the PC toolchains mounted
  # read-only, spec addendum 9); the variables it returns are applied; cargo home and target stay on the guest disk.
  if ($Build -notmatch 'Get-LabHostToolsLayout') { return 'guest-build.ps1 does not read the host tools layout' }
  if ($Build -notmatch 'Get-LabGuestToolEnv\s+\$inst\s+-Precisa\s+\$Precisa\s+-HostLayout\s+\$hostLayout') { return 'guest-build.ps1 does not pass -HostLayout to Get-LabGuestToolEnv' }
  if ($Scenario -notmatch 'Get-LabHostToolsLayout') { return 'guest-cenario.ps1 does not read the host tools layout' }
  if ($Scenario -notmatch 'Get-LabGuestToolEnv\s+\$inst\s+-Precisa node\s+-HostLayout\s+\$hostLayout') { return 'guest-cenario.ps1 does not pass -HostLayout to Get-LabGuestToolEnv' }
  if ($Build -notmatch 'foreach \(\$k in @\(\$toolEnv\.env\.Keys\)\) \{ Set-Item -Path "Env:\$k" -Value') { return 'guest-build.ps1 does not apply the environment variables of the tools (INCLUDE, LIB...)' }
  if ($Build -notmatch "\`$env:CARGO_HOME = 'C:\\labdata\\cargo-home'") { return 'CARGO_HOME is not on the guest disk (C:\labdata)' }
  if ($Build -notmatch "\`$env:CARGO_TARGET_DIR = 'C:\\labdata\\cargo-target'") { return 'CARGO_TARGET_DIR is not on the guest disk (C:\labdata)' }
  if ($Build -match '(CARGO_HOME|CARGO_TARGET_DIR|RUSTUP_HOME|npm_config_cache)[^\r\n]*hosttools') { return 'a cargo/npm folder points into the read-only host tools' }
  return $null
}
Check 'guest-build.ps1 and guest-cenario.ps1 use the PC toolchains (hostTools) through Get-LabGuestToolEnv; cargo home and target stay on the guest disk' {
  $why = Get-HostToolsWiringProblem $bld $cen
  if ($why) { throw $why }
  # Mutations of the same check, in memory (nothing is written).
  $mut = [ordered]@{
    'layout not read'          = @($bld.Replace('Get-LabHostToolsLayout', 'Get-Date'), $cen)
    'layout not passed'        = @($bld.Replace(' -HostLayout $hostLayout', ''), $cen)
    'scenario ignores the host' = @($bld, $cen.Replace(' -HostLayout $hostLayout', ''))
    'env not applied'          = @($bld.Replace('foreach ($k in @($toolEnv.env.Keys)) { Set-Item', 'foreach ($k in @()) { Set-Item'), $cen)
    'cargo home on the host'   = @($bld.Replace("`$env:CARGO_HOME = 'C:\labdata\cargo-home'", "`$env:CARGO_HOME = 'C:\hosttools\rust\cargo-home'"), $cen)
    'cargo target on the host' = @($bld.Replace("`$env:CARGO_TARGET_DIR = 'C:\labdata\cargo-target'", "`$env:CARGO_TARGET_DIR = 'C:\hosttools\target'"), $cen)
  }
  foreach ($k in $mut.Keys) {
    if ($mut[$k][0] -ceq $bld -and $mut[$k][1] -ceq $cen) { throw "mutation '$k' changed nothing (harness bug)" }
    if (-not (Get-HostToolsWiringProblem $mut[$k][0] $mut[$k][1])) { throw "mutation '$k' NOT detected" }
  }
}
Check 'test-up.ps1 wires the account check, the links check (with targets), the firewall flag and the early credential removal' {
  if ($up -notmatch 'Get-LabUpVerdict[^\r\n]*-PrecisaConta \$conta') { throw 'Get-LabUpVerdict without -PrecisaConta $conta' }
  if ($up -notmatch '\$inside = @\(ConvertTo-LabLinkList \(Get-LabReparsePoints \$src\)\)[\r\n]') { throw '$inside = @(ConvertTo-LabLinkList (Get-LabReparsePoints $src)) not found' }
  if ($up -notmatch 'Get-LabRealDataSourceRefusal[^\r\n]*-LinksInside \$inside -FileLinksInside @\(ConvertTo-LabLinkList \(Get-LabFileLinks \$inside\)\) -LinkTargets \(Get-LabLinkTargets \$inside\)') { throw 'the real-data check does not get -LinksInside, -FileLinksInside and -LinkTargets' }
  if ($up -notmatch 'internetBloqueada = \[bool\]\(Test-LabAmbienteInternetBlocked -Compose \$compose -Ambiente \$Ambiente\)') { throw 'guest.config.json does not carry internetBloqueada' }
  if ($up -notmatch 'if \(Test-LabCredentialRemovableNow [^\r\n]*\{') { throw 'the early removal does not use Test-LabCredentialRemovableNow' }
  if ($up -notmatch 'Test-LabCredentialRemovableNow[^\r\n]*\{\s*[\r\n]+\s*Remove-LabCredential') { throw 'Remove-LabCredential does not follow the condition' }
}
Check 'guest-run.ps1: the app starts only after Get-LabAppStartRefusal says no, reading guest.config.json and firewall.txt' {
  $ast = Read-Ast (Join-Path $lab 'sandbox/app/guest-run.ps1')
  $cmds = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true))
  $ref = @($cmds | Where-Object { "$($_.GetCommandName())" -eq 'Get-LabAppStartRefusal' } | Select-Object -First 1)
  $start = @($cmds | Where-Object { "$($_.GetCommandName())" -eq 'Start-Process' } | Select-Object -First 1)
  if (-not $ref) { throw 'Get-LabAppStartRefusal is not called' }
  if (-not $start) { throw 'Start-Process not found (the check would prove nothing)' }
  if ($ref[0].Extent.StartOffset -gt $start[0].Extent.StartOffset) { throw 'the refusal is decided after the app was started' }
  $ifs = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.IfStatementAst] -and $n.Extent.StartOffset -lt $start[0].Extent.StartOffset -and $n.Extent.EndOffset -gt $start[0].Extent.EndOffset }, $true))
  $gated = @($ifs | Where-Object { $_.Clauses[0].Item1.Extent.Text -match '-not \$refusal' })
  if (-not $gated.Count) { throw 'Start-Process is not inside an if (... -not $refusal)' }
  $t = $ast.Extent.Text
  if ($t -notmatch 'guest\.config\.json' -or $t -notmatch "'firewall\.txt'") { throw 'guest-run.ps1 does not read guest.config.json and firewall.txt' }
  if ($t -notmatch 'Get-LabGuestInternetPermitida') { throw 'guest-run.ps1 does not decide the internet from the config' }
}

# Link lists: 'return ,$list' in the functions below once made @(Get-LabLinksInPath ...).Count 1 with no link (test-up refused every
# mount). Rule over the syntax tree, nothing is run: (1) every call of these functions sits inside ConvertTo-LabLinkList, so no
# .Count/[0]/+ ever sees the raw return; (2) their definitions never use the unary comma (a one-element array literal) (return ,$x / ,@(...)).
$linkFns = @('Get-LabLinksInPath', 'Get-LabLinksAboveRoot', 'Get-LabReparsePoints', 'Get-LabFileLinks')
function Get-LinkListProblems([string]$Text, [string]$Label) {
  $tok = $null; $err = $null
  $ast = [System.Management.Automation.Language.Parser]::ParseInput($Text, [ref]$tok, [ref]$err)
  if ($err.Count) { return @("${Label}: does not parse: $($err[0].Message)") }
  $problems = New-Object System.Collections.Generic.List[string]
  $lines = $Text -split "`r?`n"   # a call may stay raw only with the marker comment at the end of its line (the Sandbox regression test)
  $calls = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] -and $linkFns -contains "$($n.GetCommandName())" }, $true))
  foreach ($c in $calls) {
    $p = $c.Parent; $wrapped = $false
    while ($p) { if ($p -is [System.Management.Automation.Language.CommandAst] -and "$($p.GetCommandName())" -eq 'ConvertTo-LabLinkList') { $wrapped = $true; break }; $p = $p.Parent }
    if (-not $wrapped -and $lines[$c.Extent.StartLineNumber - 1] -notmatch '# raw-call-on-purpose\s*$') { $problems.Add("${Label}:$($c.Extent.StartLineNumber): $($c.GetCommandName()) is used without ConvertTo-LabLinkList (raw return)") }
  }
  $defs = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $linkFns -contains $n.Name }, $true))
  foreach ($d in $defs) {
    $commas = @($d.FindAll({ param($n) $n -is [System.Management.Automation.Language.ArrayLiteralAst] -and $n.Elements.Count -eq 1 }, $true))   # ,$x parses as a one-element array literal
    foreach ($u in $commas) { $problems.Add("${Label}:$($u.Extent.StartLineNumber): $($d.Name) wraps a value in a unary comma (return `,`$x) ($($u.Extent.Text))") }
  }
  return @(, @{ Problems = $problems.ToArray(); Calls = $calls.Count; Defs = $defs.Count })
}
$linkFiles = @(Get-ChildItem -LiteralPath (Join-Path $scan 'scripts'), (Join-Path $scan 'scripts/lib'), (Join-Path $scan 'scripts/test'), (Join-Path $scan 'sandbox'), (Join-Path $scan 'sandbox/app') -Filter *.ps1 |
  Where-Object { $_.Name -notlike '*.test.ps1' })
Check 'link lists: every call of the link finders goes through ConvertTo-LabLinkList and none returns a comma-wrapped list' {
  $all = New-Object System.Collections.Generic.List[string]; $calls = 0; $defs = 0
  foreach ($f in $linkFiles) {
    $r = Get-LinkListProblems ([IO.File]::ReadAllText($f.FullName)) $f.Name
    if ($r -is [string] -or $r -is [array] -and $r.Count -and $r[0] -is [string]) { $all.Add([string]$r[0]); continue }
    foreach ($x in $r.Problems) { $all.Add($x) }; $calls += $r.Calls; $defs += $r.Defs
  }
  if ($all.Count) { throw ($all -join ' | ') }
  if ($calls -lt 10) { throw "only $calls call(s) found: the scan is not looking at the scripts" }
  if ($defs -ne 4) { throw "expected the 4 definitions in disco.ps1, found $defs" }
}
Check 'link lists: the check fails on the old comma return and on a raw .Count (mutations, in memory)' {
  $disco = [IO.File]::ReadAllText((Join-Path $scan 'scripts/lib/disco.ps1'))
  $up = [IO.File]::ReadAllText((Join-Path $scan 'scripts/test-up.ps1'))
  if ((Get-LinkListProblems $disco 'disco').Problems.Count) { throw 'the unmutated disco.ps1 must be clean' }
  $m1 = $disco.Replace('return $links ', 'return ,$links ')
  if ($m1 -eq $disco) { throw 'mutation 1 did not change the text' }
  if (-not (Get-LinkListProblems $m1 'disco').Problems.Count) { throw 'return ,$links was not caught' }
  $good = '@(ConvertTo-LabLinkList (Get-LabLinksInPath $h $RepoRoot))'
  if (-not $up.Contains($good)) { throw 'test-up.ps1 call not found' }
  $m2 = $up.Replace($good, '@(Get-LabLinksInPath $h $RepoRoot)')
  $p2 = (Get-LinkListProblems $m2 'test-up').Problems
  if (-not $p2.Count -or $p2[0] -notmatch 'without ConvertTo-LabLinkList') { throw 'a raw @(Get-LabLinksInPath ...).Count was not caught' }
}
# disco-z.ps1 wiring (static, nothing is run): the diskpart input is only ever produced by the two pure builders, the format
# comes after the refusal check, and no other way to touch a disk exists in the script. Mutations are parsed in memory.
function Get-DiscoZProblems($Ast) {
  $p = New-Object System.Collections.Generic.List[string]
  $cmds = @($Ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true))
  $dp = @($cmds | Where-Object { "$($_.GetCommandName())" -ieq 'Invoke-DiskPart' })
  if (-not $dp.Count) { $p.Add('no Invoke-DiskPart call found (the check would prove nothing)') }
  $createCall = $null; $formatCall = $null
  foreach ($c in $dp) {
    $arg = @($c.CommandElements | Select-Object -Skip 1)
    $inner = $null
    if ($arg.Count -eq 1 -and $arg[0] -is [System.Management.Automation.Language.ParenExpressionAst]) {
      $pl = $arg[0].Pipeline
      if ($pl -is [System.Management.Automation.Language.PipelineAst] -and $pl.PipelineElements.Count -eq 1 -and $pl.PipelineElements[0] -is [System.Management.Automation.Language.CommandAst]) { $inner = $pl.PipelineElements[0] }
    }
    $name = $(if ($inner) { "$($inner.GetCommandName())" } else { '' })
    if ($name -ceq 'Get-LabZCreateDiskPart') { if ($createCall) { $p.Add("line $($c.Extent.StartLineNumber): second create call") }; $createCall = $c }
    elseif ($name -ceq 'Get-LabZFormatDiskPart' -and $inner.CommandElements.Count -eq 1) { if ($formatCall) { $p.Add("line $($c.Extent.StartLineNumber): second format call") }; $formatCall = $c }
    else { $p.Add("line $($c.Extent.StartLineNumber): Invoke-DiskPart gets something other than Get-LabZCreateDiskPart / Get-LabZFormatDiskPart: $($c.Extent.Text)") }
  }
  if (-not $createCall) { $p.Add('Invoke-DiskPart (Get-LabZCreateDiskPart ...) not found') }
  if (-not $formatCall) { $p.Add('Invoke-DiskPart (Get-LabZFormatDiskPart) not found') }
  if ($createCall -and $formatCall) {
    $stmtOf = { param($n) $x = $n; while ($x.Parent -and $x.Parent -isnot [System.Management.Automation.Language.StatementBlockAst]) { $x = $x.Parent }; $x }
    $cs = & $stmtOf $createCall; $fs = & $stmtOf $formatCall
    if ($cs.Parent -ne $fs.Parent -or $cs.Parent -isnot [System.Management.Automation.Language.StatementBlockAst]) { $p.Add('create and format are not statements of the same block') }
    else {
      $st = @($cs.Parent.Statements); $ci = [array]::IndexOf($st, $cs); $fi = [array]::IndexOf($st, $fs)
      $ifi = -1
      for ($i = $ci + 1; $i -lt $fi; $i++) {
        $s = $st[$i]
        if ($s -is [System.Management.Automation.Language.IfStatementAst] -and $s.Clauses.Count -eq 1 -and -not $s.ElseClause -and $s.Clauses[0].Item1.Extent.Text -ceq '$s.format' -and @($s.Clauses[0].Item2.FindAll({ param($n) $n -is [System.Management.Automation.Language.ThrowStatementAst] }, $true)).Count -eq 1) { $ifi = $i }
      }
      if ($ci -ge $fi) { $p.Add('format comes before create') }
      elseif ($ifi -lt 0) { $p.Add('no  if ($s.format) { throw ... }  between create and format') }
      else {
        $fresh = $false
        for ($i = $ci + 1; $i -lt $ifi; $i++) { if ($st[$i].Extent.Text -match '^\$s\s*=\s*Get-State\s*$') { $fresh = $true } }
        if (-not $fresh) { $p.Add('the state is not re-read ($s = Get-State) between create and the refusal check') }
      }
    }
  }
  $bad = @('Format-Volume', 'Clear-Disk', 'Initialize-Disk', 'New-Partition', 'Remove-Partition', 'Resize-Partition', 'Set-Partition', 'Set-Disk')
  foreach ($c in $cmds) { if ($bad -contains "$($c.GetCommandName())") { $p.Add("line $($c.Extent.StartLineNumber): forbidden command $($c.GetCommandName())") } }
  $strs = @($Ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.StringConstantExpressionAst] -or $n -is [System.Management.Automation.Language.ExpandableStringExpressionAst] }, $true))
  foreach ($s in $strs) { if ("$($s.Extent.Text)" -match '(?i)select\s+(disk|volume)|(^|[\s''"`])clean\b|\bformat\s+fs=') { $p.Add("line $($s.Extent.StartLineNumber): literal diskpart command in the script: $($s.Extent.Text)") } }
  return @($p)
}
$discoPath = Join-Path $scan 'sandbox/disco-z.ps1'
Check 'disco-z.ps1: every Invoke-DiskPart gets only the pure builders; format only after the refusal check; no other disk command' {
  $why = @(Get-DiscoZProblems (Read-Ast $discoPath))
  if ($why.Count) { throw ($why -join ' | ') }
}
$discoText = ([IO.File]::ReadAllText($discoPath)) -replace "`r`n", "`n"
$fmtCall = 'Invoke-DiskPart (Get-LabZFormatDiskPart)'
$refuse = '    if ($s.format) { throw "nothing formatted: $($s.format)" }'
$createCallText = 'Invoke-DiskPart (Get-LabZCreateDiskPart -MaxMB $MaxMB)'
Check 'disco-z mutation harness: the real script has the lines the mutations replace' {
  foreach ($x in @($fmtCall, $refuse.Trim(), $createCallText, ('    $s = Get-State' + "`n" + $refuse))) { if ($discoText.IndexOf($x) -lt 0) { throw "line not found (update the harness): $x" } }
}
$discoMutants = [ordered]@{
  'format before the refusal check'        = $discoText.Replace($refuse + "`n", '').Replace($fmtCall, "$fmtCall`n    if (`$s.format) { throw 'late' }")
  'refusal check does not throw'           = $discoText.Replace($refuse.Trim(), 'if ($s.format) { Write-Host "ignored" }')
  'format without any check'               = $discoText.Replace($refuse + "`n", '')
  'Format-Volume directly'                 = $discoText.Replace($fmtCall, "'Z' | Format-Volume -FileSystem NTFS")
  'literal select disk 1'                  = $discoText.Replace($createCallText, "Invoke-DiskPart @('select disk 1', 'clean')")
  'literal select disk in the format call' = $discoText.Replace($fmtCall, "Invoke-DiskPart @('select disk 1', 'format fs=ntfs quick')")
  'Clear-Disk added'                       = $discoText.Replace($createCallText, "$createCallText`n    Clear-Disk -Number 1 -RemoveData")
  'state not re-read before the check'     = $discoText.Replace(('    $s = Get-State' + "`n" + $refuse), $refuse)
}
foreach ($k in $discoMutants.Keys) {
  Check "disco-z mutation must be caught: $k" {
    $text = $discoMutants[$k]
    if ($text -ceq $discoText) { throw 'the mutation did not change the text (harness bug)' }
    if (-not @(Get-DiscoZProblems (Read-AstText $text)).Count) { throw 'NOT detected: the analysis accepted the mutated script' }
  }
}
# Native pipes: 'native-command | Select-Object -First N' stops the native process early; in Windows PowerShell 5.1 $LASTEXITCODE
# then becomes -1 although the tool worked (Node in the Sandbox, 2026-10-08). Rule over the syntax tree, nothing is run: in the
# guest scripts no Select-Object -First/-Last/-Skip... sits directly after a native command (& <x>, cmd, cmd.exe, anything *.exe).
# The fix is to capture the whole output first (ConvertTo-LabVersionProbe in guest-install.lib.ps1).
function Get-NativeTruncationProblems([string]$Text, [string]$Label) {
  $tok = $null; $err = $null
  $ast = [System.Management.Automation.Language.Parser]::ParseInput($Text, [ref]$tok, [ref]$err)
  if ($err.Count) { return @("${Label}: does not parse: $($err[0].Message)") }
  $problems = New-Object System.Collections.Generic.List[string]
  $pipes = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.PipelineAst] -and $n.PipelineElements.Count -gt 1 }, $true))
  foreach ($pl in $pipes) {
    for ($i = 1; $i -lt $pl.PipelineElements.Count; $i++) {
      $sel = $pl.PipelineElements[$i]; $prev = $pl.PipelineElements[$i - 1]
      if ($sel -isnot [System.Management.Automation.Language.CommandAst] -or "$($sel.GetCommandName())" -ine 'Select-Object') { continue }
      $trunc = @($sel.CommandElements | Where-Object { $_ -is [System.Management.Automation.Language.CommandParameterAst] -and $_.ParameterName -match '^(?i)(f|fi|fir|firs|first|l|la|las|last)$' })
      if (-not $trunc.Count) { continue }
      if ($prev -isnot [System.Management.Automation.Language.CommandAst]) { continue }
      $name = "$($prev.GetCommandName())"
      $native = ($prev.InvocationOperator -eq [System.Management.Automation.Language.TokenKind]::Ampersand -and -not $name) -or
        ($name -match '^(?i)cmd(\.exe)?$') -or ($name -match '(?i)\.exe$')
      if ($native) { $problems.Add("${Label}:$($sel.Extent.StartLineNumber): Select-Object -$($trunc[0].ParameterName) directly after a native command ($($prev.Extent.Text)): the pipe is cut and `$LASTEXITCODE becomes -1; capture the whole output first") }
    }
  }
  return @($problems)
}
$nativeFiles = @(Get-ChildItem -LiteralPath (Join-Path $scan 'sandbox'), (Join-Path $scan 'sandbox/app') -Filter *.ps1)
Check 'native pipes: no Select-Object -First/-Last directly after a native command in sandbox/*.ps1 and sandbox/app/*.ps1' {
  $all = New-Object System.Collections.Generic.List[string]
  foreach ($f in $nativeFiles) { foreach ($x in @(Get-NativeTruncationProblems ([IO.File]::ReadAllText($f.FullName)) $f.Name)) { $all.Add([string]$x) } }
  if ($all.Count) { throw ($all -join ' | ') }
  if ($nativeFiles.Count -lt 10) { throw "only $($nativeFiles.Count) script(s) scanned: the scan is not looking at the scripts" }
}
$nativeInst = [IO.File]::ReadAllText((Join-Path $scan 'sandbox/instalar.ps1'))
$nativeMutants = [ordered]@{
  'node: old pipe with Select-Object -First 1'   = @('@(& $node.FullName --version 2>&1)', '(& $node.FullName --version 2>&1 | Select-Object -First 1)')
  'claude: old pipe with Select-Object -First 1' = @('@(& "$d\claude.exe" --version 2>&1)', '(& "$d\claude.exe" --version 2>&1 | Select-Object -First 1)')
  'rust: old pipe with Select-Object -First 1'   = @('@(& $rustc.FullName --version 2>&1)', '(& $rustc.FullName --version 2>&1 | Select-Object -First 1)')
  'msvc: cmd.exe pipe with Select-Object -First 1' = @('where cl.exe" 2>&1); $code', 'where cl.exe" 2>&1 | Select-Object -First 1); $code')
}
if ((Get-NativeTruncationProblems $nativeInst 'instalar.ps1').Count) { Check 'native pipes mutations need a clean instalar.ps1' { throw 'instalar.ps1 is not clean' } }
foreach ($k in $nativeMutants.Keys) {
  Check "native pipes mutation must be caught: $k" {
    $m = $nativeMutants[$k]
    $text = $nativeInst.Replace($m[0], $m[1])
    if ($text -ceq $nativeInst) { throw 'the mutation did not change the text (harness bug)' }
    if (-not @(Get-NativeTruncationProblems $text 'instalar.ps1').Count) { throw 'NOT detected: the analysis accepted the mutated script' }
  }
}
Check 'native pipes: Select-Object -First after a non-native source stays allowed' {
  $ok = '$a = Get-ChildItem x | Select-Object -First 1; $b = @(1,2) | Select-Object -First 1'
  if ((Get-NativeTruncationProblems $ok 'ok').Count) { throw 'false positive on a cmdlet pipe' }
}
# Wiring of the copia fixes (parser only): guest-run.ps1 asks for a valid cenario.json BEFORE it copies any scenario data, and
# guest-cenario.ps1 hands the PC's diagnostics\latest decision to cenario-copia.mjs through the guest.config.json fact.
function Get-WiringProblems([string]$RunText, [string]$CenText) {
  $p = New-Object System.Collections.Generic.List[string]
  $t = $null; $e = $null
  $run = [System.Management.Automation.Language.Parser]::ParseInput($RunText, [ref]$t, [ref]$e)
  $cmds = @($run.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true))
  $j = @($cmds | Where-Object { "$($_.GetCommandName())" -ceq 'Get-LabGuestScenarioJsonRefusal' })
  $rc = @($cmds | Where-Object { "$($_.GetCommandName())" -ceq 'robocopy' } | Sort-Object { $_.Extent.StartOffset })
  if (-not $j.Count) { $p.Add('guest-run.ps1 never calls Get-LabGuestScenarioJsonRefusal') }
  elseif (-not $rc.Count -or $j[0].Extent.StartOffset -gt $rc[0].Extent.StartOffset) { $p.Add('guest-run.ps1 copies data before checking cenario.json') }
  $cen = [System.Management.Automation.Language.Parser]::ParseInput($CenText, [ref]$t, [ref]$e)
  $ccmds = @($cen.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true))
  if (-not @($ccmds | Where-Object { "$($_.GetCommandName())" -ceq 'Test-LabGuestLatestLinkApproved' }).Count) { $p.Add('guest-cenario.ps1 never calls Test-LabGuestLatestLinkApproved') }
  if (-not @($cen.FindAll({ param($n) $n -is [System.Management.Automation.Language.StringConstantExpressionAst] -and $n.Value -ceq '--latest-link-aprovado' }, $true)).Count) { $p.Add('guest-cenario.ps1 never passes --latest-link-aprovado') }
  return @($p)
}
$wRun = [IO.File]::ReadAllText((Join-Path $scan 'sandbox/app/guest-run.ps1'))
$wCen = [IO.File]::ReadAllText((Join-Path $scan 'sandbox/app/guest-cenario.ps1'))
Check 'copia wiring: guest-run checks cenario.json before copying; guest-cenario passes the latest-link flag' {
  $x = Get-WiringProblems $wRun $wCen
  if ($x.Count) { throw ($x -join ' | ') }
}
$wMut = [ordered]@{
  'guest-run: no cenario.json check'     = @($wRun, 'Get-LabGuestScenarioJsonRefusal $json', '$null', $wCen)
  'guest-cenario: no approval check'     = @($wRun, '', '', $wCen.Replace('Test-LabGuestLatestLinkApproved $guestCfg', '$false'))
  'guest-cenario: flag renamed'          = @($wRun, '', '', $wCen.Replace("'--latest-link-aprovado'", "'--outro'"))
}
foreach ($k in $wMut.Keys) {
  Check "copia wiring mutation must be caught: $k" {
    $m = $wMut[$k]
    $run2 = $(if ($m[1]) { $m[0].Replace($m[1], $m[2]) } else { $m[0] })
    if ($run2 -ceq $wRun -and $m[3] -ceq $wCen) { throw 'the mutation did not change the text (harness bug)' }
    if (-not (Get-WiringProblems $run2 $m[3]).Count) { throw 'NOT detected: the analysis accepted the mutated script' }
  }
}
# [IO.File]::Replace(src, dst, backup): PowerShell passes $null (and '') to a .NET string parameter as "", which File.Replace
# refuses ("The path is not of a legal form"). The no-backup form must be [NullString]::Value. Checked on the syntax tree.
function Get-FileReplaceNullProblems([string]$Text, [string]$Label) {
  $tokens = $null; $errs = $null
  $ast = [System.Management.Automation.Language.Parser]::ParseInput($Text, [ref]$tokens, [ref]$errs)
  $problems = New-Object System.Collections.Generic.List[string]
  $calls = $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] -and $n.Static -and
      $n.Member -is [System.Management.Automation.Language.StringConstantExpressionAst] -and $n.Member.Value -ieq 'Replace' -and
      $n.Expression -is [System.Management.Automation.Language.TypeExpressionAst] -and $n.Expression.TypeName.FullName -match '^(?i)(system\.)?io\.file$' }, $true)
  foreach ($c in $calls) {
    foreach ($a in @($c.Arguments)) {
      $bad = ($a -is [System.Management.Automation.Language.VariableExpressionAst] -and $a.VariablePath.UserPath -ieq 'null') -or
        ($a -is [System.Management.Automation.Language.StringConstantExpressionAst] -and $a.Value -eq '')
      if ($bad) { $problems.Add("${Label}:$($c.Extent.StartLineNumber): [IO.File]::Replace with `$null or '' as an argument ($($c.Extent.Text)): use [NullString]::Value") }
    }
  }
  return @($problems)
}
$replaceFiles = @(Get-ChildItem -LiteralPath (Join-Path $scan 'scripts'), (Join-Path $scan 'sandbox') -Filter *.ps1 -Recurse)
Check 'File.Replace: no script of the lab passes $null or an empty string to [IO.File]::Replace' {
  $all = New-Object System.Collections.Generic.List[string]
  foreach ($f in $replaceFiles) { foreach ($x in @(Get-FileReplaceNullProblems ([IO.File]::ReadAllText($f.FullName)) $f.Name)) { $all.Add([string]$x) } }
  if ($all.Count) { throw ($all -join ' | ') }
  if ($replaceFiles.Count -lt 20) { throw "only $($replaceFiles.Count) script(s) scanned: the scan is not looking at the scripts" }
}
$replaceDisco = [IO.File]::ReadAllText((Join-Path $scan 'scripts/lib/disco.ps1'))
$replaceGood = '[IO.File]::Replace($tmp, $p.File, [NullString]::Value)'
if (-not $replaceDisco.Contains($replaceGood)) { Check 'File.Replace mutations need the NullString call in disco.ps1' { throw 'disco.ps1 does not have the expected call' } }
$replaceMutants = [ordered]@{
  'disco: the old $null argument'         = '[IO.File]::Replace($tmp, $p.File, $null)'
  'disco: an empty string argument'       = "[IO.File]::Replace(`$tmp, `$p.File, '')"
  'disco: fully qualified System.IO.File' = '[System.IO.File]::Replace($tmp, $p.File, $null)'
}
foreach ($k in $replaceMutants.Keys) {
  Check "File.Replace mutation must be caught: $k" {
    $text = $replaceDisco.Replace($replaceGood, $replaceMutants[$k])
    if ($text -ceq $replaceDisco) { throw 'the mutation did not change the text (harness bug)' }
    if (-not @(Get-FileReplaceNullProblems $text 'disco.ps1').Count) { throw 'NOT detected: the analysis accepted the mutated script' }
  }
}
Check 'File.Replace: the NullString form stays allowed' {
  if ((Get-FileReplaceNullProblems '[IO.File]::Replace($a, $b, [NullString]::Value); [IO.File]::Replace($a, $b, $c)' 'ok').Count) { throw 'false positive' }
}
# Boot id and exit code wiring (parser only, nothing runs). firewall.txt survives between Sandboxes, so:
#   bootstrap.ps1 makes the id of this boot (C:\labdata\boot-id.txt), guest-firewall.ps1 writes it in firewall.txt (and applies nothing without it),
#   guest-run.ps1 hands it to Get-LabAppStartRefusal; and every refusal of guest-run.ps1 (firewall, scenario) leaves exit 1.
function Get-BootIdWiringProblems([string]$RunText, [string]$FwText, [string]$BootText) {
  $p = New-Object System.Collections.Generic.List[string]
  $t = $null; $e = $null
  $A = 'System.Management.Automation.Language'
  $run = [System.Management.Automation.Language.Parser]::ParseInput($RunText, [ref]$t, [ref]$e)
  $fw = [System.Management.Automation.Language.Parser]::ParseInput($FwText, [ref]$t, [ref]$e)
  $bs = [System.Management.Automation.Language.Parser]::ParseInput($BootText, [ref]$t, [ref]$e)
  $isCmd = { param($n) $n -is [System.Management.Automation.Language.CommandAst] }
  # --- guest-run: the id is read from the guest disk and handed to the decision ---
  $ref = @($run.FindAll($isCmd, $true) | Where-Object { "$($_.GetCommandName())" -ceq 'Get-LabAppStartRefusal' })
  if (-not $ref.Count) { $p.Add('guest-run.ps1 never calls Get-LabAppStartRefusal') }
  else {
    $els = @($ref[0].CommandElements)
    $ix = -1; for ($i = 0; $i -lt $els.Count; $i++) { if ($els[$i] -is [System.Management.Automation.Language.CommandParameterAst] -and $els[$i].ParameterName -ceq 'ExpectedBootId') { $ix = $i } }
    if ($ix -lt 0 -or $ix + 1 -ge $els.Count) { $p.Add('guest-run.ps1 does not pass -ExpectedBootId to Get-LabAppStartRefusal') }
    elseif ($els[$ix + 1].Extent.Text -cne '$bootId') { $p.Add("guest-run.ps1 passes -ExpectedBootId $($els[$ix + 1].Extent.Text) instead of `$bootId") }
    $asg = @($run.FindAll({ param($n) $n -is [System.Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -ceq '$bootId' -and $n.Right.Extent.Text -clike '*Get-Content*boot-id.txt*' }, $true))
    if (-not $asg.Count) { $p.Add('guest-run.ps1 does not read $bootId from C:\labdata\boot-id.txt (Get-Content)') }
    elseif ($asg[0].Extent.StartOffset -gt $ref[0].Extent.StartOffset) { $p.Add('guest-run.ps1 reads the boot id after the refusal is decided') }
  }
  # --- guest-run: exit 1 after guest-run.json, for the firewall refusal and for the scenario refusal ---
  $wr = @($run.FindAll({ param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] -and "$($n.Member.Extent.Text)" -ceq 'WriteAllText' -and $n.Extent.Text -clike '*guest-run.json*' }, $true))
  $ex = @($run.FindAll({ param($n) $n -is [System.Management.Automation.Language.IfStatementAst] -and $n.Clauses[0].Item1.Extent.Text -ceq "`$r.Contains('refused')" }, $true))
  if (-not $ex.Count) { $p.Add("guest-run.ps1 has no  if (`$r.Contains('refused')) { ... exit 1 }") }
  else {
    $exit = @($ex[0].Clauses[0].Item2.FindAll({ param($n) $n -is [System.Management.Automation.Language.ExitStatementAst] }, $true))
    if (-not $exit.Count -or $exit[0].Pipeline.Extent.Text -cne '1') { $p.Add('guest-run.ps1: the refused branch does not exit 1') }
    elseif ($wr.Count -and $exit[0].Extent.StartOffset -lt $wr[0].Extent.StartOffset) { $p.Add('guest-run.ps1 exits before writing guest-run.json') }
    elseif (-not $wr.Count) { $p.Add('guest-run.ps1 never writes guest-run.json (the check would prove nothing)') }
  }
  $fwSet = @($run.FindAll({ param($n) $n -is [System.Management.Automation.Language.IfStatementAst] -and $n.Clauses[0].Item1.Extent.Text -ceq '$refusal' }, $true))
  if (-not @($fwSet | Where-Object { @($_.Clauses[0].Item2.FindAll({ param($n) $n -is [System.Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -ceq '$r.refused' -and $n.Right.Extent.Text -ceq '$true' }, $true)).Count }).Count) { $p.Add('guest-run.ps1: the firewall refusal does not set $r.refused = $true') }
  $sc = @($run.FindAll({ param($n) $n -is [System.Management.Automation.Language.IfStatementAst] -and $n.Clauses[0].Item1.Extent.Text -ceq '$why' }, $true))
  if (-not @($sc | Where-Object { @($_.Clauses[0].Item2.FindAll({ param($n) $n -is [System.Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -ceq '$r.refused' -and $n.Right.Extent.Text -ceq '$true' }, $true)).Count }).Count) { $p.Add('guest-run.ps1: the scenario refusal ( if ($why) ) does not set $r.refused = $true') }
  if (-not @($sc | Where-Object { @($_.Clauses[0].Item2.FindAll({ param($n) $n -is [System.Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -ceq '$scenRefusal' }, $true)).Count }).Count) { $p.Add('guest-run.ps1: the scenario refusal does not set $scenRefusal') }
  $start = @($run.FindAll($isCmd, $true) | Where-Object { "$($_.GetCommandName())" -ceq 'Start-Process' } | Select-Object -First 1)
  if ($start.Count) {
    $gate = @($run.FindAll({ param($n) $n -is [System.Management.Automation.Language.IfStatementAst] -and $n.Extent.StartOffset -lt $start[0].Extent.StartOffset -and $n.Extent.EndOffset -gt $start[0].Extent.EndOffset -and $n.Clauses[0].Item1.Extent.Text -clike '*-not $scenRefusal*' }, $true))
    if (-not $gate.Count) { $p.Add('guest-run.ps1: Start-Process is not gated by -not $scenRefusal') }
  } else { $p.Add('guest-run.ps1: Start-Process not found (the check would prove nothing)') }
  # --- guest-firewall: nothing is applied without the id, and the id is the first line written ---
  $g = @($fw.FindAll({ param($n) $n -is [System.Management.Automation.Language.IfStatementAst] -and $n.Clauses[0].Item1.Extent.Text -ceq '-not (Test-LabBootId $bootId)' }, $true))
  $newRule = @($fw.FindAll($isCmd, $true) | Where-Object { "$($_.GetCommandName())" -ceq 'New-NetFirewallRule' } | Select-Object -First 1)
  if (-not $g.Count) { $p.Add('guest-firewall.ps1 has no  if (-not (Test-LabBootId $bootId)) { ... exit 1 }') }
  else {
    if (-not @($g[0].Clauses[0].Item2.FindAll({ param($n) $n -is [System.Management.Automation.Language.ExitStatementAst] -and $n.Pipeline.Extent.Text -ceq '1' }, $true)).Count) { $p.Add('guest-firewall.ps1: the missing boot id does not exit 1') }
    if ($newRule.Count -and $g[0].Extent.StartOffset -gt $newRule[0].Extent.StartOffset) { $p.Add('guest-firewall.ps1 checks the boot id after it applied the rule') }
  }
  $rd = @($fw.FindAll({ param($n) $n -is [System.Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -ceq '$bootId' -and $n.Right.Extent.Text -clike '*Get-Content*boot-id.txt*' }, $true))
  if (-not $rd.Count) { $p.Add('guest-firewall.ps1 does not read $bootId from C:\labdata\boot-id.txt (Get-Content)') }
  $oo = @($fw.FindAll({ param($n) $n -is [System.Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -ceq '$o' }, $true))
  if (-not $oo.Count -or $oo[0].Right.Extent.Text -cnotlike '@(Format-LabFirewallBootLine $bootId) + *') { $p.Add('guest-firewall.ps1: $o (the firewall.txt text) does not START with Format-LabFirewallBootLine $bootId') }
  # --- bootstrap: the id is new for each guest and kept on the guest disk ---
  if (-not @($bs.FindAll({ param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] -and $n.Static -and "$($n.Member.Extent.Text)" -ceq 'NewGuid' }, $true)).Count) { $p.Add('bootstrap.ps1 does not make the boot id with [guid]::NewGuid()') }
  if (-not @($bs.FindAll({ param($n) $n -is [System.Management.Automation.Language.StringConstantExpressionAst] -and $n.Value -ceq 'boot-id.txt' }, $true)).Count) { $p.Add('bootstrap.ps1 does not write boot-id.txt') }
  $w = @($bs.FindAll({ param($n) $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] -and "$($n.Member.Extent.Text)" -ceq 'WriteAllText' -and $n.Extent.Text -clike '*$bootIdFile*' }, $true))
  if (-not $w.Count) { $p.Add('bootstrap.ps1 never writes $bootIdFile') }
  return @($p)
}
$wFw = [IO.File]::ReadAllText((Join-Path $scan 'sandbox/app/guest-firewall.ps1'))
$wBoot = [IO.File]::ReadAllText((Join-Path $scan 'sandbox/bootstrap.ps1'))
Check 'boot id wiring: bootstrap makes it, guest-firewall writes it first, guest-run passes it; refusals exit 1' {
  $x = Get-BootIdWiringProblems $wRun $wFw $wBoot
  if ($x.Count) { throw ($x -join ' | ') }
}
$bMut = [ordered]@{
  'guest-run: boot id not passed to the decision'  = @('run', ' -ExpectedBootId $bootId', '')
  'guest-run: boot id read from another file'      = @('run', "'C:\labdata\boot-id.txt'", "'C:\labdata\other.txt'")
  'guest-run: the firewall refusal exits 0'        = @('run', "`$r | ConvertTo-Json -Depth 5; exit 1 }`n`$r | ConvertTo-Json", "`$r | ConvertTo-Json -Depth 5; exit 0 }`n`$r | ConvertTo-Json")
  'guest-run: scenario refusal does not mark refused (exit 0)' = @('run', '$r.errors += $scenRefusal; $r.refused = $true }', '$r.errors += $scenRefusal }')
  'guest-run: scenario refusal still starts the app' = @('run', ' -and -not $scenRefusal) {', ') {')
  'guest-firewall: firewall.txt without the boot id line' = @('fw', '$o = @(Format-LabFirewallBootLine $bootId) + @(', '$o = @(')
  'guest-firewall: applies the rule without a boot id' = @('fw', "bootstrap did not run in this guest'; exit 1 }", "bootstrap did not run in this guest' }")
  'bootstrap: no new id per guest (fixed id)'      = @('boot', "[guid]::NewGuid().ToString('N')", "'00000000000000000000000000000000'")
  'bootstrap: id not written to the guest disk'    = @('boot', '[IO.File]::WriteAllText($bootIdFile, $status.bootId, $utf8)', '$null')
}
foreach ($k in $bMut.Keys) {
  Check "boot id wiring mutation must be caught: $k" {
    $m = $bMut[$k]
    $src = @{ run = $wRun; fw = $wFw; boot = $wBoot }[$m[0]]
    $src = $src -replace "`r`n", "`n"
    if (-not $src.Contains($m[1])) { throw "harness bug: text to mutate not found: $($m[1])" }
    $mut = $src.Replace($m[1], $m[2])
    $r2 = $(if ($m[0] -eq 'run') { $mut } else { $wRun }); $f2 = $(if ($m[0] -eq 'fw') { $mut } else { $wFw }); $b2 = $(if ($m[0] -eq 'boot') { $mut } else { $wBoot })
    $x = Get-BootIdWiringProblems $r2 $f2 $b2
    if (-not $x.Count) { throw 'NOT detected: the analysis accepted the mutated script' }
    Write-Host "    mutation '$k' -> FAILED - $($x -join ' | ')"
  }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
