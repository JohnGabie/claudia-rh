# Claudia RH

Tauri v2 desktop app (Rust backend + React/TypeScript frontend) that automates
job hunting with Claude Code. Public-facing, open source, growing non-technical
user base — see `.claude/QUALIDADE.md` for the quality/refactor roadmap.

## Language conventions (universal open-source standard)

- **Code comments: English.** Always. Even when nearby legacy comments are in Portuguese.
- **Commit messages: English.**
- **New identifiers (variables, functions, types): English**, following common
  open-source naming. (Legacy code uses Portuguese identifiers; don't mass-rename,
  but write new code in English.)
- **User-facing UI strings: Brazilian Portuguese (pt-BR).** The app ships in pt-BR
  for a non-technical audience. Do not translate UI strings to English.

## Hard rules

- **NEVER change the Tauri `identifier`** in `src-tauri/tauri.conf.json`
  (`io.github.johngabie.claudia-rh`). It determines where user data lives and the
  installer upgrade identity; changing it strands every user's profile and DB
  (it happened in v0.2.0 — see `src-tauri/src/migration.rs`). A unit test
  (`identifier_is_frozen`) enforces this.

## Build & check

- Frontend: `npm run build` (runs `tsc` + vite). Typecheck only: `npx tsc --noEmit`.
- Backend: from `src-tauri/`, `cargo check` / `cargo test` / `cargo clippy`.
- Run the app: `npm run tauri dev`.
