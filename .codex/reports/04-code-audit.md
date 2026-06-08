# Code Audit Report

## Source Assessment
- Files/directories inspected: `server/src`, `server/migrations`, `server/packaging`, `crates/fleet-shared`, `src-tauri/src`, `src-tauri/capabilities`, connection/account/settings UI, miner/part API and UI code, tests, workspace manifests, lockfiles, and the deleted legacy SQLite migrations/database implementation from `HEAD`.
- Languages/frameworks detected: Rust, Axum, SQLx/PostgreSQL/SQLite, Rustls/Reqwest, Tauri 2, TypeScript, React, TanStack Query, Vitest.
- Commands run:
  - `git status --short` and targeted PowerShell file/search inspections - passed.
  - `git show HEAD:<legacy SQLite files>` - passed.
  - `cargo check --workspace --locked` - passed.
  - `cargo test --workspace --locked` - passed; four meaningful Rust unit tests, no server API/database or Tauri client integration tests.
  - `cargo fmt --all -- --check` - passed.
  - `cargo clippy --workspace --all-targets --locked -- -D warnings` - passed.
  - `npm test -- --run` - passed; 82 tests.
  - `npm run build` - passed.
  - Lockfile inspection confirmed WebPKI roots are present in the Reqwest TLS dependency path.
- Commands not run and why:
  - Live PostgreSQL/API integration tests were not run because no disposable PostgreSQL instance or test harness is provided.
  - Live TLS interception/certificate-rotation tests were not run because they require controlled network and certificate infrastructure.
  - Debian package/systemd validation was not repeated because this Windows host lacks the required Linux tooling.
- Earlier pipeline reports used: `.codex/reports/01-dependency-audit.md`, `.codex/reports/02-license-audit.md`, and `.codex/reports/03-env-validation.md`.

## Summary
The code compiles, lints, formats, and passes its current tests, but those tests do not exercise the new security boundary. The first authenticated client flow is broken after pairing, certificate trust is additive rather than pinned, login throttling permits targeted lockout and unbounded memory growth, final-admin protection is racy, and the SQLite `abort` import policy can overwrite concurrent data. The current implementation is not safe or functionally ready to ship.

## Findings

### [HIGH] First pairing and logout leave the login form permanently disabled
- Location: `src-tauri/src/client.rs:106`, `src-tauri/src/client.rs:128-137`, `src-tauri/src/client.rs:252-257`, `src/features/connection/ConnectionGate.tsx:110-130`
- Category: Authentication / reliability
- Issue: Pairing intentionally clears the credential. `connection_state()` immediately calls the authenticated `/auth/me` proxy, where a missing keyring entry becomes the plain string `authentication required`. The UI treats only errors prefixed with `AUTH_REQUIRED:` as sign-in states; every other error disables the Sign In button.
- Evidence: `pair()` calls `clear_token()`. A missing credential is generated locally before an HTTP response, so `response_error()` never adds `AUTH_REQUIRED:`. `LoginView` computes `unavailable` as any non-prefixed error and disables its submit button. Logout creates the same state.
- Risk: A normal user cannot sign in after first pairing or after signing out. This blocks the primary application workflow.
- Fix: Model connection/authentication states with typed errors rather than string prefixes. Treat a missing local credential as unauthenticated, not unavailable, and add an end-to-end test covering pair -> login -> logout -> login.
- Validation: Pair a clean client with an empty keyring and verify the login button remains enabled; repeat after logout.

### [HIGH] The alleged certificate pin still trusts the public WebPKI root set
- Location: `src-tauri/Cargo.toml:19`, `src-tauri/src/client.rs:310-317`, `Cargo.lock`
- Category: TLS / certificate trust
- Issue: The authenticated client adds the paired certificate as an extra root but does not disable Reqwest's built-in roots. The selected `rustls-tls` feature includes WebPKI roots.
- Evidence: `build_client()` calls only `.add_root_certificate(certificate)`. It does not call `.tls_built_in_root_certs(false)` or install an exact-certificate verifier. `Cargo.lock` contains the WebPKI root packages in the resolved TLS graph.
- Risk: After pairing, DNS/network redirection to another certificate valid for the same hostname and signed by any accepted public CA can be trusted. The displayed fingerprint therefore does not remain the sole trust decision.
- Fix: Disable built-in roots for this client and verify the exact expected leaf certificate or SPKI fingerprint during every TLS handshake. Define and test certificate-rotation behavior explicitly.
- Validation: Pair certificate A, then serve the same hostname with a different publicly trusted certificate B; every authenticated request must fail.

### [HIGH] Login throttling enables account lockout and unbounded memory consumption
- Location: `server/src/api.rs:29-35`, `server/src/api.rs:241-281`
- Category: Authentication / denial of service
- Issue: The limiter keys only on normalized username, records attempts before account lookup, and never removes entries for failed or nonexistent usernames after their timestamps expire.
- Evidence: Five requests for a known username block that username for a minute regardless of source address. Repeating them sustains the lockout. Unique invented usernames each create a permanent `HashMap` entry; `retain()` removes old timestamps only when that same username is tried again.
- Risk: An unauthenticated remote client can continuously prevent selected users, including all administrators, from logging in or can grow server memory until restart/exhaustion.
- Fix: Use a bounded, expiring limiter keyed by both source address and account, apply global limits, remove expired keys, and consider a shared store if multiple server instances are supported. Preserve constant-cost handling for nonexistent users.
- Validation: Load-test millions of unique usernames and repeated attacks against one valid account; memory must remain bounded and legitimate recovery must be possible.

