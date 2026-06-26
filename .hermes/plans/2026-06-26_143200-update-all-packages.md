# Update All Packages — Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Update all Rust and npm dependencies to latest compatible versions, including major bumps for sha2, hmac, and eslint.

**Architecture:** Three-phase approach ordered by risk: (1) semver-compatible lockfile refresh, (2) Rust major-version migrations with code changes, (3) npm updates handed off to the frontend agent.

**Tech Stack:** Rust (Cargo workspace: server, src-tauri, fleet-shared), npm (Vite + React + Tauri frontend)

---

## Current State (verified 2026-06-26)

### Rust — 17 semver-compatible updates available via `cargo update`

All within existing `Cargo.toml` semver ranges. No source changes needed.

| Crate | Locked | Available | Direct dep? | Used by |
|-------|--------|-----------|-------------|---------|
| anyhow | 1.0.102 | 1.0.103 | Transitive | tauri, tauri-build |
| chacha20 | 0.10.0 | 0.10.1 | Transitive | rand, sqlx-postgres |
| generic-array | 0.14.7 | 0.14.9 | Transitive | digest 0.10, block-buffer 0.10 |
| hmac | 0.12.1 | 0.13.0 | **Direct (workspace)** | server (webhook HMAC) |
| js-sys | 0.3.102 | 0.3.103 | Transitive | web-sys (wasm-bindgen) |
| matchit | 0.8.4 | 0.8.6 | Transitive | axum |
| sha2 | 0.10.9 | 0.11.0 | **Direct (workspace)** | server, src-tauri |
| toml | 0.8.2 | 0.8.23 | Transitive | system-deps (Tauri GTK build-deps) |
| toml_datetime | 0.6.3 | 0.6.11 | Transitive | toml_edit |
| toml_edit | 0.20.2 | 0.20.7 | Transitive | proc-macro-crate (glib-macros) |
| uuid | 1.23.3 | 1.23.4 | Direct (workspace) | server, cfb |
| wasm-bindgen | 0.2.125 | 0.2.126 | Transitive | web-sys |
| wasm-bindgen-futures | 0.4.75 | 0.4.76 | Transitive | web-sys |
| wasm-bindgen-macro | 0.2.125 | 0.2.126 | Transitive | wasm-bindgen |
| wasm-bindgen-macro-support | 0.2.125 | 0.2.126 | Transitive | wasm-bindgen |
| wasm-bindgen-shared | 0.2.125 | 0.2.126 | Transitive | wasm-bindgen |
| web-sys | 0.3.102 | 0.3.103 | Transitive | tauri |

### Rust — 2 major version bumps requiring code changes

| Crate | Declared | Available | Call sites |
|-------|----------|-----------|------------|
| sha2 | "0.10" | 0.11.0 | 3 files, 4 call sites |
| hmac | "0.12" | 0.13.0 | 1 file, 2 call sites |

### npm — 4 semver-compatible updates

| Package | Current | Latest | Breaks? |
|---------|---------|--------|---------|
| @tanstack/react-query | 5.101.0 | 5.101.1 | No |
| @vitejs/plugin-react | 6.0.2 | 6.0.3 | No |
| autoprefixer | 10.5.0 | 10.5.2 | No |
| vite | 8.0.16 | 8.1.0 | No (minor) |

### npm — eslint 10 adoption (frontend agent)

| Package | Declared | Locked | Installed | Latest |
|---------|----------|--------|-----------|--------|
| eslint | ^9.39.4 | 9.39.4 | 10.5.0 | 10.6.0 |
| @eslint/js | ^9.39.4 | 9.39.4 | 10.0.1 | 10.0.1 |
| eslint-plugin-react | ^7.37.5 | 7.37.5 | 7.37.5 | 7.37.5 (peer max: ^9.7, does NOT support eslint 10) |
| eslint-plugin-react-hooks | ^7.1.1 | 7.1.1 | 7.1.1 | 7.1.1 (peer includes ^10.0.0) |
| typescript-eslint | ^8.62.0 | 8.62.0 | 8.62.0 | 8.62.0 (peer includes ^10.0.0) |

### Security

- `cargo audit`: 0 vulnerabilities across 658 crate dependencies
- `npm audit`: 0 vulnerabilities

---

## Phase 1: Rust Semver-Compatible Lockfile Refresh

No source changes. Pure lockfile update.

### Task 1: Apply cargo update

**Objective:** Update Cargo.lock to latest semver-compatible versions for all 17 packages.

