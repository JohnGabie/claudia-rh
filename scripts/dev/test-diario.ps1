# Test for the context-diary hooks (pre-compact.ps1, session-start.ps1, journal-skeleton.ps1).
# Simulated input only. Every folder it touches is a fresh temp folder created here; nothing is deleted.
# Usage: powershell -NoProfile -File scripts/dev/test-diario.ps1   -> exit 0 when all cases pass, 1 otherwise.
# The hook input format of a real Claude Code compaction is UNVERIFIED; these cases cover the documented
# "trigger" field, the "compaction_type" fallback, and inputs that must never block.
$ErrorActionPreference = "Stop"
$scripts = $PSScriptRoot
$pre = Join-Path $scripts "pre-compact.ps1"
$fail = 0

function New-TempRepo {
    $t = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $t | Out-Null
    git -C $t init -q
    git -C $t -c user.name=t -c user.email=t@t commit -q --allow-empty -m init
    return $t
}
function New-TempPlainDir {
    $t = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $t | Out-Null
    return $t
}
function Invoke-Hook($script, $json) {
    # The hook writes its refusal to stderr; do not let that become a terminating error here.
    $ErrorActionPreference = "Continue"
    $json | powershell -NoProfile -ExecutionPolicy Bypass -File $script 2>&1 | Out-Null
    return $LASTEXITCODE
}
function Check($name, $got, $want) {
    if ($got -eq $want) { Write-Output "PASS  $name (exit $got)" }
    else { Write-Output "FAILED  $name (expected $want, got $got)"; $script:fail = 1 }
}

$repo = New-TempRepo
$env:CLAUDE_PROJECT_DIR = $repo

Check "trigger manual with an unfilled diary is blocked" (Invoke-Hook $pre '{"hook_event_name":"PreCompact","trigger":"manual"}') 2
Check "trigger auto is never blocked" (Invoke-Hook $pre '{"hook_event_name":"PreCompact","trigger":"auto"}') 0
Check "compaction_type manual (fallback field) is blocked" (Invoke-Hook $pre '{"compaction_type":"manual"}') 2
Check "compaction_type auto (fallback field) is never blocked" (Invoke-Hook $pre '{"compaction_type":"auto"}') 0
Check "unknown type (no field) never blocks" (Invoke-Hook $pre '{}') 0
Check "unreadable stdin never blocks" (Invoke-Hook $pre "not json") 0

$diary = Get-ChildItem (Join-Path $repo ".journal") -Filter "*.md" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$text = [IO.File]::ReadAllText($diary.FullName).Replace("<!-- TODO-CONTEXT -->", "filled by the test")
[IO.File]::WriteAllText($diary.FullName, $text)

Check "trigger manual with a complete diary passes" (Invoke-Hook $pre '{"trigger":"manual"}') 0

$out = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
Check "session-start re-injects the newest diary" ([int](($out | Select-Object -First 1) -match "Latest context diary")) 1

# Worktree cases: the diary must land in the MAIN checkout, named after the worktree branch.
# The temp repository and worktree are left behind on purpose (nothing is removed).
$mainRepo = New-TempRepo
$wt = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-wt-" + [guid]::NewGuid().ToString("N"))
git -C $mainRepo worktree add -q -b "feat/wt-teste" $wt
$env:CLAUDE_PROJECT_DIR = $wt
Check "worktree: manual pre-compact on an unfilled diary is blocked" (Invoke-Hook $pre '{"trigger":"manual"}') 2
$inMain = @(Get-ChildItem (Join-Path $mainRepo ".journal") -Filter "*.md" -ErrorAction SilentlyContinue)
Check "worktree: the diary is created in the main checkout .journal" ([int]($inMain.Count -eq 1)) 1
Check "worktree: nothing is created in the worktree .journal" ([int](Test-Path -LiteralPath (Join-Path $wt ".journal"))) 0
Check "worktree: the diary name contains the worktree branch" ([int]($inMain.Count -ge 1 -and $inMain[0].Name -like "*-feat-wt-teste-*")) 1
$wtOut = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
$wtFirst = [string]($wtOut | Select-Object -First 1)
Check "worktree: session-start re-injects the main checkout diary" ([int](($wtFirst -match "Latest context diary") -and $wtFirst.Contains((Join-Path $mainRepo ".journal")))) 1

