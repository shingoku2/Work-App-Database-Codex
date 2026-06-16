# Frontend Agent Handoff Report — Tunnel Key Request Feature

## Context

This report is a complete briefing for a frontend agent implementing the
user-facing side of the tunnel key request feature. The backend plan lives at
`.hermes/plans/2026-06-16_tunnel-key-requests.md` in the same repo. This
document focuses specifically on what the frontend agent needs to know,
what the backend will provide, and exactly what the frontend must build.

---

## What this feature does

When a new Windows user installs Antminer Fleet Manager, they must set up an
SSH tunnel before they can pair with the server. Currently the app generates
an SSH key and shows them a textarea with instructions to manually email the
public key to the admin. There is no notification mechanism — the admin has
no idea when someone is waiting.

**After this feature:**
1. The user generates a key and enters a label (their name or machine tag).
2. The app submits the key to the server automatically.
3. The user sees a "waiting for approval" screen that auto-refreshes.
4. The admin sees a live list of pending requests in the app and clicks Approve.
5. The server runs `authorize-client-tunnel-key.sh` and marks the request approved.
6. The user's screen detects approval, the tunnel starts, and setup continues.

---

## Backend API (implemented by backend plan)

All endpoints under `/api/v1/tunnel-key-requests`.

### `POST /api/v1/tunnel-key-requests` — unauthenticated

Submit a new key request. No bearer token required. Uses the pinned certificate
for TLS verification (client must be paired first — or the call goes through
the pre-pairing HTTPS client).

Request body:
```json
{
  "label": "alice-workstation",
  "public_key": "ssh-ed25519 AAAA... antminer-fleet-tunnel"
}
```

Label validation (enforced server-side):
- Required, non-empty
- Characters: letters, numbers, dot, underscore, at, plus, dash only

Public key validation (enforced server-side):
- Must be OpenSSH format: `<type> <base64body> [comment]`
- Allowed types: `ssh-ed25519`, `ecdsa-sha2-nistp256/384/521`, `rsa-sha2-256/512`, `ssh-rsa`

Response `201 Created`:
```json
{
  "id": 42,
  "label": "alice-workstation",
  "public_key": "ssh-ed25519 AAAA...",
  "status": "pending",
  "note": null,
  "created_at": "2026-06-16T10:00:00Z"
}
```

### `GET /api/v1/tunnel-key-requests` — admin only

Returns all requests (pending + recent approved/rejected), newest first.

Response `200 OK`:
```json
[
  {
    "id": 42,
    "label": "alice-workstation",
    "public_key": "ssh-ed25519 AAAA...",
    "status": "pending",
    "note": null,
    "created_at": "2026-06-16T10:00:00Z"
  }
]
```

### `POST /api/v1/tunnel-key-requests/{id}/approve` — admin only

Runs `authorize-client-tunnel-key.sh` on the server and marks the request
approved. Returns the updated request.

Request body:
```json
{ "note": null }
```

Response `200 OK`: updated `TunnelKeyRequest` object.

Error `500` if the authorization script fails (e.g., SSH host not configured).

### `DELETE /api/v1/tunnel-key-requests/{id}` — admin only

Rejects (deletes) a request. Response `204 No Content`.

---

## Shared types — already added

### `crates/fleet-shared/src/lib.rs` (Rust — for reference)
```rust
pub struct SubmitTunnelKeyRequest {
    pub label: String,
    pub public_key: String,
}

pub struct TunnelKeyRequest {
    pub id: i64,
    pub label: String,
    pub public_key: String,
    pub status: String, // "pending" | "approved" | "rejected"
    pub note: Option<String>,
    pub created_at: String,
}

pub struct ApproveTunnelKeyRequest {
    pub note: Option<String>,
}
```

