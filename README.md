<div align="center">

<img src="src-tauri/icons/128x128.png" alt="Claudia RH logo" width="96" height="96" />

<h1>Claudia RH</h1>

<p><strong>Automated job-application agent powered by Claude + Chrome.</strong></p>

<p>
  <a href="https://github.com/JohnGabie/claudia-rh/stargazers">
    <img src="https://img.shields.io/github/stars/JohnGabie/claudia-rh?style=for-the-badge&color=D97757&labelColor=1a1a1a" alt="GitHub stars" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-yellow?style=for-the-badge&labelColor=1a1a1a" alt="License: MIT" />
  </a>
  <img src="https://img.shields.io/badge/Tauri-v2-24C8D8?style=for-the-badge&logo=tauri&logoColor=white&labelColor=1a1a1a" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=white&labelColor=1a1a1a" alt="React 19" />
  <img src="https://img.shields.io/badge/Rust-stable-CE422B?style=for-the-badge&logo=rust&logoColor=white&labelColor=1a1a1a" alt="Rust" />
  <img src="https://img.shields.io/badge/Windows-11-0078D4?style=for-the-badge&logo=windows&logoColor=white&labelColor=1a1a1a" alt="Windows 11" />
</p>

<br/>

> ⭐ **If this project inspired or helped you, please consider leaving a star — it really matters!**

</div>

---

## What is this?

**Claudia RH** is a Windows desktop app that automates job searching and application. Instead of spending hours copy-pasting your CV across job sites, it launches an AI session that browses, analyses, and submits applications on your behalf — while you do something else.

<br/>

<table>
<tr>
<td width="50%" valign="top">

### How it works

1. **Conversational profile** — tell Claude who you are, paste your CV, define search variants (e.g. "senior remote dev" vs "hybrid tech lead")
2. **Automatic triggering** — the app detects when you're idle and launches a Claude Code session connected to Chrome
3. **Job analysis** — the agent reads each listing, cross-references your profile, and decides whether to apply
4. **Document generation** — CV and cover letter tailored to each job, with honesty enforced
5. **Pending actions & notifications** — captchas, required fields, and other blockers appear as Windows notifications for manual resolution
6. **Feedback** — periodic analysis of results with improvement suggestions

</td>
<td width="50%" valign="top">

### Features

| | Feature |
|---|---|
| 🤖 | Claude Code agent with Chrome access |
| 📄 | AI-generated CV and cover letter per job |
| 🔍 | Multiple simultaneous search variants |
| 🔔 | Windows notifications for captchas / blockers |
| 💤 | Automatic triggering on user inactivity |
| 🔐 | Credentials stored in Windows Credential Manager |
| 📊 | Dashboard with real-time statistics |
| 💬 | Profile managed via conversational chat |
| 📈 | Feedback tab with trend charts |
| 🖥️ | Integrated PTY terminal |

</td>
</tr>
</table>

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Claudia RH (Tauri)                     │
│                                                          │
│  ┌──────────┐  ┌────────┐  ┌──────────┐  ┌──────────┐  │
│  │Dashboard │  │Profile │  │ Pending  │  │ Feedback │  │
│  │          │  │ (chat) │  │ actions  │  │ (graphs) │  │
│  └──────────┘  └────────┘  └──────────┘  └──────────┘  │
│                                                          │
│       React 19 + TypeScript + xterm.js                   │
│  ─────────────────────────────────────────────────────   │
│       Rust (Tauri v2) + SQLite + keyring                 │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │  PTY Manager  →  claude --chrome  →  Chrome     │    │
│  │                   (Claude Code CLI)   Extension  │    │
│  └─────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
         ↕ Tauri events (SQLite watcher, idle watcher)
    SQLite DB: jobs · applications · pending actions · sessions
