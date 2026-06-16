# Admin Console SSH Key Onboarding Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Let an admin approve and install each user's SSH tunnel public key from inside Antminer Fleet Manager, then let first-run users finish tunnel setup using the approved key and known SSH destination without using Eddie's personal SSH login.

**Architecture:** The model works, but the wording needs one correction: **the user should not enter a public key they receive from the admin.** The user's app should generate the keypair locally, keep the private key on their PC, and submit/display the public key. The admin console approves that public key and installs it into the restricted tunnel account's `authorized_keys`. The admin then gives the user the SSH destination/config values, not a private key. Current code already has most of this: key generation, request submission, admin listing/approval, and server-side `authorize-client-tunnel-key.sh`. The plan below hardens it and closes the UX gaps.

**Tech Stack:** Tauri v2, React, TypeScript, Rust, Axum, PostgreSQL, OpenSSH, NSIS, Linux server packaging.

---

## Short answer: would Eddie's idea work?

Yes — but **not with the current "submit key to server before the tunnel exists" flow.** That is a circular dependency:

```text
Need SSH key approved to create tunnel.
Need tunnel to reach backend.
Current button tries to reach backend before tunnel exists.
Result: button is dead unless backend is also reachable by LAN/VPN/public path.
```

The correct bootstrap flow is:

1. User starts app first time.
2. App generates an SSH keypair on that user's PC.
3. Private key stays local under the app's data directory.
4. App displays a copyable public-key onboarding bundle.
5. User gives that public key to Eddie/admin out-of-band: Teams, text, ticket, QR, USB, whatever is operationally acceptable.
6. Admin console can still approve/install the key, but the admin must already be connected from an admin machine.
7. Server runs `authorize-client-tunnel-key.sh` and writes the public key into the restricted tunnel account's `authorized_keys` with forced restrictions:
   - `restrict`
   - `port-forwarding`
   - `permitopen="127.0.0.1:8443"`
   - `no-agent-forwarding`
   - `no-X11-forwarding`
   - `no-pty`
8. Admin gives the user the tunnel destination/config, for example:
   - `antminer-fleet-client-tunnel@10.81.1.120`
   - port `22`
   - local port `8443`
   - remote target `127.0.0.1:8443`
9. User enters that config.
10. App starts the tunnel.
11. Pairing continues against `https://127.0.0.1:8443` or `https://localhost:8443` and pins the server cert.

Do **not** have Eddie generate private keys for users and hand them out. That creates shared/admin-known credentials. Bad smell. Generate on user machine; admin only ever sees public keys.

The `Submit Key for Admin Approval` button is only valid if there is a separate bootstrap network path to the backend, such as LAN exposure, Tailscale/WireGuard, or a temporary onboarding endpoint. If the backend is only reachable through the SSH tunnel, that button must be replaced or demoted to "copy request for admin".

---

## Existing repo context verified

The repo already includes these pieces:

- `src/features/connection/ConnectionGate.tsx`
  - first-run SSH tunnel setup
  - `generateTunnelKey()`
  - public key display
  - `submitTunnelKeyRequest(serverUrl, input)`
  - manual SSH destination/config form

- `src/features/settings/TunnelKeyRequestsView.tsx`
  - admin-side pending/recent key request UI
  - approve/reject buttons

- `src/features/connection/connectionApi.ts`
  - `submitTunnelKeyRequest(serverUrl, input)`
  - `listTunnelKeyRequests()`
  - `approveTunnelKeyRequest()`
  - `rejectTunnelKeyRequest()`

- `server/src/api.rs`
  - `POST /api/v1/tunnel-key-requests` unauthenticated submit endpoint
  - admin-only list/approve/delete endpoints
  - approval calls `authorize-client-tunnel-key.sh`

- `server/scripts/authorize-client-tunnel-key.sh`
  - writes public keys to `/etc/antminer-fleet/client-tunnel/authorized_keys`
  - uses restricted OpenSSH authorized key options

- `src-tauri/src/commands/mod.rs`
  - local key generation
  - tunnel config save
  - tunnel process start/status

