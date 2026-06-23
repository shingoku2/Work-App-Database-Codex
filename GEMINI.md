# Antminer Fleet Manager

`antminer-fleet-manager` is a Tauri v2 + React desktop client for the Antminer Fleet Manager client/server system. The production application is online-required and talks to `antminer-fleet-server`, an Axum/Rustls HTTPS API backed by PostgreSQL.

This file is context for Gemini and other LLM agents. Follow `AGENTS.md` and `CLAUDE.md` as authoritative repository instructions too.

## Project Overview

- Purpose: Manage ASIC miner assets, spreadsheet imports, parts inventory, sites, dashboard reporting, audit logs, webhooks, accounts, and server operations.
- Frontend stack: React 19, TypeScript, Vite 8, Vitest 4, TanStack Query v5, TanStack Table v8, Lucide React, Tailwind CSS, ESLint 9 (pinned; eslint-plugin-react does not accept ESLint 10).
- Desktop shell: Tauri v2 with Rust commands that proxy to the server over pinned HTTPS.
- Server stack: Rust, Axum, Rustls, SQLx, PostgreSQL, Argon2id password hashing, hashed/revocable bearer sessions.
- Shared contracts: Rust models in `crates/fleet-shared/src/lib.rs`; TypeScript models in `src/types/db.ts`.
- Production database: PostgreSQL owned exclusively by `antminer-fleet-server`.

## Hard Architecture Rules

- Do not reintroduce production local SQLite ownership in Tauri.
- Do not add a local production database, offline write queue, or automatic upload of legacy `fleet.db` data.
- Existing SQLite data moves only through the server CLI dry-run/apply importer.
- Browser/React code must call the established wrapper in `src/lib/tauri.ts`; do not add direct browser access to the server or database.
- Rust Tauri commands in `src-tauri/src/commands/mod.rs` proxy to `/api/v1`.
- Preserve API prefix `/api/v1` unless a task explicitly changes the protocol.
- Keep Rust and TypeScript field names and enum values synchronized.
- Preserve numeric optimistic-concurrency versions for miners, parts, and users unless a task explicitly changes the protocol.

## Project Structure

- `src/`: React frontend source code.
  - `features/`: Domain-specific views and API wrappers (`miners`, `inventory`, `dashboard`, `accounts`, `audit`, `connection`, `settings`, `sites`).
  - `components/`: Shared UI components.
  - `config/`: Public build-time server URL handling.
  - `lib/`: Shared utilities, Tauri command wrapper, query client, invalidation helpers.
  - `types/`: TypeScript protocol/domain definitions (`db.ts`).
  - `test/`: Vitest tests and fixtures.
- `src-tauri/`: Tauri shell, client proxy, commands, capabilities, and packaging config.
- `server/`: Axum server, PostgreSQL migrations, config, importer, auth, API routes, packaging, and tunnel scripts.
- `crates/fleet-shared/`: Shared Rust API/domain contracts.
- `.codex/pipeline/`: Staged Codex pipeline prompts.
- `.codex/reports/`: Pipeline reports.
- `scripts/`: Windows SSH tunnel helper scripts and example config.

## Building, Running, and Validation

Install dependencies with the lockfile:

```bash
npm ci
```

Frontend/client validation:

```bash
npm run build
npm test
```

Desktop development:

```bash
npm run tauri:dev
```

Server-targeted validation:

```bash
cargo check -p antminer-fleet-server --locked
cargo run -p antminer-fleet-server -- --config server/config/server.example.toml validate-config
```

Workspace validation when shared or cross-component Rust changes are made:

```bash
cargo check --workspace --locked
cargo test --workspace --locked
```

Packaging/tunnel validation when those files change:

```bash
sh -n server/scripts/run-reverse-tunnel.sh
systemd-analyze verify server/packaging/antminer-fleet-server.service server/packaging/antminer-fleet-tunnel.service
sh server/scripts/build-deb.sh
```

Always run before reporting completion:

```bash
git diff --check
```

Use the smallest relevant validation command first. Do not hide failed tests, builds, lint, typecheck, audit, or sandbox commands.

## TLS, Pairing, and Server URL

- Clients pin the exact server leaf certificate.
- Do not replace certificate pinning with normal CA-only trust.
- Do not add an insecure certificate bypass.
- `/pairing` and `/health` remain available without bearer authentication.
- `probe_server` may disable certificate validation only for the one-time certificate retrieval from `/pairing`.
- `pair_server` recomputes the SHA-256 DER fingerprint from the PEM and verifies `/health` using the pinned certificate as trust root.
- Certificate replacement requires clients to forget and re-pair.
- `VITE_FLEET_SERVER_URL` is public build configuration used only to prefill the pairing form. It must be an HTTPS origin only and never contain credentials or private material.

## SSH Tunnel Topology

- The optional backend reverse tunnel publishes the server only on the SSH host's `127.0.0.1:8443`.
- Desktop clients use a local SSH forward from their own `127.0.0.1:8443` to the SSH host's `127.0.0.1:8443`.
- Clients must not depend on container IPs.
- Backend tunnel files: `server/scripts/run-reverse-tunnel.sh`, `server/config/tunnel.example.conf`, and `server/packaging/antminer-fleet-tunnel.service`.
- Windows tunnel helper files live under `scripts/`. Keep `scripts/fleet-tunnel.local.json` local and ignored by Git.
- Tunnel automation must use batch/key authentication, reject forwarding failures, use keepalives, and avoid exposing host port `8443` publicly.

## Product and Security Rules

- Unit Registry remains list-first with a dedicated detail/edit page.
- Spreadsheet parsing remains client-side through `read-excel-file` and local CSV/TSV helpers.
- Expected import columns and notes mapping remain unchanged.
- `xlsx` remains forbidden.
- Users can read/write fleet data.
- Admins additionally manage accounts.
- The final enabled administrator cannot be disabled or demoted.
- Secrets belong only in root/service-readable server configuration or the OS credential manager.
- Never print or commit database passwords, plaintext passwords, session tokens, bearer tokens, TLS private keys, SSH private keys, deployment `known_hosts`, or local tunnel configuration.
- If a secret is found in source, report the file/key name and state that it must be rotated.

## Agent Operating Rules

- Read task prompts and relevant repository instructions before acting.
- Make small, reviewable changes.
- Do not commit, push, deploy, publish, package, or rewrite history unless explicitly asked.
- Preserve public APIs unless a task explicitly allows breaking changes.
- Report commands run and whether they passed or failed.
- When writing reports, place them in `.codex/reports/`.
