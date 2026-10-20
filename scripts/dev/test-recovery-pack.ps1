# Test for recovery-pack.ps1 (P14/P17 recovery pack on the orphan branch `recovery`).
# Everything is FAKE and lives in a fresh temp folder created here ([IO.Path]::GetTempPath() + GUID): a git
# repository with commits, a linked worktree with an uncommitted change and an untracked file, .journal diaries,
# a fake global Claude folder (CLAUDE.md, settings.json with env/apiKeyHelper/headers, a fake .credentials.json,
# a plugin list), a fake memory folder, .claude/skills and .claude/worktrees, and forbidden files.
# Never reads or writes the user's home, ~/.claude, AppData or any real repository. Nothing is deleted: every
# scenario that needs a different input gets a new folder. The temp folder is left behind.
# Usage: powershell -NoProfile -File scripts/dev/test-recovery-pack.ps1 [-ScriptPath <recovery-pack.ps1 copy>]
#   -> exit 0 when all cases pass, 1 otherwise. -ScriptPath exists so mutants can be tested.
param([string]$ScriptPath = (Join-Path $PSScriptRoot "recovery-pack.ps1"))
$ErrorActionPreference = "Stop"
$fail = 0

trap {
    Write-Output "FAILED  unexpected exception (the rest of that step was skipped): $($_.Exception.Message)"
    $script:fail = 1
    continue
}

