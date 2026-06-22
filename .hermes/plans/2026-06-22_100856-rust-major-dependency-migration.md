# Rust Major Dependency Migration Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Upgrade the remaining Rust dependency lines (`keyring 3→4`, `reqwest 0.12→0.13`, `sqlx 0.8→0.9`, `rand 0.9→0.10`, `tower-http 0.6→0.7`, `toml 0.9→1.1`) without breaking server API behavior, Tauri credential storage, TLS certificate pinning, pairing, SSH tunnel onboarding, migrations, or frontend runtime flows.

**Architecture:** Split the work into backend and frontend/Tauri tracks. Backend owns server dependency migrations (`server/Cargo.toml`, SQLx/database behavior, request/HTTP server middleware, config TOML parsing, random token/password/session generation). Frontend owns the desktop Rust layer (`src-tauri/Cargo.toml`, credential storage via `keyring`, pinned HTTPS client behavior via `reqwest`, Tauri command behavior) plus TypeScript-level regression tests that prove the UI still calls the same commands.

**Tech Stack:** Rust workspace (`cargo` stable MSVC 1.96+), Tauri v2, Axum, SQLx/PostgreSQL, Rustls, Reqwest, Keyring, React/TypeScript, Vitest, npm, PostgreSQL-backed server config under `server/config/server.local.toml`.

---

## Current Context / Assumptions

- Repo: `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex`
- Branch at plan time: `master`, clean, synced with `origin/master`.
- Latest commit at plan time: `f454aa4 chore: update dependency lockfile`.
- Current relevant manifests:
  - `server/Cargo.toml`
    - `rand = "0.9"`
    - `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }`
    - `sqlx = { version = "0.8", features = ["chrono", "postgres", "runtime-tokio", "sqlite", "tls-rustls"] }`
    - `toml = "0.9"`
    - `tower-http = { version = "0.6", features = ["limit", "trace"] }`
  - `src-tauri/Cargo.toml`
    - `keyring = { version = "3", features = ["windows-native", "apple-native", "sync-secret-service"] }`
    - `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }`
- `crates/fleet-shared/Cargo.toml` does not directly depend on these migration targets.
- Prior local workspace-wide `cargo check --workspace --locked` has shown a Windows/MSVC local proc-macro DLL/linker issue around `ctor_proc_macro`. Use targeted checks first. If workspace checks still fail for the same local linker issue, document it separately and validate server/shared/Tauri packages with isolated `CARGO_TARGET_DIR` values.
- This plan intentionally does **not** change public API contracts, the `/api/v1` prefix, TLS pinning semantics, bearer auth/session semantics, or SSH tunnel topology.

---

# Backend Migration Track

The backend agent owns this section. Keep changes backend-focused unless a shared contract forces frontend coordination.

## Backend Acceptance Criteria

When backend work is complete:

- `server/Cargo.toml` uses:
  - `rand = "0.10"`
  - `reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls"] }`
  - `sqlx = { version = "0.9", features = ["chrono", "postgres", "runtime-tokio", "sqlite", "tls-rustls"] }`
  - `toml = "1.1"`
  - `tower-http = { version = "0.7", features = ["limit", "trace"] }`
- `Cargo.lock` resolves cleanly with those versions.
- Server compile/tests pass with `--locked`.
- Config validation, migrations, admin creation, login/session, audit logging, miner/part CRUD, dashboard, tunnel-key approval/revocation, `/health`, and `/pairing` behavior still work.
- No production local SQLite ownership is introduced. Existing SQLite feature usage for tooling/tests/import compatibility must remain bounded to existing behavior.
- No certificate trust bypass is introduced.

---

## Backend Task 1: Create a backend migration branch and baseline report

**Objective:** Start from a clean, current branch and capture the exact pre-migration state.

**Files:**
- Read: `server/Cargo.toml`
- Read: `Cargo.lock`
- Create: `.codex/reports/backend-rust-major-deps-baseline.md`

**Step 1: Verify clean state**

Run:

```bash
git checkout master
git pull --ff-only origin master
git status --short --branch
```

Expected:

```text
## master...origin/master
```

**Step 2: Create branch**

Run:

```bash
git checkout -b chore/backend-rust-major-deps
```

Expected: branch switch succeeds.

**Step 3: Capture baseline commands**

Run:

```bash
cargo tree -p antminer-fleet-server --locked > .codex/reports/backend-rust-major-deps-tree-before.txt
cargo update --dry-run --workspace --verbose > .codex/reports/backend-rust-major-deps-dry-run-before.txt 2>&1
```

**Step 4: Write baseline markdown**

Create `.codex/reports/backend-rust-major-deps-baseline.md`:

```markdown
# Backend Rust Major Dependency Migration Baseline

## Scope

Backend migration for:
- rand 0.9 -> 0.10
- reqwest 0.12 -> 0.13
- sqlx 0.8 -> 0.9
- toml 0.9 -> 1.1
- tower-http 0.6 -> 0.7

## Pre-migration manifest

See `server/Cargo.toml`.

## Captured artifacts

- `.codex/reports/backend-rust-major-deps-tree-before.txt`
- `.codex/reports/backend-rust-major-deps-dry-run-before.txt`

## Known validation caveat

If workspace-wide Cargo validation fails on Windows with the known local MSVC/proc-macro DLL issue, use isolated `CARGO_TARGET_DIR` validation and document the exact failure separately. Do not hide real compile or test failures.
```

**Step 5: Commit baseline**

Run:

