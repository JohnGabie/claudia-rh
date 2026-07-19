# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities privately by emailing the maintainer via the contact
listed on their [GitHub profile](https://github.com/JohnGabie). You can also
use [GitHub's private vulnerability reporting](https://github.com/JohnGabie/claudia-rh/security/advisories/new).

Please include:
- A description of the vulnerability
- Steps to reproduce
- Potential impact

You will receive a response within 7 days. If confirmed, a fix will be released
as soon as reasonably possible.

## Security model

Claudia RH is a local-only desktop application. Understanding the threat model
helps distinguish real vulnerabilities from expected behavior:

| Area | Design decision |
|------|----------------|
| Credentials | Stored exclusively in Windows Credential Manager via the `keyring` crate — never written to disk in plaintext |
| User data | `candidate_base.yaml`, `search_variants.yaml`, `strategy.md` stay on the local machine and are excluded from version control via `.gitignore` |
| Claude sessions | Spawned with `--dangerously-skip-permissions` by design — this is required for unattended automation and is intentional, not a bug |
| Browser automation | The app controls Chrome on behalf of the user. This is the core feature, not a vulnerability |
| Network | No data is sent to any server controlled by this project. All network traffic goes through Claude Code CLI and Chrome, both controlled by the user |

## Out of scope

The following are **not** considered vulnerabilities in this project:

- The use of `--dangerously-skip-permissions` in Claude Code invocations
- Browser automation capabilities (controlling Chrome is the product)
- Absence of sandboxing for the PTY process (it runs as the current user, no privilege escalation)
