# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-06-23

### Added

- **Phase 1 — PTY shell**: Embedded terminal via `portable-pty` + `xterm.js` with ConPTY backend; locked/unlocked toggle for manual control
- **Phase 2 — Persistent state**: SQLite schema for jobs, applications, sessions, and pending items; `candidate_base.yaml` + `search_variants.yaml` profile model; `strategy.md` persistence
- **Phase 3 — Credential management**: Windows Credential Manager integration via `keyring` v3; credentials never stored in plaintext or passed through the terminal
- **Phase 4 — Session triggering**: Manual trigger button; idle detection via Win32 `GetLastInputInfo`; configurable threshold and daily budget; auto-trigger toggle
- **Phase 5 — Real session invocation**: `claude --dangerously-skip-permissions --chrome` spawned inside the PTY; system prompt built from profile + strategy + memory summary; `SESSION_CHECKPOINT_REQUESTED` signal detection and auto-relaunch; Chrome extension reconnect retry logic
- **Phase 6 — Pending items and notifications**: Windows toast notifications for unresolved pending items; configurable repeat interval; `PendenciaCard` with inline resolution, skip action, and captcha special case; click-to-navigate to Pending tab
- **Phase 7 — Dashboard and history**: Job history with status filter and application history sub-views; evolution proposal indicator; real-time updates via SQLite watcher events
- **Phase 8 — Conversational profile tab**: Chat-based profile editing via `claude --print`; bubble UI with inline markdown, streaming cursor, and section-focused edit mode; `EmptyState` onboarding with CV paste / file import / GitHub import; search variant management with normalized weight bars
- **Phase 9 — Feedback tab**: Deterministic data aggregation from SQLite; trend chart (SVG polyline, 30 days) and horizontal bar charts by result and variant; manual result marking; feedback generation trigger with milestone suggestions; sidebar dot indicator
- **Design system**: 13 color tokens + Inter typeface in `src/styles/tokens.css`; glasses logo; sidebar navigation with pending badge

[Unreleased]: https://github.com/JohnGabie/claudia-rh/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/JohnGabie/claudia-rh/releases/tag/v0.1.0
