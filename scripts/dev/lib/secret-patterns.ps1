# SECRET PATTERNS: this file IS the project's single reference list for secret scans (the "patterns of 2026-10-19"
# cited by docs/superpowers/specs/2026-10-19-regras-agentes-design.md). Change the patterns HERE and nowhere else.
# Used by (dot-sourced, `. "$PSScriptRoot\lib\secret-patterns.ps1"`):
#   scripts/dev/recovery-pack.ps1  (second layer of the recovery pack, P14/P17)
#   scripts/dev/scan-secrets.ps1   (scan of a commit range before the first push to the backup and before each PR)
# Defines, in the caller's scope:
#   $SecretPatterns  array of @(<name>, <regex>); the name is what reports show, never the matched value.
#   Hide-Secrets     replaces every match of every pattern with "[oculto]". Every message a scanner prints goes
#                    through it.

$SecretPatterns = @(
    @("anthropic-key", 'sk-ant-[A-Za-z0-9_\-]{20,}'),
    @("github-token", 'gh[opsu]_[A-Za-z0-9]{20,}'),
    @("github-pat", 'github_pat_[A-Za-z0-9_]{20,}'),
    @("aws-access-key", 'AKIA[0-9A-Z]{16}'),
    @("private-key", '-----BEGIN [A-Z ]*PRIVATE KEY-----'),
    @("slack-token", 'xox[baprs]-[A-Za-z0-9\-]{10,}'),
    @("google-api-key", 'AIza[0-9A-Za-z_\-]{35}'),
    @("linear-key", 'lin_api_[A-Za-z0-9]{20,}'),
    @("jwt", 'eyJ[A-Za-z0-9_\-]{10,}\.eyJ[A-Za-z0-9_\-]{10,}'),
    @("openai-project-key", 'sk-proj-[A-Za-z0-9_\-]{20,}'),
    @("sk-key", '(?<![A-Za-z0-9])sk-[A-Za-z0-9_\-]{20,}'),
    # name = value, quoted or not; no word boundary on the left so ANTHROPIC_AUTH_TOKEN / OPENAI_API_KEY match.
    @("generic-assignment", '(?i)(?<![A-Za-z0-9])(api[_-]?key|secret|passwd|password|token)(?:[_-][A-Za-z0-9]+)*["'']?\s*[:=]\s*["'']?[^"''\s]{16,}'),
    # camelCase / PascalCase names in plain text (accessToken, clientSecret, apiKey...): the generic pattern above
    # needs a non-alphanumeric char before the word, so "accessToken" or "clientSecret" escape it.
    @("camel-assignment", '(?i)(?<![A-Za-z0-9])(?:(?:access|refresh|auth|bearer|id|session|api|client)[_-]?token|(?:api|private|secret|client|access|signing|encryption)[_-]?key|(?:api|client|app)[_-]?secret)["'']?\s*[:=]\s*["'']?[^"''\s]{16,}')
)
# Masks every secret-looking substring. EVERY message a scanner prints (its own and git's) goes through it.
function Hide-Secrets([string]$Text) {
    foreach ($p in $script:SecretPatterns) { $Text = [regex]::Replace($Text, $p[1], "[oculto]") }
    return $Text
}
