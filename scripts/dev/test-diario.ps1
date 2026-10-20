# Test for the context-diary hooks (pre-compact.ps1, session-start.ps1, journal-skeleton.ps1).
# Simulated input only. Every folder it touches is a fresh temp folder created here; nothing is deleted.
# Usage: powershell -NoProfile -File scripts/dev/test-diario.ps1   -> exit 0 when all cases pass, 1 otherwise.
# The hook input format of a real Claude Code compaction is UNVERIFIED; these cases cover the documented
# "trigger" field, the "compaction_type" fallback, and inputs that must never block.
# A broken script under test must never abort this test: any exception is reported as FAILED and the run goes on.
$ErrorActionPreference = "Stop"
$scripts = $PSScriptRoot
$pre = Join-Path $scripts "pre-compact.ps1"
$skel = Join-Path $scripts "journal-skeleton.ps1"
$fail = 0

trap {
    Write-Output "FAILED  unexpected exception (the rest of that step was skipped): $($_.Exception.Message)"
    $script:fail = 1
    continue
}

function New-TempPlainDir {
    $t = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $t | Out-Null
    return $t
}
function New-TempPath($prefix) {
    return (Join-Path ([IO.Path]::GetTempPath()) ($prefix + [guid]::NewGuid().ToString("N")))
}
function Invoke-GitQuiet {
    # git setup steps of the test; their stderr must not become terminating errors.
    $ErrorActionPreference = "Continue"
    & git @args 2>$null | Out-Null
}
function New-TempRepo {
    $t = New-TempPlainDir
    Invoke-GitQuiet -C $t init -q
    Invoke-GitQuiet -C $t -c user.name=t -c user.email=t@t commit -q --allow-empty -m init
    return $t
}
function Invoke-Hook($script, $json) {
    # The hook writes its refusal to stderr; do not let that become a terminating error here.
    $ErrorActionPreference = "Continue"
    $json | powershell -NoProfile -ExecutionPolicy Bypass -File $script 2>&1 | Out-Null
    return $LASTEXITCODE
}
function Invoke-SessionStart {
    $ErrorActionPreference = "Continue"
    return @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1") 2>$null)
}
function Invoke-Skeleton($dir, $reason) {
    # Runs the skeleton inside $dir; returns @(exit code, printed path).
    $ErrorActionPreference = "Continue"
    Push-Location -LiteralPath $dir
    try {
        $o = @(powershell -NoProfile -ExecutionPolicy Bypass -File $skel $reason 2>$null)
        $code = $LASTEXITCODE
    } finally { Pop-Location }
    $path = if ($o.Count -gt 0) { [string]$o[-1] } else { "" }
    return @($code, $path)
}
function Get-FirstLine($lines) {
    $a = @($lines)
    if ($a.Count -eq 0 -or $null -eq $a[0]) { return "" }
    return [string]$a[0]
}
function Get-JournalFiles($repoDir) {
    $dir = Join-Path $repoDir ".journal"
    if (-not (Test-Path -LiteralPath $dir)) { return @() }
    return @(Get-ChildItem -LiteralPath $dir -Filter "*.md" -File)
}
function Check($name, $got, $want) {
    $g = if ($null -eq $got) { "<null>" } else { (@($got) | ForEach-Object { [string]$_ }) -join "," }
    if ($g -eq [string]$want) { Write-Output "PASS  $name (got $g)" }
    else { Write-Output "FAILED  $name (expected $want, got $g)"; $script:fail = 1 }
}

$repo = New-TempRepo
$env:CLAUDE_PROJECT_DIR = $repo

Check "trigger manual with an unfilled diary is blocked" (Invoke-Hook $pre '{"hook_event_name":"PreCompact","trigger":"manual"}') 2
Check "trigger auto is never blocked" (Invoke-Hook $pre '{"hook_event_name":"PreCompact","trigger":"auto"}') 0
Check "compaction_type manual (fallback field) is blocked" (Invoke-Hook $pre '{"compaction_type":"manual"}') 2
Check "compaction_type auto (fallback field) is never blocked" (Invoke-Hook $pre '{"compaction_type":"auto"}') 0
Check "unknown type (no field) never blocks" (Invoke-Hook $pre '{}') 0
Check "unreadable stdin never blocks" (Invoke-Hook $pre "not json") 0

