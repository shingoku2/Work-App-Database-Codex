# Codebase Bug Sweep Report

**Date:** 2026-06-23 12:09 CDT
**Scope:** Full Antminer Fleet Manager working tree: React/TypeScript frontend, Tauri Rust client, Rust server, shared contracts, tests, repository hygiene.
**Status:** Validation clean after small Clippy cleanup. No trailing whitespace/debug leftovers found by scan.

## Fixes applied in this pass

### Clippy cleanup

**Files:**
- `server/src/api.rs`
- `server/src/main.rs`

**Changes:**
- Removed three useless `format!` wrappers around static SQL strings in miner/part handlers.
- Changed backup/restore helper args from `&PathBuf` to `&Path`.
- Added a targeted `#[allow(clippy::too_many_arguments)]` on `audit_log`; the function is intentionally a narrow DB insert helper and changing every callsite would be churnier than useful right now.

### Frontend cleanup from async audit follow-up

**Files:**
- `src/features/connection/connectionApi.ts`
- `src/features/settings/TunnelKeyRequestsView.tsx`
- `src/features/sites/SitesView.tsx`
- `src/features/connection/ConnectionGate.tsx`
- `src/features/miners/MinersView.tsx`
- `src/test/connectionApi.test.ts`

**Changes:**
- Removed unused TypeScript imports flagged by `--noUnusedLocals`.
- Updated the login API test fixture to include the full `{ token, expires_at, user }` response shape.
- Removed two `console.error` calls from user-facing UI error paths where the error is already surfaced in UI state.

### Documented tunnel-key pre-pairing exception

**Files:**
- `AGENTS.md`
- `docs/ssh-tunnel-onboarding.md`

**Changes:**
- Documented that unauthenticated `POST /api/v1/tunnel-key-requests` and token-scoped status polling are intentional pre-pairing/onboarding exceptions. These paths stay HTTPS-only and must never send private key material.

## Validation results

| Check | Result |
| --- | --- |
| `npm test` | PASS — 9 files, 96 tests |
| `npm run build` | PASS — `tsc && vite build` |
| `npx tsc --noEmit --noUnusedLocals --noUnusedParameters --pretty false` | PASS after cleanup |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo test --workspace --locked` | PASS — Rust unit/integration/doc tests |
| `cargo clippy --workspace --locked -- -D warnings` | PASS after cleanup |
| `npm audit --omit=dev` | PASS — 0 vulnerabilities |
| `git diff --check` | PASS — no whitespace errors |

## Static hygiene scan

Scanned 146 tracked/unignored files for common low-signal landmines.

- Trailing whitespace: **0**
- Tabs in code files: **0**
- Runs of 4+ blank lines: **0**
- Frontend debug/test leftovers (`console.log`, `debugger`, `.only`, `.skip`, TODO/FIXME/HACK/XXX): **0**
- Possible secret pattern hits: reviewed as code identifiers/test fixtures/config field names, not leaked secret values.
- `scripts/fleet-tunnel.local.json`: correctly ignored/untracked.
- Async audit initially found two user-facing `console.error` calls and three unused imports; all were removed.

## Notes / follow-up

- `.claude/settings.local.json` is tracked even though it is named local. It contains Claude permission settings, not app secrets, but it is repo-hygiene questionable. If this file is meant to be machine-local, remove it from Git and add `.claude/settings.local.json` to `.gitignore`.
- `git diff --check` still prints CRLF/LF conversion warnings on several modified files. They are warnings only; no whitespace errors. If those keep annoying Git, normalize line endings with `.gitattributes` in a separate cleanup.
- I did not run a full Tauri installer build or live server/tunnel E2E. Current checks cover compile, unit/integration tests, TypeScript build, and package audit.
