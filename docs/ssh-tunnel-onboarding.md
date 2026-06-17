# SSH Tunnel User Onboarding

This guide covers onboarding a new desktop client through the restricted SSH tunnel account.

## Admin prerequisites

- Fleet server installed and running with PostgreSQL migrated
- `[tunnel_client]` configured in `/etc/antminer-fleet/server.toml` with a real `ssh_destination` (not `CHANGE_ME`)
- `sshd` `Match User antminer-fleet-client-tunnel` policy installed from `/usr/share/doc/antminer-fleet-server/sshd-client-tunnel.example.conf`
- Fleet HTTPS reachable on the SSH host at `127.0.0.1:8443` (reverse tunnel or local bind)
- Admin logged into Fleet Manager with the `admin` role

## User first-run steps

1. Open the desktop app.
2. Enter a machine label and server URL (if reachable on LAN/VPN).
3. Click **Generate This Computer's SSH Key**.
4. If the server is reachable, click **Submit Key for Admin Approval**. Otherwise click **Copy Onboarding Bundle** and send it to an admin out-of-band.
5. Wait for admin approval (the app polls every 10 seconds when a request was submitted).
6. After approval, review the prefilled tunnel settings and click **Save and Start Tunnel**.
7. Pair to `https://127.0.0.1:8443` and confirm the certificate fingerprint.
8. Sign in with a normal Fleet Manager account.

The private key never leaves the user's computer.

## Admin approval steps

1. Open **Tunnel Keys** in the sidebar.
2. Verify the request out-of-band (label, fingerprint, ticket, etc.).
3. Optionally add a note and click **Approve**.
4. Tell the user their tunnel is approved. The app prefills the tunnel destination from server config when polling succeeds.

## Revocation

1. Open **Tunnel Keys**.
2. Find the approved entry under **Recent**.
3. Click **Revoke** and confirm.
4. Verify the marker no longer appears in `/etc/antminer-fleet/client-tunnel/authorized_keys`:

```bash
sudo grep 'antminer-fleet-client:LABEL' /etc/antminer-fleet/client-tunnel/authorized_keys
```

## Troubleshooting

| Symptom | Likely cause |
|---------|----------------|
| Submit button disabled | Server URL not reachable from the client network; use **Copy Onboarding Bundle** |
| `script_not_found` on approve | Server package missing `authorize-client-tunnel-key.sh` |
| `permission denied (publickey)` | Key not in `authorized_keys`, wrong identity file, or sshd policy missing |
| Local port 8443 in use | Another process bound to 8443 on the client |
| Connection refused on `127.0.0.1:8443` | Tunnel not running or backend not listening on SSH host |
| OpenSSH missing (Windows) | Install Windows OpenSSH Client feature |
| Key pending forever | Admin has not approved; or user copied bundle without submit and admin must add key manually |

## Manual key authorization (server CLI)

When a user copied a bundle without API submit:

```bash
sudo /usr/lib/antminer-fleet-server/authorize-client-tunnel-key.sh \
  --label USER-LABEL \
  --public-key 'ssh-ed25519 AAAA... USER-LABEL'
```

Then create the pending row in the admin UI or tell the user to enter the tunnel destination manually.