**Files:**
- Modify: `Cargo.lock`

**Step 1: Dry-run preview**

Run: `cargo update --dry-run --workspace --verbose`
Expected: lists 17 packages with "available" versions, "Locking 0 packages" (dry run)

**Step 2: Apply the update**

Run: `cargo update --workspace`
Expected: "Updating" lines for each package, lockfile rewritten

**Step 3: Verify compilation**

Run: `cargo check --workspace --locked`
Expected: PASS — no errors

**Step 4: Run tests**

Run: `cargo test --workspace --locked`
Expected: PASS — all tests pass

**Step 5: Run audit**

Run: `cargo audit`
Expected: 0 vulnerabilities

**Step 6: Commit**

```bash
git add Cargo.lock
git commit -m "chore: update Rust dependencies to latest semver-compatible versions"
```

---

## Phase 2: sha2 0.10 → 0.11 Migration

sha2 0.11 moves from `digest 0.10` (generic-array based) to `digest 0.11` (block-api based).
The `Digest` trait still exists. Key API changes to watch for:

- `Sha256::digest()` return type changes from `GenericArray<u8, U32>` to `Output<Self>`.
  `Output<Self>` is still iterable (`.iter()`) and indexable.
- The `LowerHex` formatting impl (`format!("{:x}", ...)`) may or may not be available on
  `Output<Self>` in digest 0.11. If it breaks, replace with explicit `.iter().map(|b| format!("{b:02x}")).collect()`.
- `rust-version: 1.85` required by sha2 0.11 — the project uses stable (1.96+), so this is fine.

### Task 2: Bump sha2 version in workspace Cargo.toml

**Objective:** Change the workspace dependency declaration from 0.10 to 0.11.

**Files:**
- Modify: `Cargo.toml` (workspace root, line 14)

**Step 1: Edit workspace dependency**

Change line 14 of `Cargo.toml`:

```toml
# Before:
sha2 = "0.10"

# After:
sha2 = "0.11"
```

**Step 2: Update lockfile**

Run: `cargo update -p sha2`
Expected: sha2 resolves to 0.11.0, digest resolves to 0.11.x, block-buffer updates

**Step 3: Check what broke**

Run: `cargo check --workspace 2>&1`
Expected: Likely errors in `server/src/auth.rs`, `server/src/api.rs`, `src-tauri/src/client.rs` —
note the exact compiler errors for the next task.

Do NOT commit yet.

### Task 3: Fix sha2 call sites in server/src/auth.rs

**Objective:** Update the token hash function to work with sha2 0.11 API.

**Files:**
- Modify: `server/src/auth.rs` (line 7, line 34)

**Call site 1 — import (line 7):**

```rust
// Current:
use sha2::{Digest, Sha256};

// Likely unchanged — Digest and Sha256 are still exported from sha2 0.11.
// If the import fails, check whether Digest moved to the digest crate re-export.
```

**Call site 2 — token_hash (line 34):**

```rust
// Current:
pub fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

// If LowerHex is not impl'd on Output<Self> in digest 0.11, change to:
pub fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
```

**Step 1: Make the change**

Apply the fix based on what `cargo check` reported in Task 2.

**Step 2: Verify**

Run: `cargo check -p antminer-fleet-server`
Expected: auth.rs compiles clean (or remaining errors are in api.rs only)

### Task 4: Fix sha2 call sites in server/src/api.rs

**Objective:** Update certificate fingerprint computation to work with sha2 0.11 API.

**Files:**
- Modify: `server/src/api.rs` (line 25, line 364-368)

**Call site 1 — import (line 25):**

```rust
// Current:
use sha2::{Digest, Sha256};

// Likely unchanged.
```

**Call site 2 — certificate fingerprint (lines 364-368):**

```rust
// Current:
let fingerprint_sha256 = Sha256::digest(certificate_der.as_ref())
    .iter()
    .map(|byte| format!("{byte:02X}"))
    .collect::<Vec<_>>()
    .join(":");

// This pattern (iter + map + format) should work as-is with Output<Self>.
// If .iter() is not available on Output<Self>, use .as_ref() or into_iter().
```

**Step 1: Make the change**

Apply fixes based on compiler errors.

**Step 2: Verify**

Run: `cargo check -p antminer-fleet-server`
Expected: PASS

### Task 5: Fix sha2 call sites in src-tauri/src/client.rs

**Objective:** Update client-side certificate fingerprint to work with sha2 0.11 API.

**Files:**
- Modify: `src-tauri/src/client.rs` (line 11, line 397-401)