function Fill-Diary($path) {
    $t = [IO.File]::ReadAllText($path).Replace("<!-- TODO-CONTEXT -->", "filled by the test")
    [IO.File]::WriteAllText($path, $t)
}
function Unfill-Diary($path) {
    [IO.File]::WriteAllText($path, [IO.File]::ReadAllText($path) + "`n### x  <!-- TODO-CONTEXT -->`n")
}

$diary = Get-JournalFiles $repo | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Check "a diary exists after the blocked compaction" ([int]($null -ne $diary)) 1
if ($diary) { Fill-Diary $diary.FullName }

Check "trigger manual with a complete diary passes" (Invoke-Hook $pre '{"trigger":"manual"}') 0

$out = Invoke-SessionStart
Check "session-start re-injects the newest diary" ([int]((Get-FirstLine $out) -match "Latest context diary")) 1

# Worktree cases: the diary must land in the MAIN checkout, named after the worktree branch.
# The temp repository and worktree are left behind on purpose (nothing is removed).
$mainRepo = New-TempRepo
$wt = New-TempPath "diario-teste-wt-"
Invoke-GitQuiet -C $mainRepo worktree add -q -b "feat/wt-teste" $wt
$env:CLAUDE_PROJECT_DIR = $wt
Check "worktree: manual pre-compact on an unfilled diary is blocked" (Invoke-Hook $pre '{"trigger":"manual"}') 2
$inMain = @(Get-JournalFiles $mainRepo)
Check "worktree: the diary is created in the main checkout .journal" ([int]($inMain.Count -eq 1)) 1
Check "worktree: nothing is created in the worktree .journal" ([int](Test-Path -LiteralPath (Join-Path $wt ".journal"))) 0
Check "worktree: the diary name contains the worktree branch" ([int]($inMain.Count -ge 1 -and $inMain[0].Name -like "*-feat-wt-teste-*")) 1
$wtFirst = Get-FirstLine (Invoke-SessionStart)
Check "worktree: session-start re-injects the main checkout diary" ([int](($wtFirst -match "Latest context diary") -and $wtFirst.Contains((Join-Path $mainRepo ".journal")))) 1

