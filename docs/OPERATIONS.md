# Operations Runbook

This runbook covers an internal Antminer Fleet deployment. Commands assume a
Debian/Ubuntu amd64 server and the default paths installed by the Debian
package. Substitute the configured hostname, port, and PostgreSQL deployment
details.

## Installation checklist

1. Build and install the Debian package as described in
   [server/README.md](../server/README.md).
2. Create the PostgreSQL role and database.
3. Replace the placeholder in `/etc/antminer-fleet/server.toml`.
4. Generate or install the TLS certificate and private key.
5. Run `validate-config`.
6. Run `migrate`.
7. Create the first administrator.
8. Start the service and verify `/health`.
9. Record and independently distribute the certificate fingerprint.
10. Pair and sign in from one desktop before onboarding additional users.

Do not expose port `8443` beyond the internal networks that require access.
Use host and network firewalls even though the application requires HTTPS and
authentication.

## Client pairing and login

The pairing request retrieves the server certificate while TLS validation is
temporarily disabled. Security therefore depends on comparing the displayed
SHA-256 fingerprint through an independent trusted channel.

For organization-built clients, set `VITE_FLEET_SERVER_URL` in
`.env.production.local` before `npm run tauri build`. The first-run pairing
form will be pre-filled with that HTTPS origin. The setting is public build
configuration, so it must not contain credentials, tokens, certificate private
keys, or other secrets. Users must still verify and approve the certificate
fingerprint.

The administrator obtains the expected value:

```bash
openssl x509 -in /etc/antminer-fleet/tls/server.crt \
  -noout -fingerprint -sha256
```

The user then:

1. Opens the desktop client and enters the server HTTPS URL.
2. Selects **Check Server**.
3. Compares the complete fingerprint, not only its beginning or end.
4. Selects **Trust and Connect** after a match.
5. Signs in with the administrator-provided username and password.

The client accepts only the exact paired leaf certificate for later requests.
If the server becomes unavailable because the certificate changed, use
**Forget Server and Re-pair** and verify the new fingerprint. Re-pairing
removes the saved server profile and local session credential; it does not
delete server data.

### Windows SSH tunnel helper

For deployments that still require an SSH jump host, the Windows app now walks
each user through first-run tunnel setup before pairing. The setup can generate
a dedicated client key at `%USERPROFILE%\.ssh\antminer_fleet_tunnel`, displays
only the public key for authorization on the SSH host, makes **Copy Public Key
for Admin** the default out-of-band path, and stores the user's tunnel settings
at `%LOCALAPPDATA%\AntminerFleetManager\fleet-tunnel.local.json`.
Do not use a developer SSH login for deployed clients, do not bundle private
keys, and do not commit machine-local tunnel config.

Direct **Submit Key over LAN/VPN** is secondary and only works when the client
can already reach the server before the SSH tunnel exists. For the normal
locked-door bootstrap case, the user copies the public-key bundle, sends it to
an admin out-of-band, waits for approval, then enters the provided tunnel
destination manually.

The Windows NSIS installer checks for `ssh.exe` and installs the Windows
OpenSSH Client optional feature when it is missing; this requires an elevated
install and Windows access to the optional-feature payload. The desktop app
starts only the user's saved LOCALAPPDATA tunnel config during launch before
loading the saved server profile. The helper uses Windows OpenSSH with
`BatchMode`, `ExitOnForwardFailure`, and keepalives, then runs the tunnel
without a visible terminal window. Key-based or SSH-agent authentication is
required; the helper never accepts or stores an SSH password.

Install only the generated public key in the SSH account's `authorized_keys`,
preferably restricted to local forwarding for the Fleet endpoint. Keep the
private key on the client with user-only permissions.

Start it by double-clicking `Start Fleet Tunnel.cmd` or with
`npm run tunnel:start`. Use `npm run tunnel:status` and
`npm run tunnel:stop` for diagnostics and shutdown. This is an operational
convenience, not a replacement for a managed VPN or private access network.

When the backend reverse-tunnel service is enabled, configure the Windows
forward's remote target as `127.0.0.1:8443` on the SSH host. The reverse
tunnel owns that host-loopback listener and carries traffic onward to the
container. Do not configure desktop clients with the container IP.

## Account recovery

An administrator can reset a password in the desktop Accounts view. If no
administrator can sign in, use the server CLI:

```bash
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  reset-password admin
```

The prompt is hidden. A successful reset revokes all existing sessions for that
user. For protected non-interactive input, add `--password-stdin`.

The application prevents disabling or demoting the final enabled
administrator. The concurrency implementation has not yet been exercised
against a disposable live PostgreSQL instance.

## SQLite migration

The legacy SQLite file must contain the expected `miners` and `parts` tables.
The importer opens it read-only, validates every row, converts legacy
floating-point part costs to integer cents, and prints conflict counts before
writing.

