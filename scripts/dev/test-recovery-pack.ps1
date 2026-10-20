# Test for recovery-pack.ps1 (P14/P17 recovery pack on the orphan branch `recovery`).
# Everything is FAKE and lives in fresh temp folders created here ([IO.Path]::GetTempPath() + GUID): a git
# repository with commits, a linked worktree with uncommitted changes and an untracked file, .journal diaries,
# a fake global Claude folder (CLAUDE.md, settings.json with env/apiKeyHelper/headers, a fake .credentials.json,
# a plugin list), a fake memory folder, .claude/skills and .claude/worktrees, forbidden files and folders (also in
# mixed case), and junctions that point to other folders INSIDE the same temp folder. Every secret is synthetic,
# generated here at run time. T20 adds: camelCase assignments in plain text, junctions at intermediate levels
# between a fake global folder and the memory, and masking of the script's and git's error messages (a fake repo
# under a folder whose name is a synthetic secret, with a stale recovery.lock that makes update-ref fail).
# Never reads or writes the user's home, ~/.claude, AppData or any real repository. Nothing is deleted: every
# scenario that needs a different input gets a new folder (or a new fake world). Temp folders are left behind.
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
function Get-Letters([int]$n) { return (-join (1..$n | ForEach-Object { [char](Get-Random -Minimum 97 -Maximum 123) })) }
# Junction created only inside this test's temp folder; never removed.
function New-Junction([string]$link, [string]$target) {
    $parent = Split-Path -Parent $link
    if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Path $parent | Out-Null }
    New-Item -ItemType Junction -Path $link -Target $target | Out-Null
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

