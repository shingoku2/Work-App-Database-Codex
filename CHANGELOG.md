# Changelog

All notable changes to Antminer Fleet Manager will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-06-23

### Changed
- Migrated all Rust major dependencies to current upstream versions:
  `keyring 3 -> 4` (feature renames: `windows-native-keyring-store`,
  `apple-native-keyring-store`, `zbus-secret-service-keyring-store`),
  `reqwest 0.12 -> 0.13` (feature `rustls-tls` -> `rustls-no-provider`),
  `sqlx 0.8 -> 0.9`, `rand 0.9 -> 0.10`, `tower-http 0.6 -> 0.7`,
  `toml 0.9 -> 1.1`.
- Pinned ESLint to `^9.39.4` (and `@eslint/js` to `^9.39.4`) because
  `eslint-plugin-react@7.37.5` does not accept ESLint 10.
- Added MIT License to the workspace and all three crates.
- Moved `cargo-audit` config to `.cargo/audit.toml` (the location
  cargo-audit 0.22+ reads) with documented suppressions for 19 advisories
  that are transitive, platform-specific, or false positives.
- Removed `.claude/settings.local.json` from Git tracking.
- Removed stale root-level pipeline reports (`audit-report.md`,
  `audit-report-v2.md`, `dep-audit-report.md`, `env-report.md`,
  `onboarding-report.md`, `shipper-report.md`) and migration scratch
  `.txt` files from `.codex/reports/`.
- Added `src-tauri/icons/icon.png` to `.gitignore` (build artifact copied
  from `128x128.png` during Tauri builds).

### Fixed
- Resolved ESLint 10 peer-dependency conflict that broke `npm ci` after
  `eslint-plugin-react` was pinned at 7.37.5.
- Fixed `cargo-audit` config location so suppressions are actually read.

### Validation Status
- Passed: `cargo check --workspace --locked`.
- Passed: `cargo test --workspace --locked` with 24 Rust tests.
- Passed: `npm test` with 96 frontend tests across 9 files.
- Passed: `npm run build`, `cargo fmt --all -- --check`,
  `git diff --check`, and `npm audit --omit=dev` with zero npm
  vulnerabilities.
- Passed: `cargo audit` with 658 crate dependencies scanned (19
  advisories suppressed with documented rationale in `.cargo/audit.toml`).
- Passed: `npm run tauri:build` producing NSIS installer
  (`Antminer Fleet Manager_0.3.0_x64-setup.exe`, 3.4 MB) and raw binary
  (`antminer-fleet-manager.exe`, 15 MB).
- Not yet verified on target infrastructure: Debian package installation,
  systemd operation, live local or remote PostgreSQL migrations and
  concurrency, populated currency migration, SQLite conflict races,
  PostgreSQL backup/restore, certificate substitution or rotation, and a
  packaged Tauri/keyring pairing and re-login flow.
- One open GitHub Dependabot alert (RUSTSEC-2024-0429, glib unsoundness):
  transitive Tauri v2 Linux/GTK runtime dependency, not compiled on
  Windows, unsound code path not reachable from application code, cannot
  fix without Tauri upgrading to gtk-rs 0.19+. Suppressed in
  `.cargo/audit.toml` with documented rationale.

## [Unreleased]

### Breaking Changes
- Replaced the desktop-owned local SQLite architecture with a separately
  installed central server backed by PostgreSQL. The desktop client is now
  online-required and cannot read or write the legacy local `fleet.db`
  directly.
- Operators must install and configure the server, apply its PostgreSQL
  migrations, create the first administrator, start the HTTPS service, and
  pair each desktop before users can access fleet data.
- Existing SQLite data is not migrated automatically. An operator must copy
  the legacy `fleet.db` to the server and run the server-side `import-sqlite`
  CLI, beginning with a dry run and then selecting an explicit `abort`,
  `server-wins`, or `import-wins` conflict policy.
- Part costs now use integer cents throughout the API and PostgreSQL schema.
  Existing PostgreSQL floating-point values and imported SQLite values are
  converted by multiplying by 100 and rounding to the nearest cent.

### Added
- Added a separately installed Rust HTTPS server backed by PostgreSQL, with a
  Debian package definition, systemd unit, database migrations, health
  endpoint, and administrative CLI.
