# Project Guide

Antminer Fleet Manager is a self-hosted client/server asset-management application for ASIC miner fleets. Keep work focused on miners, spreadsheet import, parts inventory, dashboard reporting, sites, audit logs, webhooks, accounts, and server operations. Do not reintroduce ticketing or technician workflows.

## Commands

Use the smallest validation that proves the change before broad checks.

```bash
npm ci
npm run build
npm test
cargo check -p antminer-fleet-server --locked
cargo check --workspace --locked
cargo test --workspace --locked
npm audit --omit=dev
```

Desktop development:

```bash
npm run tauri:dev
```

Server config validation:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.example.toml validate-config
```

Packaging/tunnel validation when those files change:

```bash
sh -n server/scripts/run-reverse-tunnel.sh
systemd-analyze verify server/packaging/antminer-fleet-server.service server/packaging/antminer-fleet-tunnel.service
sh server/scripts/build-deb.sh
```

Always run `git diff --check` before reporting completion. Do not commit, push, package, deploy, publish, or rewrite history unless explicitly asked.

## Architecture

The root Cargo workspace contains:

1. `fleet-shared`: serializable API models, enums, and boundary validation.
2. `antminer-fleet-server`: Axum/Rustls HTTPS API, PostgreSQL, auth, admin CLI, server-side SQLite importer, audit logs, webhooks, and sites.
3. `antminer-fleet-manager`: Tauri v2 desktop client, React frontend, and pinned-HTTPS proxy.

### Server ownership

PostgreSQL is the only production database. It is owned exclusively by `antminer-fleet-server`. Migrations live only in `server/migrations/` and run automatically before database-dependent CLI commands or server startup.

The server owns:

- Miner, part, site, audit-log, webhook, and dashboard queries.
- Import dry-run/apply transactions and validation.
- Accounts, roles, Argon2id password hashes, hashed/revocable sessions, and authorization.
- Optimistic version checks.
- TLS termination.

### Desktop boundary

React calls only the wrapper in `src/lib/tauri.ts`. Feature APIs retain the established Tauri command names. Rust commands in `src-tauri/src/commands/mod.rs` proxy to `/api/v1`; the WebView does not receive unrestricted network access.

The desktop stores:

- One server URL and pinned certificate PEM/fingerprint in app data.
- One bearer session token in the OS credential manager.

It must not store production inventory data, passwords, a local production database, or an offline mutation queue. Do not reintroduce production local SQLite ownership in Tauri.

### API and shared contracts

- Preserve API prefix `/api/v1` unless a task explicitly changes the protocol.
- Rust protocol types live in `crates/fleet-shared/src/lib.rs`.
- Matching TypeScript types live in `src/types/db.ts`.
- Keep field names and enum values synchronized across Rust and TypeScript.
- Miners, parts, and users use numeric optimistic-concurrency `version` values. Updates and deletes must send the expected version; conflicts keep edit state open and require a reload.
- Miner serials are trimmed and model/status values validated before writes. Imports deduplicate by trimmed serial and validate the whole batch before opening the transaction.

### Pairing and TLS

`probe_server` performs a one-time TLS connection with certificate validation disabled solely to retrieve `/pairing`. The UI displays the returned SHA-256 DER fingerprint for out-of-band confirmation. `pair_server` recomputes the fingerprint from the PEM, then verifies `/health` using that certificate as the trust root. Every later request uses the pinned certificate.

Clients pin the exact server leaf certificate. Do not replace this with normal CA-only trust or add an insecure certificate bypass. `/pairing` and `/health` must remain available without bearer authentication. Certificate replacement requires forgetting and explicitly pairing the server again; automatic certificate renewal is a breaking operational change.

`VITE_FLEET_SERVER_URL` is public build configuration used only to prefill the pairing form. It must contain an HTTPS origin only, never credentials or private material.

### SSH tunnel topology

- The optional backend reverse tunnel publishes the server only on the SSH host's `127.0.0.1:8443`.
- Desktop clients use a local SSH forward from their own `127.0.0.1:8443` to the SSH host's `127.0.0.1:8443`; they must not depend on a container IP.
- Backend tunnel files: `server/scripts/run-reverse-tunnel.sh`, `server/config/tunnel.example.conf`, and `server/packaging/antminer-fleet-tunnel.service`.
- Windows helper files live under `scripts/`. Keep `scripts/fleet-tunnel.local.json` local and ignored by Git.
- Tunnel automation must use batch/key authentication, reject forwarding failures, use keepalives, and avoid exposing host port `8443` publicly.

## Product rules

- Unit Registry remains list-first with a dedicated detail/edit page.
- Spreadsheet parsing remains client-side through `read-excel-file` and local CSV/TSV helpers.
- Expected import columns and notes mapping remain unchanged.
- `xlsx` remains forbidden.
- Users can read/write fleet data.
- Admins additionally manage accounts.
- The final enabled admin cannot be disabled or demoted.
- Existing SQLite data moves only through the server CLI dry-run/apply importer. There is no automatic legacy-data upload from the client.

## Security and secrets

Secrets belong only in root/service-readable server configuration or the OS credential manager. Never print database passwords, plaintext user passwords, session tokens, TLS private keys, SSH private keys, deployment `known_hosts`, or bearer tokens. If a secret is found in source, report the file/key name and state that it must be rotated.

## Pipeline

This repository uses a staged Codex pipeline under `.codex/pipeline/`. Reports belong in `.codex/reports/`.

Order:

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