# ------------------------------------------------------------------ fake world builder
# Sets $script:root, $R (main checkout), $WT (linked worktree), $G (global folder), $M (memory), $mainTip.
# -RepoParent puts the repository one folder deeper (root\<RepoParent>\repo), e.g. a folder name that holds a
# synthetic secret, so git error messages that carry the repository path can be checked for masking.
function New-World([string]$Tag, [switch]$CommandsAsJunction, [string]$RepoParent = "") {
    $script:root = New-TempPath "rp-test-$Tag-"
    New-Item -ItemType Directory -Path $script:root | Out-Null
    $rt = $script:root
    if ($RepoParent) { $script:R = Join-Path $rt "$RepoParent\repo" } else { $script:R = Join-Path $rt "repo" }
    $repo = $script:R
    New-Item -ItemType Directory -Path $repo | Out-Null
    Invoke-G init -q $repo | Out-Null
    Write-Text (Join-Path $repo ".gitignore") ".claude/worktrees/`n.journal/`n"
    Write-Text (Join-Path $repo "a.txt") "base`n"
    # Tracked forbidden files in mixed case (their uncommitted changes must stay out of the patch).
    Write-Text (Join-Path $repo "Candidate_Base.yaml") "name: base`n"
    Write-Text (Join-Path $repo ".ENV.sample") "X=base`n"
    $sk = Join-Path $repo ".claude\skills\demo"
    Write-Text (Join-Path $sk "SKILL.md") "# demo skill`n"
    Write-Text (Join-Path $sk "config.json") '{"name":"demo","authToken":"fake-skill-value","nested":{"password":"fake-pw","keep":"keep-me"}}'
    Write-Text (Join-Path $sk ".env") "ENV-MARKER=1`n"
    Write-Text (Join-Path $sk ".ENV") "MIXED-ENV-ONE=1`n"
    Write-Text (Join-Path $sk "sub\.Env.Local") "MIXED-ENV-TWO=1`n"
    Write-Text (Join-Path $sk "candidate_base.yaml") "name: CANDIDATE-MARKER`n"
    Write-Text (Join-Path $sk "sub\Candidate_Base.yaml") "name: MIXED-CANDIDATE-MARKER`n"
    Write-Text (Join-Path $sk "server.pem") "PEM-MARKER`n"
    Write-Text (Join-Path $sk ".claude\worktrees\leak\LEAK.txt") "LEAK-MARKER`n"
    # Under another parent: the file system ignores case, so next to .claude\worktrees it would reuse that folder.
    Write-Text (Join-Path $sk "sub\.Claude\Worktrees\leak2\LEAK2.txt") "LEAK-TWO-MARKER`n"
    # Folders with forbidden names.
    Write-Text (Join-Path $sk ".env.d\inner.txt") "ENVD-MARKER`n"
    Write-Text (Join-Path $sk "My-Credentials\x.txt") "CREDDIR-MARKER`n"
    Write-Text (Join-Path $sk "Secrets\s.txt") "SECRETSDIR-MARKER`n"
    if (-not $CommandsAsJunction) {
        Write-Text (Join-Path $repo ".claude\commands\go.md") "run it`n"
        Write-Text (Join-Path $repo ".claude\commands\broken.json") "{ not json"
    }
    Write-Text (Join-Path $repo ".claude\settings.json") '{"permissions":{"allow":["Bash(ls)"]},"env":{"X":"fake-proj-env"}}'
    Write-Text (Join-Path $repo ".claude\settings.local.json") '{"permissions":{"deny":["Bash(rm)"]},"apiKey":"fake-local-key"}'
    Write-Text (Join-Path $repo ".mcp.json") '{"mcpServers":{"claudia":{"command":"c","headers":{"Authorization":"fake-header-value"},"env":{"X":"fake-mcp-env"}}}}'
    Invoke-G -C $repo add -A | Out-Null
    Invoke-G -C $repo commit -q -m "c1" | Out-Null
    Write-Text (Join-Path $repo "a.txt") "base`nsecond`n"
    Invoke-G -C $repo commit -q -am "c2" | Out-Null
    Invoke-G -C $repo branch dev | Out-Null
    $script:mainTip = (Invoke-G -C $repo rev-parse HEAD)[0]

    # Junctions (after the commits), pointing to folders inside this temp folder.
    Write-Text (Join-Path $rt "outside-skill\o.txt") "OUTSIDE-SKILL-MARKER`n"
    New-Junction (Join-Path $sk "linked") (Join-Path $rt "outside-skill")
    if ($CommandsAsJunction) {
        Write-Text (Join-Path $rt "outside-cmds\outside-cmd.md") "OUTSIDE-CMD-MARKER`n"
        New-Junction (Join-Path $repo ".claude\commands") (Join-Path $rt "outside-cmds")
    }

    # Linked worktree inside .claude/worktrees (as in the real project).
    $script:WT = Join-Path $repo ".claude\worktrees\wt1"
    $wt = $script:WT
    Invoke-G -C $repo worktree add -q -b feat/x $wt | Out-Null
    Write-Text (Join-Path $wt "a.txt") "base`nsecond`nmodified-in-wt`n"
    Write-Text (Join-Path $wt "Candidate_Base.yaml") "name: PATCH-CANDIDATE-MARKER`n"
    Write-Text (Join-Path $wt ".ENV.sample") "X=PATCH-ENV-MARKER`n"
    Write-Text (Join-Path $wt "WTMARK.txt") "UNTRACKED-CONTENT-MARKER`n"
    Write-Text (Join-Path $wt ".journal\2026-10-01-000000-feat-x-old.md") "# old diary only in the worktree`n- Branch: feat/x @ 1234567`n"
    Write-Text (Join-Path $repo ".journal\2026-10-19-000000-main-aaaa-test.md") "# main diary`n- Branch: main @ 1234567`n"
    Write-Text (Join-Path $repo ".journal\.gitignore") "*`n"

    $script:G = Join-Path $rt "global"
    Write-Text (Join-Path $script:G "CLAUDE.md") "# global rules`n"
    Write-Text (Join-Path $script:G "settings.json") '{"env":{"ANTHROPIC_BASE_URL":"fake-env-value"},"apiKeyHelper":"fake-helper-value","model":"opus","permissions":{"allow":["Read"]},"mcpServers":{"s":{"command":"c","headers":{"X-Api":"fake-header2"}}}}'
    Write-Text (Join-Path $script:G ".credentials.json") '{"claudeAiOauth":{"accessToken":"CRED-MARKER"}}'
    Write-Text (Join-Path $script:G "plugins\installed_plugins.json") '{"version":2,"plugins":{"superpowers@mkt":[{"scope":"user","installPath":"X:/fake/install/path","version":"5.0.1"}]}}'
    $script:M = Join-Path $rt "memory"
    Write-Text (Join-Path $script:M "MEMORY.md") "- note one`n"
    Write-Text (Join-Path $script:M "topic.md") "topic`n"
    Write-Text (Join-Path $script:M ".credentials.json") '{"x":"CRED-MARKER-2"}'
    Write-Text (Join-Path $script:M ".env") "MEM-ENV-MARKER=1`n"
}