This is already more than a paper design. The remaining work is mostly hardening, UX, status polling, and server packaging/policy.

---

## Required security rules

- Never distribute private keys from the admin console.
- Never store user private keys on the server.
- Use a dedicated restricted SSH account, not Eddie's account:
  - recommended: `antminer-fleet-client-tunnel`
- The SSH account must not allow shell login.
- Approved keys must only allow local forwarding to the backend endpoint.
- The app should call the tunnel destination a **Tunnel SSH Destination**, not a backend URL.
- The app should make it clear that the public key is safe to share; the private key is not.
- Admin approval should be audited.
- Revocation must remove the key from `authorized_keys`, not only delete the database row.

---

## Proposed user flow

### First-run user flow

1. App launches.
2. App sees:
   - no paired server config
   - no tunnel config, or tunnel config exists but local port is closed
3. Show `Set up SSH tunnel`.
4. User enters:
   - label: `morin-laptop`, `mgarza-fieldpc`, etc.
   - server URL used only for submitting key request, if reachable
5. User clicks `Generate This Computer's SSH Key`.
6. App generates `ed25519` keypair locally.
7. App displays:
   - public key
   - private key path
   - warning: private key stays local
8. User clicks `Submit Key for Admin Approval`.
9. App posts to `POST /api/v1/tunnel-key-requests`.
10. Waiting screen shows:
    - request id
    - label
    - public key fallback copy box
    - instructions: "Ask an admin to approve this in Settings → Tunnel Keys."
11. Once approved, user enters or receives tunnel config:
    - SSH destination: `antminer-fleet-client-tunnel@<ssh-host>`
    - SSH port: `22`
    - local port: `8443`
    - remote host: `127.0.0.1`
    - remote port: `8443`
12. User clicks `Save and Start Tunnel`.
13. App starts SSH local forward.
14. App moves to pairing screen with `https://127.0.0.1:8443` prefilled.
15. User confirms cert fingerprint and pairs.
16. User logs in with normal Fleet Manager credentials.

### Admin flow

1. Admin logs into app.
2. Admin opens Settings → Tunnel Key Requests.
3. Admin sees pending requests with:
   - label
   - created time
   - public key fingerprint
   - public key body
   - request source IP if available
4. Admin validates the user/device out-of-band.
5. Admin clicks Approve.
6. Server runs authorization script.
7. Server marks request approved.
8. Server writes audit log entry.
9. Admin gives user the SSH destination/config values.

---

## Task 1: Clarify UI wording so nobody thinks admins hand out private keys

**Objective:** Make the first-run flow explicit: users generate keys locally; admins approve public keys only.

**Files:**
- Modify: `src/features/connection/ConnectionGate.tsx`
- Test: `src/test/ConnectionGate.test.tsx`

**Changes:**

Replace wording like:

```tsx
Send this public key to the server administrator
```

with:

```tsx
Share this public key with an admin. The private key stays on this computer and must never be sent.
```

Add helper text near manual config:

```tsx
After an admin approves your public key, enter the SSH tunnel destination they provide. This should be a restricted tunnel account, not Eddie's personal SSH login.
```

**Test:**

Add/adjust assertions in `src/test/ConnectionGate.test.tsx`:

```ts
expect(screen.getByText(/private key stays on this computer/i)).toBeInTheDocument();
expect(screen.getByText(/restricted tunnel account/i)).toBeInTheDocument();
```

**Validation:**

```bash
npm test -- src/test/ConnectionGate.test.tsx
```

Expected: test passes.

---

## Task 2: Show SSH public key fingerprint in admin console

**Objective:** Let admins compare short fingerprints instead of reading full key blobs like cavemen.

**Files:**
- Modify: `crates/fleet-shared/src/lib.rs`
- Modify: `server/src/api.rs`
- Modify: `src/types/db.ts`
- Modify: `src/features/settings/TunnelKeyRequestsView.tsx`
- Test: server API tests if existing, otherwise shared unit/helper test

**Approach:**

Add a computed `fingerprint_sha256` field to `TunnelKeyRequest`.

