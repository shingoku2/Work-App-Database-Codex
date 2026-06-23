# Frontend/Tauri Rust Major Dependency Runtime Validation

## Dependency targets

- keyring 4
- reqwest 0.13

## Frontend plan status

| Plan item | Status | Notes |
|---|---:|---|
| Frontend Task 1: baseline report | DONE | Existing commits `216c8db` and artifacts under `.codex/reports/frontend-rust-major-deps-*`. |
| Frontend Task 2: manifest/lockfile bump | DONE | Existing commit `cfcfdb2`; `src-tauri/Cargo.toml` uses `keyring = "4"` and `reqwest = "0.13"`. Actual feature names are the current crate feature names: `windows-native-keyring-store`, `apple-native-keyring-store`, `zbus-secret-service-keyring-store`, plus `rustls-no-provider` with explicit `rustls` dependency. |
| Frontend Task 3: compile/catalog errors | DONE | Compile now passes; no compile-error report needed. |
| Frontend Task 4: keyring migration | DONE | `Entry::new(...)` is handled as `Result<Entry, Error>`; token save/read/delete remains OS credential-manager backed. No plaintext fallback added. |
| Frontend Task 5: reqwest migration | DONE | `tls_danger_accept_invalid_certs(true)` is retained only for one-shot pre-pairing endpoints. Pinned client still uses a custom rustls verifier matching the exact paired leaf certificate bytes. |
| Frontend Task 6: TypeScript/UI regression | DONE | Focused connection tests, full test suite, and build pass. |
| Frontend Task 7: Tauri build/runtime validation | PARTIAL | Installer build passes. Manual live pairing/login/tunnel runtime checks were not performed in this session. |

## Automated validation

| Command | Result | Notes |
|---|---:|---|
| `test -f src-tauri/icons/icon.png || cp src-tauri/icons/128x128.png src-tauri/icons/icon.png` | PASS | Icon existed or was created for Tauri validation. |
| `CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-frontend-major-deps-target' cargo check -p antminer-fleet-manager --locked` | PASS | Finished dev profile. |
| `CARGO_TARGET_DIR='C:/Users/deped/AppData/Local/Temp/afm-frontend-major-deps-target' cargo test -p antminer-fleet-manager --locked` | PASS | 5 Rust unit tests passed. |
| `npm test -- src/test/connectionApi.test.ts src/test/ConnectionGate.test.tsx` | PASS | 12 focused connection tests passed. |
| `npm test` | PASS | 9 files / 96 tests passed. |
| `npm run build` | PASS | TypeScript + Vite production build passed. |
| `npm run tauri:build` | PASS | Built `target/release/antminer-fleet-manager.exe` and NSIS installer. |
| `npm audit --omit=dev` | PASS | 0 production vulnerabilities. |
| `git diff --check` | PASS | No whitespace errors; Git reported LF-to-CRLF warnings only. |

## Build artifacts

- Raw binary: `target/release/antminer-fleet-manager.exe` (15,468,544 bytes)
- Installer: `target/release/bundle/nsis/Antminer Fleet Manager_0.3.0_x64-setup.exe` (3,561,824 bytes)

## Manual/runtime validation

| Flow | Result | Notes |
|---|---:|---|
| First-run server URL entry | SKIPPED | Requires interactive desktop/server runtime. |
| `/health` pre-auth check | SKIPPED | Requires live server/tunnel endpoint. |
| Pairing fingerprint display | SKIPPED | Requires live server certificate flow. |
| Login stores token in OS credential manager | SKIPPED | Requires interactive app login against live server. |
| Restart token load behavior | SKIPPED | Requires interactive app restart. |
| Logout/delete credential | SKIPPED | Requires interactive app login/logout. |
| Generate tunnel key | SKIPPED | Covered by TypeScript mocks and Rust helper compile/tests, not live key generation. |
| Save tunnel config | SKIPPED | Covered by command-level code/tests, not live Windows helper execution. |
| Start tunnel connection | SKIPPED | Requires local tunnel config and reachable SSH host. |

## Security checks

- No plaintext bearer-token fallback added.
- Credential storage remains through `keyring` / OS credential manager.
- Pinned authenticated client still verifies exact paired server certificate bytes via custom rustls verifier.
- Unpinned invalid-cert acceptance remains narrowly scoped to pre-pairing one-shot endpoints only.
- No direct browser access to server/database added.
- No SSH private key material was printed or committed by this work.

## Runtime gaps

The desktop app was packaged successfully, which strongly suggests the antivirus/NSIS build problem is out of the way on this machine. Live pairing/login/token/tunnel behavior still needs an interactive smoke test against a running Fleet server/tunnel endpoint before calling runtime validation 100% complete.
