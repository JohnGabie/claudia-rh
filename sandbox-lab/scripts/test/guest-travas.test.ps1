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
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
