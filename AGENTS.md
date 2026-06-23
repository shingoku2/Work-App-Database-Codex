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

## Project Scope

Antminer Fleet Manager is a self-hosted client/server asset-management
application for ASIC miner fleets. Keep work focused on miners, spreadsheet
import, parts inventory, dashboard reporting, sites, audit logs, webhooks,
accounts, and server operations. Do not reintroduce ticketing or technician
workflows.

## Architecture Boundaries

- The production application is client/server. PostgreSQL is the only
  production database and is owned exclusively by `antminer-fleet-server`.
- Server-owned behavior includes miner, part, site, audit-log, webhook,
  dashboard, account, auth/session, import, migration, and TLS handling.
- The React frontend runs inside Tauri. Browser code must call the established
  Tauri command wrapper; do not add direct browser access to the server or
  database.
- The Tauri Rust layer proxies to `/api/v1`, performs HTTPS requests with
  Rustls, and stores the bearer session token in the operating-system
  credential manager.
- The client is online-required. Do not add a local production database,
  offline write queue, or automatic upload of legacy `fleet.db` data. Do not
  reintroduce production local SQLite ownership in Tauri.
- Shared Rust API/domain contracts live in `crates/fleet-shared`; matching
  TypeScript contracts live in `src/types/db.ts`. Keep field names and enum
  values synchronized.
- Preserve API prefix `/api/v1` and numeric optimistic-concurrency versions for
  miners, parts, and users unless a task explicitly changes the protocol.

## TLS and Pairing

- Clients pin the exact server leaf certificate. Do not replace this with
  normal CA-only trust or add an insecure certificate bypass.
- `/pairing` and `/health` must remain available without bearer
  authentication.
- Tunnel-key onboarding has a narrow pre-pairing exception:
  `POST /api/v1/tunnel-key-requests` and
  `GET /api/v1/tunnel-key-requests/{id}/status?token=...` are unauthenticated
  so a new desktop client can request tunnel access before it has a pinned
  server certificate or bearer session. These calls must stay HTTPS-only,
  must not expose private key material, and must be treated as pre-trust
  onboarding paths only.
- Pairing retrieves the certificate, requires independent full SHA-256
  fingerprint confirmation, and verifies `/health` using the pinned
  certificate.
- Certificate replacement requires clients to forget and re-pair. Treat
  automatic certificate renewal as a breaking operational change.
- `VITE_FLEET_SERVER_URL` is public build configuration used only to prefill
  the pairing form. It must contain an HTTPS origin only, never credentials or
  private material.

## SSH Tunnel Topology

- The optional backend reverse tunnel publishes the server only on the SSH
  host's `127.0.0.1:8443`.
- Desktop clients use a local SSH forward from their own
  `127.0.0.1:8443` to the SSH host's `127.0.0.1:8443`; they must not depend on
  a container IP.
- Backend tunnel implementation files are:
  `server/scripts/run-reverse-tunnel.sh`,
  `server/config/tunnel.example.conf`, and
  `server/packaging/antminer-fleet-tunnel.service`.
- Windows tunnel helper files are under `scripts/`. Keep
  `scripts/fleet-tunnel.local.json` local and ignored by Git.
- Tunnel automation must use batch/key authentication, reject forwarding
  failures, use keepalives, and avoid exposing host port `8443` publicly.
- Never commit SSH private keys, `known_hosts` deployment files, local tunnel
  configuration, database passwords, TLS private keys, or bearer tokens.

## Validation by Area

- Frontend-only: `npm ci` if dependencies are missing or stale, then
  `npm run build` and `npm test`.
- Rust/server-only: start with `cargo check -p antminer-fleet-server --locked`;
  add targeted server tests when behavior changes.
- Shared or cross-component Rust changes: `cargo check --workspace --locked`
  and the relevant workspace tests, usually `cargo test --workspace --locked`.
- Packaging/tunnel changes: run `sh -n` on changed shell scripts,
  `systemd-analyze verify` on changed units when available,
  `sh server/scripts/build-deb.sh`, and inspect the resulting package.
- Tunnel runtime changes: verify both `/health` and `/pairing` through the
  complete tunnel path. Test reconnect behavior when the change affects
  supervision or keepalives.
- Always run `git diff --check` before reporting completion.

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

- Tauri build expects `src-tauri/icons/icon.png`, but the repo only tracks `128x128.png` and `icon.ico`. The build process copies `128x128.png` to `icon.png` automatically; `icon.png` is gitignored as a build artifact.
- Linux Secret Service (keyring) may be unavailable in headless/cloud VMs. Desktop login can fail with credential-storage errors unless D-Bus secret service is running. API-level E2E via `curl` against the server still validates core fleet behavior.
- `npm run dev` alone serves the React UI on `http://127.0.0.1:1420` but Tauri `invoke` calls will not work without `npm run tauri:dev`.

### Validation commands (from `README.md`)

| Check | Command |
|-------|---------|
| Frontend tests | `npm test` |
| Frontend build | `npm run build` |
| Rust format | `cargo fmt --all -- --check` |
| Rust compile | `cargo check --workspace --locked` |
| Rust tests | `cargo test --workspace --locked` (needs `icon.png` — run `cp src-tauri/icons/128x128.png src-tauri/icons/icon.png` if missing) |
| Prod audit | `npm audit --omit=dev` |

### Services

| Service | Start |
|---------|-------|
| PostgreSQL | `sudo service postgresql start` |
| Fleet server | tmux + `cargo run -p antminer-fleet-server -- --config server/config/server.local.toml serve` |
| Desktop client | `npm run tauri:dev` (starts Vite on port 1420 automatically) |
