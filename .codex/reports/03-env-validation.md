# Environment and Config Validation Report

## Source Assessment
- Config inspected: server TOML, tunnel configuration, Cargo manifests, and package metadata
- Deployment inspected: server and tunnel systemd units, Debian maintainer scripts, package builder, and operations documentation
- Runtime inspected: PostgreSQL, server process, TLS identity, application accounts, reverse SSH tunnel, health, and pairing
- Commands run: tool/package inventory, locked server build check, config validation, migrations, package build, shell syntax checks, systemd unit verification, PostgreSQL readiness, and tunneled endpoint checks

## Config Inventory
- Server configuration: `/etc/antminer-fleet/server.toml`
- Tunnel configuration: `/etc/antminer-fleet/tunnel.conf`
- TLS certificate: `/etc/antminer-fleet/tls/server.crt`
- TLS private key: `/etc/antminer-fleet/tls/server.key`
- Tunnel identity: `/var/lib/antminer-fleet/.ssh/host_forward`
- Tunnel known hosts: `/var/lib/antminer-fleet/.ssh/known_hosts`
- Optional environment variable: `RUST_LOG`
- Secret management: database credentials and private keys are deployment files with restricted permissions and are not committed

## Findings

### [LOW] Container lifecycle does not use systemd
- Location: current development sandbox
- Issue: the container does not run systemd as PID 1 and has no Podman restart policy.
- Impact: the active watchdog reconnects SSH failures, but a full container stop requires the host to restart the container and its processes.
- Fix: use the packaged systemd services on the deployment host or configure a host-managed Podman unit/restart policy.
- Secret exposure: no

## Runtime Status
- PostgreSQL 16: accepting connections
- Database: `antminer_fleet`
- Migrations: applied
- Server: running as `antminer-fleet`
- Listen address: `0.0.0.0:8443`
- Health endpoint: passed through the reverse tunnel
- Pairing endpoint: passed through the reverse tunnel
- API version: `v1`
- Enabled administrator: present
- Reverse tunnel: running as `antminer-fleet`
- SSH keepalive and reconnect behavior: verified by terminating the SSH child and observing successful reconnection

## TLS Status
- Certificate identities: `localhost`, `127.0.0.1`
- Fingerprint:
  `85:49:20:37:3A:19:0F:41:60:7D:D3:E9:5D:37:F0:3D:BC:E3:73:C4:EB:3D:55:D6:C3:A6:4E:13:DB:FB:9B:22`
- Certificate/private-key validation: passed
- Secret exposure: no

## Installed Prerequisites
- Rust stable: installed
- C compiler/build tools: installed
- `pkg-config`: installed
- OpenSSL: installed
- `dpkg-deb`: installed
- PostgreSQL 16 server/client: installed
- OpenSSH client: installed
- `curl` and CA certificates: installed
- Node.js: below the frontend requirement, but not required for the standalone server

## Validation Results
- `cargo check -p antminer-fleet-server --locked`: passed
- Server configuration validation: passed
- Database migration: passed
- Shell syntax: passed
- systemd unit verification: passed
- `git diff --check`: passed
- Debian package build: passed
- Package content inspection: passed
- Host-tunneled `/health`: passed
- Host-tunneled `/pairing`: passed
- Forced SSH reconnect test: passed

## Summary Metrics
- Critical: 0
- High: 0
- Medium: 0
- Low: 1
- Hardcoded secrets: 0

## Verdict
ISSUES FOUND

The backend, PostgreSQL database, TLS configuration, and reverse SSH tunnel are
operational. The remaining issue is lifecycle management specific to the
development container; the packaged deployment includes systemd integration.

## Next Actions
1. Deploy the generated Debian package on a systemd-managed host.
2. Install deployment-specific tunnel configuration and SSH identity.
3. Enable both server and tunnel services.
4. Configure PostgreSQL, server configuration, TLS, and database backups.
