# Tunnel Key Request API — Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Allow a Windows desktop user to submit their SSH public key from the
setup wizard, and allow an admin to review and authorize pending requests
directly inside the app, without any out-of-band communication.

**Architecture:**
A new `tunnel_key_requests` PostgreSQL table stores pending requests. The
desktop client submits a request (unauthenticated, like `/pairing`) and then
polls for its own status. The admin views pending requests in the app and
approves them, which calls the server to run `authorize-client-tunnel-key.sh`
and marks the request approved. Rejected requests are deleted. Approved
requests cause the client to proceed past the tunnel setup screen.

**Tech stack:**
Rust/Axum server (`server/src/api.rs`), PostgreSQL (sqlx), `fleet-shared`
types (`crates/fleet-shared/src/lib.rs`), Tauri commands
(`src-tauri/src/commands/mod.rs`), React/TypeScript frontend
(`src/features/connection/ConnectionGate.tsx`).

**Key conventions to follow (read before starting):**
- All server routes live in `server/src/api.rs` — monolithic, no sub-files.
- Shared Rust↔TypeScript types go in `crates/fleet-shared/src/lib.rs` AND
  `src/types/db.ts` simultaneously, snake_case fields, kept in sync.
- Tauri commands are `pub fn/async fn` in `src-tauri/src/commands/mod.rs` and
  registered in `src-tauri/src/lib.rs` `invoke_handler!`.
- Frontend API calls go through `command<T>("name", args)` in
  `src/features/connection/connectionApi.ts`.
- Auth: `authenticated_user()` = any logged-in user; `require_admin()` =
  admin only. The submit endpoint is unauthenticated (like `/pairing`).
- Migration files: `server/migrations/000N_description.sql`, sequential.
- `audit_log()` is called for significant admin actions (see existing usage in
  `create_user`, `update_user` handlers in `api.rs`).
- Validation errors return `AppError::bad_request(msg)`.
- Do not commit, push, or deploy — implement and verify locally only.

---

## Task 1: Add migration for `tunnel_key_requests` table

**Objective:** Create the database schema for pending tunnel key requests.

**Files:**
- Create: `server/migrations/0006_tunnel_key_requests.sql`

**Step 1: Write the migration**

```sql
CREATE TABLE tunnel_key_requests (
    id          BIGSERIAL PRIMARY KEY,
    label       TEXT NOT NULL,
    public_key  TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected')),
    note        TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX tunnel_key_requests_status_idx ON tunnel_key_requests (status);
CREATE INDEX tunnel_key_requests_created_at_idx ON tunnel_key_requests (created_at);
```

**Step 2: Verify syntax**

```bash
cd /Work-App-Database-Codex
# Dry-check: no psql required, just look for obvious SQL errors
grep -n 'CREATE\|ALTER\|INSERT' server/migrations/0006_tunnel_key_requests.sql
```

Expected: 4 lines printed, no error.

**Step 3: Verify it fits the migration sequence**

```bash
ls server/migrations/
```

Expected: 6 files, `0006_tunnel_key_requests.sql` is last.

**Step 4: Commit**

```bash
git add server/migrations/0006_tunnel_key_requests.sql
git commit -m "feat: add tunnel_key_requests migration"
```

---

## Task 2: Add shared types to `fleet-shared`

**Objective:** Define the Rust structs that cross the server↔client boundary
for tunnel key request creation, listing, and approval.

**Files:**
- Modify: `crates/fleet-shared/src/lib.rs` (append to end of file)

**Step 1: Append these types**

```rust
// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitTunnelKeyRequest {
    pub label: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelKeyRequest {
    pub id: i64,
    pub label: String,
    pub public_key: String,
    pub status: String, // "pending" | "approved" | "rejected"
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveTunnelKeyRequest {
    pub note: Option<String>,
}
```

**Step 2: Verify it compiles**

```bash
cd /Work-App-Database-Codex
cargo check -p fleet-shared --locked 2>&1 | grep -E '^error'
```

Expected: no output (zero errors).

**Step 3: Commit**

```bash
git add crates/fleet-shared/src/lib.rs
git commit -m "feat: add TunnelKeyRequest shared types"
```

---

## Task 3: Add matching TypeScript types to `src/types/db.ts`

**Objective:** Keep Rust↔TypeScript contracts in sync.

