# Antminer Fleet Server

The server is the only production database owner. It exposes an authenticated HTTPS API backed by PostgreSQL and is intended to run as a systemd service on Debian or Ubuntu amd64.

## PostgreSQL setup

Create a database and restricted login using locally appropriate credentials:

```sql
CREATE ROLE antminer_fleet LOGIN PASSWORD 'replace-with-a-generated-password';
CREATE DATABASE antminer_fleet OWNER antminer_fleet;
```

Copy `/usr/share/doc/antminer-fleet-server/server.example.toml` to `/etc/antminer-fleet/server.toml`, set the PostgreSQL URL, and restrict the file:

```bash
sudo chown root:antminer-fleet /etc/antminer-fleet/server.toml
sudo chmod 0640 /etc/antminer-fleet/server.toml
```

`database.url` must use `postgres://` or `postgresql://`, include a host and database name, and contain no placeholder. Percent-encode reserved characters in credentials. `database.max_connections` defaults to `10` and accepts `1-100`; size it below the PostgreSQL connection allowance.

`listen` is required. `127.0.0.1:8443` is local-only; `0.0.0.0:8443` exposes every IPv4 interface and requires host firewall rules. `session_days` defaults to `30` and accepts `1-365`; shorter sessions reduce stolen-token lifetime.

## TLS and first administrator

Generate a self-signed certificate containing every DNS name or IP address clients use:

```bash
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml generate-tls \
  --host fleet-server.example.lan --host 192.168.1.20
sudo chown root:antminer-fleet /etc/antminer-fleet/tls/server.*
sudo chmod 0644 /etc/antminer-fleet/tls/server.crt
sudo chmod 0640 /etc/antminer-fleet/tls/server.key
```

Both TLS paths are required. `validate-config` requires the files to exist, parse, and contain a matching certificate/private-key pair. `generate-tls` validates only its output paths so it can run before the database credential is configured.

Apply migrations and create the first administrator. Password input is hidden:

```bash
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml migrate
sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  create-admin admin "Fleet Administrator"
sudo systemctl enable --now antminer-fleet-server
```

For non-interactive input, pipe from a protected secret provider using the explicit stdin flag:

```bash
secret-tool lookup service antminer-fleet bootstrap admin | \
  sudo antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  create-admin admin "Fleet Administrator" --password-stdin
```

Do not place passwords in command arguments, shell history, unit files, or environment variables.

The desktop pairing screen displays the server certificate SHA-256 fingerprint. Confirm it against:

```bash
openssl x509 -in /etc/antminer-fleet/tls/server.crt -noout -fingerprint -sha256
```

The desktop pins the exact leaf certificate. Certificate renewal or replacement requires every desktop to use **Forget Server and Re-pair** and verify the new fingerprint out of band. Restart the service after replacing the certificate and key.

## Existing SQLite import

Copy the old `fleet.db` to the server. Always run the preview first:

```bash
sudo -u antminer-fleet antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  import-sqlite /path/to/fleet.db
```

Apply only after reviewing counts:

```bash
sudo -u antminer-fleet antminer-fleet-server --config /etc/antminer-fleet/server.toml \
  import-sqlite /path/to/fleet.db --apply --conflict=abort
```

If conflicts exist, rerun with `server-wins` or `import-wins`. The apply operation uses one serializable PostgreSQL transaction. Under `abort`, late conflicts roll back the complete import instead of overwriting current data.

## Build the Debian package

On Debian/Ubuntu with Rust and `dpkg-deb` installed:

```bash
sh server/scripts/build-deb.sh
sudo dpkg -i server/package/antminer-fleet-server_0.3.0_amd64.deb
```

Logs are available through `journalctl -u antminer-fleet-server`.

The default log filter is `antminer_fleet_server=info,tower_http=info`. Configure `RUST_LOG` with a systemd drop-in:

```ini
# /etc/systemd/system/antminer-fleet-server.service.d/logging.conf
[Service]
Environment=RUST_LOG=antminer_fleet_server=debug,tower_http=info
```

Run `sudo systemctl daemon-reload && sudo systemctl restart antminer-fleet-server` after changes. Avoid `trace` logging in production because verbose request and dependency diagnostics may expose operational metadata.

The Debian package recommends local PostgreSQL but does not require it. Remote PostgreSQL is supported when the service host can reach it securely.