Preferred format:

```text
SHA256:abc123...
```

Use `ssh-keygen -l -f` in the server authorization script already for validation, but API should compute fingerprint without shelling out if practical. If doing it in Rust is too much for this task, acceptable first version: store/display the public key comment/body and leave fingerprint for Task 3. Better version: add a helper using an SSH key parsing crate only if dependency impact is acceptable.

**Minimal version:**

Keep database unchanged. Add a helper:

```rust
fn public_key_fingerprint_display(public_key: &str) -> Option<String> {
    // If no parser is added, return None for now.
    // Do not fake hashes.
    None
}
```

But do not expose bogus values. No made-up fingerprints.

**Better version:**

Add crate if acceptable after review:

```toml
ssh-key = { version = "0.6", features = ["ed25519"] }
```

Then compute SHA256 over the key blob per OpenSSH format.

**Validation:**

```bash
cargo check -p antminer-fleet-server --locked
npm test -- src/test/connectionApi.test.ts src/test/ConnectionGate.test.tsx
```

---

## Task 3: Add admin note field when approving/rejecting

**Objective:** Let admin record why a key was approved/rejected and what user/device it belongs to.

**Files:**
- Modify: `src/features/settings/TunnelKeyRequestsView.tsx`
- Existing backend already supports `ApproveTunnelKeyRequest { note: Option<String> }`
- Existing DB already has `note`

**UI behavior:**

For each pending request, show a note input:

```tsx
<textarea
  className="..."
  placeholder="Optional note: user/device verified, ticket number, laptop tag..."
/>
```

Approval sends:

```ts
approveTunnelKeyRequest(req.id, { note: note.trim() || null })
```

Reject should ideally also store a note, but current delete endpoint removes the row. See Task 7 for proper rejection tracking.

**Validation:**

```bash
npm test -- src/test/connectionApi.test.ts
npm run build
```

---

## Task 4: Add a server-side tunnel client config endpoint for approved users

**Objective:** After approval, let the first-run app retrieve safe tunnel defaults instead of making users type five fields manually.

**Important:** This endpoint must not leak secrets. It returns only connection metadata.

**Files:**
- Modify: `crates/fleet-shared/src/lib.rs`
- Modify: `server/src/api.rs`
- Modify: `src/types/db.ts`
- Modify: `src/features/connection/connectionApi.ts`
- Modify: `src/features/connection/ConnectionGate.tsx`

**New DTO:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelClientConfig {
    pub ssh_destination: String,
    pub ssh_port: u16,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}
```

**Server config additions:**

Add to server TOML config:

```toml
[tunnel_client]
ssh_destination = "antminer-fleet-client-tunnel@10.81.1.120"
ssh_port = 22
local_port = 8443
remote_host = "127.0.0.1"
remote_port = 8443
```

**Endpoint options:**

Simplest public-ish endpoint:

```text
GET /api/v1/tunnel-client-config
```

This is not secret, but only expose if the server is already reachable to first-run clients.

Safer tied-to-request endpoint:

```text
GET /api/v1/tunnel-key-requests/{id}/client-config
```

Only returns config when request status is `approved`.

**Recommendation:** Use tied-to-request endpoint.

**Behavior:**

- Before approval: returns `409 pending` or `{ status: "pending" }`.
- After approval: returns tunnel config.
- Rejected/missing: returns clear error.

**Validation:**

```bash
cargo check -p antminer-fleet-server --locked
npm test -- src/test/ConnectionGate.test.tsx
```

---

## Task 5: Make WaitingForApproval actually poll approval status

**Objective:** Current waiting screen invalidates `tunnel` query, but approval status lives in `tunnelKeyRequests`; first-run user has no endpoint to check their own request. Add one.

**Files:**
- Modify: `server/src/api.rs`
- Modify: `src/features/connection/connectionApi.ts`
- Modify: `src/features/connection/ConnectionGate.tsx`
- Modify: `src/test/ConnectionGate.test.tsx`

**New endpoint:**

```text
GET /api/v1/tunnel-key-requests/{id}/status
```

Response:

```ts
interface TunnelKeyRequestStatus {
  id: number;
  status: "pending" | "approved" | "rejected";
  note: string | null;
  client_config: TunnelClientConfig | null;
}
```

**Polling behavior:**

- Poll every 10 seconds while pending.
- If approved and `client_config` exists:
  - prefill tunnel config form
  - show `Approved — Start Tunnel` button
- If rejected:
  - show reason/note if available
  - let user regenerate/resubmit

**Validation:**

```bash
npm test -- src/test/ConnectionGate.test.tsx
cargo check -p antminer-fleet-server --locked
```

---

## Task 6: Harden the Linux SSH tunnel account setup in packaging

**Objective:** Make sure approving keys works on a real installed server, not just from source checkout.

**Files:**
- Modify: `server/scripts/build-deb.sh`
- Modify: `server/packaging/antminer-fleet-server.service` if needed
- Modify/create: `server/packaging/postinst` or packaging script section if present
- Modify: `server/scripts/authorize-client-tunnel-key.sh`
- Possibly modify: `server/config/server.example.toml`

**Server account requirements:**

Create user/group:

```bash
sudo groupadd --system antminer-fleet-client-tunnel
sudo useradd --system \
  --gid antminer-fleet-client-tunnel \
  --home-dir /var/lib/antminer-fleet/client-tunnel \
  --shell /usr/sbin/nologin \
  antminer-fleet-client-tunnel
