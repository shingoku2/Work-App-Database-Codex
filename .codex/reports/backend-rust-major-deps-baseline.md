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

This run is on Linux, not the Windows/MSVC environment referenced by the plan. If workspace-wide Cargo validation fails on a platform-specific linker/proc-macro issue, use isolated `CARGO_TARGET_DIR` validation and document the exact failure separately. Do not hide real compile or test failures.