### `src/types/db.ts` — add these (if not already present)
```typescript
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

---

## Tauri commands — already registered

The following commands are registered in `src-tauri/src/lib.rs`
`invoke_handler!` and implemented in `src-tauri/src/commands/mod.rs`:

| Command name | What it does |
|---|---|
| `submit_tunnel_key_request` | POST (unauthenticated) — submits the key |
| `list_tunnel_key_requests` | GET (admin) — lists all requests |
| `approve_tunnel_key_request` | POST (admin) — approves one request |
| `reject_tunnel_key_request` | DELETE (admin) — rejects one request |

---

## Existing codebase conventions (read before writing any code)

### File locations
```
src/
  features/
    connection/
      ConnectionGate.tsx     ← main connection gating component (modify)
      connectionApi.ts       ← all Tauri command wrappers (modify)
    settings/                ← add TunnelKeyRequestsView.tsx here
  types/
    db.ts                    ← all shared TS types (modify)
  test/
    ConnectionGate.test.tsx  ← existing connection tests (reference)
    connectionApi.test.ts    ← existing api tests (reference)
```

### How to call a Tauri command
```typescript
import { command } from "@/lib/tauri";

// All Tauri command calls go through this wrapper.
// Rust snake_case param names are converted to camelCase automatically by Tauri v2.
// Example:
export function submitTunnelKeyRequest(input: SubmitTunnelKeyRequest): Promise<TunnelKeyRequest> {
  return command<TunnelKeyRequest>("submit_tunnel_key_request", { input });
}
```

### Tauri camelCase argument rule
Tauri v2 converts Rust snake_case parameter names to camelCase for the JS
invoke layer. So a Rust param `identity_file` must be sent as `identityFile`
from TypeScript. This is already handled consistently throughout the codebase.
`input`, `id`, `url` etc. are single-word and don't change.

### TanStack Query patterns
```typescript
// Query
const { data, isLoading, error } = useQuery({
  queryKey: ["tunnelKeyRequests"],
  queryFn: listTunnelKeyRequests,
  refetchInterval: 15_000,
});