```bash
git add .codex/reports/backend-rust-major-deps-baseline.md .codex/reports/backend-rust-major-deps-tree-before.txt .codex/reports/backend-rust-major-deps-dry-run-before.txt
git commit -m "docs: capture backend rust dependency baseline"
```

---

## Backend Task 2: Update backend Cargo dependency versions

**Objective:** Change only the backend manifest dependency version requirements for the scoped crates.

**Files:**
- Modify: `server/Cargo.toml:13-26`
- Modify: `Cargo.lock`

**Step 1: Edit `server/Cargo.toml`**

Change the dependency block to this exact target for scoped crates:

```toml
rand = "0.10"
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls"] }
sqlx = { version = "0.9", features = ["chrono", "postgres", "runtime-tokio", "sqlite", "tls-rustls"] }
toml = "1.1"
tower-http = { version = "0.7", features = ["limit", "trace"] }
```

Leave all unrelated dependencies unchanged.

**Step 2: Resolve lockfile**

Run:

```bash
cargo update -p rand -p reqwest -p sqlx -p toml -p tower-http
```

If Cargo rejects multiple `-p` flags on this host, run individually:

```bash
cargo update -p rand
cargo update -p reqwest
cargo update -p sqlx
cargo update -p toml
cargo update -p tower-http
```

**Step 3: Inspect lockfile diff**

Run:

```bash
git diff -- server/Cargo.toml Cargo.lock
```

Expected:
- `server/Cargo.toml` only changes scoped version lines.
- `Cargo.lock` changes transitive Rust crates only.
- No app source code changes yet.

**Step 4: Commit manifest update**

Run:

```bash
git add server/Cargo.toml Cargo.lock
git commit -m "chore(server): bump rust dependency requirements"
```

---

## Backend Task 3: Compile and catalog migration errors

**Objective:** Identify actual API migration errors before touching code.

**Files:**
- Create: `.codex/reports/backend-rust-major-deps-compile-errors.md`

**Step 1: Run targeted backend check**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p antminer-fleet-server --locked
```

Expected:
- Either PASS, or compiler errors from API changes.

**Step 2: If it fails, capture errors**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p antminer-fleet-server --locked > .codex/reports/backend-rust-major-deps-compile-errors.txt 2>&1
```

Create `.codex/reports/backend-rust-major-deps-compile-errors.md`:

```markdown
# Backend Rust Major Dependency Compile Errors

## Command

`CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p antminer-fleet-server --locked`

## Summary

- [ ] rand API changes
- [ ] reqwest API changes
- [ ] sqlx API changes
- [ ] toml API changes
- [ ] tower-http API changes
- [ ] transitive/toolchain issue

## Raw output

See `.codex/reports/backend-rust-major-deps-compile-errors.txt`.
```

**Step 3: Commit compile report if failures exist**

Run only if a report was created:

```bash
git add .codex/reports/backend-rust-major-deps-compile-errors.md .codex/reports/backend-rust-major-deps-compile-errors.txt
git commit -m "docs(server): catalog rust dependency migration errors"
```

---

## Backend Task 4: Migrate `rand 0.9 -> 0.10` backend usage

**Objective:** Update random generation code while preserving password/session/token entropy and behavior.

**Files:**
- Inspect: `server/src/**/*.rs`
- Likely modify: files found by search for `rand::`, `thread_rng`, `rng`, `random`, `Alphanumeric`, `Rng`, `OsRng`
- Tests: existing backend unit/integration tests around auth/session/tunnel keys

**Step 1: Locate usages**

Run:

```bash
rg -n "rand::|thread_rng|OsRng|Alphanumeric|random\(|rng\(" server/src server/tests crates/fleet-shared/src
```

**Step 2: Read each usage**

For every hit, inspect the surrounding function. Pay special attention to:
- session token generation
- password salt/hash randomness
- tunnel key/request IDs if applicable
- rate-limit identifiers if random-backed

**Step 3: Apply minimal API changes**

Common expected migration patterns, depending on actual compiler errors:

```rust
// Old-ish pattern examples — do not apply blindly
let mut rng = rand::thread_rng();
```

Potential replacement if required by `rand 0.10`:

```rust
let mut rng = rand::rng();
```

For distribution imports, follow compiler guidance from `rand 0.10` docs and errors. Preserve cryptographic randomness. If any existing code uses non-crypto RNG for secrets, fix it here and add a test/report note.

