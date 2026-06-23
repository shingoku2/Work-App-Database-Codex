# Rust Major Dependency Migration Final Report

## Summary

Integrated backend commit `863e4fe` and latest `origin/master` commit `af5b259` into `chore/frontend-tauri-major-deps`, then resolved lockfile/server/frontend conflicts with a regenerated coherent lockfile.

Migrated:
- `keyring` 3 -> 4
- `reqwest` 0.12 -> 0.13
- `sqlx` 0.8 -> 0.9
- `rand` 0.9 -> 0.10
- `tower-http` 0.6 -> 0.7
- `toml` 0.9 -> 1.1

## Changed files

Dependency/integration:
- `Cargo.lock`
- `Cargo.toml`
- `server/Cargo.toml`
- `src-tauri/Cargo.toml` (committed earlier on branch)
- `package.json`
- `package-lock.json`
- `crates/fleet-shared/Cargo.toml`

Backend/source and tests:
- `server/src/api.rs`
- `server/src/auth.rs` (from backend merge)
- `server/src/config.rs`
- `server/src/main.rs`
- `server/tests/config_cli.rs` (from backend merge)
- `server/tests/tunnel_key_scripts.rs` (from backend merge)

Frontend/Tauri/source and tests:
- `src-tauri/src/client.rs`
- `src-tauri/src/commands/mod.rs`
- `src/features/connection/ConnectionGate.tsx`
- `src/features/connection/connectionApi.ts`
- `src/features/dashboard/DashboardView.tsx`
- `src/features/inventory/InventoryView.tsx`
- `src/features/inventory/partApi.ts`
- `src/features/miners/MinersView.tsx`
- `src/features/settings/TunnelKeyRequestsView.tsx`
- `src/features/sites/SitesView.tsx`
- `src/test/ConnectionGate.test.tsx`
- `src/test/connectionApi.test.ts`
- `src/test/partApi.test.ts`
- `src/types/db.ts`

Docs/reports:
- `AGENTS.md`
- `docs/ssh-tunnel-onboarding.md`
- `.codex/reports/backend-rust-major-deps-*` (from backend merge)
- `.codex/reports/backend-to-frontend-rust-major-deps-handoff.md` (from backend merge)
- `.codex/reports/frontend-rust-major-deps-*`
- `.codex/reports/frontend-rust-major-deps-runtime-validation.md`
- `.codex/reports/CODEX_AUDITOR.md`
- `.codex/reports/rust-major-dependency-migration-final.md`
- `audit.toml`
- `eslint.config.js`

## Backend validation

| Command | Result | Notes |
|---|---:|---|
| `cargo fmt --all -- --check` | PASS | Ran after conflict resolution/formatting. |
| `CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-server-target' cargo check -p antminer-fleet-server --locked` | PASS | Server compile passed with integrated lockfile. |
| `CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-server-target' cargo test -p fleet-shared -p antminer-fleet-server --locked` | PASS | 19 Rust backend/shared tests passed: 11 server unit, 4 config CLI, 1 tunnel script, 3 shared. |
| `CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-workspace-target' cargo check --workspace --locked` | PASS | Full workspace compile passed. |
| `CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-workspace-target' cargo test --workspace --locked` | PASS | Full workspace tests passed: Tauri 5, server 16, shared 3, doc tests 0. |
| `validate-config` | SKIPPED | `server/config/server.local.toml` is absent on this host. |
| `migrate` | SKIPPED | Local PostgreSQL/runtime config absent. |
| `/health` smoke | SKIPPED | Requires live local server/tunnel endpoint. |
| `/pairing` smoke | SKIPPED | Requires live local server/tunnel endpoint. |

## Frontend/Tauri validation