```

Authorized keys path:

```text
/etc/antminer-fleet/client-tunnel/authorized_keys
```

SSHD config snippet should restrict this user:

```text
Match User antminer-fleet-client-tunnel
    AuthorizedKeysFile /etc/antminer-fleet/client-tunnel/authorized_keys
    AllowTcpForwarding local
    PermitTTY no
    X11Forwarding no
    AllowAgentForwarding no
    PermitTunnel no
    GatewayPorts no
    PasswordAuthentication no
    PubkeyAuthentication yes
```

**Note:** `ForceCommand /usr/sbin/nologin` can break pure forwarding depending on sshd behavior/config. Test before adding it. The authorized key options already include `no-pty` and no shell-use options, but account shell should still be `/usr/sbin/nologin`.

**Validation on Linux server/package environment:**

```bash
sh -n server/scripts/authorize-client-tunnel-key.sh
sh server/scripts/build-deb.sh
```

If `systemd-analyze` exists:

```bash
systemd-analyze verify server/packaging/antminer-fleet-server.service server/packaging/antminer-fleet-tunnel.service
```

Runtime validation:

```bash
sudo /usr/lib/antminer-fleet-server/authorize-client-tunnel-key.sh \
  --label test-client \
  --public-key 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA... test-client'
```

Then inspect only non-secret public config:

```bash
sudo grep 'antminer-fleet-client:test-client' /etc/antminer-fleet/client-tunnel/authorized_keys
```

---

## Task 7: Implement real key revocation

**Objective:** Admins must be able to remove approved users from SSH access, not just reject pending requests.

**Current issue:** `DELETE /api/v1/tunnel-key-requests/{id}` deletes the DB row and logs `tunnel_key.rejected`, but does not remove an already-approved key from `authorized_keys`.

**Files:**
- Modify: `server/scripts/authorize-client-tunnel-key.sh` or create `server/scripts/revoke-client-tunnel-key.sh`
- Modify: `server/src/api.rs`
- Modify: `src/features/settings/TunnelKeyRequestsView.tsx`

**Recommended script:**

Create:

```text
server/scripts/revoke-client-tunnel-key.sh
```

Behavior:

```bash
revoke-client-tunnel-key.sh --label LABEL
```

It removes lines ending with:

```text
antminer-fleet-client:LABEL
```

from authorized_keys.

**API behavior:**

- Rename UI action from `Reject` to:
  - `Reject` for pending
  - `Revoke` for approved
- For approved rows, call revoke script before updating DB.
- Preserve row with `status = 'rejected'` or better add `revoked` as status.

**Better schema:**

Update allowed statuses to:

```text
pending | approved | rejected | revoked
```

**Validation:**

```bash
sh -n server/scripts/revoke-client-tunnel-key.sh
cargo check -p antminer-fleet-server --locked
npm test -- src/test/connectionApi.test.ts
```

Manual runtime:

```bash
sudo /usr/lib/antminer-fleet-server/revoke-client-tunnel-key.sh --label test-client
sudo ! grep -q 'antminer-fleet-client:test-client' /etc/antminer-fleet/client-tunnel/authorized_keys
```

Do not run destructive revocation against production labels without explicit confirmation.

---

## Task 8: Add admin console location/navigation polish

**Objective:** Make tunnel key administration easy to find.

**Files:**
- Modify: `src/components/layout/AppShell.tsx`
- Modify: `src/App.tsx`
- Maybe split: `src/features/settings/TunnelKeyRequestsView.tsx`

**Options:**

Option A — keep under Settings:

```text
Settings → SSH Tunnel Keys
```

Option B — add admin nav item:

```text
Tunnel Keys
```

**Recommendation:** Add an admin nav item if this is used during onboarding multiple users. It saves clicks and makes pending requests obvious.

**Badge:**

Later enhancement: show pending count in nav. Not required for first implementation.

**Validation:**

```bash
npm run build
npm test
```

---

## Task 9: Add complete tests for the API wrapper

**Objective:** Prevent another camelCase/snake_case Tauri argument mismatch.

**Files:**
- Modify: `src/test/connectionApi.test.ts`

**Test cases:**

```ts
it("submits tunnel key request with serverUrl and input", async () => {
  await submitTunnelKeyRequest("https://10.81.1.120:8443", {
    label: "morin-laptop",
    public_key: "ssh-ed25519 AAAA test",
  });

  expect(command).toHaveBeenCalledWith("submit_tunnel_key_request", {
    serverUrl: "https://10.81.1.120:8443",
    input: {
      label: "morin-laptop",
      public_key: "ssh-ed25519 AAAA test",
    },
  });
});
```

Add tests for:

- `listTunnelKeyRequests()`
- `approveTunnelKeyRequest(id, input)`
- `rejectTunnelKeyRequest(id)`
- new status/config endpoint wrappers from Tasks 4-5

**Validation:**

```bash
npm test -- src/test/connectionApi.test.ts
```

---

## Task 10: Add operational documentation

**Objective:** Make the process clear enough that an admin can onboard a laptop without bugging the developer every time.

**Files:**
- Create/modify: `docs/ssh-tunnel-onboarding.md`
- Modify: `README.md` or server README with link

**Doc outline:**

```markdown
# SSH Tunnel User Onboarding