**Step 4: Run targeted tests**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo test -p antminer-fleet-server auth::tests --locked
```

Expected: auth/session tests pass.

**Step 5: Commit rand migration**

Run:

```bash
git add server crates Cargo.lock
git commit -m "fix(server): migrate rand usage"
```

If no source changes were needed, skip commit and note that manifest/lockfile was sufficient.

---

## Backend Task 5: Migrate `reqwest 0.12 -> 0.13` backend usage

**Objective:** Preserve backend outbound HTTPS behavior, especially any tunnel/pairing/admin helper HTTP clients.

**Files:**
- Inspect: `server/src/**/*.rs`
- Likely modify: files found by `reqwest::`, `Client`, `ClientBuilder`, `.json(`, `.send(`, `.error_for_status(`
- Tests: server tests covering tunnel key scripts/config and any HTTP client helpers

**Step 1: Locate usages**

Run:

```bash
rg -n "reqwest::|ClientBuilder|Client::builder|\.send\(|\.json\(|error_for_status" server/src server/tests
```

**Step 2: Compile to see exact API errors**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p antminer-fleet-server --locked
```

**Step 3: Fix only compiler-proven API changes**

Rules:
- Keep `default-features = false`.
- Keep `rustls-tls`.
- Do **not** add native TLS.
- Do **not** add invalid certificate acceptance to backend production clients.
- Do **not** change `/health` or `/pairing` authentication behavior.

**Step 4: Test**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo test -p antminer-fleet-server --locked
```

Expected: all backend tests pass.

**Step 5: Commit reqwest migration**

Run:

```bash
git add server Cargo.lock
git commit -m "fix(server): migrate reqwest usage"
```

Skip if no source changes were needed.

---

## Backend Task 6: Migrate `sqlx 0.8 -> 0.9` backend database code

**Objective:** Preserve PostgreSQL production behavior, migrations, connection pooling, import behavior, and tests.

**Files:**
- Inspect: `server/src/**/*.rs`
- Inspect: `server/migrations/*.sql`
- Likely modify: SQLx imports/types/query callsites found by search
- Tests: `server/tests/*.rs`

**Step 1: Locate SQLx usage**

Run:

```bash
rg -n "sqlx::|PgPool|Pool<Postgres>|query!|query_as!|query\(|query_as\(|migrate!|Executor|Acquire|Transaction" server/src server/tests
```

**Step 2: Compile to see exact API errors**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p antminer-fleet-server --locked
```

**Step 3: Apply SQLx migration fixes**

Likely areas to validate/fix:
- connection pool options / connect calls
- transaction executor trait changes
- `Executor` bounds
- migration runner call shape
- `query_as` mapping strictness
- chrono feature behavior
- SQLite feature retained only where existing code needs it

Do **not** rewrite queries or migrations unless SQLx 0.9 requires it. Preserve schema and API response shapes.

**Step 4: Run database-independent tests**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo test -p antminer-fleet-server --locked
```

Expected: unit/integration tests pass.

**Step 5: Run local PostgreSQL validation if available**

Prerequisite: `server/config/server.local.toml` exists and points to local PostgreSQL.

Run:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml validate-config
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml migrate
```

Expected:
- config validates
- migrations complete without SQLx/runtime errors

**Step 6: Runtime smoke test server if local config exists**

Start server in background terminal/session:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml serve
```

From another shell:

```bash
curl -k https://127.0.0.1:8443/health
curl -k https://127.0.0.1:8443/pairing
```

Expected:
- `/health` returns healthy JSON/status.
- `/pairing` returns pairing/certificate metadata without bearer auth.

Stop server after smoke test.

**Step 7: Commit SQLx migration**

Run:

```bash
git add server Cargo.lock
git commit -m "fix(server): migrate sqlx usage"
```

Skip if no source changes were needed.

---

## Backend Task 7: Migrate `toml 0.9 -> 1.1` backend config parsing

**Objective:** Preserve server config file parsing, validation diagnostics, and local config workflows.

**Files:**
- Inspect: `server/src/config*.rs`, or actual config files found by search
- Inspect: `server/config/server.example.toml`
- Tests: `server/tests/config_cli.rs`, `server/src/config.rs` tests if present

**Step 1: Locate TOML usage**

Run:

```bash
rg -n "toml::|from_str|to_string|to_string_pretty|Value|de::Error" server/src server/tests
```

**Step 2: Compile and fix exact errors**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p antminer-fleet-server --locked
```

Apply only required fixes. Preserve error messages where tests assert them.

**Step 3: Run config tests**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo test -p antminer-fleet-server config --locked
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo test -p antminer-fleet-server --test config_cli --locked
```

Expected: all config tests pass.

**Step 4: Validate example config still parses**

If a local config exists:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml validate-config
```

Expected: passes.

**Step 5: Commit TOML migration**

Run:

```bash
git add server Cargo.lock
git commit -m "fix(server): migrate toml config parsing"
```

Skip if no source changes were needed.

---

## Backend Task 8: Migrate `tower-http 0.6 -> 0.7` middleware

**Objective:** Preserve request size limits, tracing, and middleware behavior.

**Files:**
- Inspect: server routing/bootstrap files found by `tower_http`
- Likely modify: `server/src/api*.rs`, `server/src/main.rs`, or router construction module
- Tests: existing API/rate-limit/status tests

**Step 1: Locate middleware usage**

Run:

```bash
rg -n "tower_http|TraceLayer|RequestBodyLimitLayer|DefaultMakeSpan|DefaultOnResponse|limit|trace" server/src server/tests
```

**Step 2: Compile and fix API errors**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p antminer-fleet-server --locked
```

Rules:
- Preserve existing body size limits.
- Preserve tracing/logging level and span behavior as closely as possible.
- Do not remove middleware just to make compile pass. That's dumb and will bite runtime diagnostics.

**Step 3: Run API tests**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo test -p antminer-fleet-server api::tests --locked
```

Expected: API/rate-limit tests pass.

**Step 4: Commit tower-http migration**

Run:

```bash
git add server Cargo.lock
git commit -m "fix(server): migrate tower-http middleware"
```

Skip if no source changes were needed.

---

## Backend Task 9: Full backend validation matrix

**Objective:** Prove backend behavior after dependency migration.

**Files:**
- Create: `.codex/reports/backend-rust-major-deps-validation.md`

**Step 1: Formatting**

Run:

```bash
cargo fmt --all -- --check
```

Expected: pass.

**Step 2: Targeted compile**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p antminer-fleet-server --locked
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo check -p fleet-shared --locked
```

Expected: pass.

**Step 3: Backend/shared tests**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-backend-major-deps-target' cargo test -p fleet-shared -p antminer-fleet-server --locked
```

Expected: all tests pass.

**Step 4: Optional full workspace check**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-workspace-major-deps-target' cargo check --workspace --locked
```

Expected: pass. If it fails with the known local Windows/MSVC proc-macro DLL issue, capture exact output and do not mislabel it as a code failure.

**Step 5: Runtime validation with local PostgreSQL config**

If local PostgreSQL config exists:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml validate-config
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml migrate
```

Then start server and run:

```bash
curl -k https://127.0.0.1:8443/health
curl -k https://127.0.0.1:8443/pairing
```

Expected: both unauthenticated endpoints still reachable.

**Step 6: Write validation report**

Create `.codex/reports/backend-rust-major-deps-validation.md`:

```markdown
# Backend Rust Major Dependency Migration Validation

## Dependency targets

- rand 0.10
- reqwest 0.13
- sqlx 0.9
- toml 1.1
- tower-http 0.7

## Commands run

| Command | Result | Notes |
|---|---:|---|
| `cargo fmt --all -- --check` | PASS/FAIL | |
| `cargo check -p antminer-fleet-server --locked` | PASS/FAIL | |
| `cargo check -p fleet-shared --locked` | PASS/FAIL | |
| `cargo test -p fleet-shared -p antminer-fleet-server --locked` | PASS/FAIL | |
| `cargo check --workspace --locked` | PASS/FAIL/SKIPPED | Document Windows linker issue if hit |
| `validate-config` | PASS/FAIL/SKIPPED | |
| `migrate` | PASS/FAIL/SKIPPED | |
| `/health` smoke | PASS/FAIL/SKIPPED | |
| `/pairing` smoke | PASS/FAIL/SKIPPED | |

## Source changes

Summarize exact files changed and why.

## Runtime gaps

List anything not tested against live PostgreSQL/TLS/tunnel runtime.
```

**Step 7: Commit validation report**

Run:

```bash
git add .codex/reports/backend-rust-major-deps-validation.md
git commit -m "docs(server): record rust dependency migration validation"
```

---

## Backend Task 10: Backend handoff for frontend agent

**Objective:** Provide the frontend agent enough information to update Tauri safely.

**Files:**
- Create: `.codex/reports/backend-to-frontend-rust-major-deps-handoff.md`

**Step 1: Write handoff**

Create `.codex/reports/backend-to-frontend-rust-major-deps-handoff.md`:

```markdown
# Backend to Frontend Handoff: Rust Major Dependency Migration

## Backend completed

- [ ] rand 0.10
- [ ] reqwest 0.13
- [ ] sqlx 0.9
- [ ] toml 1.1
- [ ] tower-http 0.7

## Backend validation status

Paste results from `.codex/reports/backend-rust-major-deps-validation.md`.

## Shared contracts changed?

Expected: No.

If yes, list exact fields/types/endpoints changed. This should normally remain empty.

## Frontend/Tauri impact

- Tauri Rust layer still needs `keyring 4` migration.
- Tauri Rust layer shares `reqwest 0.13`; verify pinned cert HTTPS client code and one-shot no-auth request code.
- TypeScript frontend should not need API contract changes unless backend report says otherwise.

## Runtime notes

Document any server URL, TLS, pairing, or tunnel behavior that changed. Expected: none.
```

**Step 2: Commit handoff**

Run:

```bash
git add .codex/reports/backend-to-frontend-rust-major-deps-handoff.md
git commit -m "docs: add backend dependency migration handoff"
```

---

# Frontend / Tauri Migration Track

The frontend agent owns this section after backend has either completed or provided a handoff branch. This track covers the desktop Rust layer and TypeScript UI tests.

## Frontend Acceptance Criteria

When frontend work is complete:

- `src-tauri/Cargo.toml` uses:
  - `keyring = { version = "4", features = ["windows-native", "apple-native", "sync-secret-service"] }`
  - `reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls"] }`
- If backend did not already upgrade workspace-level lockfile dependencies, `Cargo.lock` resolves cleanly.
- Tauri command behavior remains unchanged from the TypeScript frontend's point of view.
- Credential storage still uses OS credential manager. No plaintext bearer token storage.
- TLS pinning behavior remains exact-leaf-certificate pinning. No normal CA fallback, no global insecure bypass.
- Pairing and `/health` remain unauthenticated.
- SSH tunnel onboarding still generates/saves/starts tunnel config as before.
- `npm test`, `npm run build`, and relevant Tauri/Rust validation pass.

---

## Frontend Task 1: Create frontend migration branch and baseline report

**Objective:** Start frontend migration from backend-completed branch or clean `master` and capture baseline.

**Files:**
- Read: `src-tauri/Cargo.toml`
- Read: `src-tauri/src/**/*.rs`
- Create: `.codex/reports/frontend-rust-major-deps-baseline.md`

**Step 1: Start from correct branch**

If backend migration was merged to `master`:

```bash
git checkout master
git pull --ff-only origin master
git checkout -b chore/frontend-tauri-major-deps
```

If backend migration is on a handoff branch:

```bash
git fetch origin
git checkout <backend-handoff-branch>
git checkout -b chore/frontend-tauri-major-deps
```

**Step 2: Capture baseline**

Run:

```bash
cargo tree -p antminer-fleet-manager --locked > .codex/reports/frontend-rust-major-deps-tree-before.txt
rg -n "keyring::|Entry::|Credential|reqwest::|ClientBuilder|danger_accept_invalid_certs|certificate|cert|pinned|post_no_auth|one_shot" src-tauri/src > .codex/reports/frontend-rust-major-deps-usage-before.txt
```

**Step 3: Write baseline report**

Create `.codex/reports/frontend-rust-major-deps-baseline.md`:

```markdown
# Frontend/Tauri Rust Major Dependency Migration Baseline

## Scope

Frontend/Tauri migration for:
- keyring 3 -> 4
- reqwest 0.12 -> 0.13

## Security-sensitive areas

- OS credential storage for bearer/session token
- pinned TLS certificate handling
- pairing and `/health` verification
- one-shot tunnel key submission before saved server config
- SSH tunnel key/config storage under local app data

## Captured artifacts

- `.codex/reports/frontend-rust-major-deps-tree-before.txt`
- `.codex/reports/frontend-rust-major-deps-usage-before.txt`
```

**Step 4: Commit baseline**

Run:

```bash
git add .codex/reports/frontend-rust-major-deps-baseline.md .codex/reports/frontend-rust-major-deps-tree-before.txt .codex/reports/frontend-rust-major-deps-usage-before.txt
git commit -m "docs: capture frontend rust dependency baseline"
```

---

## Frontend Task 2: Update Tauri dependency versions

**Objective:** Change only Tauri manifest dependency requirements for scoped crates.

**Files:**
- Modify: `src-tauri/Cargo.toml:18-19`
- Modify: `Cargo.lock`

**Step 1: Edit `src-tauri/Cargo.toml`**

Change:

```toml
keyring = { version = "3", features = ["windows-native", "apple-native", "sync-secret-service"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
```

to:

```toml
keyring = { version = "4", features = ["windows-native", "apple-native", "sync-secret-service"] }
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls"] }
```

**Step 2: Resolve lockfile**

Run:

```bash
cargo update -p keyring -p reqwest
```

If Cargo rejects combined package flags:

```bash
cargo update -p keyring
cargo update -p reqwest
```

**Step 3: Inspect diff**

Run:

```bash
git diff -- src-tauri/Cargo.toml Cargo.lock
```

Expected:
- Only scoped manifest lines changed in `src-tauri/Cargo.toml`.
- Lockfile changes are dependency resolution only.

**Step 4: Commit manifest update**

Run:

```bash
git add src-tauri/Cargo.toml Cargo.lock
git commit -m "chore(tauri): bump rust dependency requirements"
```

---

## Frontend Task 3: Compile and catalog Tauri migration errors

**Objective:** Let the compiler identify real `keyring`/`reqwest` API changes before editing code.

**Files:**
- Create: `.codex/reports/frontend-rust-major-deps-compile-errors.md`

**Step 1: Ensure Tauri icon exists for Rust/Tauri tests**

Run:

```bash
test -f src-tauri/icons/icon.png || cp src-tauri/icons/128x128.png src-tauri/icons/icon.png
```

**Step 2: Run Tauri compile check**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-frontend-major-deps-target' cargo check -p antminer-fleet-manager --locked
```

Expected:
- Either PASS, or compiler errors around `keyring`/`reqwest` APIs.

**Step 3: Capture failures if any**

Run only if check fails:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-frontend-major-deps-target' cargo check -p antminer-fleet-manager --locked > .codex/reports/frontend-rust-major-deps-compile-errors.txt 2>&1
```

Create `.codex/reports/frontend-rust-major-deps-compile-errors.md`:

```markdown
# Frontend/Tauri Rust Major Dependency Compile Errors

## Command

`CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-frontend-major-deps-target' cargo check -p antminer-fleet-manager --locked`

## Summary

- [ ] keyring API changes
- [ ] reqwest API changes
- [ ] transitive/toolchain issue

## Raw output

See `.codex/reports/frontend-rust-major-deps-compile-errors.txt`.
```

**Step 4: Commit compile report if failures exist**

Run:

```bash
git add .codex/reports/frontend-rust-major-deps-compile-errors.md .codex/reports/frontend-rust-major-deps-compile-errors.txt
git commit -m "docs(tauri): catalog rust dependency migration errors"
```

---

## Frontend Task 4: Migrate `keyring 3 -> 4` credential storage

**Objective:** Preserve OS-native credential storage and token lifecycle with the new `keyring` API.

**Files:**
- Inspect/modify: files found by `keyring::`, `Entry::new`, `.set_password`, `.get_password`, `.delete_credential`, `.delete_password`
- Likely path: `src-tauri/src/client.rs` or credential/session storage module
- Tests: existing Tauri/Rust tests if present; otherwise add small focused tests around any wrapper logic that does not hit OS keyring directly

**Step 1: Locate keyring usage**

Run:

```bash
rg -n "keyring::|Entry::new|set_password|get_password|delete_password|delete_credential|credential" src-tauri/src src-tauri/tests
```

**Step 2: Read the credential wrapper code**

For each hit, inspect surrounding functions. Identify exact semantics:
- service name
- account/key name
- save token
- load token
- delete token/logout
- error handling when credential missing

**Step 3: Apply minimal API migration**

Follow compiler errors and `keyring 4` docs. Preserve this behavior:
- Missing credential is not a crash during unauthenticated startup.
- Stored bearer token is not logged or printed.
- Logout/delete removes credential.
- Credential errors surface as user-safe messages.

If `Entry::new(...)` changed from returning `Entry` to `Result<Entry, Error>` or vice versa, handle it explicitly. Example pattern if `Entry::new` returns `Result`:

```rust
let entry = keyring::Entry::new(SERVICE_NAME, ACCOUNT_NAME)
    .map_err(|error| ClientError::CredentialStore(error.to_string()))?;
```

Do not use plaintext files as fallback. If OS credential storage fails, return a clear error.

**Step 4: Add/adjust wrapper tests where possible**

If existing credential code is wrapped behind helper functions, add tests for non-OS logic only. Avoid tests that require real Windows Credential Manager in CI unless the project already does that.

Possible test file:
- `src-tauri/src/client.rs` module tests, if testable
- or no new test if the API directly hits OS keyring and existing compile/runtime validation is the only sane coverage

**Step 5: Run Tauri check**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-frontend-major-deps-target' cargo check -p antminer-fleet-manager --locked
```

Expected: keyring-related compile errors resolved.

**Step 6: Commit keyring migration**

Run:

```bash
git add src-tauri Cargo.lock
git commit -m "fix(tauri): migrate keyring credential storage"
```

Skip if no source changes were needed.

---

## Frontend Task 5: Migrate `reqwest 0.12 -> 0.13` Tauri HTTP clients

**Objective:** Preserve pinned-cert HTTPS behavior, pairing flow, no-auth `/health`, and one-shot tunnel key submission.

**Files:**
- Inspect/modify: `src-tauri/src/client.rs` and any file found by `reqwest::`
- Tests: Rust client tests if present; TypeScript `src/test/ConnectionGate.test.tsx`, `src/test/connectionApi.test.ts`, related mocks

**Step 1: Locate reqwest usage**

Run:

```bash
rg -n "reqwest::|Client::builder|ClientBuilder|Certificate|Identity|danger_accept_invalid_certs|https_only|add_root_certificate|resolve|connect_timeout|timeout|\.send\(|\.json\(" src-tauri/src src-tauri/tests
```

**Step 2: Read certificate pinning and one-shot request code**

Specifically verify these behaviors before editing:
- server leaf certificate pinning remains exact fingerprint/certificate based
- `/pairing` and `/health` remain available without bearer token
- `post_no_auth_to_url` / one-shot tunnel key submission still works before saved config
- normal authenticated API calls still require saved config + bearer token

**Step 3: Compile and fix exact reqwest errors**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-frontend-major-deps-target' cargo check -p antminer-fleet-manager --locked
```

Rules:
- Keep `default-features = false` and `rustls-tls`.
- Do **not** add native-tls.
- Do **not** replace pinned certificate trust with CA-only trust.
- Do **not** add a global insecure bypass.
- If one-shot pre-pairing code intentionally uses `danger_accept_invalid_certs(true)`, keep it narrowly scoped to that one-shot bootstrap path only and document why.

**Step 4: Add/adjust tests for frontend command contract**

If backend handoff says no API contract changes, TypeScript tests should need no contract change. Still run and inspect:

```bash
npm test -- src/test/connectionApi.test.ts src/test/ConnectionGate.test.tsx
```

Expected:
- existing connection/tunnel setup tests pass
- no mocked Tauri command names/argument shapes changed unless intentionally documented

**Step 5: Commit reqwest migration**

Run:

```bash
git add src-tauri src/test Cargo.lock
git commit -m "fix(tauri): migrate reqwest client usage"
```

Skip if no source changes were needed.

---

## Frontend Task 6: Frontend TypeScript/UI regression pass

**Objective:** Prove the UI still builds and calls unchanged Tauri command contracts.

**Files:**
- Inspect: `src/test/connectionApi.test.ts`
- Inspect: `src/test/ConnectionGate.test.tsx`
- Inspect: UI files only if tests fail

**Step 1: Run focused connection tests**

Run:

```bash
npm test -- src/test/connectionApi.test.ts src/test/ConnectionGate.test.tsx
```

Expected: pass.

**Step 2: Run full frontend test suite**

Run:

```bash
npm test
```

Expected: all tests pass.

**Step 3: Run frontend build**

Run:

```bash
npm run build
```

Expected: TypeScript + Vite production build passes.

**Step 4: Commit any test/UI adjustments**

Only if files changed:

```bash
git add src package.json package-lock.json
git commit -m "test: preserve connection flow after rust dependency migration"
```

If no files changed, skip commit.

---

## Frontend Task 7: Tauri runtime validation

**Objective:** Prove desktop app behavior, not just compile-time readiness.

**Files:**
- Create: `.codex/reports/frontend-rust-major-deps-runtime-validation.md`

**Step 1: Ensure icon exists**

Run:

```bash
test -f src-tauri/icons/icon.png || cp src-tauri/icons/128x128.png src-tauri/icons/icon.png
```

**Step 2: Run Tauri package check/build**

Run:

```bash
npm run tauri:build
```

Expected:
- Rust compile passes
- Tauri bundle generation passes
- installer lands under `target/release/bundle/nsis/`

If this fails due to local NSIS/tooling, capture exact output and run `cargo check -p antminer-fleet-manager --locked` plus `npm run build` as fallback. Do not claim installer validation passed if it did not.

**Step 3: Pairing/tunnel runtime smoke with local server**

Prerequisite: backend server running at expected local tunnel endpoint `https://127.0.0.1:8443` or local dev server config.

Run the app and verify manually or with available automation:
- first-run connection screen opens
- server URL accepts `https://127.0.0.1:8443`
- `/health` verification succeeds
- pairing fingerprint display still appears
- login stores bearer token in OS credential manager
- app restart loads saved server config/token or prompts login as expected
- logout/delete credential works
- generate tunnel key still creates local key under `%LOCALAPPDATA%\AntminerFleetManager\`
- tunnel config save still writes local app config only; no private key committed
- start tunnel still calls the intended helper path

**Step 4: Write runtime report**

Create `.codex/reports/frontend-rust-major-deps-runtime-validation.md`:

```markdown
# Frontend/Tauri Rust Major Dependency Runtime Validation

## Dependency targets

- keyring 4
- reqwest 0.13

## Automated validation

| Command | Result | Notes |
|---|---:|---|
| `cargo check -p antminer-fleet-manager --locked` | PASS/FAIL | |
| `npm test -- src/test/connectionApi.test.ts src/test/ConnectionGate.test.tsx` | PASS/FAIL | |
| `npm test` | PASS/FAIL | |
| `npm run build` | PASS/FAIL | |
| `npm run tauri:build` | PASS/FAIL/SKIPPED | |
| `git diff --check` | PASS/FAIL | |

## Manual/runtime validation

| Flow | Result | Notes |
|---|---:|---|
| First-run server URL entry | PASS/FAIL/SKIPPED | |
| `/health` pre-auth check | PASS/FAIL/SKIPPED | |
| Pairing fingerprint display | PASS/FAIL/SKIPPED | |
| Login stores token in OS credential manager | PASS/FAIL/SKIPPED | |
| Restart token load behavior | PASS/FAIL/SKIPPED | |
| Logout/delete credential | PASS/FAIL/SKIPPED | |
| Generate tunnel key | PASS/FAIL/SKIPPED | |
| Save tunnel config | PASS/FAIL/SKIPPED | |
| Start tunnel connection | PASS/FAIL/SKIPPED | |

## Runtime gaps

List any skipped live checks and why.
```

**Step 5: Commit runtime report**

Run:

```bash
git add .codex/reports/frontend-rust-major-deps-runtime-validation.md
git commit -m "docs(tauri): record dependency migration validation"
```

---

# Final Integration Track

This can be done by either agent after both backend and frontend tracks are complete.

## Integration Acceptance Criteria

- One branch contains both backend and frontend migrations.
- `Cargo.lock` has a single coherent resolution.
- No uncommitted files except intentionally ignored local artifacts.
- All validation commands are documented.
- App is buildable and installer is produced if local tooling supports it.
- GitHub Dependabot alert status is checked after push.

---

## Integration Task 1: Merge/rebase backend and frontend branches

**Objective:** Combine both tracks without losing validation reports.

**Step 1: Pick integration branch**

Run:

```bash
git checkout master
git pull --ff-only origin master
git checkout -b chore/rust-major-dependency-migration
```

**Step 2: Merge backend**

Run:

```bash
git merge --no-ff chore/backend-rust-major-deps
```

Resolve conflicts only in:
- `Cargo.lock`
- `.codex/reports/*`
- manifest files if both tracks touched them

**Step 3: Merge frontend**

Run:

```bash
git merge --no-ff chore/frontend-tauri-major-deps
```

**Step 4: If `Cargo.lock` conflicts, regenerate**

Run:

```bash
cargo update
cargo check -p antminer-fleet-server --locked
cargo check -p antminer-fleet-manager --locked
```

Then inspect:

```bash
git diff -- Cargo.lock
```

Do not hand-edit lockfile unless resolving conflict markers before regeneration.

---

## Integration Task 2: Full validation suite

**Objective:** Prove the repo is ready to ship.

**Step 1: Node validation**

Run:

```bash
npm ci
npm test
npm run build
npm audit --omit=dev
npm outdated --json
```

Expected:
- tests pass
- build passes
- audit shows 0 prod vulnerabilities
- `npm outdated --json` returns `{}` or only intentionally deferred packages documented

**Step 2: Rust formatting and checks**

Run:

```bash
cargo fmt --all -- --check
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-server-target' cargo check -p antminer-fleet-server --locked
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-tauri-target' cargo check -p antminer-fleet-manager --locked
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-server-target' cargo test -p fleet-shared -p antminer-fleet-server --locked
```

Expected: pass.

**Step 3: Full workspace check/test if local toolchain allows**

Run:

```bash
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-workspace-target' cargo check --workspace --locked
CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-integrated-workspace-target' cargo test --workspace --locked
```

Expected: pass. If the known Windows/MSVC proc-macro DLL issue appears, document exact output in final report and do not call workspace validation passed.

**Step 4: Tauri installer build**

Run:

```bash
test -f src-tauri/icons/icon.png || cp src-tauri/icons/128x128.png src-tauri/icons/icon.png
npm run tauri:build
```

Expected:
- installer generated under `target/release/bundle/nsis/`
- raw binary generated under `target/release/antminer-fleet-manager.exe`

**Step 5: Git checks**

Run:

```bash
git diff --check
git status --short
```

Expected:
- diff check passes
- only intended modified files before commit

---

## Integration Task 3: Live runtime smoke test

**Objective:** Catch the stuff compilers don't: TLS, pairing, credential storage, tunnel flow, DB connection behavior.

**Prerequisites:**
- Local PostgreSQL running with `server/config/server.local.toml`, or documented skip.
- Local Fleet server can bind `127.0.0.1:8443`.
- Desktop app can run on Eddie's Windows workstation.

**Step 1: Backend local runtime**

Run:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml validate-config
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml migrate
```

Start server:

```bash
cargo run -p antminer-fleet-server -- --config server/config/server.local.toml serve
```

Smoke endpoints:

```bash
curl -k https://127.0.0.1:8443/health
curl -k https://127.0.0.1:8443/pairing
```

Expected: both pass.

**Step 2: Desktop runtime smoke**

Run:

```bash
npm run tauri:dev
```

Verify:
- app launches
- pairing flow reaches server
- fingerprint confirmation works
- login works
- token persists via OS credential manager
- logout clears token
- dashboard loads through `/api/v1`
- miner list loads
- parts inventory loads
- tunnel setup screen still generates key and saves config

**Step 3: Tunnel runtime smoke if environment available**

Using local tunnel config only, never committed:

```bash
npm run tunnel:status
npm run tunnel:start
npm run tunnel:status
curl -k https://127.0.0.1:8443/health
npm run tunnel:stop
```

Expected:
- tunnel status/start/stop works
- `/health` reachable through local `127.0.0.1:8443`
- no host port `8443` exposed publicly

---

## Integration Task 4: Final report and commit

**Objective:** Leave a copy-paste-ready handoff with exact validation status.

**Files:**
- Create: `.codex/reports/rust-major-dependency-migration-final.md`

**Step 1: Write final report**

Create `.codex/reports/rust-major-dependency-migration-final.md`:

```markdown
# Rust Major Dependency Migration Final Report

## Summary

Migrated:
- keyring 3 -> 4
- reqwest 0.12 -> 0.13
- sqlx 0.8 -> 0.9
- rand 0.9 -> 0.10
- tower-http 0.6 -> 0.7
- toml 0.9 -> 1.1

## Changed files

List exact changed files.

## Backend validation

Paste table from backend report.

## Frontend/Tauri validation

Paste table from frontend report.

## Runtime validation

Paste live smoke results.

## Security checks

- [ ] No plaintext credentials added
- [ ] No SSH private keys committed
- [ ] No TLS pinning bypass introduced
- [ ] `/health` and `/pairing` remain unauthenticated
- [ ] API bearer auth still required for protected endpoints
- [ ] Tunnel topology still uses local `127.0.0.1:8443`

## Known gaps

List any skipped live checks and why.

## Commands run

Paste exact commands and PASS/FAIL.
```

**Step 2: Commit final integrated changes**

Run:

```bash
git add Cargo.toml Cargo.lock server/Cargo.toml src-tauri/Cargo.toml server src-tauri src .codex/reports
git commit -m "chore: migrate rust major dependencies"
```

If previous task commits already exist, this commit may only include final report.

---

## Integration Task 5: Push and monitor GitHub

**Objective:** Publish only after local validation is honest and complete.

**Step 1: Push branch**

If using PR workflow:

```bash
git push -u origin chore/rust-major-dependency-migration
```

Open PR:

```bash
gh pr create \
  --base master \
  --head chore/rust-major-dependency-migration \
  --title "chore: migrate rust major dependencies" \
  --body-file .codex/reports/rust-major-dependency-migration-final.md
```

If Eddie explicitly wants direct-to-master after validation:

```bash
git checkout master
git merge --ff-only chore/rust-major-dependency-migration
git push origin master
```

**Step 2: Check GitHub alerts/CI**

Run:

```bash
gh run list --branch chore/rust-major-dependency-migration --limit 5
```

If no GitHub Actions exist, check Dependabot alert manually:

```text
https://github.com/shingoku2/Work-App-Database-Codex/security/dependabot/3
```

Expected:
- prior moderate alert is resolved or explicitly identified as unrelated.

---

# Risks / Tradeoffs / Open Questions

## Risks

- `keyring 4` may change error types or credential entry construction, risking login persistence/logout behavior.
- `reqwest 0.13` may change TLS/client builder APIs; careless fixes could weaken cert pinning. Do not do that.
- `sqlx 0.9` may change transaction/executor behavior and query mapping. Compile success is not enough; run migrations and server tests.
- `tower-http 0.7` may change middleware layer construction; do not drop body limit/tracing layers just to compile.
- `rand 0.10` may alter RNG/distribution APIs. Secret generation must remain cryptographically sound.
- `toml 1.1` may alter parse/error behavior. Config diagnostics and validation tests must prove user-facing behavior remains sane.
- Existing Windows/MSVC local linker issue can muddy workspace validation. Use isolated target dirs and document exact failure if it recurs.

## Tradeoffs

- This plan favors small commits per migration target over one giant dependency commit. More commits, less mystery meat.
- It keeps runtime behavior unchanged instead of opportunistically refactoring config/client/database code. Boring is good here.
- It requires live validation for credential storage/TLS/tunnel flows because compile-time checks won't catch those failures.

## Open Questions

- Is local PostgreSQL configured on the machine running the backend migration? If not, backend runtime DB validation must be marked skipped and performed on a machine with `server/config/server.local.toml`.
- Does CI exist for this repo? `.github/` was not present in the repo during earlier checks, so GitHub Actions may be absent.
- Should the final integration go through PR review or direct push to `master`? Default should be PR unless Eddie explicitly says direct push.
- Is GitHub Dependabot alert #3 caused by one of these Rust crates, a dev-only npm package, or something outside local `npm audit --omit=dev`? Check after migration.

---

# Final Done Definition

This migration is done only when:

1. Backend dependency versions are updated and backend tests pass.
2. Frontend/Tauri dependency versions are updated and UI/Tauri tests/builds pass.
3. Runtime smoke checks confirm:
   - backend `/health` and `/pairing`
   - Tauri pairing/login/token storage/logout
   - SSH tunnel setup path where available
4. `npm audit --omit=dev` reports zero production vulnerabilities.
5. `git diff --check` passes.
6. Final report exists at `.codex/reports/rust-major-dependency-migration-final.md`.
7. Branch is pushed or master is updated per Eddie's instruction.