**Call site 1 — import (line 11):**

```rust
// Current:
use sha2::{Digest, Sha256};

// Likely unchanged.
```

**Call site 2 — certificate_fingerprint (lines 397-401):**

```rust
// Current:
Ok(Sha256::digest(der.as_ref())
    .iter()
    .map(|byte| format!("{byte:02X}"))
    .collect::<Vec<_>>()
    .join(":"))

// Same pattern as server. Should work as-is or need .as_ref() adjustment.
```

**Step 1: Make the change**

Apply fixes based on compiler errors.

**Step 2: Verify**

Run: `cargo check -p antminer-fleet-manager`
Expected: PASS

### Task 6: Full workspace verification for sha2 migration

**Objective:** Confirm the entire workspace compiles and tests pass with sha2 0.11.

**Step 1: Workspace check**

Run: `cargo check --workspace --locked`
Expected: PASS

**Step 2: Workspace tests**

Run: `cargo test --workspace --locked`
Expected: PASS — all tests pass

**Step 3: Audit**

Run: `cargo audit`
Expected: 0 vulnerabilities

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock server/src/auth.rs server/src/api.rs src-tauri/src/client.rs
git commit -m "chore: migrate sha2 0.10 → 0.11"
```

---

## Phase 3: hmac 0.12 → 0.13 Migration

hmac 0.13 moves to `digest 0.11` traits (same as sha2 0.11, so they share the digest
version now). Key API changes to watch for:

- `Hmac::new_from_slice()` may be renamed to `Hmac::new_from_key()` in 0.13.
  If `new_from_slice` still exists but is deprecated, use the new name.
- `Mac::finalize().into_bytes()` return type may change. The `.iter()` pattern
  should still work.
- `Mac` trait import path may change. Currently imported as `use hmac::{Hmac, Mac};`.

### Task 7: Bump hmac version in workspace Cargo.toml

**Objective:** Change the workspace dependency from 0.12 to 0.13.

**Files:**
- Modify: `Cargo.toml` (workspace root, line 13)

**Step 1: Edit workspace dependency**

Change line 13 of `Cargo.toml`:

```toml
# Before:
hmac = "0.12"

# After:
hmac = "0.13"
```

**Step 2: Update lockfile**

Run: `cargo update -p hmac`
Expected: hmac resolves to 0.13.0

**Step 3: Check what broke**

Run: `cargo check --workspace 2>&1`
Expected: Errors in `server/src/api.rs` — note exact compiler errors.

Do NOT commit yet.

### Task 8: Fix hmac call sites in server/src/api.rs

**Objective:** Update webhook HMAC signing to work with hmac 0.13 API.

**Files:**
- Modify: `server/src/api.rs` (line 23, line 37, line 1648-1649)

**Call site 1 — import (line 23):**

```rust
// Current:
use hmac::{Hmac, Mac};

// If Mac trait moved to digest crate:
use hmac::Hmac;
use digest::Mac;
// Or if hmac 0.13 still re-exports Mac:
use hmac::{Hmac, Mac};
```

**Call site 2 — type alias (line 37):**

```rust
// Current:
type HmacSha256 = Hmac<Sha256>;

// Likely unchanged — Hmac<T> generic still exists in 0.13.
```

**Call site 3 — webhook_signature (lines 1648-1649):**

```rust
// Current:
let mut mac =
    HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");

// If new_from_slice is renamed to new_from_key:
let mut mac =
    HmacSha256::new_from_key(secret.as_bytes().to_vec()).expect("HMAC accepts keys of any length");
// Or if it accepts &[u8] directly:
let mut mac =
    HmacSha256::new_from_key(secret.as_bytes()).expect("HMAC accepts keys of any length");
```

**Step 1: Make the changes**

Apply fixes based on compiler errors from Task 7.

**Step 2: Verify**

Run: `cargo check -p antminer-fleet-server`
Expected: PASS

### Task 9: Full workspace verification for hmac migration

**Step 1: Workspace check**

Run: `cargo check --workspace --locked`
Expected: PASS

**Step 2: Workspace tests**

Run: `cargo test --workspace --locked`
Expected: PASS — all tests pass, including `webhook_signature_is_hmac_sha256_hex`

**Step 3: Audit**

Run: `cargo audit`
Expected: 0 vulnerabilities

**Step 4: Format check**

Run: `cargo fmt --all -- --check`
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock server/src/api.rs
git commit -m "chore: migrate hmac 0.12 → 0.13"
```

---

