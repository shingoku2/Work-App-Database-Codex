# Fix Report

## Source Assessment
- Findings source: `.codex/reports/04-code-audit.md`
- Files inspected: miner/part Rust commands, models, schema migrations, frontend API wrappers, and existing tests
- Assumptions: serial identity is whitespace-insensitive; Tauri commands are the validation boundary
- Commands run before changes: `npm run build`, `npm test`, `cargo check`, `cargo test`

## Fixes Applied

### [HIGH] Bulk import bypasses miner model and status validation
- Source finding: code audit finding 1
- Files changed: `src-tauri/src/commands/miners.rs`
- What changed: create, update, and import now use one validation function; imports validate the entire deduplicated batch before opening a transaction.
- Why this resolves the issue: invalid model/status values are rejected consistently before database writes.
- Tests/validation: shared validation unit tests added; full Rust suite pending in stage 6.
- Side effects or follow-up: import errors now identify invalid data before any batch mutation.

### [MEDIUM] Serial identity differs between manual writes and imports
- Source finding: code audit finding 2
- Files changed: `src-tauri/src/commands/miners.rs`
- What changed: serials are trimmed before create/update persistence and import deduplication/upsert.
- Why this resolves the issue: whitespace-equivalent serials use one canonical database identity.
- Tests/validation: unit tests assert normalization and trimmed deduplication.
- Side effects or follow-up: existing rows containing leading/trailing whitespace are not migrated automatically.

### [LOW] Missing records are reported as successful mutations
- Source finding: code audit finding 4
- Files changed: `src-tauri/src/commands/miners.rs`, `src-tauri/src/commands/parts.rs`
- What changed: update/delete commands check `rows_affected()` and return a not-found error when no row changed.
- Why this resolves the issue: stale operations no longer appear successful.
- Tests/validation: compile and existing suite pending in stage 6; database integration tests remain desirable.
- Side effects or follow-up: frontend users may now see an explicit not-found error after stale edits.

## Skipped or Deferred Findings

### [MEDIUM] Dual, non-atomic migration execution
- Reason skipped: the current architecture deliberately registers migrations in both `lib.rs` and `db.rs`; changing ownership can affect existing local databases.
- Required context/decision: choose one migration owner and test fresh, current, and partially migrated database files.
- Recommended owner: backend maintainer with release database fixtures.

## Files Changed
- `src-tauri/src/commands/miners.rs`
- `src-tauri/src/commands/parts.rs`

## Validation Summary
- Command: `cargo test`; `cargo check`; `npm test`; `npm run build`; targeted `rustfmt --check`
- Result: passed after correcting one Rust ownership error found by the first compile
- Notes: 13 Rust tests and 79 frontend tests pass. The first Rust run failed because a normalized serial string was moved into the deduplication set before assignment; cloning for the set resolved it.

## Manual Action Required
- Secret rotations: none
- Config updates: none
- Migrations: existing whitespace-padded serials may need a future data-cleanup migration
- Deployment notes: do not distribute until the license blocker in report 02 is resolved

## Final Summary
- Fixed: 3 findings
- Partially fixed: 0
- Skipped: 1 migration finding
- Remaining risk: migration synchronization and project licensing