## Admin prerequisites
- server installed
- restricted tunnel user exists
- sshd Match User config installed
- Fleet server reachable from SSH host at 127.0.0.1:8443

## User first-run steps
1. Open app
2. Generate this computer's SSH key
3. Submit key request
4. Wait for admin approval
5. Enter approved tunnel destination
6. Start tunnel
7. Pair to https://127.0.0.1:8443

## Admin approval steps
1. Open Settings/Tunnel Keys
2. Verify request out-of-band
3. Approve
4. Give user destination: antminer-fleet-client-tunnel@HOST

## Revocation
1. Open Tunnel Keys
2. Click Revoke
3. Confirm authorized_keys no longer contains marker

## Troubleshooting
- OpenSSH missing
- port 8443 already used locally
- key pending forever
- script_not_found
- permission denied publickey
- connection refused on local 8443
```

**Validation:**

```bash
git diff --check
```

---

## Task 11: End-to-end validation on Windows client and Linux server

**Objective:** Prove the whole path works before shipping another installer.

**Windows client validation:**

From a fresh install/user profile:

```powershell
Get-Command ssh.exe
Get-Command ssh-keygen.exe
```

App flow:

1. Launch app.
2. Generate key.
3. Submit key request.
4. Confirm no private key is printed or uploaded.
5. After admin approval, save tunnel config.
6. Start tunnel.
7. Confirm local port:

```powershell
Test-NetConnection 127.0.0.1 -Port 8443
```

8. Confirm backend through tunnel:

```powershell
curl.exe -k https://127.0.0.1:8443/health
curl.exe -k https://127.0.0.1:8443/pairing
```

**Linux server validation:**

```bash
sudo grep 'antminer-fleet-client:' /etc/antminer-fleet/client-tunnel/authorized_keys
sudo journalctl -u ssh --since '10 minutes ago'
sudo journalctl -u antminer-fleet-server --since '10 minutes ago'
```

**Repo validation:**

```bash
npm test
npm run build
cargo check -p antminer-fleet-server --locked
git diff --check
```

Full build when ready:

```bash
npm run tauri build
```

---

## Risks and tradeoffs

### Risk: unauthenticated key submission endpoint spam

`POST /api/v1/tunnel-key-requests` is unauthenticated by design so first-run users can submit before pairing. That means anyone who can reach that endpoint can spam pending requests.

Mitigations:

- keep endpoint only reachable through LAN/tunnel/protected network
- rate limit by source IP if exposed
- add request deduplication by public key fingerprint
- cap pending requests
- admin must approve manually

### Risk: admin approval can write to server filesystem

Approval runs `authorize-client-tunnel-key.sh`. This is powerful enough to touch SSH authorization. Keep it boring and locked down.

Mitigations:

- validate label strictly
- validate public key format
- no shell interpolation with untrusted input
- script path fixed
- server service account permissions limited if possible
- audit every approval/revocation

### Risk: restricted SSH account accidentally allows shell

This is the big one. If the tunnel account gets shell access, users get more than a tunnel.

Mitigations:

- `/usr/sbin/nologin` shell
- sshd `Match User` block
- authorized key restrictions
- `AllowTcpForwarding local`
- `PermitOpen 127.0.0.1:8443`
- no password auth

### Risk: users type wrong SSH destination

Mitigation:

- server exposes safe client config after approval
- UI pre-fills it
- docs show exact expected format

### Risk: cert pinning with localhost

If everyone pairs through `https://127.0.0.1:8443`, the server certificate must support the name/IP used in pairing. Keep pairing URL consistent. Prefer one canonical local tunnel URL:

