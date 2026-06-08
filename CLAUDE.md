# Project Guide

Antminer Fleet Manager is a self-hosted client/server asset-management application. Keep work focused on miners, spreadsheet import, parts inventory, dashboard reporting, accounts, and server operations. Do not reintroduce ticketing or technician workflows.

## Commands

```bash
npm ci
npm run build
npm test
cargo check --workspace
cargo test --workspace
npm audit --omit=dev
```

Desktop development:

```bash
npm run tauri:dev
```

Server development:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.example.toml validate-config
```

Do not commit, push, package, or deploy unless explicitly asked.

## Architecture

The root Cargo workspace contains:

1. `fleet-shared`: serializable API models, enums, and boundary validation.
2. `antminer-fleet-server`: Axum/rustls HTTPS API, PostgreSQL, auth, admin CLI, and SQLite importer.
3. `antminer-fleet-manager`: Tauri desktop client and pinned-HTTPS proxy.

### Server ownership

PostgreSQL is the only production database. Migrations live only in `server/migrations/` and run automatically before database-dependent CLI commands or server startup.

The server owns:

- Miner/part/dashboard queries.
- Import transactions and validation.
- Accounts, roles, password hashes, sessions, and authorization.
- Optimistic version checks.
- TLS termination.

### Desktop boundary

React calls only the wrapper in `src/lib/tauri.ts`. Feature APIs retain the established Tauri command names. Rust commands in `src-tauri/src/commands/mod.rs` proxy to `/api/v1`; the WebView does not receive unrestricted network access.

The desktop stores:

- One server URL and pinned certificate PEM/fingerprint in app data.
- One session token in the OS credential manager.

It does not store inventory data, passwords, or an offline mutation queue.

### Pairing

`probe_server` performs a one-time TLS connection with certificate validation disabled solely to retrieve `/pairing`. The UI displays the returned SHA-256 DER fingerprint for out-of-band confirmation. `pair_server` recomputes the fingerprint from the PEM, then verifies `/health` using that certificate as the trust root. Every later request uses the pinned certificate.

Certificate replacement requires forgetting and explicitly pairing the server again. Do not add an insecure bypass.

### Shared contracts

Rust protocol types live in `crates/fleet-shared/src/lib.rs`; matching TypeScript types live in `src/types/db.ts`. Keep field names and enum values synchronized.

Miner serials are trimmed and model/status values validated before writes. Imports deduplicate by trimmed serial and validate the whole batch before opening the transaction.

Miners, parts, and users use numeric `version` values. Updates and deletes must send the expected version; conflicts keep edit state open and require a reload.

## Product rules

- Unit Registry remains list-first with a dedicated detail/edit page.
- Spreadsheet parsing remains client-side through `read-excel-file` and local CSV/TSV helpers.
- Expected import columns and notes mapping remain unchanged.
- `xlsx` remains forbidden.
- Users can read/write fleet data.
- Admins additionally manage accounts.
- The final enabled admin cannot be disabled or demoted.
- Existing SQLite data moves only through the server CLI dry-run/apply importer.

## Operations

The first server package target is Debian/Ubuntu amd64. Packaging files are under `server/packaging/`; `server/scripts/build-deb.sh` stages the binary, systemd unit, config example, and maintainer scripts.

Secrets belong only in root/service-readable server configuration or the OS credential manager. Never print database passwords, plaintext user passwords, session tokens, or private keys.