Place the file where the operator account can read it. For example:

```bash
sudo install -m 0640 -o antminer-fleet -g antminer-fleet \
  /path/to/fleet.db /var/lib/antminer-fleet/import-fleet.db
```

Run a dry-run preview first:

```bash
sudo -u antminer-fleet antminer-fleet-server \
  --config /etc/antminer-fleet/server.toml \
  import-sqlite /var/lib/antminer-fleet/import-fleet.db
```

Apply with one conflict policy:

```bash
sudo -u antminer-fleet antminer-fleet-server \
  --config /etc/antminer-fleet/server.toml \
  import-sqlite /var/lib/antminer-fleet/import-fleet.db \
  --apply --conflict=abort
```

- `abort`: fail the complete transaction if any existing or late conflicting
  miner serial/part SKU is found.
- `server-wins`: insert new records and leave existing server records unchanged.
- `import-wins`: insert new records and replace matching server records with
  imported values.

Apply runs in one serializable PostgreSQL transaction. Keep the legacy file
unchanged until record counts and representative miners/parts have been
verified in the desktop client. The conflict policies and concurrent conflict
behavior have not yet been tested against a live PostgreSQL server.

## Backup and restore

Back up both PostgreSQL and server configuration. The inventory, accounts,
password hashes, and sessions are in PostgreSQL. The database backup does not
contain `/etc/antminer-fleet/server.toml` or TLS material.

For a local PostgreSQL server:

```bash
sudo install -d -m 0750 -o postgres -g postgres \
  /var/backups/antminer-fleet
sudo -u postgres pg_dump --format=custom antminer_fleet \
  --file=/var/backups/antminer-fleet/antminer-fleet.dump
sudo tar -C /etc -czf \
  /var/backups/antminer-fleet/antminer-fleet-config.tgz antminer-fleet
sudo chmod 0600 /var/backups/antminer-fleet/*
```

The `tar` archive contains the database credential and TLS private key. Limit
access, encrypt backup media according to organizational policy, and do not
store it in the repository.

For remote PostgreSQL, run `pg_dump` from a trusted host using an approved
credential mechanism such as a permission-restricted `.pgpass` file. Do not
put the database password directly in a shell command.

Test restores on an isolated host. A basic restore into an empty database is:

```bash
sudo -u postgres createdb --owner=antminer_fleet antminer_fleet_restore
sudo -u postgres pg_restore --dbname=antminer_fleet_restore \
  /var/backups/antminer-fleet/antminer-fleet.dump
```

Do not restore over the active production database without an approved outage,
a verified backup, and a rollback plan. PostgreSQL backup and restore commands
have not been exercised in this Windows workspace.

## Upgrade basics

Database migrations run automatically before server startup and before
database-dependent CLI commands. Apply them explicitly during a controlled
upgrade so migration failures occur before the service is restarted:

```bash
sudo systemctl stop antminer-fleet-server
# Create and verify the database/configuration backup described above.
sudo dpkg -i antminer-fleet-server_0.3.0_amd64.deb
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  validate-config
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml migrate
sudo systemctl start antminer-fleet-server
sudo systemctl status antminer-fleet-server
curl --cacert /etc/antminer-fleet/tls/server.crt \
  https://fleet-server.example.lan:8443/health
```

Replace the package filename with the version being installed. Review release
notes and migration files before the upgrade. Database migrations are forward
changes; reinstalling an older binary does not reverse them.

Desktop clients reject unsupported server API versions during pairing. Test
the intended server/client version combination on an internal staging system
before a broad client rollout.

## Routine checks

Useful service checks:

```bash
sudo systemctl is-active antminer-fleet-server
sudo journalctl -u antminer-fleet-server --since today
curl --cacert /etc/antminer-fleet/tls/server.crt \
  https://fleet-server.example.lan:8443/health
```

Also monitor PostgreSQL storage, connection use, backup completion, certificate
replacement dates, and the availability of at least two enabled administrator
accounts.

## Known unverified deployment steps

The following behavior is supported by source and unit/CLI tests but has not
been executed in the current Windows validation environment:

- Debian package construction and installation with `dpkg-deb`.
- systemd unit validation and service hardening on Debian/Ubuntu.
- Local and remote PostgreSQL creation, migrations, and connectivity.
- Concurrent final-administrator and SQLite conflict behavior on PostgreSQL.
- Migration of populated pre-`0002` part-cost data.
- PostgreSQL backup and restore commands.
- A live HTTPS certificate-substitution/rotation test.
- A packaged Tauri client using the operating-system keyring.
- End-to-end client pairing, login, logout, and re-login against the Linux
  server.

Complete these checks on disposable infrastructure before the first production
rollout and after material operating-system, PostgreSQL, TLS, or packaging
changes.