```text
https://127.0.0.1:8443
```

or:

```text
https://localhost:8443
```

Pick one and generate TLS SANs accordingly.

---

## Open questions before implementation

1. What exact SSH host should users connect to?
   - Example: `10.81.1.120`, DNS name, or separate jump host?

2. Should the restricted tunnel account be shared with per-key identity, or one Linux account per Fleet user?
   - Recommendation: shared restricted account + per-user keys + markers. Simpler and secure enough if restrictions are correct.

3. Should the admin console create normal Fleet app user accounts and SSH tunnel approval in one wizard?
   - Recommendation: not yet. Keep account creation and tunnel key approval separate for now.

4. Should rejected/revoked requests be retained forever for audit?
   - Recommendation: yes. Do not delete audit-relevant rows by default.

---

## Final acceptance criteria

- Fresh Windows user can install app and generate local SSH keypair.
- Private key never leaves the user's machine.
- Public key can be submitted to the server before pairing.
- Admin can see pending key request in app.
- Admin can approve request from app.
- Approval writes restricted authorized key on Linux host.
- User can save tunnel config without using Eddie's login.
- App starts SSH tunnel on startup.
- Pairing works through `https://127.0.0.1:8443`.
- Admin can revoke approved tunnel access.
- All approval/revocation actions are audited.
- Validation passes:

```bash
npm test
npm run build
cargo check -p antminer-fleet-server --locked
git diff --check
```

---

## Implementation order

1. UI wording fixes.
2. Admin note support.
3. Approval polling/status endpoint.
4. Client config endpoint and prefill.
5. Packaging hardening for restricted tunnel user.
6. Revocation support.
7. Admin navigation polish.
8. Documentation.
9. End-to-end Windows/Linux validation.

Do not start with packaging or revocation first. Prove the user/admin flow, then harden the server install path.
