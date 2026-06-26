# Dependency Update Bug Sweep Report

Date: 2026-06-26 16:08:19 CDT
Branch: master
Scope: Audit codebase after frontend npm updates and Rust sha2/hmac migration.

## Summary

No runtime, compile, lint, typecheck, or audit regressions were found from the dependency updates.

One real issue was found and fixed: the root `package.json` still declared `node >=20`, but the updated frontend toolchain requires Node 20.19.0 or newer on the Node 20 line. That mismatch could let a developer run `npm ci` on Node 20.0-20.18 and hit engine/tooling failures. The Node requirement is now aligned in `package.json`, `package-lock.json`, `README.md`, and agent context files.

Stale ESLint 9 documentation was also corrected after ESLint 10 adoption.

## Files Changed

- `package.json`
  - Changed `engines.node` from `>=20` to `>=20.19.0`.
- `package-lock.json`
  - Refreshed root package engine metadata via `npm install`.
- `README.md`
  - Updated desktop development prerequisite to Node.js 20.19.0+.
- `AGENTS.md`
  - Updated toolchain requirement to Node.js 20.19.0+.
- `GEMINI.md`
  - Updated frontend stack from ESLint 9 pinned to ESLint 10.
- `CHANGELOG.md`
  - Replaced stale ESLint 9 pin notes with ESLint 10 adoption notes.
  - Updated cargo-audit scanned dependency count from 658 to 659.

## Review Notes

### Rust sha2/hmac migration

Reviewed the `d5a06fb` migration diff:

- `Cargo.toml`
  - `sha2` moved from `0.10` to `0.11`.
  - `hmac` moved from `0.12` to `0.13`.
- `server/src/auth.rs`
  - `token_hash()` no longer relies on `LowerHex` for the digest output and now formats each byte explicitly as two lowercase hex characters.
- `server/src/api.rs`
  - `hmac::KeyInit` was imported for `HmacSha256::new_from_slice()` under hmac 0.13.

The relevant regression tests passed:

- `auth::tests::session_tokens_are_random_and_hash_stably`
- `api::tests::webhook_signature_is_hmac_sha256_hex`
- certificate fingerprint and tunnel onboarding tests included in full suites below

### Frontend ESLint/npm update

Reviewed the `212c476` frontend dependency diff:

- ESLint 10 and `@eslint/js` 10 are installed.
- `eslint-plugin-react` is no longer installed and is not referenced by `eslint.config.js`.
- `eslint-plugin-react-hooks`, `typescript-eslint`, and the flat ESLint config remain compatible.

Verified installed/root engine constraints:

- root package: `node >=20.19.0`
- `eslint`: `^20.19.0 || ^22.13.0 || >=24`
- `@eslint/js`: `^20.19.0 || ^22.13.0 || >=24`
- `vite`: `^20.19.0 || >=22.12.0`
- `@vitejs/plugin-react`: `^20.19.0 || >=22.12.0`
- `jsdom`: `^20.19.0 || ^22.13.0 || >=24.0.0`

## Validation Commands Run

### Frontend / Node

- `npm ci`
  - Passed: added 270 packages, audited 271 packages, 0 vulnerabilities.
- `npm run build`
  - Passed: TypeScript + Vite production build completed.
- `npm test`
  - Passed: 9 test files, 97 tests.
- `npx eslint src/`
  - Passed: no output, exit 0.
- `npx tsc --noEmit --noUnusedLocals --noUnusedParameters --pretty false`
  - Passed: no output, exit 0.
- `npm audit --omit=dev`
  - Passed: 0 vulnerabilities.
- `npm audit`
  - Passed: 0 vulnerabilities.

### Rust

- `cargo fmt --all -- --check`
  - Passed.
- `cargo check --workspace --locked`
  - Passed.
- `cargo test --workspace --locked`
  - Passed: 24 Rust tests.
- `cargo clippy --workspace --locked -- -D warnings`
  - Passed.
- `cargo audit`
  - Passed: scanned 659 crate dependencies with configured suppressions.

### Repo Hygiene

- `git diff --check`
  - Passed: no whitespace errors.
- `git status --short --branch`
  - Working tree has only the intended audit fixes listed above.

Git emitted line-ending warnings for `CHANGELOG.md` and `README.md` because they are currently LF in the working tree and may be normalized to CRLF when Git touches them on this Windows checkout. No content or whitespace errors were reported by `git diff --check`.

## Remaining Runtime Gaps

Not exercised in this sweep:

- Live PostgreSQL migrations/concurrency against a real database.
- Packaged Tauri/keyring pairing and re-login flow.
- Debian package install/systemd runtime validation.
- Full SSH tunnel path through deployed infrastructure.

Those are environment/runtime checks, not static dependency-update regressions.
