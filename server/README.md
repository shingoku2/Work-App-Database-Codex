# Antminer Fleet Server

The server is the only production database owner. It exposes an authenticated
HTTPS API backed by PostgreSQL and is intended to run as a systemd service on
Debian or Ubuntu amd64.

## Build and install

On Debian/Ubuntu with Rust and `dpkg-deb` installed:

```bash
sh server/scripts/build-deb.sh
sudo dpkg -i server/package/antminer-fleet-server_0.3.0_amd64.deb
```

The package creates the `antminer-fleet` service account and these paths:

- `/etc/antminer-fleet/server.toml`
- `/etc/antminer-fleet/tls/`
- `/var/lib/antminer-fleet/`
- `/usr/bin/antminer-fleet-server`

It also creates a restricted SSH account for desktop client local-forward
onboarding:

- user/group: `antminer-fleet-client-tunnel`
- authorized keys: `/etc/antminer-fleet/client-tunnel/authorized_keys`
- helper: `/usr/lib/antminer-fleet-server/authorize-client-tunnel-key.sh`
- example sshd policy: `/usr/share/doc/antminer-fleet-server/sshd-client-tunnel.example.conf`

The package recommends PostgreSQL but does not require a local PostgreSQL
service. A remote organization-controlled PostgreSQL host is supported when
network policy and PostgreSQL authentication permit the connection.

## PostgreSQL setup

For a local PostgreSQL installation, create a restricted login and database:

```bash
sudo -u postgres psql
```

```sql
CREATE ROLE antminer_fleet LOGIN;
\password antminer_fleet
CREATE DATABASE antminer_fleet OWNER antminer_fleet;
\q
```

Edit `/etc/antminer-fleet/server.toml` and replace the example database
credential with the password entered at the hidden `\password` prompt. Then
restrict the file:

```bash
sudo chown root:antminer-fleet /etc/antminer-fleet/server.toml
sudo chmod 0640 /etc/antminer-fleet/server.toml
```

`database.url` must:

- use `postgres://` or `postgresql://`;
- include a host and database name;
- contain no shipped placeholder text; and
- percent-encode reserved characters in usernames and passwords.

`database.max_connections` defaults to `10` and accepts `1-100`. Keep the pool
below the PostgreSQL connection allowance after reserving capacity for
administration, monitoring, and backups.

`listen` is required. `127.0.0.1:8443` accepts only local connections;
`0.0.0.0:8443` accepts connections on every IPv4 interface and requires host
firewall rules. `session_days` defaults to `30` and accepts `1-365`.

## TLS generation

Generate a self-signed certificate containing every DNS name or IP address
that desktop clients will use:

```bash
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  generate-tls --host fleet-server.example.lan --host 192.168.1.20
sudo chown root:antminer-fleet /etc/antminer-fleet/tls/server.crt \
  /etc/antminer-fleet/tls/server.key
sudo chmod 0644 /etc/antminer-fleet/tls/server.crt
sudo chmod 0640 /etc/antminer-fleet/tls/server.key
```

Use `--force` only when intentionally replacing existing TLS files.
Certificate replacement breaks the existing client pin and requires every
desktop to forget and re-pair the server.

The generated private key is unencrypted on disk so the systemd service can
start unattended. Protect it with filesystem permissions and host access
controls. An organization-issued certificate and matching PEM private key may
be installed at the configured paths instead.

## Validate, migrate, and create the first administrator

```bash
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  validate-config
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml migrate
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  create-admin admin "Fleet Administrator"
```

`validate-config` verifies the database URL structure and ranges, TLS file
existence, PEM parsing, and certificate/private-key compatibility. It does not
connect to PostgreSQL. `migrate` performs the database connection and applies
all pending migrations.

Interactive administrator creation uses hidden password input. Passwords must
contain at least 12 characters.

For automation, provide the password through protected standard input:

```bash
secret-tool lookup service antminer-fleet bootstrap admin | \
  sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  create-admin admin "Fleet Administrator" --password-stdin
```

Do not place passwords in command arguments, shell history, unit files, or
environment variables. The same `--password-stdin` option is available for
`reset-password`; resetting a password revokes the user's existing sessions.

## Start and verify the service

```bash
sudo systemctl enable --now antminer-fleet-server
sudo systemctl status antminer-fleet-server
curl --cacert /etc/antminer-fleet/tls/server.crt \
  https://fleet-server.example.lan:8443/health
```

Use a hostname or IP included in the certificate. Service logs are available
with:

```bash
sudo journalctl -u antminer-fleet-server
```

The default log filter is
`antminer_fleet_server=info,tower_http=info`. To change it, create a systemd
drop-in:

