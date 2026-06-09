# AGENTS.md

## Project Agent Instructions

This repository uses a staged Codex agent pipeline under `.codex/pipeline/`.

General rules for all Codex work in this repository:

- Read the task prompt and relevant repository instructions before acting.
- Make small, reviewable changes.
- Do not commit, push, deploy, publish, or rewrite history unless explicitly asked.
- Do not expose or print secret values.
- If a secret is found in source, report the file/key name and state that it must be rotated.
- Prefer the smallest relevant validation command before broad checks.
- Report commands run and whether they passed or failed.
- Do not hide test, build, lint, typecheck, audit, or sandbox failures.
- Preserve public APIs unless a task explicitly allows breaking changes.
- When writing reports, place them in `.codex/reports/`.

## Pipeline Order

1. `CODEX_DEP_AUDITOR.md`
2. `CODEX_LICENSE.md`
3. `CODEX_ENV_VALIDATOR.md`
4. `CODEX_AUDITOR.md`
5. `CODEX_FIXER.md`
6. `CODEX_TEST_WRITER.md`
7. `CODEX_REFACTOR.md`
8. `CODEX_DOCS_WRITER.md`
9. `CODEX_CHANGELOG.md`
10. `CODEX_ONBOARDING.md`

## Cursor Cloud specific instructions

### Toolchain

- **Node.js 20+** and **npm ci** at the repo root (see `README.md`).
- **Rust stable** via rustup (`rustup default stable`). The VM image may ship Rust 1.83 as default; Cargo 1.83 cannot build current lockfile dependencies (edition 2024). Always use stable (1.96+ as of 2026-05).
- **PostgreSQL** is required for the server runtime and full E2E flows. Install/start the distro package locally; create role `antminer_fleet` and database `antminer_fleet` per `server/README.md`.
- **Tauri v2 Linux deps** (for `npm run tauri:dev`): `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`.

### Local server config (not committed)

Copy `server/config/server.example.toml` to `server/config/server.local.toml` with:

- `listen = "127.0.0.1:8443"`
- A real `database.url` for your local PostgreSQL user/password
- Writable TLS paths under `server/config/local-tls/`

Bootstrap once per environment:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml generate-tls --host localhost --host 127.0.0.1
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml validate-config
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml migrate
printf 'YourAdminPass123!\n' | cargo run -p antminer-fleet-server -- --config server/config/server.local.toml create-admin admin "Dev Admin" --password-stdin
```

Start the server in a tmux session: `cargo run -p antminer-fleet-server -- --config server/config/server.local.toml serve`.

### Tauri desktop gotchas

- Tauri build expects `src-tauri/icons/icon.png`, but the repo only tracks `128x128.png` and `icon.ico`. Copy or symlink before `tauri dev` / `cargo test -p antminer-fleet-manager`: `cp src-tauri/icons/128x128.png src-tauri/icons/icon.png`.
- Linux Secret Service (keyring) may be unavailable in headless/cloud VMs. Desktop login can fail with credential-storage errors unless D-Bus secret service is running. API-level E2E via `curl` against the server still validates core fleet behavior.
- `npm run dev` alone serves the React UI on `http://127.0.0.1:1420` but Tauri `invoke` calls will not work without `npm run tauri:dev`.

### Validation commands (from `README.md`)

| Check | Command |
|-------|---------|
| Frontend tests | `npm test` |
| Frontend build | `npm run build` |
| Rust format | `cargo fmt --all -- --check` |
| Rust compile | `cargo check --workspace --locked` |
| Rust tests | `cargo test --workspace --locked` (needs `icon.png` symlink) |
| Prod audit | `npm audit --omit=dev` |

### Services

| Service | Start |
|---------|-------|
| PostgreSQL | `sudo service postgresql start` |
| Fleet server | tmux + `cargo run -p antminer-fleet-server -- --config server/config/server.local.toml serve` |
| Desktop client | `npm run tauri:dev` (starts Vite on port 1420 automatically) |
