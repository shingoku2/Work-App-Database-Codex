# Changelog

All notable changes to Antminer Fleet Manager will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Added a separately installed Rust HTTPS server backed by PostgreSQL, with Debian/systemd packaging and administrative CLI commands.
- Added named accounts, Argon2id password hashing, revocable sessions, Admin/User roles, login throttling, and account administration.
- Added certificate-fingerprint pairing and certificate pinning for one configured server per desktop installation.
- Added a server-side dry-run/apply importer for existing desktop `fleet.db` files with explicit conflict policies.
- Added optimistic concurrency versions for miners, parts, and users.

### Changed
- Converted the Tauri application from a local SQLite owner into an online-required remote client.
- Moved all production migrations and database access to the central server.
- Store desktop session tokens in the operating-system credential manager.
- Store part costs as exact integer cents instead of binary floating point.
- Made spreadsheet miner import administrator-only and conflict-preserving.

### Fixed
- Enforced exact paired-certificate verification and recoverable versioned desktop server profiles.
- Added bounded source-aware login throttling and transactional final-administrator protection.
- Hardened server configuration validation and removed plaintext password CLI arguments.
- Normalized miner serials at the backend boundary so manual create/update and bulk import treat whitespace-equivalent serials as the same asset identity.
- Applied the same supported model and lifecycle-status validation to bulk imports that manual miner writes already use; invalid batches are rejected before database mutation.
- Return explicit not-found errors when an update or delete targets a miner or inventory part that no longer exists.

### Security
- Documented that builds are internal-use only until the project declares a license and assembles required third-party license attribution.

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
