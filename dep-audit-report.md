# Dependency Audit Report

## Summary
Tauri v2 + React 19 + TypeScript + Vite desktop app with a Rust + sqlx + tauri-plugin-sql backend, SQLite local-file persistence. Single root `package.json` (12 prod / 10 dev deps), single `package-lock.json` (224 resolved packages), single `Cargo.toml` + `Cargo.lock` (~480 crates). No workspace, no monorepo, no alternate package manager configs. The forbidden `xlsx` package is **not present** anywhere — confirmed by `package.json`, `package-lock.json`, and `node_modules`. Migrations `0001` / `0003` / `0004` are registered in **both** `src-tauri/src/lib.rs` (tauri-plugin-sql) and `src-tauri/src/db.rs` (custom `schema_migrations` runner) — consistent. All licenses are MIT/ISC/Apache-2.0/BSD-3-Clause — no GPL/AGPL/SSPL family anywhere. **0 critical, 1 high, 4 medium, 4 low** findings.

## Critical findings
None.

## High findings

- **[HIGH] `unzipper@0.12.3` (transitive via `read-excel-file@9.0.10`)** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\package-lock.json:3070`
  - **Issue:** `unzipper` has a history of advisories (notably CVE-2023-31414 — DoS via crafted zip; historical path-traversal concerns in earlier versions).
  - **Evidence:** Locked at `0.12.3`, the version installed in `node_modules`. Cannot confirm current CVE status without live lookup — flagging the pattern.
  - **Risk:** Threat model is offline desktop app where the user picks an Excel file via Tauri file dialog. The remote-attack surface is low, but a maliciously crafted `.xlsx` chosen by the user (or a social-engineered one) could trigger a DoS or extraction-side-effects. No known RCE.
  - **Recommendation:** Evaluate swapping `read-excel-file` for `xlsx-parse-stream` or a maintained alternative that does not depend on `unzipper`. If keeping `read-excel-file`, document the threat model and consider wrapping the import in a try/catch with a size cap.

## Medium findings

- **[MED] `sqlx` feature `chrono` is unused in queries** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\Cargo.toml:20`
  - **Issue:** `sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "chrono"] }`. The schema stores all timestamps as `TEXT` (CURRENT_TIMESTAMP defaults, `acquired_date TEXT`). No Rust code uses `chrono::DateTime` round-trips against the DB. The `chrono` feature is therefore not exercised by `sqlx`.
  - **Risk:** Binary bloat (chrono pulls `iana-time-zone`, `num-traits`, etc.) and a slightly larger attack surface for no functional gain.
  - **Recommendation:** Drop the `chrono` feature from sqlx. If Rust ever needs to format timestamps in commands, add `chrono` as a direct dep there — it already is.

- **[MED] `tauri-plugin-sql@2.4.0` is behind the 2.11.x family** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\Cargo.lock:3945`
  - **Issue:** `tauri` itself is at `2.11.1`; `tauri-plugin-sql` is at `2.4.0`. The plugin version floats, not pinned to `2`, so resolution can drift. The 0.7-version gap covers months of fixes.
  - **Risk:** Missed bug fixes and security patches on the plugin side. No known active CVE.
  - **Recommendation:** Bump to the latest 2.x: `tauri-plugin-sql = { version = "2.11", features = ["sqlite"] }` and re-run `cargo update -p tauri-plugin-sql`.

- **[MED] `read-excel-file@9.0.10` is unmaintained-but-acceptable** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\package.json:23`
  - **Issue:** `read-excel-file` has had low release velocity and the project is functionally frozen on the 9.x line. It is preferred over `xlsx` per project policy, but a more actively maintained alternative would reduce long-term risk.
  - **Risk:** Stale parser may eventually break on newer Excel formats; security fixes in its transitive tree (see HIGH above) may lag.
  - **Recommendation:** Periodically re-evaluate `xlsx-parse-stream`, `exceljs` (read-only), or a pure-Rust XLSX reader invoked from a `#[tauri::command]`. Not urgent.