**Files:**
- Modify: `src/types/db.ts` (append to end of file)

**Step 1: Append these types**

```typescript
// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

export interface SubmitTunnelKeyRequest {
  label: string;
  public_key: string;
}

export interface TunnelKeyRequest {
  id: number;
  label: string;
  public_key: string;
  status: "pending" | "approved" | "rejected";
  note: string | null;
  created_at: string;
}

export interface ApproveTunnelKeyRequest {
  note: string | null;
}
```

**Step 2: Verify TypeScript build is clean**

```bash
cd /Work-App-Database-Codex
npm run build 2>&1 | grep -E 'error TS'
```

Expected: no output.

**Step 3: Commit**

```bash
git add src/types/db.ts
git commit -m "feat: add TunnelKeyRequest TypeScript types"
```

---

## Task 4: Add server route handlers in `api.rs`

**Objective:** Implement three server endpoints:
- `POST /api/v1/tunnel-key-requests` — unauthenticated; client submits key
- `GET  /api/v1/tunnel-key-requests` — admin only; list all requests
- `POST /api/v1/tunnel-key-requests/{id}/approve` — admin only; runs script, marks approved
- `DELETE /api/v1/tunnel-key-requests/{id}` — admin only; reject/delete

**Files:**
- Modify: `server/src/api.rs`

### Step 1: Add imports at top of `api.rs`

The existing imports already cover what's needed except the new shared types.
Find the `use fleet_shared::{...}` block (around line 13) and add
`ApproveTunnelKeyRequest, SubmitTunnelKeyRequest, TunnelKeyRequest` to it.

```rust
// Add to the existing fleet_shared use block:
use fleet_shared::{
    // ... existing items ...
    ApproveTunnelKeyRequest, SubmitTunnelKeyRequest, TunnelKeyRequest,
    // ...
};
```

### Step 2: Register four new routes in `serve()` (around line 295)

Find the `.route("/api/v1/sites/{id}", ...)` line and add after it:

```rust
.route(
    "/api/v1/tunnel-key-requests",
    post(submit_tunnel_key_request).get(list_tunnel_key_requests),
)
.route(
    "/api/v1/tunnel-key-requests/{id}/approve",
    post(approve_tunnel_key_request),
)
.route(
    "/api/v1/tunnel-key-requests/{id}",
    axum::routing::delete(delete_tunnel_key_request),
)
```

### Step 3: Add handler implementations

Append these four handler functions to the end of `api.rs`, before the
closing of the file (after the last existing handler):