```

---

## Tech stack

<table>
<tr>
<th>Layer</th>
<th>Technology</th>
<th>Version</th>
<th>Purpose</th>
</tr>
<tr>
<td>Desktop shell</td>
<td><img src="https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white" /></td>
<td>v2</td>
<td>Native Windows app without Electron</td>
</tr>
<tr>
<td>Frontend</td>
<td><img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=white" /></td>
<td>19.1</td>
<td>Reactive UI</td>
</tr>
<tr>
<td>UI language</td>
<td><img src="https://img.shields.io/badge/TypeScript-5.8-3178C6?logo=typescript&logoColor=white" /></td>
<td>5.8</td>
<td>Type-safety on the frontend</td>
</tr>
<tr>
<td>Backend</td>
<td><img src="https://img.shields.io/badge/Rust-stable-CE422B?logo=rust&logoColor=white" /></td>
<td>2021 edition</td>
<td>System logic, PTY, SQLite, credentials</td>
</tr>
<tr>
<td>Database</td>
<td><img src="https://img.shields.io/badge/SQLite-bundled-003B57?logo=sqlite&logoColor=white" /></td>
<td>via rusqlite</td>
<td>Jobs, applications, sessions, pending actions</td>
</tr>
<tr>
<td>Terminal</td>
<td><code>xterm.js + portable-pty</code></td>
<td>6.x</td>
<td>Real PTY embedded inside the app</td>
</tr>
<tr>
<td>AI</td>
<td><img src="https://img.shields.io/badge/Claude_Code-CLI-D97757" /></td>
<td>latest</td>
<td>Application agent with Chrome access</td>
</tr>
<tr>
<td>Credentials</td>
<td><code>keyring v3</code></td>
<td>3.x</td>
<td>Windows Credential Manager — never plain text</td>
</tr>
</table>

---

## Prerequisites

<table>
<tr>
<td>

**Required**
- Windows 11 (build ≥ 26200)
- [Claude Code CLI](https://claude.ai/code) installed and authenticated
- **Claude in Chrome** extension installed in Chrome
- Rust toolchain (`rustup`)
- Node.js 20+

</td>
<td>

**Recommended**
- Claude Pro or Team account (for session volume)
- Chrome as your primary browser
- 8 GB RAM+

</td>
</tr>
</table>

---

## Claude in Chrome extension setup — critical step

> **Without this step the agent cannot control the browser and will do nothing.**

<table>
<tr>
<td width="50%" valign="top">

### Install the extension

1. Open Chrome and go to the **Chrome Web Store**
2. Search for **"Claude"** (official Anthropic extension)
3. Install the **Claude** extension in Chrome

> ⚠️ **The Google/Claude account used in Chrome must be exactly the same one authenticated in the Claude Code CLI.** If they differ, the connection fails silently.

</td>
<td width="50%" valign="top">

### Enable Chrome mode

After installing the extension, open a terminal and run Claude Code:

```bash
claude
```

Inside the session, type:

```
/chrome
```

When the **"Enable by default"** option appears, select **Yes**.

Close Claude, reopen it — from then on Chrome connects automatically in every session.

</td>
</tr>
</table>

---

## Installation

```bash
# 1. Clone
git clone https://github.com/JohnGabie/claudia-rh.git
cd claudia-rh

# 2. Install dependencies
npm install

# 3. Dev mode
npm run tauri dev

# 4. Production build
npm run tauri build
```

The `.msi` installer will be at `src-tauri/target/release/bundle/msi/`.

---

## Project structure

```
claudia-rh/
├── src/                        # React frontend
│   ├── components/
│   │   ├── Dashboard.tsx       # Statistics and session control
│   │   ├── Perfil.tsx          # Conversational profile chat
│   │   ├── Vagas.tsx           # Job history and applications
│   │   ├── Pendencias.tsx      # Pending blockers (captcha, etc.)
│   │   ├── Feedback.tsx        # Results analysis + charts
│   │   ├── Configuracoes.tsx   # Credentials, triggering, strategy
│   │   ├── Terminal.tsx        # Integrated PTY terminal
│   │   └── Sidebar.tsx         # Navigation + badges
│   └── styles/tokens.css       # Design tokens (colours, typography)
│
└── src-tauri/src/              # Rust backend
    ├── commands/
    │   ├── perfil.rs           # candidate_base.yaml + search_variants.yaml
    │   ├── sessao.rs           # Session triggering and management
    │   ├── curriculos.rs       # CV generation
    │   ├── cover_letter.rs     # Cover letter generation
    │   ├── feedback.rs         # Feedback analysis
    │   ├── credenciais.rs      # Keyring (Windows Credential Manager)
    │   └── estado.rs           # Statistics queries
    ├── db/schema.sql           # Full SQLite schema
    ├── pty_manager.rs          # PTY process management + session signals
    ├── idle_watcher.rs         # Inactivity detection (Win32 GetLastInputInfo)
    └── notificacoes.rs         # Windows notifications (Tauri plugin)
```

---

## Disclaimer

This tool automates browser actions on your behalf. Ensure the terms of service of the job platforms you use allow automation. The author is not responsible for account bans or other effects resulting from using this tool.

---

## Contributing

Issues and PRs are welcome. Before opening a large PR, open an issue to discuss the approach.

Read the [contributing guide](CONTRIBUTING.md) for dev setup, commit conventions, and PR process. This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). To report security vulnerabilities, see the [security policy](SECURITY.md).

---

<div align="center">

Built with ☕ and way too many rejected applications.

<br/>

**If you made it this far and found it useful — a star makes a difference! ⭐**

<a href="https://github.com/JohnGabie/claudia-rh/stargazers">
  <img src="https://img.shields.io/github/stars/JohnGabie/claudia-rh?style=social" alt="Star on GitHub" />
</a>

</div>