## Phase 4: Final Rust Verification

### Task 10: Full workspace gate

**Objective:** Confirm all Rust changes are clean together.

**Step 1: Clean check**

Run: `cargo check --workspace --locked`
Expected: PASS

**Step 2: Full test suite**

Run: `cargo test --workspace --locked`
Expected: PASS

**Step 3: Format**

Run: `cargo fmt --all -- --check`
Expected: PASS

**Step 4: Audit**

Run: `cargo audit`
Expected: 0 vulnerabilities

**Step 5: Clippy**

Run: `cargo clippy --workspace --locked -- -D warnings`
Expected: PASS (or only pre-existing warnings)

**Step 6: git diff check**

Run: `git diff --check`
Expected: no whitespace errors

---

## Phase 5: Frontend Package Updates (Handoff to Frontend Agent)

This section is a brief for the frontend agent. The Rust agent does not execute these steps.

### Task 11: npm semver-compatible updates

**Objective:** Update 4 packages to latest compatible versions.

**Step 1: Apply updates**

Run: `npm update`
Expected: Updates @tanstack/react-query, @vitejs/plugin-react, autoprefixer, vite to latest

**Step 2: Verify build**

Run: `npm run build`
Expected: PASS

**Step 3: Verify tests**

Run: `npm test`
Expected: PASS

**Step 4: Commit**

```bash
git add package.json package-lock.json
git commit -m "chore: update npm dependencies to latest semver-compatible versions"
```

### Task 12: ESLint 10 adoption

**Objective:** Reconcile package.json, lockfile, and node_modules on eslint 10.

**Context:**
- `node_modules` already has eslint 10.5.0 and @eslint/js 10.0.1 installed.
- `npx eslint src/` passes clean (exit 0) with eslint 10.5.0.
- `eslint.config.js` is a flat config using `@eslint/js` recommended + `typescript-eslint` recommended.
- `eslint-plugin-react` 7.37.5 (latest) has peerDep `eslint: ^9.7` max — does NOT support eslint 10.
  It is declared in devDependencies but NOT imported in `eslint.config.js`. It should be removed.
- `eslint-plugin-react-hooks` 7.1.1 peerDep includes `^10.0.0` — compatible.
- `typescript-eslint` 8.62.0 peerDep includes `^10.0.0` — compatible.

**Step 1: Remove unused eslint-plugin-react**

Edit `package.json` devDependencies: remove the `eslint-plugin-react` line.

**Step 2: Bump eslint and @eslint/js ranges**

Edit `package.json` devDependencies:

```json
// Before:
"eslint": "^9.39.4",
"@eslint/js": "^9.39.4",

// After:
"eslint": "^10",
"@eslint/js": "^10",
```

**Step 3: Reinstall from updated package.json**

Run: `npm install`
Expected: lockfile updated, eslint resolves to 10.x, no peer dep errors

**Step 4: Verify lint passes**

Run: `npx eslint src/`
Expected: PASS — exit 0, zero warnings

**Step 5: Verify build**

Run: `npm run build`
Expected: PASS

**Step 6: Verify tests**

Run: `npm test`
Expected: PASS

**Step 7: Audit**

Run: `npm audit`
Expected: 0 vulnerabilities

**Step 8: Commit**

```bash
git add package.json package-lock.json
git commit -m "chore: adopt eslint 10, remove unused eslint-plugin-react"
```

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| sha2 0.11 API breakage on `format!("{:x}", ...)` | Medium | Fallback to explicit `.iter().map()` pattern (already used in other call sites) |
| hmac 0.13 `new_from_slice` rename | Low | Compiler will flag it; rename to `new_from_key` |
| digest 0.11 trait import path changes | Low | Compiler will flag it; adjust imports |
| Transitive crate version conflicts (digest 0.10 + 0.11 coexist) | Low | argon2, ssh-key still use digest 0.10; Cargo handles duplicate major versions fine |
| eslint-plugin-react removal breaks something | Very Low | It's not imported in eslint.config.js — verified |
| vite 8.0.16 → 8.1.0 minor breakage | Low | Frontend agent runs `npm run build` + `npm test` to verify |

## Out of Scope

- Bumping transitive-only Rust crates beyond what `cargo update` provides (e.g. axum 0.8 → 0.9 if released)
- Bumping direct Rust deps with no available update (argon2, axum, sqlx, etc.)
- Upgrading Node.js runtime version
- Upgrading PostgreSQL
- Any Tauri framework version bump (tauri = "2" — would be a separate task)