### [HIGH] Concurrent admin updates can disable or demote every administrator
- Location: `server/src/api.rs:387-424`
- Category: Authorization / race condition
- Issue: The final-admin check reads the target role and counts enabled administrators outside a transaction, then performs the versioned update separately.
- Evidence: Two administrators can concurrently update different admin rows. Both requests can observe an enabled-admin count of two, both pass the check, and both updates can commit because they target different versioned rows.
- Risk: The system can be left with no enabled administrator, removing all in-app account administration and requiring server-side recovery.
- Fix: Enforce the invariant in one transaction with appropriate row/table locking or a database constraint/trigger designed for concurrent writes. Include role, enabled state, and optimistic version checks in that transaction.
- Validation: Run concurrent demotion/disable requests against the last two administrators and assert exactly one fails.

### [HIGH] SQLite import `abort` policy can overwrite data created after preview
- Location: `server/src/import.rs:72-107`, `server/src/import.rs:109-144`
- Category: Migration correctness / data loss
- Issue: Conflict detection happens before the write transaction. When policy is `Abort` and the preview sees no conflict, the write path uses the same `ON CONFLICT DO UPDATE` statements as `ImportWins`.
- Evidence: Only `ServerWins` has a distinct write branch. A row inserted or changed after conflict counting but before its import statement is silently overwritten under `Abort`.
- Risk: An operator selecting the explicitly conservative abort policy can lose newer server data while the live server is active.
- Fix: Perform detection and writes in one serializable transaction or lock the affected keys. For `Abort`, use plain inserts or an explicit conflict that rolls back the entire transaction; never use the import-wins upsert.
- Validation: Pause an abort import after preview, insert a conflicting row concurrently, resume, and assert the whole import rolls back without modifying the row.

### [MEDIUM] Bulk miner import bypasses optimistic concurrency and overwrites current records
- Location: `server/src/api.rs:586-633`
- Category: Data integrity / concurrency
- Issue: Normal miner edits and deletes require a version, but bulk import upserts every matching serial without an expected version or conflict report.
- Evidence: `ON CONFLICT (serial) DO UPDATE` replaces all imported fields and increments whatever current version exists. The preceding `SELECT EXISTS` is only used for counts.
- Risk: A stale spreadsheet can overwrite edits made moments earlier, defeating the concurrency guarantees presented by the rest of the API.
- Fix: Make overwrite policy explicit and admin-restricted, return per-row conflicts, or require expected versions for updates. At minimum, provide a dry-run and confirmation workflow.
- Validation: Edit a miner after an import snapshot is created, then import the stale row and verify the server reports a conflict instead of overwriting it.

### [MEDIUM] Currency is stored and transported as binary floating point
- Location: `server/migrations/0001_server_schema.sql:58`, `crates/fleet-shared/src/lib.rs:178`, `crates/fleet-shared/src/lib.rs:191`
- Category: Data correctness
- Issue: `unit_cost` uses PostgreSQL `DOUBLE PRECISION` and Rust/TypeScript floating-point values.
- Evidence: No decimal scale or integer-minor-unit representation is enforced.
- Risk: Repeated edits, calculations, and reports can produce rounding artifacts and inaccurate financial totals.
- Fix: Store currency as `NUMERIC(precision, scale)` or integer minor units and use a decimal/integer API type.
- Validation: Round-trip and aggregate values such as `0.1`, `0.2`, and high-volume totals with exact expected results.

### [LOW] Part update paths are assembled without URL encoding
- Location: `src-tauri/src/commands/mod.rs:153-157`, `crates/fleet-shared/src/lib.rs:228-241`
- Category: API correctness
- Issue: Delete encodes the SKU, but update inserts the raw SKU into the URL. Validation allows reserved characters such as `/`, `?`, and `#`.
- Evidence: `format!("/api/v1/parts/{}", input.sku)` is passed to `Url::parse`.
- Risk: Valid stored SKUs containing reserved characters cannot be updated reliably and may address a different path/query than intended.
- Fix: Encode the path segment for update and define a consistent SKU character policy.
- Validation: Create, update, and delete SKUs containing spaces and URL-reserved characters.

## Security Findings Summary
- Critical: 0
- High: 5
- Medium: 2
- Low: 1

## Test Gaps
- Critical path: Pairing, credential absence, login, logout, and expired-session recovery.
- Missing test: No Tauri client-state integration test exercises the keyring and UI state transition; frontend tests mock the command boundary.
- Recommended test: Run a temporary HTTPS server and test clean pair -> login -> authenticated request -> logout -> login.
- Critical path: Server authentication, role authorization, session revocation, login throttling, and final-admin concurrency.
- Missing test: No API/database integration tests exist; the server has only two auth helper unit tests.
- Recommended test: Add disposable PostgreSQL integration tests with concurrent requests and real migrations.
- Critical path: SQLite conflict policies and optimistic concurrency.
- Missing test: No migration/import test exercises all policies or races.
- Recommended test: Build a temporary legacy SQLite database and verify abort/server-wins/import-wins behavior transactionally.
- Critical path: Certificate pin enforcement.
- Missing test: No test proves rejection of a different publicly trusted certificate for the paired hostname.
- Recommended test: Exercise exact-certificate/SPKI verification and certificate rotation.

## Fix Order
1. Repair the no-credential authentication state so a clean client can log in.
2. Enforce exact TLS pinning and add certificate substitution tests.
3. Replace the login limiter with bounded source/account-aware throttling.
4. Make final-admin enforcement transactional.
5. Correct SQLite abort semantics and protect bulk import from stale overwrites.
6. Add PostgreSQL/API/Tauri integration tests for the new trust boundary.
7. Convert currency to an exact representation and encode part path segments.

## Verdict
DO NOT SHIP
