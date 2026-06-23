# Antminer Fleet Manager

Antminer Fleet Manager is an internal, self-hosted asset-management system for
Antminer ASIC units and replacement-parts inventory.

It has two separately installed components:

- **Antminer Fleet Server**: a Linux systemd service providing the HTTPS API,
  named-account authentication, and PostgreSQL persistence.
- **Antminer Fleet Client**: a Tauri desktop application for Windows, macOS,
  and Linux. It requires an active server connection and stores no production
  inventory database.

The product covers miner registration, spreadsheet import, lifecycle status,
dashboard reporting, account administration, and parts inventory. It does not
include ticketing or technician workflows.

## Repository layout

- `server/`: Axum HTTPS server, PostgreSQL migrations, administrative CLI,
  SQLite importer, and Debian packaging.
- `crates/fleet-shared/`: shared API/domain contracts and validation.
- `src-tauri/`: desktop networking, certificate pinning, credential storage,
  and Tauri commands.
- `src/`: React user interface.
- `docs/OPERATIONS.md`: deployment, pairing, backup, and upgrade runbook.
- `docs/INTERNAL_COMPLIANCE.md`: internal dependency-attribution record and
  unresolved compliance checks.

## Deployment overview

The supported initial server package target is Debian/Ubuntu amd64. PostgreSQL
may run locally or on another organization-controlled host.

The deployment sequence is:

1. Build and install the Debian package.
2. Create the PostgreSQL role and database.
3. Configure `/etc/antminer-fleet/server.toml`.
4. Generate or install the server TLS certificate.
5. Validate the configuration and apply migrations.
6. Create the first administrator.
7. Start the systemd service and verify `/health`.
8. Pair each desktop client after independently confirming the certificate
   SHA-256 fingerprint.

See [server/README.md](server/README.md) for exact setup commands and
[docs/OPERATIONS.md](docs/OPERATIONS.md) for the operating runbook.

## Desktop client

Development prerequisites:

- Node.js 20 or newer.
- Rust stable.
- Tauri v2 platform prerequisites for the development host.

Install and launch:

```bash
npm ci
npm run tauri:dev
```

On Windows, build the configured NSIS desktop installer:

```bash
npm run tauri build
```

To ship a client preconfigured for the hosted server, copy `.env.example` to
`.env.production.local`, set `VITE_FLEET_SERVER_URL` to the hosted HTTPS
origin, and build the installer. This value is embedded in the frontend and
must contain only the server origin, never credentials or private material.

On first launch, if the server is reachable only through SSH, complete the
copy-first tunnel onboarding flow before pairing: generate this computer's SSH
key, use **Copy Public Key for Admin**, send the bundle to an administrator
out-of-band, then enter the approved tunnel destination and start the tunnel.
Direct **Submit Key over LAN/VPN** is secondary and only works when the server is
already reachable without the SSH tunnel. After the tunnel is running, pair to
`https://127.0.0.1:8443`, compare the displayed SHA-256 certificate fingerprint
with the value supplied by the server administrator, and then sign in with a
named account.

Each desktop installation stores one server profile. The server URL, pinned
certificate, and fingerprint are stored in application data. The bearer
session token is stored in the operating-system credential manager. Passwords
and fleet data are not persisted by the desktop client.

The client is online-required. There is no local write queue or offline
database.

### Windows SSH tunnel helper

When the server is reachable only through SSH, each Windows install creates its
own tunnel connection during first setup. The app can generate a dedicated
client key at `%USERPROFILE%\.ssh\antminer_fleet_tunnel`, shows only the public
key for server-side authorization, then writes the user-specific tunnel config
to `%LOCALAPPDATA%\AntminerFleetManager\fleet-tunnel.local.json`. Do not reuse a
developer SSH login, do not bundle private keys, and do not commit
machine-local tunnel config.

See [docs/ssh-tunnel-onboarding.md](docs/ssh-tunnel-onboarding.md) for the full admin and user onboarding flow.

SSH key or agent authentication must work without a password prompt. The Windows
installer checks for OpenSSH Client and installs the Windows optional feature
when `ssh.exe` is missing, then the desktop app starts the user's saved tunnel
helper on launch before loading the saved server profile.

Users can then double-click `scripts/Start Fleet Tunnel.cmd`, or run:

```powershell
npm run tunnel:start
npm run tunnel:status
npm run tunnel:stop
```

The tunnel runs in a hidden background process, forwards local port `8443` to
the configured server target, rejects forwarding startup failures, and uses
SSH keepalives. The local config is ignored by Git and must not contain
passwords or private-key contents.

In the packaged backend topology, the backend reverse tunnel publishes Fleet
Server on the SSH host's `127.0.0.1:8443`. The Windows helper therefore uses
`remote_host: "127.0.0.1"`; clients do not route directly to the container IP.

## Development and testing

Run from the repository root:

```bash
npm ci
npm run build
npm test
cargo check --workspace --locked
cargo test --workspace --locked
cargo fmt --all -- --check
npm audit --omit=dev
```

The current automated suite covers frontend behavior, copy-first SSH tunnel
onboarding, shared validation, server configuration rejection, login-limiter
behavior, exact certificate matching, saved-profile recovery, and command path
encoding.

Live PostgreSQL concurrency/migration tests, a real HTTPS certificate
substitution test, a packaged Tauri/keyring flow, and Debian/systemd validation
are not currently automated. See the accuracy flags in
[docs/OPERATIONS.md](docs/OPERATIONS.md).

## Security model

- API prefix: `/api/v1`.
- Password hashing: Argon2id.
- Authentication: revocable opaque bearer sessions stored hashed on the
  server.
- Roles: `admin` and `user`.
- TLS: direct server HTTPS; paired clients accept only the exact paired leaf
  certificate.
- Concurrency: miners, parts, and users carry numeric versions; stale writes
  return conflict errors.
- Server request bodies are limited to 30 MB.
- Login attempts use bounded, expiring source-and-account limits.
- The final enabled administrator cannot be disabled or demoted.

## Existing SQLite data

Legacy desktop `fleet.db` data moves to PostgreSQL only through the server CLI.
The importer defaults to a dry-run preview and requires an explicit conflict
policy when changes are applied. Desktop clients do not upload local databases
automatically.

See [SQLite migration](docs/OPERATIONS.md#sqlite-migration).

## Internal-use status

This repository is operated as an internal application, not a public
distribution. Internal use still requires the organization to retain
third-party license and notice records and to reassess obligations before
sharing source or binaries outside the organization.

The current compliance record is
[docs/INTERNAL_COMPLIANCE.md](docs/INTERNAL_COMPLIANCE.md). It is an inventory
and operating record, not a complete artifact-level third-party notices bundle
or legal opinion.