function Fill-Diary($path) {
    $t = [IO.File]::ReadAllText($path).Replace("<!-- TODO-CONTEXT -->", "filled by the test")
    [IO.File]::WriteAllText($path, $t)
}
function Unfill-Diary($path) {
    [IO.File]::WriteAllText($path, [IO.File]::ReadAllText($path) + "`n### x  <!-- TODO-CONTEXT -->`n")
}
function Get-DiaryFor($repoDir, $branchName) {
    # The diary whose skeleton recorded exactly this branch (null when there is none).
    $dir = Join-Path $repoDir ".journal"
    if (-not (Test-Path -LiteralPath $dir)) { return $null }
    Get-ChildItem $dir -Filter "*.md" | Where-Object {
        $m = Select-String -LiteralPath $_.FullName -Pattern '^- Branch: (.*) @ ' | Select-Object -First 1
        $m -and ($m.Matches[0].Groups[1].Value -ceq $branchName)
    } | Select-Object -First 1
}
function Get-DiaryForWorktree($repoDir, $wtDir) {
    # The diary whose skeleton recorded exactly this worktree folder (null when there is none).
    $dir = Join-Path $repoDir ".journal"
    if (-not (Test-Path -LiteralPath $dir)) { return $null }
    $want = [IO.Path]::GetFullPath($wtDir).TrimEnd('\')
    Get-ChildItem $dir -Filter "*.md" | Where-Object {
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
    $o0 = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
    Check "$label`: session-start of B does not show the diary of A" $o0.Count 0
    Check "$label`: a complete diary of A does not free the manual compaction of B" (Invoke-Hook $pre '{"trigger":"manual"}') 2
    $dB = & $finder $repoDir $wtB
    if (-not (Test-Exists "$label`: diary of B exists" $dB)) { return }
    Fill-Diary $dB.FullName
    Unfill-Diary $dA.FullName
    Check "$label`: an incomplete diary of A does not block B once B is filled" (Invoke-Hook $pre '{"trigger":"manual"}') 0
    $o1 = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
    Check "$label`: session-start of B shows the diary of B" ([int]([string]($o1 | Select-Object -First 1)).Contains($dB.Name)) 1
    # Make B the newest diary (deterministic), then A must still be blocked by its own incomplete diary.
    [IO.File]::SetLastWriteTime($dB.FullName, (Get-Date).AddMinutes(1))
    $env:CLAUDE_PROJECT_DIR = $wtA
    Check "$label`: A is still blocked by its own incomplete diary" (Invoke-Hook $pre '{"trigger":"manual"}') 2
}

if ($inMain.Count -ge 1) { Fill-Diary $inMain[0].FullName }
Check "worktree: manual compaction passes after filling the main-checkout diary" (Invoke-Hook $pre '{"trigger":"manual"}') 0

# Isolation between two worktrees of the same repository (branch B name has branch A name as a prefix).
$repo2 = New-TempRepo
$wtA = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-wt-" + [guid]::NewGuid().ToString("N"))
$wtB = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-wt-" + [guid]::NewGuid().ToString("N"))
git -C $repo2 worktree add -q -b "feat/aaa" $wtA
git -C $repo2 worktree add -q -b "feat/aaa-b" $wtB
Test-IsolationPair "isolation" $wtA $wtB $repo2 { param($r, $s) Get-DiaryFor $r $(if ($s -eq $wtA) { "feat/aaa" } else { "feat/aaa-b" }) }

# Detached HEAD in two worktrees of the same repository: isolation falls back to the worktree folder.
$repo4 = New-TempRepo
$dA = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-wt-" + [guid]::NewGuid().ToString("N"))
$dB = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-wt-" + [guid]::NewGuid().ToString("N"))
git -C $repo4 worktree add -q --detach $dA
git -C $repo4 worktree add -q --detach $dB
Test-IsolationPair "detached" $dA $dB $repo4 { param($r, $s) Get-DiaryForWorktree $r $s }

# The main-root detection must not depend on the state of the main work tree.
$repo5 = New-TempRepo
[IO.File]::WriteAllText((Join-Path $repo5 "a.txt"), "a")
[IO.File]::WriteAllText((Join-Path $repo5 "b.txt"), "b")
git -C $repo5 add a.txt b.txt
git -C $repo5 -c user.name=t -c user.email=t@t commit -q -m files
$wt5 = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-wt-" + [guid]::NewGuid().ToString("N"))
git -C $repo5 worktree add -q -b "feat/five" $wt5
Move-Item -LiteralPath (Join-Path $repo5 "a.txt") (Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-moved-" + [guid]::NewGuid().ToString("N")))
$env:CLAUDE_PROJECT_DIR = $wt5
Check "main-root: first tracked file deleted without commit: manual blocked" (Invoke-Hook $pre '{"trigger":"manual"}') 2
Check "main-root: first tracked file deleted without commit: diary in the main checkout" ([int](Test-Path -LiteralPath (Join-Path $repo5 ".journal"))) 1
Check "main-root: first tracked file deleted without commit: nothing in the worktree" ([int](Test-Path -LiteralPath (Join-Path $wt5 ".journal"))) 0

$repo6 = New-TempRepo
[IO.File]::WriteAllText((Join-Path $repo6 "a.txt"), "a")
[IO.File]::WriteAllText((Join-Path $repo6 "b.txt"), "b")
git -C $repo6 add a.txt b.txt
git -C $repo6 -c user.name=t -c user.email=t@t commit -q -m files
$wt6 = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-wt-" + [guid]::NewGuid().ToString("N"))
git -C $repo6 worktree add -q -b "feat/six" $wt6
git -C $repo6 sparse-checkout init --no-cone
git -C $repo6 sparse-checkout set "/b.txt"
$env:CLAUDE_PROJECT_DIR = $wt6
Check "main-root: sparse checkout excluding the first tracked file: manual blocked" (Invoke-Hook $pre '{"trigger":"manual"}') 2
Check "main-root: sparse checkout excluding the first tracked file: diary in the main checkout" ([int](Test-Path -LiteralPath (Join-Path $repo6 ".journal"))) 1
Check "main-root: sparse checkout excluding the first tracked file: nothing in the worktree" ([int](Test-Path -LiteralPath (Join-Path $wt6 ".journal"))) 0

# Repository without any commit: a diary of another branch must not be re-injected.
$repo7 = New-TempPlainDir
git -C $repo7 init -q
New-Item -ItemType Directory -Path (Join-Path $repo7 ".journal") | Out-Null
[IO.File]::WriteAllText((Join-Path $repo7 ".journal\2026-01-01-000000-other-x.md"), "# other`n- Branch: other-branch @ abc`n- Worktree: $repo7`n")
$env:CLAUDE_PROJECT_DIR = $repo7
$o7 = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
Check "unborn branch: session-start does not re-inject the diary of another branch" $o7.Count 0

# .journal ignores itself, even when the repository .gitignore has no rule for it.
$repo3 = New-TempRepo
Push-Location $repo3
powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "journal-skeleton.ps1") "t" | Out-Null
Pop-Location
Check "skeleton: .journal does not show in git status" ([int]((@(git -C $repo3 status --porcelain) -join "`n") -match "journal")) 0
Check "skeleton: the diary file exists" ([int](@(Get-ChildItem (Join-Path $repo3 ".journal") -Filter "*.md").Count)) 1

# --separate-git-dir: the diary stays in the work tree, never next to the detached git dir.
$base = Join-Path ([IO.Path]::GetTempPath()) ("diario-teste-sep-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path (Join-Path $base "store") -Force | Out-Null
$sepWork = Join-Path $base "work"
$sepStore = Join-Path $base "store\.git"
git init -q --separate-git-dir $sepStore $sepWork
[IO.File]::WriteAllText((Join-Path $sepWork "a.txt"), "a")
git -C $sepWork add a.txt
git -C $sepWork -c user.name=t -c user.email=t@t commit -q -m init
$env:CLAUDE_PROJECT_DIR = $sepWork
Check "separate-git-dir: manual pre-compact is blocked (skeleton created)" (Invoke-Hook $pre '{"trigger":"manual"}') 2
Check "separate-git-dir: the diary is in the work tree" ([int](Test-Path -LiteralPath (Join-Path $sepWork ".journal"))) 1
Check "separate-git-dir: nothing is created beside the git dir" ([int](Test-Path -LiteralPath (Join-Path $base "store\.journal"))) 0

$env:CLAUDE_PROJECT_DIR = New-TempPlainDir
Check "a folder that is not a git repository does not crash or block" (Invoke-Hook $pre '{"trigger":"manual"}') 0


$oPlain = @(powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scripts "session-start.ps1"))
Check "no git identity: session-start injects nothing" $oPlain.Count 0
exit $fail
