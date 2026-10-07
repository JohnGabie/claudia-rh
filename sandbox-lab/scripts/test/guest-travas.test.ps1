# Pure static test: only READS the text of the guest scripts (parser, never executes anything). For every script that
# changes a disk, the firewall or the processes, the guest guard (Test-LabInGuest) must come BEFORE the first effect, and
# the refusal must only print and exit (no file written). Also pins a few wirings that no other test reaches.
#   pwsh -NoProfile -File sandbox-lab/scripts/test/guest-travas.test.ps1
$ErrorActionPreference = 'Stop'
$script:failed = 0
function Check([string]$Name, [scriptblock]$Body) { try { & $Body; Write-Host "ok - $Name" } catch { $script:failed++; Write-Host "FAILED - ${Name}: $($_.Exception.Message)" } }

$lab = Join-Path $PSScriptRoot '../..'
$guarded = @(
  'sandbox/instalar.ps1', 'sandbox/disco-z.ps1', 'sandbox/layout-msvc.ps1', 'sandbox/bootstrap-mcp.ps1',
  'sandbox/app/guest-run.ps1', 'sandbox/app/guest-cenario.ps1', 'sandbox/app/guest-build.ps1', 'sandbox/app/guest-firewall.ps1'
)
# Commands and .NET methods that change something. Function BODIES are skipped (they only define, they do not run).
$effectCommands = @('New-Item', 'Save-Result', 'robocopy', 'diskpart', 'Start-Process', 'Set-Location', 'New-NetFirewallRule', 'Set-NetFirewallProfile', 'Copy-Item', 'Expand-Archive', 'msiexec')
$effectMethods = @('WriteAllText', 'AppendAllText', 'WriteAllLines')

function Get-LabStaticFacts([string]$RelPath) {
  $path = Join-Path $lab $RelPath
  $tokens = $null; $errs = $null
  $ast = [System.Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errs)
  if (@($errs).Count) { throw "$RelPath does not parse: $($errs[0].Message)" }
  $lines = $ast.Extent.Text -split "\r?\n"
  $inFunction = {
    param($n)
    for ($p = $n.Parent; $p; $p = $p.Parent) { if ($p -is [System.Management.Automation.Language.FunctionDefinitionAst]) { return $true } }
    return $false
  }
  $nodes = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] -or $n -is [System.Management.Automation.Language.InvokeMemberExpressionAst] }, $true) | Where-Object { -not (& $inFunction $_) })
  $guard = $null; $dot = $null; $effect = $null
  foreach ($n in $nodes) {
    if ($n -is [System.Management.Automation.Language.CommandAst]) {
      $name = "$($n.GetCommandName())" -replace '\.exe$', ''
      if ($name -ieq 'Test-LabInGuest' -and -not $guard) { $guard = $n }
      if ($n.InvocationOperator -eq 'Dot' -and $n.Extent.Text -like '*guest-install.lib.ps1*' -and -not $dot) { $dot = $n }
      if ($name -and ($effectCommands -contains $name) -and (-not $effect -or $n.Extent.StartOffset -lt $effect.Extent.StartOffset)) { $effect = $n }
    } else {
      $m = "$($n.Member.Value)"
      if ($effectMethods -contains $m -and (-not $effect -or $n.Extent.StartOffset -lt $effect.Extent.StartOffset)) { $effect = $n }
    }
  }
  [pscustomobject]@{ Path = $RelPath; Guard = $guard; Dot = $dot; Effect = $effect; Lines = $lines }
}

foreach ($rel in $guarded) {
  Check "$rel : guard exists, comes first, and the refusal writes nothing" {
    $f = Get-LabStaticFacts $rel
    if (-not $f.Effect) { throw 'no effect found: the check would prove nothing' }
    if (-not $f.Guard) { throw 'no Test-LabInGuest call outside a function' }
    if (-not $f.Dot) { throw 'guest-install.lib.ps1 is not dot-sourced (the guard would not exist)' }
    if ($f.Dot.Extent.StartOffset -gt $f.Guard.Extent.StartOffset) { throw 'the lib is dot-sourced after the guard' }
    if ($f.Guard.Extent.StartOffset -gt $f.Effect.Extent.StartOffset) {
      throw "effect before the guard: line $($f.Effect.Extent.StartLineNumber) ($($f.Effect.Extent.Text.Split("`n")[0].Trim())) vs guard at line $($f.Guard.Extent.StartLineNumber)"
    }
    $line = $f.Lines[$f.Guard.Extent.StartLineNumber - 1]
    if ($line -notmatch '\bexit\s+1\b') { throw "the guard line does not 'exit 1': $line" }
    if ($line -match 'Save-Result|Stop-With|\bSave\b|WriteAll|Set-Content|Out-File|New-Item') { throw "the refusal writes something: $line" }
  }
}
Check 'bootstrap-mcp: the guard is skipped only for -DryRun' {
  $f = Get-LabStaticFacts 'sandbox/bootstrap-mcp.ps1'
  $line = $f.Lines[$f.Guard.Extent.StartLineNumber - 1]
  if ($line -notmatch '-not \$DryRun\s+-and') { throw "guard line: $line" }
}

# Wirings: a refactor that drops one of these would not fail any other pure test.
$js = Get-Content -LiteralPath (Join-Path $lab '../.claude/skills/claudia-teste-sandbox/scripts/testes-no-guest.mjs') -Raw
$up = Get-Content -LiteralPath (Join-Path $lab 'scripts/test-up.ps1') -Raw
$cen = Get-Content -LiteralPath (Join-Path $lab 'sandbox/app/guest-cenario.ps1') -Raw
Check 'testes-no-guest.mjs and guest-cenario.ps1 ask only for node (-Precisa node)' {
  if ($js -notmatch '-Precisa node') { throw 'testes-no-guest.mjs does not pass -Precisa node' }
  if ($cen -notmatch 'Get-LabGuestToolEnv\s+\$inst\s+-Precisa node') { throw 'guest-cenario.ps1 does not use -Precisa node' }
}
Check 'test-up.ps1 wires the account check, the links check and the early credential removal' {
  if ($up -notmatch 'Get-LabUpVerdict[^\r\n]*-PrecisaConta \$conta') { throw 'Get-LabUpVerdict without -PrecisaConta $conta' }
  if ($up -notmatch '\$inside = Get-LabReparsePoints \$src[\r\n]') { throw '$inside = Get-LabReparsePoints $src (no @()) not found' }
  if ($up -notmatch 'Get-LabRealDataSourceRefusal[^\r\n]*-LinksInside \$inside -FileLinksInside \(Get-LabFileLinks \$inside\)') { throw 'the real-data check does not get -LinksInside and -FileLinksInside' }
  if ($up -notmatch 'if \(Test-LabCredentialRemovableNow [^\r\n]*\{') { throw 'the early removal does not use Test-LabCredentialRemovableNow' }
  if ($up -notmatch 'Test-LabCredentialRemovableNow[^\r\n]*\{\s*[\r\n]+\s*Remove-LabCredential') { throw 'Remove-LabCredential does not follow the condition' }
}
if ($script:failed) { Write-Host "$script:failed FAILED"; exit 1 }
Write-Host 'all passed'; exit 0