```rust
// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

async fn submit_tunnel_key_request(
    State(state): State<AppState>,
    Json(input): Json<SubmitTunnelKeyRequest>,
) -> AppResult<(StatusCode, Json<TunnelKeyRequest>)> {
    let label = input.label.trim().to_string();
    let public_key = input.public_key.trim().to_string();

    if label.is_empty() {
        return Err(AppError::bad_request("label is required"));
    }
    if !label.chars().all(|c| c.is_alphanumeric() || "._@+-".contains(c)) {
        return Err(AppError::bad_request(
            "label may contain only letters, numbers, dot, underscore, at, plus, and dash",
        ));
    }
    if public_key.is_empty() {
        return Err(AppError::bad_request("public_key is required"));
    }
    // Basic OpenSSH public key format check: must have at least two whitespace-separated tokens
    let mut parts = public_key.splitn(3, char::is_whitespace);
    let key_type = parts.next().unwrap_or("").to_string();
    let key_body = parts.next().unwrap_or("").to_string();
    if key_type.is_empty() || key_body.is_empty() {
        return Err(AppError::bad_request("public_key must be in OpenSSH format"));
    }
    let allowed_types = [
        "ssh-ed25519",
        "ecdsa-sha2-nistp256",
        "ecdsa-sha2-nistp384",
        "ecdsa-sha2-nistp521",
        "rsa-sha2-256",
        "rsa-sha2-512",
        "ssh-rsa",
    ];
    if !allowed_types.contains(&key_type.as_str()) {
        return Err(AppError::bad_request("unsupported public key type"));
    }

    let row = sqlx::query(
        "INSERT INTO tunnel_key_requests (label, public_key)
         VALUES ($1, $2)
         RETURNING id, label, public_key, status, note, created_at",
    )
    .bind(&label)
    .bind(&public_key)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;

    Ok((StatusCode::CREATED, Json(tunnel_key_request_from_row(&row))))
}

async fn list_tunnel_key_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<TunnelKeyRequest>>> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query(
        "SELECT id, label, public_key, status, note, created_at
         FROM tunnel_key_requests
         ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::database)?;
    Ok(Json(rows.iter().map(tunnel_key_request_from_row).collect()))
}

async fn approve_tunnel_key_request(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ApproveTunnelKeyRequest>,
) -> AppResult<Json<TunnelKeyRequest>> {
    let admin = require_admin(&state, &headers).await?;

    let row = sqlx::query(
        "SELECT id, label, public_key, status, note, created_at
         FROM tunnel_key_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?
    .ok_or_else(|| AppError::not_found("tunnel key request not found"))?;

    if row.get::<String, _>("status") == "approved" {
        return Err(AppError::bad_request("already approved"));
    }

    let label: String = row.get("label");
    let public_key: String = row.get("public_key");

    // Run authorize-client-tunnel-key.sh
    let script_path = "/usr/lib/antminer-fleet-server/authorize-client-tunnel-key.sh";
    let output = tokio::process::Command::new(script_path)
        .args(["--label", &label, "--public-key", &public_key])
        .output()
        .await
        .map_err(|e| AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "script_failed",
            message: format!("Could not run key authorization script: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let msg = if stderr.is_empty() { stdout } else { stderr };
        return Err(AppError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "script_failed",
            message: format!("Key authorization script failed: {msg}"),
        });
    }

    let updated = sqlx::query(
        "UPDATE tunnel_key_requests
         SET status = 'approved', note = $1, updated_at = NOW()
         WHERE id = $2
         RETURNING id, label, public_key, status, note, created_at",
    )
    .bind(input.note.as_deref())
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::database)?;

    audit_log(
        &state,
        Some(admin.id),
        Some(&admin.username),
        "tunnel_key.approved",
        Some("tunnel_key_request"),
        Some(&id.to_string()),
        None,
        None,
        Some(&serde_json::json!({"label": label})),
        Some(&remote.ip().to_string()),
    )
    .await;

    Ok(Json(tunnel_key_request_from_row(&updated)))
}

async fn delete_tunnel_key_request(
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let admin = require_admin(&state, &headers).await?;

    let row = sqlx::query(
        "DELETE FROM tunnel_key_requests WHERE id = $1
         RETURNING id, label, public_key, status, note, created_at",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::database)?;

    if let Some(row) = row {
        let label: String = row.get("label");
        audit_log(
            &state,
            Some(admin.id),
            Some(&admin.username),
            "tunnel_key.rejected",
            Some("tunnel_key_request"),
            Some(&id.to_string()),
            None,
            None,
            Some(&serde_json::json!({"label": label})),
            Some(&remote.ip().to_string()),
        )
        .await;
    }

    Ok(StatusCode::NO_CONTENT)
}

fn tunnel_key_request_from_row(row: &sqlx::postgres::PgRow) -> TunnelKeyRequest {
    TunnelKeyRequest {
        id: row.get("id"),
        label: row.get("label"),
        public_key: row.get("public_key"),
        status: row.get("status"),
        note: row.get("note"),
        created_at: row
            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .to_rfc3339(),
    }
}
```

**Note on `AppError::not_found`:** Check whether this helper exists in `api.rs`.
If not, add it alongside the existing `AppError` impl block:

```rust
fn not_found(message: impl Into<String>) -> Self {
    Self {
        status: StatusCode::NOT_FOUND,
        code: "not_found",
        message: message.into(),
    }
}
```

### Step 4: Verify server compiles

```bash
cd /Work-App-Database-Codex
cargo check -p antminer-fleet-server --locked 2>&1 | grep -E '^error'
```

Expected: no output.

### Step 5: Commit

```bash
git add server/src/api.rs
git commit -m "feat: add tunnel key request server endpoints"
```

---

## Task 5: Add Tauri commands

**Objective:** Expose the three relevant endpoints to the frontend via Tauri
commands. The client needs: submit (unauthenticated), list (admin), approve,
and reject.

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

### Step 1: Add imports to `commands/mod.rs`

Add to the existing `use fleet_shared::{ ... }` block:

```rust
use fleet_shared::{
    // ... existing items ...
    ApproveTunnelKeyRequest, SubmitTunnelKeyRequest, TunnelKeyRequest,
};
```

### Step 2: Add four commands to `commands/mod.rs`

Append after the `delete_site` command:

```rust
// ---------------------------------------------------------------------------
// Tunnel key requests
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn submit_tunnel_key_request(
    state: State<'_, ClientState>,
    input: SubmitTunnelKeyRequest,
) -> Result<TunnelKeyRequest, String> {
    state.post_unauthenticated("/api/v1/tunnel-key-requests", &input).await
}

#[tauri::command]
pub async fn list_tunnel_key_requests(
    state: State<'_, ClientState>,
) -> Result<Vec<TunnelKeyRequest>, String> {
    state.get("/api/v1/tunnel-key-requests").await
}

#[tauri::command]
pub async fn approve_tunnel_key_request(
    state: State<'_, ClientState>,
    id: i64,
    input: ApproveTunnelKeyRequest,
) -> Result<TunnelKeyRequest, String> {
    state
        .post(&format!("/api/v1/tunnel-key-requests/{id}/approve"), &input)
        .await
}

#[tauri::command]
pub async fn reject_tunnel_key_request(
    state: State<'_, ClientState>,
    id: i64,
) -> Result<(), String> {
    state
        .delete(&format!("/api/v1/tunnel-key-requests/{id}"))
        .await
}
```

**Note on `post_unauthenticated`:** The submit endpoint is unauthenticated
(like `/pairing`), so the client must send the request without a Bearer token.
Check whether `ClientState` already has an unauthenticated POST method.
If not, add one alongside the existing methods (it should use the pinned
HTTPS client but omit the Authorization header). If the client is not yet
paired, use the pre-pairing HTTP client path (same one used by `probe_server`).
Look at how `probe_server` calls `ClientState::probe()` for the unauthenticated
pattern — match it.

### Step 3: Register all four commands in `lib.rs`

Add to the `invoke_handler!` list after `delete_site`:

```rust
commands::submit_tunnel_key_request,
commands::list_tunnel_key_requests,
commands::approve_tunnel_key_request,
commands::reject_tunnel_key_request,
```

### Step 4: Verify Tauri compiles (ignoring the icon panic on Linux)

```bash
cd /Work-App-Database-Codex
cargo check -p antminer-fleet-manager --locked 2>&1 | grep -E '^error'
```

Expected: only the pre-existing `generate_context!` icon panic if running on
Linux without a full Tauri build env — no new errors.

### Step 5: Commit

```bash
git add src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add tunnel key request Tauri commands"
```

---

## Task 6: Add API functions to `connectionApi.ts`

**Objective:** Expose the four commands to the React layer.

**Files:**
- Modify: `src/features/connection/connectionApi.ts`

### Step 1: Add the imports

Add to the existing type imports at the top of `connectionApi.ts`:

```typescript
import type {
  // ... existing ...
  ApproveTunnelKeyRequest,
  SubmitTunnelKeyRequest,
  TunnelKeyRequest,
} from "@/types/db";
```

### Step 2: Append four functions

```typescript
export function submitTunnelKeyRequest(
  input: SubmitTunnelKeyRequest,
): Promise<TunnelKeyRequest> {
  return command<TunnelKeyRequest>("submit_tunnel_key_request", { input });
}

export function listTunnelKeyRequests(): Promise<TunnelKeyRequest[]> {
  return command<TunnelKeyRequest[]>("list_tunnel_key_requests");
}

export function approveTunnelKeyRequest(
  id: number,
  input: ApproveTunnelKeyRequest,
): Promise<TunnelKeyRequest> {
  return command<TunnelKeyRequest>("approve_tunnel_key_request", { id, input });
}

export function rejectTunnelKeyRequest(id: number): Promise<void> {
  return command<void>("reject_tunnel_key_request", { id });
}
```

### Step 3: Verify build is clean

```bash
cd /Work-App-Database-Codex
npm run build 2>&1 | grep -E 'error TS'
```

Expected: no output.

### Step 4: Commit

```bash
git add src/features/connection/connectionApi.ts
git commit -m "feat: add tunnel key request API functions"
```

---

## Task 7: Update `TunnelSetupView` in `ConnectionGate.tsx` to submit key to server

