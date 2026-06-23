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
| `cargo fmt -p antminer-fleet-server -- --check` | PASS | Backend package formatting passes. Full workspace fmt was not used because unrelated frontend/Tauri files from the pulled branch need formatting. |
| `CARGO_TARGET_DIR=/tmp/afm-backend-major-deps-target cargo check -p antminer-fleet-server --locked` | PASS | Server compiles with migrated backend dependencies. |
| `CARGO_TARGET_DIR=/tmp/afm-backend-major-deps-target cargo check -p fleet-shared --locked` | PASS | Shared crate compiles. |
| `CARGO_TARGET_DIR=/tmp/afm-backend-major-deps-target cargo test -p fleet-shared -p antminer-fleet-server --locked` | PASS | 14 tests passed: 6 server unit tests, 4 config CLI tests, 1 tunnel script test, 3 fleet-shared tests. |
| `CARGO_TARGET_DIR=/tmp/afm-backend-workspace-target cargo check --workspace --locked` | PASS | Workspace compile passes on Linux. It still includes two reqwest versions until the frontend/Tauri track migrates `src-tauri`. |
| `cargo run -p antminer-fleet-server -- --config server/config/server.local.toml validate-config` | SKIPPED | `server/config/server.local.toml` is not present on this host. |
| `cargo run -p antminer-fleet-server -- --config server/config/server.local.toml migrate` | SKIPPED | Local PostgreSQL config is not present. |
| `/health` smoke | SKIPPED | Requires local server config/runtime. |
| `/pairing` smoke | SKIPPED | Requires local server config/runtime. |
| `git diff --check` | PASS | No whitespace errors. |

## Source changes

- `server/Cargo.toml`
  - Migrated backend dependency requirements to `rand = "0.10"`, `reqwest = "0.13"`, `sqlx = "0.9"`, `toml = "1.1"`, `tower-http = "0.7"`.
  - Reqwest 0.13 no longer exposes `rustls-tls`; this migration uses `features = ["json", "rustls"]` with `default-features = false` to keep the Rustls TLS stack and avoid native TLS.
- `Cargo.lock`
  - Resolved migrated backend crates and transitive dependencies.
- `server/src/auth.rs`
  - Replaced direct `OsRng` salt generation with rand 0.10-backed random bytes plus `SaltString::encode_b64`.
  - Kept session token generation at 32 random bytes encoded as hex.
- `server/src/api.rs`
  - Wrapped existing audited dynamic SQL strings with `sqlx::AssertSqlSafe` for SQLx 0.9.
  - The affected SQL strings are built from static fragments, whitelisted clauses, bind placeholders, and numeric limit/offset values; user-provided values remain SQLx bind parameters.
- `server/tests/tunnel_key_scripts.rs`
  - Fixed the script path to point at `server/scripts/...` from `CARGO_MANIFEST_DIR`.
  - Adjusted the expected authorized key assertion to account for the script intentionally replacing the source key comment with `antminer-fleet-client:<label>`.
- `server/tests/config_cli.rs`
  - Backend formatter normalized one existing line break.

## Runtime gaps

- Live local PostgreSQL validation was not run because `server/config/server.local.toml` is absent on this host.
- Live HTTPS `/health` and `/pairing` smoke tests were not run for the same reason.
- Production service/tunnel processes were not touched.
