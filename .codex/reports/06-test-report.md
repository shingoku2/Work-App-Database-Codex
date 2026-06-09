# Test Report

## Coverage Assessment
- Test framework detected: Rust built-in test harness and Vitest 3 with React Testing Library/JSDOM.
- Existing test quality: generally focused and readable; frontend transformations and command wrappers were well covered, while the new server trust boundary depended mostly on in-module Rust unit tests and had no black-box CLI coverage.
- Critical path coverage: medium.
- Audit/fix reports used: `.codex/reports/04-code-audit.md` and the reconciled `.codex/reports/05-fix-report.md`.
- Largest gaps: live PostgreSQL authorization/concurrency behavior, SQLite-to-PostgreSQL import conflict races, a real HTTPS certificate-substitution handshake, and an end-to-end Tauri/keyring relogin flow.

## Tests Added or Updated

### Module/Feature: Unauthenticated Relogin
- Files changed: `src/test/ConnectionGate.test.tsx`.
- Tests added: one.
- Behavior covered: a paired client with no current credential accepts username/password, invokes login, refreshes connection state, and renders authenticated content.
- Regression caught: the login button/state becoming unusable after pairing, logout, or local credential loss.
- Limitations: the Tauri keyring and remote HTTPS server are mocked at the command boundary.

### Module/Feature: Integer Currency
- Files changed: `src/test/InventoryView.test.tsx`.
- Tests added: two.
- Behavior covered: integer cents render as an exact two-decimal dollar amount, and a user-entered dollar value is converted to integer cents before update.
- Regression caught: returning to floating-point API payloads or incorrectly scaling edited costs.
- Limitations: PostgreSQL migration and database round-trip behavior require live infrastructure.

### Module/Feature: Server Configuration Validation
- Files changed: `server/tests/config_cli.rs`.
- Tests added: three.
- Behavior covered: the compiled server CLI rejects non-PostgreSQL and placeholder database URLs, unsafe pool/session ranges, and reuse of one path for the TLS certificate and private key.
- Regression caught: `validate-config` accepting unsafe or incomplete base deployment settings.
- Limitations: these tests intentionally stop at invalid base settings and do not require real TLS files or PostgreSQL.

### Module/Feature: Existing Security Regression Coverage Revalidated
- Files changed: none.
- Tests run: exact leaf-certificate rejection, malformed saved-config quarantine, login limiter source isolation/expiry/capacity, and reserved-character SKU path encoding.
- Behavior covered: fixed findings from reports 04 and 05 that already had meaningful Rust unit tests.
- Regression caught: additive TLS trust, startup failure on corrupt client config, source-insensitive/unbounded login limiting, and raw reserved characters in part paths.
- Limitations: exact pinning is verified at the Rust verifier boundary, not through a live TLS server.

## Validation
- Command: `npm test -- src/test/ConnectionGate.test.tsx src/test/InventoryView.test.tsx src/test/partApi.test.ts`
- Result: passed, 9 tests.
- Notes: targeted frontend regression suite.

- Command: `cargo test -p antminer-fleet-server --test config_cli --locked`
- Result: passed, 3 tests.
- Notes: first parallel attempt timed out while waiting on Cargo package-cache locks; the sequential rerun passed.

- Command: `cargo test -p antminer-fleet-server --bin antminer-fleet-server --locked login_limiter`
- Result: passed, 2 tests.
- Notes: verifies source isolation, expiry, and bounded storage.

- Command: `cargo test -p antminer-fleet-manager --lib --locked pinned_verifier_rejects_every_other_certificate`
- Result: passed, 1 test.
- Notes: first parallel attempt timed out on Cargo locks; the sequential rerun passed.

- Command: `cargo test -p antminer-fleet-manager --lib --locked malformed_saved_config_is_quarantined`
- Result: passed, 1 test.
- Notes: corrupt saved configuration is moved aside and does not remain active.

- Command: `cargo test -p antminer-fleet-manager --lib --locked part_paths_encode_reserved_characters`
- Result: passed, 1 test.
- Notes: spaces and `/`, `?`, and `#` are encoded as one path segment.

- Command: `cargo test --workspace --locked`
- Result: passed, 13 tests.
- Notes: includes the three new black-box CLI tests.

- Command: `npm test`
- Result: passed, 87 tests across 8 files.
- Notes: suite increased from 84 to 87 tests.

- Command: `cargo fmt --all -- --check`
- Result: passed.
- Notes: the first check found formatting only in the new Rust test file; `cargo fmt --all` corrected it and the rerun passed.

- Command: `npm run build`
- Result: passed.
- Notes: TypeScript compilation and Vite production build completed successfully.

## Gaps Not Filled
- Gap: concurrent final-administrator protection.
- Reason: meaningful verification requires two real PostgreSQL transactions and concurrent API requests.
- Required follow-up: run a disposable PostgreSQL integration test that concurrently disables or demotes the last two administrators and proves one transaction fails.

- Gap: SQLite import `abort`, `server-wins`, and `import-wins` transaction semantics.
- Reason: the critical behavior occurs between SQLite input and a live serializable PostgreSQL transaction; mocking SQL would test the mock rather than the conflict guarantee.
- Required follow-up: build temporary SQLite fixtures and run all policies against disposable PostgreSQL, including a conflict inserted after preview.

- Gap: full certificate substitution and rotation behavior.
- Reason: the current unit test proves exact verifier matching, but a full test needs temporary HTTPS endpoints presenting two certificates for the same host.
- Required follow-up: pair certificate A, substitute certificate B, and prove every authenticated request fails until explicit re-pairing.

- Gap: end-to-end pair, login, logout, and relogin.
- Reason: frontend tests mock the Tauri command boundary and cannot exercise the operating-system keyring.
- Required follow-up: run a packaged/dev Tauri client against a disposable HTTPS server with an isolated test keyring.

- Gap: PostgreSQL currency migration and exact persistence.
- Reason: no disposable PostgreSQL instance is configured in this workspace.
- Required follow-up: apply migrations to a populated pre-migration database and verify exact cent values and aggregates.

## Summary
- Tests written: 6.
- Tests modified: 1 existing test file.
- Critical paths covered: unauthenticated relogin, exact certificate verifier behavior, malformed config quarantine, server config rejection, limiter isolation/expiry/capacity, encoded SKUs, and integer-cent UI/API conversion.
- Remaining high-priority gaps: live PostgreSQL concurrency/import tests and real Tauri/HTTPS/keyring end-to-end tests.