function Get-DiaryFor($repoDir, $branchName) {
    # The diary whose skeleton recorded exactly this branch (null when there is none).
    Get-JournalFiles $repoDir | Where-Object {
        $m = Select-String -LiteralPath $_.FullName -Pattern '^- Branch: (.*) @ ' | Select-Object -First 1
        $m -and ($m.Matches[0].Groups[1].Value -ceq $branchName)
    } | Select-Object -First 1
}
function Get-DiaryForWorktree($repoDir, $wtDir) {
    # The diary whose skeleton recorded exactly this worktree folder (null when there is none).
    $want = [IO.Path]::GetFullPath($wtDir).TrimEnd('\')
    Get-JournalFiles $repoDir | Where-Object {
        $m = Select-String -LiteralPath $_.FullName -Pattern '^- Worktree: (.*)$' | Select-Object -First 1
        $m -and ([IO.Path]::GetFullPath($m.Matches[0].Groups[1].Value.Trim()).TrimEnd('\') -ieq $want)
    } | Select-Object -First 1
}
function Test-Exists($name, $item) {
    # Registers a failing check (instead of throwing) when a diary the case needs is missing.
    Check $name ([int]($null -ne $item)) 1 | Out-Host
    return ($null -ne $item)
}
function Test-IsolationPair($label, $wtA, $wtB, $repoDir, $finder) {
    # $finder: scriptblock(diaryRepoDir, sessionDir) -> the diary of that session.
    $env:CLAUDE_PROJECT_DIR = $wtA
    Check "$label`: A manual with an unfilled diary is blocked" (Invoke-Hook $pre '{"trigger":"manual"}') 2
    $dA = & $finder $repoDir $wtA
    if (-not (Test-Exists "$label`: diary of A exists" $dA)) { return }
    Fill-Diary $dA.FullName
    Check "$label`: A manual passes after filling A" (Invoke-Hook $pre '{"trigger":"manual"}') 0
    $env:CLAUDE_PROJECT_DIR = $wtB
    $o0 = Invoke-SessionStart
    Check "$label`: session-start of B does not show the diary of A" $o0.Count 0
    Check "$label`: a complete diary of A does not free the manual compaction of B" (Invoke-Hook $pre '{"trigger":"manual"}') 2
    $dB = & $finder $repoDir $wtB
    if (-not (Test-Exists "$label`: diary of B exists" $dB)) { return }
    Fill-Diary $dB.FullName
    Unfill-Diary $dA.FullName
    Check "$label`: an incomplete diary of A does not block B once B is filled" (Invoke-Hook $pre '{"trigger":"manual"}') 0
    $first1 = Get-FirstLine (Invoke-SessionStart)
    Check "$label`: session-start of B shows the diary of B" ([int]$first1.Contains([string]$dB.Name)) 1
    # Make B the newest diary (deterministic), then A must still be blocked by its own incomplete diary.
    [IO.File]::SetLastWriteTime($dB.FullName, (Get-Date).AddMinutes(1))
    $env:CLAUDE_PROJECT_DIR = $wtA
    Check "$label`: A is still blocked by its own incomplete diary" (Invoke-Hook $pre '{"trigger":"manual"}') 2
}

if ($inMain.Count -ge 1) { Fill-Diary $inMain[0].FullName }
Check "worktree: manual compaction passes after filling the main-checkout diary" (Invoke-Hook $pre '{"trigger":"manual"}') 0

# Isolation between two worktrees of the same repository (branch B name has branch A name as a prefix).
$repo2 = New-TempRepo
$wtA = New-TempPath "diario-teste-wt-"
$wtB = New-TempPath "diario-teste-wt-"
Invoke-GitQuiet -C $repo2 worktree add -q -b "feat/aaa" $wtA
Invoke-GitQuiet -C $repo2 worktree add -q -b "feat/aaa-b" $wtB
Test-IsolationPair "isolation" $wtA $wtB $repo2 { param($r, $s) Get-DiaryFor $r $(if ($s -eq $wtA) { "feat/aaa" } else { "feat/aaa-b" }) }

# Detached HEAD in two worktrees of the same repository: isolation falls back to the worktree folder.
$repo4 = New-TempRepo
$dA = New-TempPath "diario-teste-wt-"
$dB = New-TempPath "diario-teste-wt-"
Invoke-GitQuiet -C $repo4 worktree add -q --detach $dA
Invoke-GitQuiet -C $repo4 worktree add -q --detach $dB
Test-IsolationPair "detached" $dA $dB $repo4 { param($r, $s) Get-DiaryForWorktree $r $s }

# Two detached worktrees: their diary names must differ beyond the time stamp (both are "HEAD"),
# so two skeletons in the same second never collide.
$repo9 = New-TempRepo
$cA = New-TempPath "diario-teste-wt-"
$cB = New-TempPath "diario-teste-wt-"
Invoke-GitQuiet -C $repo9 worktree add -q --detach $cA
Invoke-GitQuiet -C $repo9 worktree add -q --detach $cB
$rA = Invoke-Skeleton $cA "same"
$rB = Invoke-Skeleton $cB "same"
Check "collision: skeleton of detached A exits 0" $rA[0] 0
Check "collision: skeleton of detached B exits 0" $rB[0] 0
$nA = if ($rA[1]) { (Split-Path -Leaf $rA[1]) } else { "" }
$nB = if ($rB[1]) { (Split-Path -Leaf $rB[1]) } else { "" }
$tailA = if ($nA.Length -gt 18) { $nA.Substring(18) } else { "" }
$tailB = if ($nB.Length -gt 18) { $nB.Substring(18) } else { "" }
Check "collision: the two detached diary names differ beyond the time stamp" ([int]($tailA -and $tailB -and ($tailA -ne $tailB))) 1
# Same second for real: start both skeletons at the start of a second, in parallel.
$outDir = New-TempPlainDir
while ((Get-Date).Millisecond -gt 100) { Start-Sleep -Milliseconds 20 }
$pA = Start-Process powershell -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "`"$skel`"", "par") -WorkingDirectory $cA -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $outDir "a.txt") -RedirectStandardError (Join-Path $outDir "ae.txt")
$pB = Start-Process powershell -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "`"$skel`"", "par") -WorkingDirectory $cB -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $outDir "b.txt") -RedirectStandardError (Join-Path $outDir "be.txt")
# Touching Handle keeps the process handle open, so ExitCode is readable after the process ends.
$null = $pA.Handle; $null = $pB.Handle
$pA.WaitForExit(); $pB.WaitForExit()
Check "collision: parallel skeleton of detached A exits 0" $pA.ExitCode 0
Check "collision: parallel skeleton of detached B exits 0" $pB.ExitCode 0
$par = @(Get-JournalFiles $repo9 | Where-Object { $_.Name -like "*-par.md" })
Check "collision: both parallel diaries exist" $par.Count 2
$sameSecond = ($par.Count -eq 2 -and $par[0].Name.Substring(0, 17) -eq $par[1].Name.Substring(0, 17))
Write-Output "INFO  collision: parallel skeletons landed in the same second: $sameSecond"

# The main-root detection must not depend on the state of the main work tree.
$repo5 = New-TempRepo
[IO.File]::WriteAllText((Join-Path $repo5 "a.txt"), "a")
[IO.File]::WriteAllText((Join-Path $repo5 "b.txt"), "b")
Invoke-GitQuiet -C $repo5 add a.txt b.txt
Invoke-GitQuiet -C $repo5 -c user.name=t -c user.email=t@t commit -q -m files
$wt5 = New-TempPath "diario-teste-wt-"
Invoke-GitQuiet -C $repo5 worktree add -q -b "feat/five" $wt5
Move-Item -LiteralPath (Join-Path $repo5 "a.txt") (New-TempPath "diario-teste-moved-")
$env:CLAUDE_PROJECT_DIR = $wt5
Check "main-root: first tracked file deleted without commit: manual blocked" (Invoke-Hook $pre '{"trigger":"manual"}') 2
Check "main-root: first tracked file deleted without commit: diary in the main checkout" ([int](Test-Path -LiteralPath (Join-Path $repo5 ".journal"))) 1
Check "main-root: first tracked file deleted without commit: nothing in the worktree" ([int](Test-Path -LiteralPath (Join-Path $wt5 ".journal"))) 0

$repo6 = New-TempRepo
[IO.File]::WriteAllText((Join-Path $repo6 "a.txt"), "a")
[IO.File]::WriteAllText((Join-Path $repo6 "b.txt"), "b")
Invoke-GitQuiet -C $repo6 add a.txt b.txt
Invoke-GitQuiet -C $repo6 -c user.name=t -c user.email=t@t commit -q -m files
$wt6 = New-TempPath "diario-teste-wt-"
Invoke-GitQuiet -C $repo6 worktree add -q -b "feat/six" $wt6
Invoke-GitQuiet -C $repo6 sparse-checkout init --no-cone
Invoke-GitQuiet -C $repo6 sparse-checkout set "/b.txt"
$env:CLAUDE_PROJECT_DIR = $wt6
Check "main-root: sparse checkout excluding the first tracked file: manual blocked" (Invoke-Hook $pre '{"trigger":"manual"}') 2
Check "main-root: sparse checkout excluding the first tracked file: diary in the main checkout" ([int](Test-Path -LiteralPath (Join-Path $repo6 ".journal"))) 1
Check "main-root: sparse checkout excluding the first tracked file: nothing in the worktree" ([int](Test-Path -LiteralPath (Join-Path $wt6 ".journal"))) 0

# Repository without any commit: a diary of another branch must not be re-injected.
$repo7 = New-TempPlainDir
Invoke-GitQuiet -C $repo7 init -q
New-Item -ItemType Directory -Path (Join-Path $repo7 ".journal") | Out-Null
[IO.File]::WriteAllText((Join-Path $repo7 ".journal\2026-01-01-000000-other-x.md"), "# other`n- Branch: other-branch @ abc`n- Worktree: $repo7`n")
$env:CLAUDE_PROJECT_DIR = $repo7
Check "unborn branch: session-start does not re-inject the diary of another branch" (Invoke-SessionStart).Count 0

# Repository without any commit: "symbolic-ref" still names the unborn branch, the skeleton records
# "@ unborn" and a manual compaction is blocked until the diary is filled.
$repo8 = New-TempPlainDir
Invoke-GitQuiet -C $repo8 init -q
$unbornBranch = ""
try { $ErrorActionPreference = "Continue"; $unbornBranch = [string](@(& git -C $repo8 symbolic-ref --short HEAD 2>$null) | Select-Object -First 1) } finally { $ErrorActionPreference = "Stop" }
$env:CLAUDE_PROJECT_DIR = $repo8
Check "unborn repo: manual pre-compact is blocked (skeleton created)" (Invoke-Hook $pre '{"trigger":"manual"}') 2
$u8 = @(Get-JournalFiles $repo8)
$u8Line = if ($u8.Count -ge 1) { [string](Select-String -LiteralPath $u8[0].FullName -Pattern '^- Branch: ' | Select-Object -First 1) } else { "" }
Check "unborn repo: the diary records '- Branch: $unbornBranch @ unborn'" ([int]($unbornBranch -and $u8Line.EndsWith("- Branch: $unbornBranch @ unborn"))) 1

# .journal ignores itself, even when the repository .gitignore has no rule for it.
$repo3 = New-TempRepo
$r3 = Invoke-Skeleton $repo3 "t"
Check "skeleton: exits 0 in a fresh repository" $r3[0] 0
$st3 = ""
try { $ErrorActionPreference = "Continue"; $st3 = (@(& git -C $repo3 status --porcelain 2>$null) -join "`n") } finally { $ErrorActionPreference = "Stop" }
Check "skeleton: .journal does not show in git status" ([int]($st3 -match "journal")) 0
Check "skeleton: the diary file exists" (@(Get-JournalFiles $repo3).Count) 1

# --separate-git-dir: the diary stays in the work tree, never next to the detached git dir.
$base = New-TempPath "diario-teste-sep-"
New-Item -ItemType Directory -Path (Join-Path $base "store") -Force | Out-Null
$sepWork = Join-Path $base "work"
$sepStore = Join-Path $base "store\.git"
Invoke-GitQuiet init -q --separate-git-dir $sepStore $sepWork
[IO.File]::WriteAllText((Join-Path $sepWork "a.txt"), "a")
Invoke-GitQuiet -C $sepWork add a.txt
Invoke-GitQuiet -C $sepWork -c user.name=t -c user.email=t@t commit -q -m init
$env:CLAUDE_PROJECT_DIR = $sepWork
Check "separate-git-dir: manual pre-compact is blocked (skeleton created)" (Invoke-Hook $pre '{"trigger":"manual"}') 2
Check "separate-git-dir: the diary is in the work tree" ([int](Test-Path -LiteralPath (Join-Path $sepWork ".journal"))) 1
Check "separate-git-dir: nothing is created beside the git dir" ([int](Test-Path -LiteralPath (Join-Path $base "store\.journal"))) 0

# --separate-git-dir whose git dir is literally named ".git", plus a linked worktree: git itself reports
# "store" as the main work tree, but the diary must never go to store\.journal.
$sepWt = Join-Path $base "linked"
Invoke-GitQuiet -C $sepWork worktree add -q -b "feat/sep" $sepWt
$env:CLAUDE_PROJECT_DIR = $sepWt
Check "separate-git-dir linked: manual pre-compact is blocked (skeleton created)" (Invoke-Hook $pre '{"trigger":"manual"}') 2
Check "separate-git-dir linked: nothing is created beside the git dir" ([int](Test-Path -LiteralPath (Join-Path $base "store\.journal"))) 0
Check "separate-git-dir linked: the diary falls back to the session folder" ([int](Test-Path -LiteralPath (Join-Path $sepWt ".journal"))) 1

$env:CLAUDE_PROJECT_DIR = New-TempPlainDir
Check "a folder that is not a git repository does not crash or block" (Invoke-Hook $pre '{"trigger":"manual"}') 0
Check "no git identity: session-start injects nothing" (Invoke-SessionStart).Count 0

# No identity (folder outside git) WITH an incomplete, recent diary in its .journal: the diary must not
# be re-injected and must not block, because nothing ties it to this session.
$plain = New-TempPlainDir
New-Item -ItemType Directory -Path (Join-Path $plain ".journal") | Out-Null
$stale = Join-Path $plain ".journal\2026-01-01-000000-x-00000000-pre-compact.md"
[IO.File]::WriteAllText($stale, "# x`n- Branch: x @ abc`n- Worktree: $plain`n### 1. Goal  <!-- TODO-CONTEXT -->`n")
$env:CLAUDE_PROJECT_DIR = $plain
Check "no identity with an incomplete diary: session-start prints nothing" (Invoke-SessionStart).Count 0
Check "no identity with an incomplete diary: manual pre-compact does not block" (Invoke-Hook $pre '{"trigger":"manual"}') 0

exit $fail