# A secret case: a new memory folder with MEMORY.md plus one file; the run must exit 1 and leave the ref alone.
function Test-SecretCase([string]$Label, [string]$FileName, [byte[]]$Bytes, [string]$Pattern, [string]$Value, [string]$Where) {
    $dir = Join-Path $script:root ("mem-" + [guid]::NewGuid().ToString("N"))
    Write-Text (Join-Path $dir "MEMORY.md") "- note one`n"
    [IO.File]::WriteAllBytes((Join-Path $dir $FileName), $Bytes)
    $before = Get-Recovery
    $res = Invoke-Pack -RepoDir $script:R -GlobalClaudeDir $script:G -MemoryDir $dir
    Assert-Exit $res "$Label : exit 1" 1
    Assert-Case "$Label : recovery ref unchanged" ((Get-Recovery) -eq $before) "recovery moved to $(Get-Recovery)"
    Assert-Case "$Label : output names the pattern $Pattern" ($res.Text.Contains(":: $Pattern")) "output:`n$($res.Text)"
    Assert-Case "$Label : output names the place ($Where)" ($res.Text.Contains($Where)) "output:`n$($res.Text)"
    Assert-Case "$Label : output never shows the value" (-not $res.Text.Contains($Value)) "value leaked in output"
    Assert-Case "$Label : warns that the temp copy must be deleted by hand" ($res.Text -match 'contem a copia com o possivel segredo') "output:`n$($res.Text)"
}
function Get-Utf8Bytes([string]$s) { return $script:Utf8.GetBytes($s) }