# git must never walk up out of the temp folders into some real repository.
$env:GIT_CEILING_DIRECTORIES = ([IO.Path]::GetTempPath()).TrimEnd('\')

$Utf8 = New-Object System.Text.UTF8Encoding($false)
function New-TempPath([string]$prefix) {
    return (Join-Path ([IO.Path]::GetTempPath()) ($prefix + [guid]::NewGuid().ToString("N")))
}
function Write-Text([string]$path, [string]$text) {
    $parent = Split-Path -Parent $path
    if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent | Out-Null }
    [IO.File]::WriteAllText($path, $text, $script:Utf8)
}
# Runs git; returns stdout lines. stderr is dropped (PS 5.1 would turn it into a terminating error).
function Invoke-G {
    $ErrorActionPreference = "Continue"
    $out = @(& git -c user.name=t -c user.email=t@t -c init.defaultBranch=main -c advice.detachedHead=false -c core.autocrlf=false @args 2>$null)
    return ,@($out | ForEach-Object { [string]$_ })
}
function Invoke-GCode {
    $ErrorActionPreference = "Continue"
    & git @args 2>$null | Out-Null
    return $LASTEXITCODE
}
function Invoke-Pack {
    $ErrorActionPreference = "Continue"
    $text = (& powershell -NoProfile -ExecutionPolicy Bypass -File $ScriptPath @args 2>&1 | ForEach-Object { [string]$_ }) -join "`n"
    return [pscustomobject]@{ Code = $LASTEXITCODE; Text = $text }
}
function Assert-Case([string]$name, [bool]$ok, [string]$why) {
    if ($ok) { Write-Output "PASS    $name" }
    else { Write-Output "FAILED  $name :: $why"; $script:fail = 1 }
}
# NOTE: PowerShell variables are case-insensitive: results are $res, never $r (it would clobber $R, the repository).
function Assert-Exit($res, [string]$name, [int]$code) {
    Assert-Case $name ($res.Code -eq $code) "expected exit $code, got $($res.Code). Output:`n$($res.Text)"
}
function Get-Recovery { $l = Invoke-G -C $script:R rev-parse -q --verify "refs/heads/recovery"; if ($l.Count) { return $l[0] } return $null }
function Get-PackPaths { return (Invoke-G -C $script:R ls-tree -r --name-only "refs/heads/recovery") }
function Get-PackBlob([string]$path) { return ((Invoke-G -C $script:R show "refs/heads/recovery:$path") -join "`n") }
function Get-PackAll {
    $all = ""
    foreach ($p in (Get-PackPaths)) { $all += "`n=== $p`n" + (Get-PackBlob $p) }
    return $all
}
function Get-FileHashOrNone([string]$p) { if (Test-Path -LiteralPath $p) { return (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash } return "none" }
# Everything of the user's repository that the script must never change (all refs except refs/heads/recovery).
function Get-Snapshot {
    $parts = @()
    $parts += "HEAD=" + ((Invoke-G -C $script:R symbolic-ref HEAD) -join "")
    $parts += "HEADSHA=" + ((Invoke-G -C $script:R rev-parse HEAD) -join "")
    $parts += "WTHEAD=" + ((Invoke-G -C $script:WT symbolic-ref HEAD) -join "") + " " + ((Invoke-G -C $script:WT rev-parse HEAD) -join "")
    $parts += (Invoke-G -C $script:R for-each-ref "--format=%(refname) %(objectname)") | Where-Object { $_ -notmatch '^refs/heads/recovery ' }
    $parts += "INDEX=" + (Get-FileHashOrNone (Join-Path $script:R ".git\index"))
    $parts += "WTINDEX=" + (Get-FileHashOrNone (Join-Path $script:R ".git\worktrees\wt1\index"))
    $parts += "STATUS=" + ((Invoke-G -C $script:R status --porcelain) -join "|")
    $parts += "WTSTATUS=" + ((Invoke-G -C $script:WT status --porcelain) -join "|")
    return ($parts -join "`n")
}

# ------------------------------------------------------------------ fake world
$root = New-TempPath "rp-test-"
New-Item -ItemType Directory -Path $root | Out-Null
$R = Join-Path $root "repo"
New-Item -ItemType Directory -Path $R | Out-Null
Invoke-G init -q $R | Out-Null
Write-Text (Join-Path $R ".gitignore") ".claude/worktrees/`n.journal/`n"
Write-Text (Join-Path $R "a.txt") "base`n"
Write-Text (Join-Path $R ".claude\skills\demo\SKILL.md") "# demo skill`n"
Write-Text (Join-Path $R ".claude\skills\demo\config.json") '{"name":"demo","authToken":"fake-skill-value","nested":{"password":"fake-pw","keep":"keep-me"}}'
Write-Text (Join-Path $R ".claude\skills\demo\.env") "ENV-MARKER=1`n"
Write-Text (Join-Path $R ".claude\skills\demo\candidate_base.yaml") "name: CANDIDATE-MARKER`n"
Write-Text (Join-Path $R ".claude\skills\demo\server.pem") "PEM-MARKER`n"
Write-Text (Join-Path $R ".claude\skills\demo\.claude\worktrees\leak\LEAK.txt") "LEAK-MARKER`n"
Write-Text (Join-Path $R ".claude\commands\go.md") "run it`n"
Write-Text (Join-Path $R ".claude\commands\broken.json") "{ not json"
Write-Text (Join-Path $R ".claude\settings.json") '{"permissions":{"allow":["Bash(ls)"]},"env":{"X":"fake-proj-env"}}'
Write-Text (Join-Path $R ".claude\settings.local.json") '{"permissions":{"deny":["Bash(rm)"]},"apiKey":"fake-local-key"}'
Write-Text (Join-Path $R ".mcp.json") '{"mcpServers":{"claudia":{"command":"c","headers":{"Authorization":"fake-header-value"},"env":{"X":"fake-mcp-env"}}}}'
Invoke-G -C $R add -A | Out-Null
Invoke-G -C $R commit -q -m "c1" | Out-Null
Write-Text (Join-Path $R "a.txt") "base`nsecond`n"
Invoke-G -C $R commit -q -am "c2" | Out-Null
Invoke-G -C $R branch dev | Out-Null
$mainTip = (Invoke-G -C $R rev-parse HEAD)[0]

# Linked worktree inside .claude/worktrees (as in the real project), with an uncommitted change and an untracked file.
$WT = Join-Path $R ".claude\worktrees\wt1"
Invoke-G -C $R worktree add -q -b feat/x $WT | Out-Null
Write-Text (Join-Path $WT "a.txt") "base`nsecond`nmodified-in-wt`n"
Write-Text (Join-Path $WT "WTMARK.txt") "UNTRACKED-CONTENT-MARKER`n"
Write-Text (Join-Path $WT ".journal\2026-10-01-000000-feat-x-old.md") "# old diary only in the worktree`n- Branch: feat/x @ 1234567`n"
Write-Text (Join-Path $R ".journal\2026-10-19-000000-main-aaaa-test.md") "# main diary`n- Branch: main @ 1234567`n"
Write-Text (Join-Path $R ".journal\.gitignore") "*`n"

# Fake global Claude folder and memory.
$G = Join-Path $root "global"
Write-Text (Join-Path $G "CLAUDE.md") "# global rules`n"
Write-Text (Join-Path $G "settings.json") '{"env":{"ANTHROPIC_BASE_URL":"fake-env-value"},"apiKeyHelper":"fake-helper-value","model":"opus","permissions":{"allow":["Read"]},"mcpServers":{"s":{"command":"c","headers":{"X-Api":"fake-header2"}}}}'
Write-Text (Join-Path $G ".credentials.json") '{"claudeAiOauth":{"accessToken":"CRED-MARKER"}}'
Write-Text (Join-Path $G "plugins\installed_plugins.json") '{"version":2,"plugins":{"superpowers@mkt":[{"scope":"user","installPath":"X:/fake/install/path","version":"5.0.1"}]}}'
$M = Join-Path $root "memory"
Write-Text (Join-Path $M "MEMORY.md") "- note one`n"
Write-Text (Join-Path $M "topic.md") "topic`n"
Write-Text (Join-Path $M ".credentials.json") '{"x":"CRED-MARKER-2"}'
Write-Text (Join-Path $M ".env") "MEM-ENV-MARKER=1`n"

$before = Get-Snapshot

# ------------------------------------------------------------------ 1. first run
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "1 first run: exit 0" 0
$rec1 = Get-Recovery
Assert-Case "1 first run: refs/heads/recovery exists" ([bool]$rec1) "no recovery ref. Output:`n$($res.Text)"
$parents = (Invoke-G -C $R rev-list --parents -n 1 "refs/heads/recovery") -join " "
Assert-Case "1 first run: commit has no parent" (($parents.Trim() -split '\s+').Count -eq 1) "rev-list --parents: $parents"
$mb = Invoke-GCode -C $R merge-base refs/heads/main refs/heads/recovery
Assert-Case "1 first run: no common ancestor with main (merge-base fails)" ($mb -ne 0) "merge-base exit $mb"
Assert-Case "2 HEAD, branch, index and other refs unchanged" ((Get-Snapshot) -ceq $before) "before:`n$before`nafter:`n$(Get-Snapshot)"

$paths = Get-PackPaths
$all = Get-PackAll
# Guards against vacuous checks below: the listing and the content must really be read.
Assert-Case "3 sanity: tree listing is flat and non-empty" ($paths.Count -ge 10 -and ($paths[0] -is [string])) "paths: $($paths.Count)"
Assert-Case "3 sanity: blob content was read" ($all.Contains("# demo skill") -and $all.Contains("- note one")) "content not read"
foreach ($p in @("project/.claude/skills/demo/SKILL.md", "project/.claude/skills/demo/config.json", "project/.claude/commands/go.md",
        "project/.claude/settings.json", "project/.claude/settings.local.json", "project/.mcp.json", "global/CLAUDE.md",
        "global/settings.json", "global/plugins.txt", "global/memory/MEMORY.md", "global/memory/topic.md", "STATE.md",
        "patches/feat-x.patch", "journal/2026-10-19-000000-main-aaaa-test.md", "journal/2026-10-01-000000-feat-x-old.md")) {
    Assert-Case "3 allowlist entered: $p" ($paths -ccontains $p) "missing; tree:`n$($paths -join "`n")"
}
Assert-Case "3 no .claude/worktrees path in the pack" (-not ($paths | Where-Object { $_ -match '(?i)\.claude/worktrees' })) "tree:`n$($paths -join "`n")"
foreach ($bad in @(".credentials.json", ".env", "candidate_base.yaml", "server.pem", "broken.json", "WTMARK.txt", "LEAK.txt")) {
    Assert-Case "3 not in the pack: $bad" (-not ($paths | Where-Object { $_ -like "*$bad" })) "found in tree:`n$($paths -join "`n")"
}
foreach ($mk in @("LEAK-MARKER", "CRED-MARKER", "ENV-MARKER", "MEM-ENV-MARKER", "CANDIDATE-MARKER", "PEM-MARKER", "UNTRACKED-CONTENT-MARKER")) {
    Assert-Case "3 content never in the pack: $mk" (-not $all.Contains($mk)) "marker found"
}
$state = Get-PackBlob "STATE.md"
Assert-Case "3 STATE.md lists the untracked file by name" ($state.Contains("WTMARK.txt")) "STATE.md:`n$state"
Assert-Case "3 STATE.md names the worktree branch" ($state.Contains("feat/x")) "STATE.md:`n$state"
Assert-Case "3 STATE.md uses relative paths (no temp root)" (-not $state.Contains($root) -and -not $state.Contains(($root -replace '\\', '/'))) "STATE.md:`n$state"
Assert-Case "3 STATE.md records the invalid JSON" ($state -match 'broken\.json: JSON invalido') "STATE.md:`n$state"
Assert-Case "3 patch has the uncommitted change" ((Get-PackBlob "patches/feat-x.patch").Contains("+modified-in-wt")) "patch missing the change"
$plug = Get-PackBlob "global/plugins.txt"
Assert-Case "3 plugins.txt has name and version only" ($plug.Contains("superpowers@mkt 5.0.1") -and -not $plug.Contains("fake/install")) "plugins.txt:`n$plug"

# ------------------------------------------------------------------ 4. JSON cleanup
$jsonFiles = @("global/settings.json", "project/.claude/settings.json", "project/.claude/settings.local.json", "project/.mcp.json", "project/.claude/skills/demo/config.json")
foreach ($jf in $jsonFiles) {
    $t = Get-PackBlob $jf
    Assert-Case "4 $jf has no env/headers keys" ($t -notmatch '"(env|headers)"\s*:') "content:`n$t"
    Assert-Case "4 $jf has no key/token/secret/password/auth keys" ($t -notmatch '(?i)"[^"]*(key|token|secret|password|auth)[^"]*"\s*:') "content:`n$t"
    Assert-Case "4 $jf has no fake secret values" ($t -notmatch 'fake-') "content:`n$t"
}
$gs = Get-PackBlob "global/settings.json"
Assert-Case "4 global settings keeps harmless keys" ($gs.Contains('"model"') -and $gs.Contains('"permissions"')) "content:`n$gs"
Assert-Case "4 nested harmless key kept" ((Get-PackBlob "project/.claude/skills/demo/config.json").Contains("keep-me")) "keep-me lost"

# ------------------------------------------------------------------ 5. second run without change, then with change
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "5 second run, no change: exit 0" 0
Assert-Case "5 second run, no change: no new commit" ((Get-Recovery) -eq $rec1) "recovery moved from $rec1 to $(Get-Recovery). Output:`n$($res.Text)"
Write-Text (Join-Path $M "new.md") "a new memory`n"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "5 run with change: exit 0" 0
$rec2 = Get-Recovery
Assert-Case "5 run with change: new commit" ($rec2 -and $rec2 -ne $rec1) "recovery still $rec2"
$p2 = ((Invoke-G -C $R rev-list --parents -n 1 $rec2) -join " ").Trim() -split '\s+'
Assert-Case "5 run with change: parent is the previous recovery commit" ($p2.Count -eq 2 -and $p2[1] -eq $rec1) "parents: $($p2 -join ' ')"
Assert-Case "5 still no common ancestor with main" ((Invoke-GCode -C $R merge-base refs/heads/main refs/heads/recovery) -ne 0) "merge-base found one"

# 5b. Only the generation date of STATE.md differs -> still "nothing changed". The fake repo's recovery gets a
# hand-made child commit whose STATE.md has an old date (temporary index in this test's own temp folder).
$oldState = (Get-PackBlob "STATE.md") -replace '(?m)^- Gerado em: .*$', '- Gerado em: 2000-01-01 00:00'
$stateFile = Join-Path $root "old-state.md"
Write-Text $stateFile ($oldState + "`n")
$blob = (Invoke-G -C $R hash-object -w $stateFile)[0]
$env:GIT_INDEX_FILE = Join-Path $root "test-index"
Invoke-G -C $R read-tree refs/heads/recovery | Out-Null
Invoke-G -C $R update-index --cacheinfo "100644,$blob,STATE.md" | Out-Null
$t5 = (Invoke-G -C $R write-tree)[0]
$env:GIT_INDEX_FILE = $null
$c5 = (Invoke-G -C $R commit-tree $t5 -p $rec2 -m "old date")[0]
Invoke-G -C $R update-ref refs/heads/recovery $c5 $rec2 | Out-Null
Assert-Case "5b setup: recovery points to the old-date commit" ((Get-Recovery) -eq $c5 -and $c5) "setup failed"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "5b only the date differs: exit 0" 0
Assert-Case "5b only the date differs: no new commit" ((Get-Recovery) -eq $c5) "recovery moved to $(Get-Recovery). Output:`n$($res.Text)"
$rec2 = Get-Recovery

# ------------------------------------------------------------------ 6. synthetic secret in memory -> exit 1, ref unchanged
$M2 = Join-Path $root "memory-secret"
Write-Text (Join-Path $M2 "MEMORY.md") "- note one`n"
$fake = "sk-ant-" + (-join (1..30 | ForEach-Object { [char](Get-Random -Minimum 97 -Maximum 123) }))
Write-Text (Join-Path $M2 "leak.md") "the value is $fake here`n"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M2
Assert-Exit $res "6 secret: exit 1" 1
Assert-Case "6 secret: recovery ref unchanged" ((Get-Recovery) -eq $rec2) "recovery moved to $(Get-Recovery)"
Assert-Case "6 secret: output names the file and the pattern" ($res.Text.Contains("global/memory/leak.md") -and $res.Text.Contains("anthropic-key")) "output:`n$($res.Text)"
Assert-Case "6 secret: output never shows the value" (-not $res.Text.Contains($fake) -and -not $res.Text.Contains($fake.Substring(7))) "value leaked in output"

# ------------------------------------------------------------------ 7. -WhatIf writes nothing
Write-Text (Join-Path $M "new2.md") "another memory`n"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -WhatIf
Assert-Exit $res "7 WhatIf: exit 0" 0
Assert-Case "7 WhatIf: recovery ref unchanged" ((Get-Recovery) -eq $rec2) "recovery moved to $(Get-Recovery)"
Assert-Case "7 WhatIf: lists what would enter" ($res.Text.Contains("global/memory/new2.md")) "output:`n$($res.Text)"

# ------------------------------------------------------------------ 8. default -MemoryDir derived from the main checkout path
$mainNorm = [IO.Path]::GetFullPath(((Invoke-G -C $R rev-parse --show-toplevel)[0]).Trim()).TrimEnd('\')
$enc = $mainNorm -replace '[^A-Za-z0-9]', '-'
Write-Text (Join-Path $G "projects\$enc\memory\DERIVED.md") "derived`n"
$res = Invoke-Pack -RepoDir $WT -GlobalClaudeDir $G -WhatIf
Assert-Exit $res "8 derived memory dir (run from the worktree): exit 0" 0
Assert-Case "8 derived memory dir: memory found from the main checkout path" ($res.Text.Contains("global/memory/DERIVED.md")) "output:`n$($res.Text)"

# ------------------------------------------------------------------ 9. concurrent writer: expected old value protects the ref
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -TestPretendNoRecovery
Assert-Exit $res "9 race: exit 1" 1
Assert-Case "9 race: recovery ref unchanged" ((Get-Recovery) -eq $rec2) "recovery moved to $(Get-Recovery). Output:`n$($res.Text)"

# ------------------------------------------------------------------ 10. ref guard: only refs/heads/recovery
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -TargetRef refs/heads/main
Assert-Exit $res "10 guard (main): exit 1" 1
Assert-Case "10 guard (main): main unchanged" (((Invoke-G -C $R rev-parse refs/heads/main)[0]) -eq $mainTip) "main moved"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -TargetRef refs/heads/recovery2
Assert-Exit $res "10 guard (recovery2): exit 1" 1
Assert-Case "10 guard (recovery2): ref not created" ((Invoke-G -C $R rev-parse -q --verify refs/heads/recovery2).Count -eq 0) "recovery2 exists"

# ------------------------------------------------------------------ 11. outside a repository
$NR = Join-Path $root "not-a-repo"
New-Item -ItemType Directory -Path $NR | Out-Null
$res = Invoke-Pack -RepoDir $NR -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "11 outside a repository: exit 2" 2

Assert-Case "12 after all runs: HEAD, branch, index and other refs unchanged" ((Get-Snapshot) -ceq $before) "before:`n$before`nafter:`n$(Get-Snapshot)"

Write-Output "Temp folder (left behind): $root"
if ($fail -eq 0) { Write-Output "ALL PASSED" } else { Write-Output "SOME FAILED" }
exit $fail
