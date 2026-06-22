# Backend to Frontend Handoff: Rust Major Dependency Migration

## Backend completed

- [x] rand 0.10
- [x] reqwest 0.13
- [x] sqlx 0.9
- [x] toml 1.1
- [x] tower-http 0.7

## Backend validation status

See `.codex/reports/backend-rust-major-deps-validation.md`.

Summary:

| Command | Result | Notes |
|---|---:|---|
| `cargo fmt -p antminer-fleet-server -- --check` | PASS | Backend package formatting passes. |
| `cargo check -p antminer-fleet-server --locked` | PASS | Server compiles. |
| `cargo check -p fleet-shared --locked` | PASS | Shared crate compiles. |
| `cargo test -p fleet-shared -p antminer-fleet-server --locked` | PASS | 14 tests passed. |
| `cargo check --workspace --locked` | PASS | Linux workspace check passed; frontend still has its own migration pending. |
| live PostgreSQL/TLS smoke | SKIPPED | `server/config/server.local.toml` absent on this host. |

## Shared contracts changed?

No shared Rust/TypeScript API contracts were intentionally changed.

## Frontend/Tauri impact

- Tauri Rust layer still needs `keyring 4` migration.
- Tauri Rust layer still declares `reqwest 0.12`; after backend migration, the workspace temporarily resolves both `reqwest 0.12.28` and `reqwest 0.13.4`.
- Reqwest 0.13 does not expose the old `rustls-tls` feature. Backend used `features = ["json", "rustls"]` with `default-features = false`. The frontend migration should verify the correct reqwest 0.13 feature choice against Tauri's pinned certificate code before applying the same pattern.
- TypeScript frontend should not need API contract changes from backend work.

## Runtime notes

- No server URL, TLS pinning, pairing endpoint, or tunnel topology behavior was intentionally changed.
- Production service/tunnel processes were not touched.