```ini
# /etc/systemd/system/antminer-fleet-server.service.d/logging.conf
[Service]
Environment=RUST_LOG=antminer_fleet_server=debug,tower_http=info
```

Apply the change:

```bash
sudo systemctl daemon-reload
sudo systemctl restart antminer-fleet-server
```

Avoid `trace` logging during normal operation because verbose request and
dependency diagnostics may expose operational metadata.

## Optional automatic reverse SSH tunnel

For a server running inside a container without a published HTTPS port, the
package includes a companion service that keeps a reverse SSH forward alive.
Install the deployment-specific configuration and SSH files:

```bash
sudo install -m 0640 -o root -g antminer-fleet \
  /usr/share/doc/antminer-fleet-server/tunnel.example.conf \
  /etc/antminer-fleet/tunnel.conf
sudo install -d -m 0700 -o antminer-fleet -g antminer-fleet \
  /var/lib/antminer-fleet/.ssh
sudo install -m 0600 -o antminer-fleet -g antminer-fleet HOST_FORWARD_KEY \
  /var/lib/antminer-fleet/.ssh/host_forward
sudo install -m 0600 -o antminer-fleet -g antminer-fleet HOST_KNOWN_HOSTS \
  /var/lib/antminer-fleet/.ssh/known_hosts
sudo systemctl enable antminer-fleet-tunnel.service
sudo systemctl restart antminer-fleet-server.service
```

Edit `tunnel.conf` for the SSH host/user and forwarding ports before enabling
the service. The SSH account must authorize the public half of
`host_forward`. The tunnel binds to host loopback by default, so a laptop can
use its own SSH local forward without exposing the server port to the LAN.

The tunnel service is started with the server, checks `/health` before
connecting, uses SSH keepalives, and reconnects after failures. Never place the
private key in the repository or package.

## Desktop client SSH onboarding

Desktop clients should not use a technician's personal SSH login. The package
creates the dedicated `antminer-fleet-client-tunnel` account and a helper for
adding public keys generated by the app during first-run setup. Install the
example sshd policy on the SSH host:

```bash
sudo install -m 0644 \
  /usr/share/doc/antminer-fleet-server/sshd-client-tunnel.example.conf \
  /etc/ssh/sshd_config.d/antminer-fleet-client-tunnel.conf
sudo sshd -t
sudo systemctl reload ssh
```

When the app displays a user's public key, authorize it with a stable label:

```bash
sudo /usr/lib/antminer-fleet-server/authorize-client-tunnel-key.sh \
  --label alice-laptop \
  --public-key 'ssh-ed25519 AAAA... antminer-fleet-tunnel'
```

The helper validates the OpenSSH public key and writes a restricted
`authorized_keys` entry allowing only local TCP forwarding to `127.0.0.1:8443`.
Give the user this SSH destination for the app's tunnel setup:

```text
antminer-fleet-client-tunnel@SSH_HOST_OR_IP
```

If the reverse tunnel publishes the Fleet HTTPS server on a different SSH-host
loopback port, set `ANTMINER_FLEET_CLIENT_TUNNEL_PERMIT_OPEN` before running the
helper and update the sshd `PermitOpen` value to match.

## Certificate fingerprint and client pairing

Display the server certificate SHA-256 fingerprint:

```bash
openssl x509 -in /etc/antminer-fleet/tls/server.crt \
  -noout -fingerprint -sha256
```

Provide this value to client users through an independently trusted channel.
On the desktop:

1. Enter the HTTPS URL, including port `8443` unless another port is configured.
2. Select **Check Server**.
3. Compare the displayed fingerprint with the administrator-provided value.
4. Select **Trust and Connect** only after they match.
5. Sign in with the assigned named account.

The desktop stores one server profile and pins the exact certificate. A public
CA certificate is not accepted merely because it chains to a trusted root; it
must be the certificate that was explicitly paired.

## SQLite migration

See [the operations runbook](../docs/OPERATIONS.md#sqlite-migration) for the
preview, conflict policies, apply command, and post-import checks.

## Backup and upgrade

See [the operations runbook](../docs/OPERATIONS.md#backup-and-restore) for
PostgreSQL/configuration backups and
[upgrade basics](../docs/OPERATIONS.md#upgrade-basics) before installing a new
server package.

## Current verification boundary

The Rust CLI help, configuration-rejection tests, workspace tests, and desktop
production build have been verified on Windows. The Debian package build,
systemd unit, a live PostgreSQL deployment, backup/restore, and end-to-end
HTTPS/Tauri pairing remain to be exercised on their target infrastructure.