// Mutation
const approve = useMutation({
  mutationFn: (id: number) => approveTunnelKeyRequest(id, { note: null }),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ["tunnelKeyRequests"] }),
});
```

### Styling conventions
- Background: `bg-[#101821]` (page), `bg-[#0b1219]` (card)
- Borders: `border border-white/10`
- Text: `text-slate-100` (primary), `text-slate-300` (secondary), `text-slate-400/500` (muted)
- Buttons: use `primaryButtonClass` / `secondaryButtonClass` from `@/components/ui/Panel`
- Input fields: use `fieldClass` from `@/components/ui/Panel`
- Error text: `text-red-300`, `text-sm`
- Amber warning box: `rounded-md border border-amber-400/30 bg-amber-400/10 p-3 text-amber-100`
- Success/approved: `text-emerald-400`
- Rejected: `text-red-400`

### Test conventions (copy from existing tests)
```typescript
// Mock the whole api module
vi.mock("@/features/connection/connectionApi", () => ({
  getConnectionState: vi.fn(),
  getTunnelStatus: vi.fn(),
  // ... all exports must be listed even if not used in this test
}));

// Always reset mocks between tests
beforeEach(() => {
  mockedFn.mockReset();
});

// Render with QueryClientProvider
function renderComponent() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <ComponentUnderTest />
    </QueryClientProvider>,
  );
}
```

---

## What the frontend agent must build

### 1. Add types to `src/types/db.ts`
Append the three interfaces above (`SubmitTunnelKeyRequest`, `TunnelKeyRequest`,
`ApproveTunnelKeyRequest`). Check first — the backend task may have already done this.

### 2. Add API functions to `src/features/connection/connectionApi.ts`

```typescript
import type { ApproveTunnelKeyRequest, SubmitTunnelKeyRequest, TunnelKeyRequest } from "@/types/db";

export function submitTunnelKeyRequest(input: SubmitTunnelKeyRequest): Promise<TunnelKeyRequest> {
  return command<TunnelKeyRequest>("submit_tunnel_key_request", { input });
}

export function listTunnelKeyRequests(): Promise<TunnelKeyRequest[]> {
  return command<TunnelKeyRequest[]>("list_tunnel_key_requests");
}

export function approveTunnelKeyRequest(id: number, input: ApproveTunnelKeyRequest): Promise<TunnelKeyRequest> {
  return command<TunnelKeyRequest>("approve_tunnel_key_request", { id, input });
}

export function rejectTunnelKeyRequest(id: number): Promise<void> {
  return command<void>("reject_tunnel_key_request", { id });
}
```

### 3. Update `TunnelSetupView` in `ConnectionGate.tsx`

Current flow:
- User clicks "Generate This Computer's SSH Key"
- Public key appears in a readonly textarea with a note to email it to the admin
- User fills in SSH destination form and clicks "Save and Start Tunnel"

New flow:
- Add a **label input** above the generate button (placeholder: "Your name or machine tag, e.g. alice-workstation")
- "Generate This Computer's SSH Key" button is disabled until label is non-empty
- After key generation, show the public key textarea as before
- Add a new **"Submit Key for Approval"** primary button that calls `submitTunnelKeyRequest`
- On success, render `<WaitingForApproval request={result} />` which replaces the form
- Keep the SSH destination form available as a secondary/manual option (collapsed
  or below a "Configure manually instead" toggle) for edge cases

### 4. Add `WaitingForApproval` component (inside `ConnectionGate.tsx`)

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
        Key submitted (request{" "}
        <span className="font-mono text-slate-100">#{request.id}</span>, label:{" "}
        <span className="font-mono text-slate-100">{request.label}</span>).
        Ask your server administrator to approve it in the Fleet Manager admin
        panel. This screen checks for approval every 10 seconds.
      </p>
      <div className="rounded-md border border-white/10 bg-black/20 p-4">
        <div className="mb-2 text-xs uppercase text-slate-500">
          Public key (manual fallback)
        </div>
        <textarea
          className={`${fieldClass} h-20 w-full font-mono text-xs`}
          readOnly
          value={request.public_key}
        />
      </div>
      <button className="text-xs text-slate-400 underline" onClick={onCancel}>
        Start over
      </button>
    </div>
  );
}
```

The `queryClient.invalidateQueries({ queryKey: ["tunnel"] })` re-runs
`getTunnelStatus`. Once the admin approves and the user clicks "Save and Start
Tunnel" (or the tunnel auto-starts), `tunnel.data.local_port_open` becomes
`true` and the `ConnectionGate` advances past the tunnel setup screen.

### 5. Create `src/features/settings/TunnelKeyRequestsView.tsx`

Admin-only view showing pending and recent key requests with Approve/Reject
buttons. Full component code is in the plan file at
`.hermes/plans/2026-06-16_tunnel-key-requests.md` (Task 8).

Auto-refreshes every 15 seconds via `refetchInterval: 15_000`.

### 6. Wire `TunnelKeyRequestsView` into the admin navigation

Inspect the current settings/navigation structure. Look at:
```
src/features/settings/
src/components/       (shell/nav components)
```
Find where the admin nav is rendered and add an "SSH Key Requests" entry
visible only when `user.role === "admin"`.

### 7. Write tests in `src/test/tunnelKeyRequests.test.tsx`

Cover at minimum:
1. Label required — generate button disabled when label empty
2. Submit key — after generate + submit, `WaitingForApproval` is shown
3. Admin list — pending request renders label, public key, Approve and Reject buttons
4. Admin approve — clicking Approve calls `approveTunnelKeyRequest(id, { note: null })`
5. Admin reject — clicking Reject calls `rejectTunnelKeyRequest(id)`

---

## Validation checklist

Run these before declaring the frontend work complete:

```bash
cd /Work-App-Database-Codex

# TypeScript build — must be zero errors
npm run build 2>&1 | grep -E 'error TS'

# All tests — must all pass
npm test -- --run 2>&1 | tail -10

# Whitespace check
git diff --check
```

---

## What NOT to do

- Do not add direct fetch/XHR calls to the server — all backend calls go through
  `command()` → Tauri command → server.
- Do not add a local database, offline queue, or cache of key requests on the client.
- Do not store the private key anywhere other than the path returned by
  `generate_tunnel_key` (already in `~/.ssh/antminer_fleet_tunnel`).
- Do not add any mechanism for the client to poll `/api/v1/tunnel-key-requests`
  directly — it doesn't have a session token at the time it submits the key.
  The approval detection is indirect: once approved, the admin's server runs
  the script which writes to `authorized_keys`, and then when the user saves
  tunnel config the SSH connection either succeeds or fails.
- Do not widen `npm install` — use `npm ci` only.
