# Antminer Fleet Manager

Self-hosted asset management for Antminer ASIC units and replacement-parts inventory.

The product has two separately installed components:

- **Antminer Fleet Server**: Linux systemd service, HTTPS API, authentication, and PostgreSQL database.
- **Antminer Fleet Client**: Tauri desktop application for Windows/macOS/Linux. It stores no production inventory database and requires a server connection.

The application remains focused on asset registry, miner spreadsheet import, lifecycle status, dashboard reporting, and parts inventory. It does not include ticketing or technician workflows.

## Repository layout

- `server/`: Axum HTTPS server, PostgreSQL migrations, administrative CLI, SQLite importer, and Debian packaging.
- `crates/fleet-shared/`: shared API/domain contracts and validation.
- `src-tauri/`: desktop networking, certificate pinning, credential storage, and Tauri commands.
- `src/`: React user interface.

## Server installation

The initial package target is Debian/Ubuntu amd64. PostgreSQL must be installed and managed on the server or another operator-controlled host.

Build and install:

```bash
sh server/scripts/build-deb.sh
sudo dpkg -i server/package/antminer-fleet-server_0.3.0_amd64.deb
```

Then:

1. Create the PostgreSQL role/database.
2. Edit `/etc/antminer-fleet/server.toml`.
3. Generate/import TLS.
4. Run migrations.
5. Create the first administrator.
6. Enable the service.

See [server/README.md](./server/README.md) for exact commands, certificate fingerprint verification, and legacy `fleet.db` import.

## Desktop client

Prerequisites:

- Node.js 20 or newer.
- Rust stable.
- Tauri v2 platform prerequisites.

Development:

```bash
npm ci
npm run tauri:dev
```

On first launch:

1. Enter the server HTTPS URL.
2. Verify the displayed SHA-256 certificate fingerprint with the administrator.
3. Trust the certificate.
4. Sign in with a named account.

Each desktop installation stores one server profile. The pinned certificate and server URL are stored in application data; the bearer session token is stored in the operating-system credential manager. Passwords are never persisted.

The client is online-required. When the server cannot be reached, data operations are unavailable. There is no local write queue or offline database.

## Verification

From the repository root:

```bash
npm run build
npm test
cargo check --workspace
cargo test --workspace
npm audit --omit=dev
```

Database-backed integration tests are not yet included.

## API and security

- API prefix: `/api/v1`.
- Password hashing: Argon2id.
- Authentication: revocable opaque bearer sessions, stored hashed on the server.
- Roles: `admin` and `user`.
- TLS: direct server HTTPS; the desktop accepts only the exact paired leaf certificate.
- Optimistic concurrency: miners, parts, and users carry numeric versions; stale edits/deletes return conflict errors.
- Server request bodies are limited to 30 MB.
- Login attempts use bounded, expiring source-and-account limits.

Admins can create/disable users, assign roles, and reset passwords. The final enabled administrator cannot be disabled or demoted.

## Existing data

The server CLI imports the old desktop SQLite `fleet.db` with a dry-run preview and explicit conflict policy:

```bash
antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  import-sqlite /path/to/fleet.db
```

The server becomes the sole source of truth after import. Desktop clients never upload local databases automatically. Spreadsheet import is administrator-only and insert-only: existing serials are reported as conflicts instead of overwritten.

## Distribution status

The repository does not currently declare a project license or generate a third-party attribution bundle. Treat builds as internal-use artifacts until those obligations are resolved.