- Added named accounts with `admin` and `user` roles. Administrators can create
  and disable accounts, assign roles, and reset passwords; the final enabled
  administrator cannot be disabled or demoted.
- Added Argon2id password hashing and revocable opaque sessions. Desktop
  session tokens are stored in the operating-system credential manager.
- Added first-launch server pairing for one configured server per desktop.
  Users must compare the complete SHA-256 certificate fingerprint through an
  independent trusted channel before accepting the server.
- Added exact leaf-certificate pinning for all requests after pairing.
  Replacing the server certificate requires clients to forget and re-pair the
  server.
- Added a dry-run/apply SQLite importer with explicit conflict reporting and
  transactional conflict policies.
- Added optimistic concurrency versions for miners, parts, and users so stale
  updates and deletes return conflicts instead of silently overwriting newer
  changes.

### Changed
- Converted the Tauri application into an online-required client of the
  central HTTPS API. It no longer stores the production fleet database.
- Moved production database ownership and migrations to the central server.
- Made spreadsheet miner import administrator-only and insert-only for
  existing serial numbers; conflicts are reported rather than overwritten.
- Store and transport part costs as exact integer cents instead of binary
  floating point.

### Fixed
- Preserved the login form after pairing, logout, or local credential loss so a
  paired client can authenticate again without forgetting the server.
- Quarantine malformed saved client configuration instead of preventing the
  desktop application from starting.
- Added bounded, expiring, source-aware login throttling.
- Hardened server configuration validation for PostgreSQL URLs, connection
  limits, session duration, TLS paths, PEM parsing, and matching certificate
  and private-key material.
- Removed plaintext password command arguments from administrator CLI flows;
  use hidden prompts or protected standard input.
- Normalized miner serials at the server boundary and applied the same model
  and lifecycle-status validation to manual and spreadsheet writes.
- Return explicit not-found errors when an update or delete targets a miner or
  inventory part that no longer exists.
- Percent-encode inventory SKU path segments for update and delete requests.

### Security
- Require named authentication for fleet operations and restrict account
  administration and spreadsheet import to administrators.
- Pin the exact certificate accepted during fingerprint-confirmed pairing
  rather than extending the desktop's normal certificate trust store.
- Limit server request bodies to 30 MB and keep login-throttle state bounded.

### Upgrade Instructions
1. Prepare the Debian/Ubuntu server and PostgreSQL database described in
   `server/README.md`.
2. Configure TLS and `/etc/antminer-fleet/server.toml`, then run
   `validate-config` and `migrate`.
3. Create the first administrator and start the systemd service.
4. If upgrading from the local SQLite desktop release, preserve the original
   `fleet.db`, run `import-sqlite` without `--apply`, review the reported
   conflicts, and then apply one explicit conflict policy.
5. Verify record counts and representative miner and part records before
   treating PostgreSQL as the source of truth.
6. Distribute the server certificate fingerprint through a trusted channel,
   pair each desktop, and sign in with a named account.

Detailed installation, migration, pairing, backup, and rollback preparation
are documented in `server/README.md` and `docs/OPERATIONS.md`.

## [0.2.0] - 2026-06-02

### Added
- Added automated test suite: frontend tests run with `npm test` (vitest, jsdom, @testing-library); backend tests run with `cargo test` from `src-tauri/`.
- Added per-row import statistics. The "Imported N miners" message now distinguishes inserted rows from updated rows and skipped (empty or duplicate) rows.
- Added a `README.md` with prerequisites (Node 20+, Rust stable, platform Tauri deps), first-build steps, verification commands, and a "common first-build failures" section. The repo's onboarding path now starts at `README.md`; `CLAUDE.md` remains the binding rule set.

### Changed
- Collapsed the dashboard's four database round-trips into two: the three scalar counts (units, parts, low-stock parts) are now returned from a single query.
- Reduced per-row work during spreadsheet import: the header key normaliser is built once per row instead of recomputed for every field access.
- Tightened the Content Security Policy: `img-src` no longer allows `data:` URIs.
- Tightened the Vite path alias to use `fileURLToPath(new URL("./src", import.meta.url))` so it matches the `tsconfig.json` glob form, instead of the absolute-string `"/src"` form.
- Narrowed the vitest `include` glob to `src/test/**/*.test.{ts,tsx}` so it matches the "Frontend tests live under `src/test/`" rule in `CLAUDE.md` (previously the glob covered the whole `src/` tree).
- Re-labelled command fences in `CLAUDE.md` from `powershell` to `bash` (the commands are platform-agnostic), added `npm ci` as the install step, and added a one-sentence product description to the top of the file.