**Objective:** After the user generates a key, automatically submit it to the
server instead of just displaying it for manual copy-paste. The UI then shows
a "waiting for approval" state and polls for status.

**Files:**
- Modify: `src/features/connection/ConnectionGate.tsx`

### Step 1: Add the imports

Add to existing imports in `ConnectionGate.tsx`:

```typescript
import type { TunnelKeyRequest } from "@/types/db";
import {
  // ... existing imports ...
  submitTunnelKeyRequest,
} from "./connectionApi";
```

### Step 2: Update `TunnelSetupView`

Replace the `TunnelSetupView` component with a version that:
1. Asks the user for a **label** (their name or machine name) before generating the key — this becomes the `authorized_keys` entry label the admin sees.
2. On key generation success, immediately calls `submitTunnelKeyRequest` with the label and public key.
3. If the submission succeeds, shows a "waiting for approval" state with the request ID — the user does nothing further.
4. Keeps the existing "Save and Start Tunnel" form flow for when the tunnel is already configured but the port isn't open (the `status.configured && !status.local_port_open` branch).

The waiting state should show:
- A clear message: "Your key has been submitted. Ask your server administrator to approve it in the app. This screen will refresh automatically."
- The submitted public key (readonly textarea) in case the admin needs it out-of-band as a fallback.
- An auto-refresh every 10 seconds via `useEffect` + `queryClient.invalidateQueries(["tunnel"])`.

The polling re-evaluates `connection` and `tunnel` queries. Once the admin
approves the key, `start_tunnel_connection` should succeed. The "Save and
Start Tunnel" form is shown in a collapsed/secondary section for users who
want to configure manually (e.g., if the server-side script path is
non-standard on their deployment).

**Key state additions to `TunnelSetupView`:**

```typescript
const [label, setLabel] = useState("");
const [pendingRequest, setPendingRequest] = useState<TunnelKeyRequest | null>(null);

const submitKey = useMutation({
  mutationFn: () =>
    submitTunnelKeyRequest({
      label: label.trim(),
      public_key: key!.public_key,
    }),
  onSuccess: (req) => setPendingRequest(req),
});
```

Generate key button should now require `label.trim()` to be non-empty before
enabling. After key is generated, a "Submit key for admin approval" button
replaces the manual copy-paste textarea.

When `pendingRequest` is set, render the waiting UI instead of the form:

```tsx
{pendingRequest && (
  <WaitingForApproval
    request={pendingRequest}
    onCancel={() => setPendingRequest(null)}
  />
)}
```

### Step 3: Write the `WaitingForApproval` sub-component

```tsx
function WaitingForApproval({
  request,
  onCancel,
}: {
  request: TunnelKeyRequest;
  onCancel: () => void;
}) {
  const queryClient = useQueryClient();

  useEffect(() => {
    const id = setInterval(
      () => queryClient.invalidateQueries({ queryKey: ["tunnel"] }),
      10_000,
    );
    return () => clearInterval(id);
  }, [queryClient]);

  return (
    <div className="space-y-4 text-sm text-slate-300">
      <p>
        Your SSH key has been submitted (request #{request.id}, label:{" "}
        <span className="font-mono text-slate-100">{request.label}</span>).
        Ask your server administrator to approve it in the Fleet Manager admin
        panel. This screen checks for approval every 10 seconds.
      </p>
      <div className="rounded-md border border-white/10 bg-black/20 p-4">
        <div className="mb-2 text-xs uppercase text-slate-500">
          Public key (for manual fallback)
        </div>
        <textarea
          className="h-20 w-full resize-none rounded border border-white/10 bg-black/30 p-2 font-mono text-xs text-slate-200"
          readOnly
          value={request.public_key}
        />
      </div>
      <button
        className="text-xs text-slate-400 underline"
        onClick={onCancel}
      >
        Start over
      </button>
    </div>
  );
}
```

### Step 4: Verify build

```bash
cd /Work-App-Database-Codex
npm run build 2>&1 | grep -E 'error TS'
```

Expected: no output.

### Step 5: Run tests

```bash
npm test -- --run 2>&1 | tail -10
```

Expected: all existing tests pass (90/90 or more).

### Step 6: Commit

```bash
git add src/features/connection/ConnectionGate.tsx
git commit -m "feat: auto-submit tunnel key to server on generation, show waiting state"
```