# ================================================================== main world
New-World "main"
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
Assert-Case "3 no .claude/worktrees path in the pack (any case)" (-not ($paths | Where-Object { $_ -match '(?i)\.claude/worktrees' })) "tree:`n$($paths -join "`n")"
Assert-Case "3 no .env* / candidate_base path in the pack (any case)" (-not ($paths | Where-Object { $_ -match '(?i)(^|/)\.env|candidate_base' })) "tree:`n$($paths -join "`n")"
foreach ($bad in @(".credentials.json", "server.pem", "broken.json", "WTMARK.txt", "LEAK.txt", "LEAK2.txt", "inner.txt", "x.txt", "s.txt", "o.txt")) {
    Assert-Case "3 not in the pack: $bad" (-not ($paths | Where-Object { $_ -like "*/$bad" })) "found in tree:`n$($paths -join "`n")"
}
foreach ($mk in @("LEAK-MARKER", "LEAK-TWO-MARKER", "CRED-MARKER", "ENV-MARKER", "MEM-ENV-MARKER", "CANDIDATE-MARKER", "PEM-MARKER",
        "UNTRACKED-CONTENT-MARKER", "MIXED-ENV-ONE", "MIXED-ENV-TWO", "MIXED-CANDIDATE-MARKER", "ENVD-MARKER", "CREDDIR-MARKER",
        "SECRETSDIR-MARKER", "OUTSIDE-SKILL-MARKER", "PATCH-CANDIDATE-MARKER", "PATCH-ENV-MARKER")) {
    Assert-Case "3 content never in the pack: $mk" (-not $all.Contains($mk)) "marker found"
}
$state = Get-PackBlob "STATE.md"
Assert-Case "3 STATE.md lists the untracked file by name" ($state.Contains("WTMARK.txt")) "STATE.md:`n$state"
Assert-Case "3 STATE.md names the worktree branch" ($state.Contains("feat/x")) "STATE.md:`n$state"
Assert-Case "3 STATE.md uses relative paths (no temp root)" (-not $state.Contains($root) -and -not $state.Contains(($root -replace '\\', '/'))) "STATE.md:`n$state"
Assert-Case "3 STATE.md records the invalid JSON" ($state -match 'broken\.json: JSON invalido') "STATE.md:`n$state"
Assert-Case "3 STATE.md records the junction as not followed" ($state -match 'skills/demo/linked/: link \(nao seguido\)') "STATE.md:`n$state"
$patch = Get-PackBlob "patches/feat-x.patch"
Assert-Case "3 patch has the uncommitted change" ($patch.Contains("+modified-in-wt")) "patch missing the change"
Assert-Case "3 patch leaves out Candidate_Base.yaml and .ENV.sample (mixed case)" ($patch -notmatch '(?i)candidate_base|\.env\.sample') "patch:`n$patch"
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

# ------------------------------------------------------------------ 6. synthetic secrets -> exit 1, ref unchanged
$v = "sk-ant-" + (Get-Letters 30)
Test-SecretCase "6a secret in memory content" "leak.md" (Get-Utf8Bytes "the value is $v here`n") "anthropic-key" $v "global/memory/leak.md"
$v = "ghp_" + (Get-Letters 30)
Test-SecretCase "6b secret in a file NAME" "note-$v.md" (Get-Utf8Bytes "harmless`n") "github-token" $v "(no NOME do arquivo)"
$v = Get-Letters 40
Test-SecretCase "6c ANTHROPIC_AUTH_TOKEN quoted" "env-note.md" (Get-Utf8Bytes "`"ANTHROPIC_AUTH_TOKEN`": `"$v`"`n") "generic-assignment" $v "global/memory/env-note.md"
$v = Get-Letters 24
Test-SecretCase "6d password without quotes" "pw.md" (Get-Utf8Bytes "password = $v`n") "generic-assignment" $v "global/memory/pw.md"
$v = "sk-proj-" + (Get-Letters 30)
Test-SecretCase "6e OPENAI_API_KEY=sk-proj-" "openai.md" (Get-Utf8Bytes "OPENAI_API_KEY=$v`n") "openai-project-key" $v "global/memory/openai.md"
$v = "sk-ant-" + (Get-Letters 30)
Test-SecretCase "6f UTF-16 WITH BOM is scanned" "wide-bom.md" ([byte[]](@(0xFF, 0xFE) + [Text.Encoding]::Unicode.GetBytes("value $v`n"))) "anthropic-key" $v "global/memory/wide-bom.md"
# 6g. Secret in a tracked, uncommitted change (lands in the patch). The worktree file is put back afterwards.
$v = "sk-ant-" + (Get-Letters 30)
Write-Text (Join-Path $WT "a.txt") "base`nsecond`nmodified-in-wt`nkey $v`n"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "6g secret in the patch: exit 1" 1
Assert-Case "6g secret in the patch: recovery ref unchanged" ((Get-Recovery) -eq $rec2) "recovery moved"
Assert-Case "6g secret in the patch: names the patch and the pattern" ($res.Text.Contains("patches/feat-x.patch :: anthropic-key")) "output:`n$($res.Text)"
Assert-Case "6g secret in the patch: value never shown" (-not $res.Text.Contains($v)) "value leaked"
Write-Text (Join-Path $WT "a.txt") "base`nsecond`nmodified-in-wt`n"

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

# ------------------------------------------------------------------ 8b. junction as the ROOT of a source is never followed
Write-Text (Join-Path $root "outside-mem\outside-mem.md") "OUTSIDE-MEM-MARKER`n"
New-Junction (Join-Path $root "mem-link") (Join-Path $root "outside-mem")
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir (Join-Path $root "mem-link") -WhatIf
Assert-Exit $res "8b memory dir junction: exit 0" 0
Assert-Case "8b memory dir junction: not followed" (-not $res.Text.Contains("outside-mem.md") -and $res.Text.Contains("memory/: link (nao seguido)")) "output:`n$($res.Text)"
New-Junction (Join-Path $root "global-link") $G
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir (Join-Path $root "global-link") -MemoryDir $M -WhatIf
Assert-Exit $res "8b global dir junction: exit 0" 0
Assert-Case "8b global dir junction: not followed" (-not $res.Text.Contains("global/CLAUDE.md") -and $res.Text.Contains("global/: link (nao seguido)")) "output:`n$($res.Text)"

# ------------------------------------------------------------------ 9. concurrent writer: expected old value protects the ref
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -TestPretendNoRecovery
Assert-Exit $res "9 race: exit 1" 1
Assert-Case "9 race: recovery ref unchanged" ((Get-Recovery) -eq $rec2) "recovery moved to $(Get-Recovery). Output:`n$($res.Text)"

# ------------------------------------------------------------------ 10. ref guards: only refs/heads/recovery
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -TargetRef refs/heads/main
Assert-Exit $res "10a early guard (main): exit 1" 1
Assert-Case "10a early guard (main): refused before building anything" (-not $res.Text.Contains("Pasta temporaria") -and $res.Text.Contains("Nada foi montado")) "output:`n$($res.Text)"
Assert-Case "10a early guard (main): main unchanged" (((Invoke-G -C $R rev-parse refs/heads/main)[0]) -eq $mainTip) "main moved"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -TargetRef refs/heads/main -TestLateGuard
Assert-Exit $res "10b late guard (main): exit 1" 1
Assert-Case "10b late guard (main): main unchanged" (((Invoke-G -C $R rev-parse refs/heads/main)[0]) -eq $mainTip) "main moved"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -TargetRef refs/heads/recovery2 -TestLateGuard
Assert-Exit $res "10c late guard (recovery2): exit 1" 1
Assert-Case "10c late guard (recovery2): ref not created" ((Invoke-G -C $R rev-parse -q --verify refs/heads/recovery2).Count -eq 0) "recovery2 exists"

# ------------------------------------------------------------------ 10d. BOM-less UTF-16 file: not scannable -> never enters
$MU = Join-Path $root "mem-utf16"
Write-Text (Join-Path $MU "MEMORY.md") "- note one`n"
[IO.File]::WriteAllBytes((Join-Path $MU "wide.md"), [Text.Encoding]::Unicode.GetBytes("value sk-ant-" + (Get-Letters 30) + "`n"))
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $MU
Assert-Exit $res "10d UTF-16 without BOM: exit 0" 0
Assert-Case "10d UTF-16 without BOM: not in the pack" (-not ((Get-PackPaths) | Where-Object { $_ -like "*wide.md" })) "wide.md entered"
Assert-Case "10d UTF-16 without BOM: recorded in STATE.md" ((Get-PackBlob "STATE.md") -match 'memory/wide\.md: nao varrivel') "STATE.md:`n$(Get-PackBlob 'STATE.md')"

# ------------------------------------------------------------------ 11. outside a repository
$NR = Join-Path $root "not-a-repo"
New-Item -ItemType Directory -Path $NR | Out-Null
$res = Invoke-Pack -RepoDir $NR -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "11 outside a repository: exit 2" 2

Assert-Case "12 after all runs: HEAD, branch, index and other refs unchanged" ((Get-Snapshot) -ceq $before) "before:`n$before`nafter:`n$(Get-Snapshot)"
Write-Output "Temp folder (left behind): $root"

# ================================================================== world B: secret in the NAME of an untracked file
New-World "untracked-name"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "13 setup run: exit 0" 0
$recB = Get-Recovery
$v = "ghp_" + (Get-Letters 30)
Write-Text (Join-Path $WT "untracked-$v.txt") "harmless`n"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "13 secret in an untracked name: exit 1" 1
Assert-Case "13 secret in an untracked name: recovery ref unchanged" ($recB -and (Get-Recovery) -eq $recB) "recovery moved"
Assert-Case "13 secret in an untracked name: STATE.md and pattern named" ($res.Text.Contains("STATE.md :: github-token")) "output:`n$($res.Text)"
Assert-Case "13 secret in an untracked name: value never shown" (-not $res.Text.Contains($v)) "value leaked"
Write-Output "Temp folder (left behind): $root"

# ================================================================== world C: .claude\commands is a junction
New-World "cmd-junction" -CommandsAsJunction
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "14 commands junction: exit 0" 0
$allC = Get-PackAll
Assert-Case "14 commands junction: not followed" (-not $allC.Contains("OUTSIDE-CMD-MARKER") -and -not ((Get-PackPaths) | Where-Object { $_ -like "*outside-cmd.md" })) "outside content entered"
Assert-Case "14 commands junction: recorded" ((Get-PackBlob "STATE.md") -match '\.claude/commands/: link \(nao seguido\)') "STATE.md:`n$(Get-PackBlob 'STATE.md')"
Write-Output "Temp folder (left behind): $root"

# ================================================================== world D: T20 (camelCase names, links in between, masking)
New-World "t20"
# 15. camelCase / PascalCase assignments in plain text -> exit 1, ref unchanged, value never shown.
$v = Get-Letters 30
Test-SecretCase "15a accessToken: in plain text" "camel1.md" (Get-Utf8Bytes "accessToken: $v`n") "camel-assignment" $v "global/memory/camel1.md"
$v = Get-Letters 30
Test-SecretCase "15b clientSecret = quoted" "camel2.md" (Get-Utf8Bytes "clientSecret = `"$v`"`n") "camel-assignment" $v "global/memory/camel2.md"
$v = Get-Letters 30
Test-SecretCase "15c apiKey= without quotes" "camel3.md" (Get-Utf8Bytes "apiKey=$v`n") "camel-assignment" $v "global/memory/camel3.md"
$v = Get-Letters 30
Test-SecretCase "15d RefreshToken (PascalCase)" "camel4.md" (Get-Utf8Bytes "RefreshToken: '$v'`n") "camel-assignment" $v "global/memory/camel4.md"
# 15e. Ordinary text with those words is not a finding.
$benign = Join-Path $root "mem-benign"
Write-Text (Join-Path $benign "MEMORY.md") "- note one`n"
Write-Text (Join-Path $benign "words.md") ("the token is described in docs`napiKey: <your key here>`naccessToken: see the docs for details`n" +
    "clientSecret = short`nThe accessToken field holds the value returned by the login call.`n")
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $benign
Assert-Exit $res "15e ordinary text: exit 0 (no finding)" 0
Assert-Case "15e ordinary text: entered the pack" ((Get-PackPaths) -ccontains "global/memory/words.md") "output:`n$($res.Text)"

# 16. A junction at an intermediate level between the global folder and the memory keeps the memory out.
$mainNormD = [IO.Path]::GetFullPath(((Invoke-G -C $R rev-parse --show-toplevel)[0]).Trim()).TrimEnd('\')
$encD = $mainNormD -replace '[^A-Za-z0-9]', '-'
# 16 control: a real (non-link) projects\<encoded>\memory under a fresh global folder enters.
$G3 = Join-Path $root "global3"
Write-Text (Join-Path $G3 "CLAUDE.md") "# g3`n"
Write-Text (Join-Path $G3 "projects\$encD\memory\CONTROL.md") "control`n"
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G3 -WhatIf
Assert-Case "16 control: real folders, memory enters" ($res.Code -eq 0 -and $res.Text.Contains("global/memory/CONTROL.md")) "output:`n$($res.Text)"
# 16a. <global>\projects is a junction.
$G4 = Join-Path $root "global4"
Write-Text (Join-Path $G4 "CLAUDE.md") "# g4`n"
Write-Text (Join-Path $root "outside-projects\$encD\memory\MEMORY.md") "JUNCTION-PROJECTS-MARKER`n"
New-Junction (Join-Path $G4 "projects") (Join-Path $root "outside-projects")
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G4
Assert-Exit $res "16a projects junction: exit 0" 0
$st = Get-PackBlob "STATE.md"
Assert-Case "16a projects junction: memory not in the pack" (-not ((Get-PackPaths) | Where-Object { $_ -like "global/memory/*" }) -and -not (Get-PackAll).Contains("JUNCTION-PROJECTS-MARKER")) "paths:`n$((Get-PackPaths) -join "`n")"
Assert-Case "16a projects junction: reason in STATE.md" ($st -match 'memory/: link num nivel entre a pasta global e a memoria \(global/projects\)') "STATE.md:`n$st"
Assert-Case "16a projects junction: STATE.md hides the encoded project folder" (-not $st.Contains($encD)) "STATE.md:`n$st"
# 16b. <global>\projects\<encoded> is a junction.
$G5 = Join-Path $root "global5"
Write-Text (Join-Path $G5 "CLAUDE.md") "# g5`n"
Write-Text (Join-Path $root "outside-enc\memory\MEMORY.md") "JUNCTION-ENC-MARKER`n"
New-Junction (Join-Path $G5 "projects\$encD") (Join-Path $root "outside-enc")
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G5
Assert-Exit $res "16b encoded folder junction: exit 0" 0
$st = Get-PackBlob "STATE.md"
Assert-Case "16b encoded folder junction: memory not in the pack" (-not ((Get-PackPaths) | Where-Object { $_ -like "global/memory/*" }) -and -not (Get-PackAll).Contains("JUNCTION-ENC-MARKER")) "paths:`n$((Get-PackPaths) -join "`n")"
Assert-Case "16b encoded folder junction: reason in STATE.md" ($st -match 'memory/: link num nivel entre a pasta global e a memoria \(global/projects/\.\.\.\)') "STATE.md:`n$st"
Assert-Case "16b encoded folder junction: STATE.md hides the encoded project folder" (-not $st.Contains($encD)) "STATE.md:`n$st"

# 17. The script's own messages are masked.
$v = "sk-ant-" + (Get-Letters 30)
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M -TargetRef "refs/heads/$v"
Assert-Exit $res "17a early guard message: exit 1" 1
Assert-Case "17a early guard message: masked" ($res.Text.Contains("[oculto]") -and -not $res.Text.Contains($v)) "output:`n$($res.Text)"
$v = "sk-ant-" + (Get-Letters 30)
$NR2 = Join-Path $root "not-a-repo-$v"
New-Item -ItemType Directory -Path $NR2 | Out-Null
$res = Invoke-Pack -RepoDir $NR2 -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "17b not-a-repository message: exit 2" 2
Assert-Case "17b not-a-repository message: masked" ($res.Text.Contains("[oculto]") -and -not $res.Text.Contains($v)) "output:`n$($res.Text)"
Write-Output "Temp folder (left behind): $root"

# ================================================================== world E: a git error that carries a secret is masked
# The repository lives under a folder whose NAME is a synthetic secret (nothing in the pack shows that folder, so
# the scan passes). A stale refs/heads/recovery.lock makes `git update-ref` fail with a message that holds the
# absolute path of the .git folder, i.e. the secret.
$vLock = "sk-ant-" + (Get-Letters 30)
New-World "lock" -RepoParent $vLock
Write-Text (Join-Path $R ".git\refs\heads\recovery.lock") ""
$res = Invoke-Pack -RepoDir $R -GlobalClaudeDir $G -MemoryDir $M
Assert-Exit $res "18 git update-ref error: exit 1" 1
Assert-Case "18 git update-ref error: the git message is printed" ($res.Text.Contains("nao pode ser gravada") -and $res.Text.Contains("recovery.lock")) "output:`n$($res.Text)"
Assert-Case "18 git update-ref error: secret masked" ($res.Text.Contains("[oculto]") -and -not $res.Text.Contains($vLock)) "output:`n$($res.Text)"
Assert-Case "18 git update-ref error: recovery ref not created" (-not (Get-Recovery)) "recovery exists"
Write-Output "Temp folder (left behind): $root"

if ($fail -eq 0) { Write-Output "ALL PASSED" } else { Write-Output "SOME FAILED" }
exit $fail
