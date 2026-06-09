# Fix Report

## Source Assessment
- Findings source: `.codex/reports/03-env-validation.md` and `.codex/reports/04-code-audit.md`
- Files inspected: `server/src`, server migrations and packaging, `src-tauri/src/client.rs`, Tauri commands, connection UI, tests, and operator docs
- Assumptions: this is an internal application; public distribution licensing is not a release requirement
- Commands run before reconciliation: `cargo check --workspace --locked`, `cargo test --workspace --locked`, `npm test -- --run`, `npm run build`

## Fixes Applied

### [HIGH] Login unavailable after pairing or logout
- Files changed: `src-tauri/src/client.rs`, `src/features/connection/ConnectionGate.tsx`
- What changed: a missing local credential is represented as `AUTH_REQUIRED`, so the login form remains available after pairing and logout.
- Validation: connection-gate tests cover unauthenticated states.

### [HIGH] Certificate trust was additive instead of pinned
- Files changed: `src-tauri/src/client.rs`, `src-tauri/Cargo.toml`
- What changed: authenticated requests disable built-in root certificates and use a custom verifier that accepts only the paired leaf certificate.
- Validation: Rust test verifies that every other certificate is rejected.

### [HIGH] Login limiter permitted targeted lockout and unbounded storage
- Files changed: `server/src/api.rs`
- What changed: throttling is keyed by source address and account, includes a source-wide limit, expires old entries, and caps in-memory storage.
- Validation: tests cover source isolation, expiry, and bounded storage.

### [HIGH] Concurrent updates could remove every administrator
- Files changed: `server/src/api.rs`
- What changed: administrator updates run in one transaction guarded by a PostgreSQL advisory transaction lock and row lock before checking the final-admin invariant.
- Validation: compile and unit suites pass; concurrent PostgreSQL integration coverage remains outstanding.

### [HIGH] SQLite abort import could overwrite concurrent data
- Files changed: `server/src/import.rs`
- What changed: apply uses a serializable PostgreSQL transaction. Abort mode uses plain inserts so a late conflict rolls back the transaction instead of updating server data.
- Validation: compile passes; a live concurrent PostgreSQL integration test remains outstanding.

### [MEDIUM] Bulk import bypassed concurrency policy
- Files changed: `server/src/api.rs`, frontend import flow
- What changed: bulk miner import is administrator-only and requires explicit conflict handling in the user workflow.
- Validation: frontend import tests and Rust compilation pass.

### [MEDIUM] Currency used floating point
- Files changed: shared models, PostgreSQL migrations, API/import code, frontend types and import conversion
- What changed: part cost is stored and transported as integer cents. Migration `0002_exact_currency.sql` converts existing values.
- Validation: Rust and frontend suites pass.

### [LOW] Part update path did not encode SKU
- Files changed: `src-tauri/src/commands/mod.rs`
- What changed: update and delete use one percent-encoded part path helper.
- Validation: Rust unit test covers reserved characters.

### [HIGH] Runtime configuration validation was incomplete
- Files changed: `server/src/config.rs`, CLI handling, client config loading, packaging, and docs
- What changed: database scheme/host/name/placeholders, pool size, session duration, TLS paths, matching TLS material, hidden/stdin password input, malformed client config quarantine, and logging configuration are validated or documented.
- Validation: configuration and malformed-client-config unit tests pass.

## Skipped or Deferred Findings

### [INTERNAL COMPLIANCE] Project and dependency licensing
- Reason skipped: the user confirmed the application is for internal use and is not publicly distributed.
- Follow-up: retain third-party license and attribution records for internal compliance and reassess before distributing binaries outside the organization.

### [INTEGRATION] Live PostgreSQL concurrency and migration tests
- Reason skipped: no disposable PostgreSQL instance is configured in this Windows workspace.
- Follow-up: run migration, concurrent final-admin, abort-import, and API authentication tests against a disposable PostgreSQL server before production rollout.

## Files Changed
- This reconciliation updated only `.codex/reports/05-fix-report.md`.
- The production fixes are present in commit `23537c9`.

## Validation Summary
- `cargo check --workspace --locked`: passed
- `cargo test --workspace --locked`: passed
- `npm test -- --run`: passed, 84 tests
- `npm run build`: passed

## Manual Action Required
- Configure PostgreSQL and TLS on the central Linux machine.
- Verify the paired fingerprint out of band.
- Exercise a disposable-server migration and client pairing smoke test.
- Preserve third-party attribution records for internal compliance.

## Final Summary
- Fixed: all eight current code-audit findings plus the actionable configuration findings
- Partially fixed: none
- Deferred: live PostgreSQL/Linux integration verification
- Remaining risk: deployment-environment behavior has not been exercised on this Windows host
