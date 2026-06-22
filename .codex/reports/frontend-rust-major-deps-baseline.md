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

## Pre-migration manifest

See `src-tauri/Cargo.toml` lines 18-19:
- `keyring = { version = "3", features = ["windows-native", "apple-native", "sync-secret-service"] }`
- `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }`

## Keyring usage (all in `src-tauri/src/client.rs`)

- Line 2: `use keyring::Entry;`
- Line 250-252: `credential_entry()?.set_password(&response.token)` — stores session token
- Line 263-265: `credential_entry()?.delete_credential()` — logout/clear, handles `NoEntry`
- Line 270-273: `credential_entry()?.get_password()` — reads session token, handles `NoEntry`
- Line 549-551: `Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_ACCOUNT)` — constructs credential entry

## Reqwest usage (all in `src-tauri/src/client.rs`)

- Line 3: `use reqwest::{Method, StatusCode};`
- Lines 404-422: `build_client()` — pinned cert HTTPS client with `use_preconfigured_tls`, `https_only(true)`, `tls_built_in_root_certs(false)`
- Lines 563-579: `one_shot_request()` — pre-pairing bootstrap with `danger_accept_invalid_certs(true)` + `https_only(true)`
- Lines 347, 359: `request.send().await` — authenticated API calls
- Lines 132-134, 189, 245: `.send().await` — pairing, connection state, login
- Lines 553, 568, 581, 589: `reqwest::Error` / `reqwest::Response` type references

## Captured artifacts

- `.codex/reports/frontend-rust-major-deps-tree-before.txt`
- `.codex/reports/frontend-rust-major-deps-usage-before.txt`

## Known validation caveat

If workspace-wide Cargo validation fails on Windows with the known local MSVC/proc-macro DLL issue, use isolated `CARGO_TARGET_DIR` validation and document the exact failure separately. Do not hide real compile or test failures.