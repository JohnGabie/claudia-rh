# Contributing to Claudia RH

Thank you for your interest in contributing! Here's everything you need to get started.

## Prerequisites

Before contributing, make sure you have:

- **Windows 11** (build ≥ 26200) — PTY and idle detection are Win32-only
- **Rust** stable toolchain via `rustup`
- **Node.js** 20+
- **Claude Code CLI** installed and authenticated (`claude --version`)
- **Chrome** with the official Claude extension installed and enabled

## Development setup

```bash
git clone https://github.com/JohnGabie/claudia-rh.git
cd claudia-rh
npm install
npm run tauri dev
```

The dev build hot-reloads the frontend. Rust changes require a full rebuild.

## Project layout

```
src/                    # React 19 + TypeScript frontend
src-tauri/src/          # Rust backend (Tauri v2)
src-tauri/src/commands/ # Tauri commands exposed to frontend
src-tauri/src/db/       # SQLite schema and helpers
```

## Making changes

### Small fixes (typos, docs, UI polish)
Open a PR directly. No issue needed.

### Larger changes (new features, architecture)
Open an issue first to discuss the approach before writing code. This avoids wasted effort if the direction doesn't fit the project.

### Commit style
Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add export to PDF
fix: correct idle timer reset on mouse move
docs: update prerequisites section
refactor: extract prompt builder to separate module
```

## Pull request checklist

- [ ] The app builds without errors (`npm run tauri build`)
- [ ] No personal data, API keys, or hardcoded paths in code
- [ ] User-facing strings are bilingual (PT + EN) where the rest of the UI is
- [ ] For UI changes: tested manually in dev mode

## What's in scope

- Bug fixes
- Documentation improvements
- UI improvements that follow the design tokens in `src/styles/tokens.css`
- Performance improvements in the Rust backend
- Support for additional job platforms in the agent prompt

## What requires prior discussion

- Changes to the SQLite schema (migration strategy needed)
- New Tauri plugins or major dependencies
- Changes to how Claude Code sessions are spawned or terminated
- macOS/Linux portability (the project is currently Windows-only by design)

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). By participating you agree to uphold it.