| Command | Result | Notes |
|---|---:|---|
| `npm ci` | PASS | 415 packages installed/audited after latest `origin/master` introduced ESLint tooling. |
| `npm test` | PASS | 9 files / 96 tests passed. |
| `npm run build` | PASS | TypeScript + Vite production build passed. |
| `npm audit --omit=dev` | PASS | 0 production vulnerabilities. |
| `npm outdated --json` | NON-BLOCKING | Returned patch/minor available updates for `@tanstack/react-query`, `@vitejs/plugin-react`, `autoprefixer`, and `vite`; also reports ESLint 10 as latest but ESLint was pinned to 9.39.4 because `eslint-plugin-react@7.37.5` does not accept ESLint 10. No prod vulnerability. Deferred from this Rust migration. |
| `CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-tauri-target' cargo check -p antminer-fleet-manager --locked` | PASS | Tauri Rust compile passed. |
| `npm run tauri:build` | PASS | Built raw binary and NSIS installer. |
| `git diff --check` | PASS | No whitespace errors; LF-to-CRLF warnings only. |

## Runtime validation

| Flow | Result | Notes |
|---|---:|---|
| Tauri package build | PASS | Installer generated successfully. |
| Raw binary | PASS | `target/release/antminer-fleet-manager.exe` — 15,408,640 bytes. |
| NSIS installer | PASS | `target/release/bundle/nsis/Antminer Fleet Manager_0.3.0_x64-setup.exe` — 3,553,676 bytes. |
| Backend local PostgreSQL/TLS smoke | SKIPPED | `server/config/server.local.toml` absent. |
| Desktop pairing/login/token smoke | SKIPPED | Requires interactive desktop run against live server. |
| SSH tunnel runtime smoke | SKIPPED | Requires local tunnel config and reachable SSH host. |

## Security checks

- [x] No plaintext bearer-token storage added.
- [x] Tauri token storage remains OS credential-manager backed through `keyring`.
- [x] No SSH private keys committed.
- [x] TLS pinning remains exact paired leaf-certificate verification for authenticated API clients.
- [x] Pre-pairing invalid-cert acceptance remains narrowly scoped to one-shot HTTPS bootstrap endpoints only.
- [x] `/health` and `/pairing` remain unauthenticated by contract.
- [x] Tunnel-key request/status pre-pairing exceptions are documented as HTTPS-only and public-key-only.
- [x] API bearer auth still required for protected endpoints.
- [x] Tunnel topology still uses local `127.0.0.1:8443`.

## Known gaps

- Live PostgreSQL/TLS smoke tests were skipped because `server/config/server.local.toml` is absent on this host.
- Interactive desktop runtime checks for pairing/login/credential persistence/logout were skipped.
- SSH tunnel start/status smoke was skipped because it needs local tunnel config and a reachable SSH host.
- `npm outdated --json` is not empty; listed packages are patch/minor frontend updates and intentionally deferred from this Rust dependency migration. ESLint 10 is also listed as latest, but ESLint is intentionally pinned to 9.39.4 until `eslint-plugin-react` supports ESLint 10.

## Commands run

| Command | Result |
|---|---:|
| `git fetch origin master chore/backend-rust-major-deps || git fetch origin` | PASS with expected missing remote backend branch; `origin/master` fetched. |
| `git merge master` | PASS after resolving `Cargo.lock` conflict. |
| `git stash apply stash@{0}` | PASS after resolving conflicts in `Cargo.lock`, `server/Cargo.toml`, `server/src/api.rs`, and `server/tests/tunnel_key_scripts.rs`. |
| `git merge origin/master` | PASS after resolving conflicts in `Cargo.lock`, `server/src/api.rs`, `src/features/connection/connectionApi.ts`, and `src/features/settings/TunnelKeyRequestsView.tsx`. |
| `npm install --save-dev eslint@^9.39.4 @eslint/js@^9.39.4` | PASS; fixed latest `origin/master`'s npm peer-dependency conflict with `eslint-plugin-react`. |
| `cargo generate-lockfile` | PASS |
| `cargo fmt --all` | PASS |
| `cargo fmt --all -- --check` | PASS |
| `npm ci` | PASS |
| `npm test` | PASS |
| `npm run build` | PASS |
| `npm audit --omit=dev` | PASS |
| `npm outdated --json` | NON-BLOCKING deferred updates |
| `cargo check -p antminer-fleet-server --locked` | PASS |
| `cargo check -p antminer-fleet-manager --locked` | PASS |
| `cargo test -p fleet-shared -p antminer-fleet-server --locked` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo test --workspace --locked` | PASS |
| `npm run tauri:build` | PASS |
| `git diff --check` | PASS |