### Removed
- Removed the unused `chrono` and `thiserror` crates from the backend dependency list, and dropped the unused `chrono` feature from `sqlx`. No code change required.

### Fixed
- Fixed import-result count overstating unique miners. Re-importing a facility export that contains duplicates previously reported every row as a fresh insert; the count now reflects unique upserts.
- Fixed inconsistent storage of blank optional miner fields. Hand-entered units and imported units now both store `NULL` for empty fields, so future reporting queries do not need to check for both empty string and null.
- Fixed the Excel import's silent failure mode. Files larger than 25 MB are now rejected with a clear status message; a corrupt or non-ZIP file is detected by its magic bytes and reported as a parse error rather than throwing a raw library message.
- Fixed the form's optional-field handlers: clearing an input now writes `null` to the model so the value round-trips correctly.
- Fixed out-of-range date handling in the import date normaliser. Inputs outside the `mm/dd/yyyy` valid ranges are now rejected instead of being stored as a nonsensical date.
- Fixed lockfile drift: `read-excel-file` is now pinned to exact version `9.0.10` in both `package.json` and `package-lock.json` to match the installed version.
- Fixed `normalizeKey` recomputation by pre-building a key lookup map per row, removing redundant O(n) scans during large imports.
- Fixed the `format!` macro being used for a constant SQL concatenation in the list-miners query.
- Fixed the import status message colouring spreadsheet-side errors in the success colour; errors now render in red to match backend-rejected imports.
- Fixed misleading variable naming in the date normaliser: the destructure names now match the captured positions.

### Security
- Hardened Excel/CSV import: capped accepted file size at 25 MB, validated the first two bytes of `.xlsx` files against the ZIP magic before parsing, and replaced the blocking `window.alert()` error path with an in-page status message so a malformed file can no longer wedge the WebView UI.
- Hardened backend write paths: `create_miner` and `update_miner` now reject empty `serial` values and reject `model`/`status` values that are not in the known enum set, before the SQL `CHECK` constraint would have failed with an opaque error.
- Hardened the Tauri capability set: the `tauri-plugin-sql` plugin registration is documented in `capabilities/default.json` as deliberately not granted to the frontend, so future maintainers do not assume JS-side SQL access is available.
- Documented the custom migration runner's `;` splitter limits (no SQL line comments, no semicolons inside string literals, no `BEGIN;...COMMIT;` blocks) in `db.rs` so the next migration author knows the constraint.

## [0.1.0] - 2026-05-01

### Added
- Initial internal release of Antminer Fleet Manager.
- Miner unit registry with import from CSV, TSV, and XLSX; list view with filter, sort, and pagination via a shared `DataTable`.
- Inventory tracking for parts with reorder threshold and low-stock surfacing on the dashboard.
- Local SQLite persistence (`fleet.db` in the platform app-data directory) via Tauri v2 + sqlx.
- Dashboard summary: unit count, part count, low-stock count, status breakdown, and top ten low-stock parts.

## Verification

Run from the repo root:

```powershell
npm ci                  # install exact audited dependencies
npm run build           # tsc + vite build
npm test                # frontend test suite (vitest)
npm audit --omit=dev    # dependency audit
cd src-tauri
cargo check             # backend type check
cargo test              # backend test suite
```

To launch the desktop app:

```powershell
npm run tauri:dev       # development run
npm run tauri:build     # NSIS installer (current-user install)
```

What to look for:
- The dashboard should load in the same wall-clock time as before, but with fewer SQLite round-trips per refresh.
- Importing a 30 MB XLSX should now show "File is too large. Imports are limited to 25 MB." in the status area below the import button, not block the UI.
- Importing a file that is not a real ZIP (rename any binary to `.xlsx`) should show a short "Could not read the spreadsheet" message, not a raw library stack trace.
- Re-importing a facility export that contains rows already in the database should show e.g. "Imported 12 new miners, 1,180 updated, 0 skipped" instead of "Imported 1,192 miners".
- Spreadsheet-side import errors (oversize file, corrupt ZIP, no importable rows) now render in the error colour (red) under the import button, matching the colour used for backend-rejected imports.