- **[MED] Frontend deps are caret-pinned, not exact** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\package.json:14-37`
  - **Issue:** All direct deps use `^x.y.z`. The lockfile pins resolved versions, but a fresh `npm install` on a different date could pick up a newer minor and silently change behavior.
  - **Risk:** Drift between CI/build machine and the audited lockfile state. Mediocre reproducibility.
  - **Recommendation:** Acceptable as long as the lockfile is always committed (it is) and CI uses `npm ci` (recommended in CLAUDE.md verification steps). No code change needed; just enforce `npm ci` in the build pipeline.

## Low findings

- **[LOW] No `Cargo.lock` for a binary, but it IS committed** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\Cargo.lock` (135 KB, 5609 lines)
  - Note only: Cargo.lock is committed (good for reproducibility of a binary crate). No action.

- **[LOW] `package-lock.json` shows `lucide-react@0.555.0` as resolved** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\package-lock.json:2322`
  - **Issue:** `lucide-react` releases very frequently; even with caret pinning, lockfile churn is expected. No security concern; just be aware of the noise in future audits.

- **[LOW] `@vitejs/plugin-react@5.2.0` pulls `@rolldown/pluginutils@1.0.0-rc.3` (pre-release)** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\package-lock.json:1631`
  - **Issue:** Pre-release transitive. The plugin itself is stable, but its dep is RC. No CVE. Track for stable.

- **[LOW] Capability file is minimal but no Tauri plugin permissions are wired** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\capabilities\default.json`
  - **Issue:** Only `core:default` granted. `tauri-plugin-sql` is used from Rust only, so no JS-side permission is needed (consistent with the architecture). Flagging only because the audit brief asked to verify plugin/permission wiring — there is none, and that is correct here.

## Forbidden/Blocker items
- **`xlsx` package presence: NOT FOUND** — confirmed absent from `package.json`, `package-lock.json`, and `node_modules/`. Compliant with project CLAUDE.md and AGENTS.md.
- **GPL-family transitive deps: NONE** — full scan of `package-lock.json` license fields found only MIT, ISC, Apache-2.0, BSD-3-Clause, and `Apache-2.0 OR MIT` (Tauri). No GPL, AGPL, LGPL, SSPL, Commons Clause, or non-commercial licenses. Safe for internal distribution.
- **Migration registration consistency: VERIFIED** — `lib.rs` registers `[(1, initial_schema), (3, remove_ticketing), (4, miner_import_fields)]`; `db.rs` runs the same three via its custom `schema_migrations` table. Identical SQL sources (via `include_str!`).
- **`tauri-plugin-sql` dual-presence note:** The audit brief stated the plugin "exists on both frontend and backend manifests." It actually exists **only on the backend** (Rust `Cargo.toml` line 17, resolved 2.4.0). The frontend `package.json` has no `@tauri-apps/plugin-sql` — only `@tauri-apps/api@2.11.0` and `@tauri-apps/cli@2.11.1`. This is the correct architecture for this app because **all DB access goes through `command<T>()` invokes to Rust `#[tauri::command]` handlers** (per CLAUDE.md); the JS-side SQL plugin bindings are not used. No version mismatch to fix.

## Recommended actions
1. **Verify and (if confirmed) bump `tauri-plugin-sql` to a current 2.11.x release.** Closes the version drift gap and pulls in any plugin-side fixes since 2.4.0.
2. **Drop the unused `chrono` feature from the `sqlx` dependency** in `src-tauri/Cargo.toml`. Smaller binary, smaller surface.
3. **Schedule a follow-up audit on `unzipper@0.12.3`** to confirm current advisory status (CVE-2023-31414 or successors). If still affected and a maintained drop-in is available, replace `read-excel-file`'s dependency chain.
4. **Add `npm ci` to the verification script** in CLAUDE.md (replacing/augmenting `npm audit --omit=dev`) so builds use the exact audited lockfile rather than resolving `^` ranges fresh.
5. **Optional: pin `lucide-react` to exact version** in `package.json` (no caret) to reduce lockfile churn. Low priority.

---
**Files reviewed (absolute paths):**
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\package.json`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\package-lock.json`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\Cargo.toml`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\Cargo.lock`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\lib.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\db.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\tauri.conf.json`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\capabilities\default.json`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\migrations\0001_initial_schema.sql`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\migrations\0003_remove_ticketing.sql`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\migrations\0004_miner_import_fields.sql`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\CLAUDE.md`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\AGENTS.md`
