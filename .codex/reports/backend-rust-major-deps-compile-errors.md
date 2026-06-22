# Backend Rust Major Dependency Compile Errors

## Command

`CARGO_TARGET_DIR=/tmp/afm-backend-major-deps-target cargo check -p antminer-fleet-server --locked`

## Summary

- [x] rand API changes
  - `argon2::password_hash::rand_core::OsRng` was unavailable through the existing dependency graph after the migration.
  - `rand::RngCore` is no longer the import used by this code path; `rand::Rng` provides the needed `fill_bytes` method.
- [ ] reqwest API changes
  - No backend source API changes required.
  - Manifest feature changed from the plan's impossible `rustls-tls` to reqwest 0.13's `rustls` feature. `default-features = false` remains set; native TLS was not enabled.
- [x] sqlx API changes
  - SQLx 0.9 rejects owned/dynamic SQL strings unless they are audited and wrapped in `AssertSqlSafe`.
  - Existing dynamic SQL strings are assembled only from static query fragments, fixed clauses, bind placeholders, numeric limit/offset values, and whitelisted condition fragments. User input remains bound through SQLx parameters.
- [ ] toml API changes
  - No backend source API changes required.
- [ ] tower-http API changes
  - No backend source API changes required.
- [ ] transitive/toolchain issue
  - None observed on this Linux host.

## Raw output

See `.codex/reports/backend-rust-major-deps-compile-errors.txt`.