---

## Task 8: Add admin `TunnelKeyRequestsView` component

**Objective:** Give the admin a view inside the app to see pending key requests
and approve or reject them with one click.

**Files:**
- Create: `src/features/settings/TunnelKeyRequestsView.tsx`

### Step 1: Write the component

```tsx
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  approveTunnelKeyRequest,
  listTunnelKeyRequests,
  rejectTunnelKeyRequest,
} from "@/features/connection/connectionApi";

export function TunnelKeyRequestsView() {
  const queryClient = useQueryClient();
  const { data: requests = [], isLoading, error } = useQuery({
    queryKey: ["tunnelKeyRequests"],
    queryFn: listTunnelKeyRequests,
    refetchInterval: 15_000,
  });

  const approve = useMutation({
    mutationFn: (id: number) =>
      approveTunnelKeyRequest(id, { note: null }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["tunnelKeyRequests"] }),
  });

  const reject = useMutation({
    mutationFn: (id: number) => rejectTunnelKeyRequest(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["tunnelKeyRequests"] }),
  });

  if (isLoading) return <p className="text-sm text-slate-400">Loading...</p>;
  if (error)
    return (
      <p className="text-sm text-red-300">
        Failed to load requests: {String(error)}
      </p>
    );

  const pending = requests.filter((r) => r.status === "pending");
  const recent = requests.filter((r) => r.status !== "pending");

  return (
    <div className="space-y-6">
      <section>
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-400">
          Pending ({pending.length})
        </h2>
        {pending.length === 0 ? (
          <p className="text-sm text-slate-500">No pending key requests.</p>
        ) : (
          <ul className="space-y-3">
            {pending.map((req) => (
              <li
                key={req.id}
                className="rounded-lg border border-white/10 bg-[#0b1219] p-4"
              >
                <div className="mb-2 flex items-center justify-between">
                  <span className="font-mono text-sm text-slate-100">
                    {req.label}
                  </span>
                  <span className="text-xs text-slate-500">
                    {new Date(req.created_at).toLocaleString()}
                  </span>
                </div>
                <textarea
                  className="mb-3 h-16 w-full resize-none rounded border border-white/10 bg-black/30 p-2 font-mono text-xs text-slate-300"
                  readOnly
                  value={req.public_key}
                />
                <div className="flex gap-2">
                  <button
                    className="rounded bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
                    disabled={approve.isPending}
                    onClick={() => approve.mutate(req.id)}
                  >
                    Approve
                  </button>
                  <button
                    className="rounded bg-red-700 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-600 disabled:opacity-50"
                    disabled={reject.isPending}
                    onClick={() => reject.mutate(req.id)}
                  >
                    Reject
                  </button>
                </div>
                {(approve.error || reject.error) && (
                  <p className="mt-2 text-xs text-red-300">
                    {String(approve.error ?? reject.error)}
                  </p>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      {recent.length > 0 && (
        <section>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-400">
            Recent
          </h2>
          <ul className="space-y-2">
            {recent.map((req) => (
              <li
                key={req.id}
                className="flex items-center justify-between rounded border border-white/10 bg-[#0b1219] px-4 py-2 text-sm"
              >
                <span className="font-mono text-slate-200">{req.label}</span>
                <span
                  className={
                    req.status === "approved"
                      ? "text-emerald-400"
                      : "text-red-400"
                  }
                >
                  {req.status}
                </span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
```

### Step 2: Wire into the admin settings area

Find where the admin navigation/settings panel is rendered. Look at:
- `src/features/settings/` — list existing files
- Any `<nav>` or admin tab list in the shell

Add "SSH Key Requests" as a tab/section visible only to admin users, rendering
`<TunnelKeyRequestsView />`. Exact wiring depends on the existing settings/nav
structure — inspect the current code before editing.

### Step 3: Verify build

```bash
npm run build 2>&1 | grep -E 'error TS'
```

Expected: no output.

### Step 4: Commit

```bash
git add src/features/settings/TunnelKeyRequestsView.tsx
git add src/  # any navigation file touched
git commit -m "feat: add admin tunnel key request management view"
```

---

## Task 9: Write tests

**Objective:** Cover the new behavior with unit tests.

**Files:**
- Create: `src/test/tunnelKeyRequests.test.tsx`

### Step 1: Write tests covering:

1. **Submit key — happy path:** mock `submitTunnelKeyRequest` to resolve, verify the waiting-for-approval UI appears after generate+submit.
2. **Submit key — label required:** verify the generate button is disabled when label is empty.
3. **Admin list — shows pending requests:** mock `listTunnelKeyRequests` with one pending item, verify label and public key are rendered.
4. **Admin approve:** mock `approveTunnelKeyRequest`, click Approve, verify it is called with the right `id`.
5. **Admin reject:** mock `rejectTunnelKeyRequest`, click Reject, verify it is called with the right `id`.

Follow the exact test structure used in `src/test/ConnectionGate.test.tsx` and
`src/test/connectionApi.test.ts` — same `QueryClientProvider` wrapper, same
`vi.mock` pattern, same `userEvent.setup()` for interactions.

### Step 2: Run tests

```bash
npm test -- --run 2>&1 | tail -10
```

Expected: all tests pass, including the new ones.

### Step 3: Commit

```bash
git add src/test/tunnelKeyRequests.test.tsx
git commit -m "test: tunnel key request submit and admin approval"
```

---

## Task 10: Final validation and `git diff --check`

**Objective:** Confirm everything is clean before declaring done.

```bash
cd /Work-App-Database-Codex

# 1. Rust server
cargo check -p antminer-fleet-server --locked 2>&1 | grep -E '^error'

# 2. Rust client (icon panic is pre-existing on Linux, not a new error)
cargo check -p antminer-fleet-manager --locked 2>&1 | grep -E '^error'

# 3. Frontend build
npm run build 2>&1 | grep -E 'error TS'

# 4. All tests
npm test -- --run 2>&1 | tail -10

# 5. Shell scripts untouched — no re-check needed unless modified

# 6. Whitespace
git diff --check
```

Expected:
- Steps 1–3: no output (zero errors)
- Step 4: all tests pass
- Step 6: no output (clean)

---

## Files changed summary

| File | Change |
|------|--------|
| `server/migrations/0006_tunnel_key_requests.sql` | New migration |
| `crates/fleet-shared/src/lib.rs` | Add 3 new shared structs |
| `src/types/db.ts` | Add 3 matching TypeScript interfaces |
| `server/src/api.rs` | Add 4 route handlers + `tunnel_key_request_from_row` |
| `src-tauri/src/commands/mod.rs` | Add 4 Tauri commands |
| `src-tauri/src/lib.rs` | Register 4 commands in `invoke_handler!` |
| `src/features/connection/connectionApi.ts` | Add 4 API functions |
| `src/features/connection/ConnectionGate.tsx` | Update `TunnelSetupView` + add `WaitingForApproval` |
| `src/features/settings/TunnelKeyRequestsView.tsx` | New admin view |
| `src/test/tunnelKeyRequests.test.tsx` | New tests |

---

## Open questions / risks

1. **`post_unauthenticated` on `ClientState`:** The client needs to hit
   `/api/v1/tunnel-key-requests` without a Bearer token, but with the pinned
   certificate. Inspect `ClientState` before implementing Task 5 to see if
   this exists. If not, it's a small addition — mirror how `probe_server` uses
   a pre-pairing client, except here the client IS already paired (certificate
   is pinned), just not authenticated. The simplest approach: add a
   `post_no_auth` method that uses the pinned client without the Authorization
   header.

2. **Script path on the server:** `authorize-client-tunnel-key.sh` is installed
   to `/usr/lib/antminer-fleet-server/` by the Debian package. In development
   (source checkout), the script is at `server/scripts/authorize-client-tunnel-key.sh`.
   The `approve_tunnel_key_request` handler should check both paths, preferring
   the installed path and falling back to the source path for dev — same
   pattern used by `tunnel_script_path()` in `commands/mod.rs`.

3. **Admin notification:** Task 8 adds auto-refresh every 15 seconds. This is
   enough for a small fleet. A future improvement would be a webhook event
   (`tunnel_key.request_submitted`) fired from the submit handler — that's
   out of scope for this plan but the audit log entry provides a trail.

4. **Request cleanup:** Approved/rejected requests accumulate in the table.
   Add a periodic cleanup or a maximum retention (e.g., delete rows older
   than 30 days with status != 'pending') in a future maintenance task.
   Not blocking for initial implementation.
