# Code Audit Report

## Summary
The codebase is small, well-organized, and consistent with the documented architecture. Every Tauri command has a registered handler, every frontend `command()` call maps 1:1 to a backend `#[tauri::command]`, and every SQL string uses parameterized `?N` bindings. The audit found **0 Critical, 0 High, 7 Medium, 10 Low** findings. The largest cluster of issues is around the Excel/CSV import path (no size cap, blocking `alert()`, misnamed parser variables) and around silent inconsistencies between the form-driven write path and the import-driven write path. Migration registration parity holds; both `lib.rs` and `db.rs` use the same `include_str!` paths. TS ↔ Rust struct drift is clean for `Miner` / `Part` / `DashboardSummary`. The forbidden `xlsx` package is absent.

Files reviewed (absolute paths):
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\App.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\main.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\styles.css`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\lib\tauri.ts`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\lib\queryClient.ts`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\types\db.ts`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\components\layout\AppShell.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\components\ui\DataTable.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\components\ui\Panel.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\components\ui\StatusBadge.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\dashboard\DashboardView.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\dashboard\dashboardApi.ts`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\inventory\InventoryView.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\inventory\partApi.ts`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\minerApi.ts`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\Cargo.toml`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\lib.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\db.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\models.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\main.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\commands\mod.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\commands\miners.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\commands\parts.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\commands\dashboard.rs`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\tauri.conf.json`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\capabilities\default.json`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\migrations\0001_initial_schema.sql`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\migrations\0003_remove_ticketing.sql`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\migrations\0004_miner_import_fields.sql`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\vite.config.ts`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\tsconfig.json`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\tailwind.config.ts`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\index.html`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\package.json`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\CLAUDE.md`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\AGENTS.md`
- `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\GEMINI.md`

## Critical findings (BLOCKERS)
None.

## High findings
None.

## Medium findings

- **[M-1] Excel/CSV import has no file-size cap, no row-count cap, and uses blocking `window.alert()` for errors** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:92-104, 168-175, 314-324`
  - **Issue:** `handleImport` accepts any `File` from the `<input type="file">` element, calls `await file.text()` (full read into memory), then either runs the hand-rolled `parseDelimited` or dynamic-imports `read-excel-file/browser` and runs `readSheet(file)`. On any error in this path, the `.catch` uses `alert(String(importError))` — a blocking browser dialog that freezes the WebView.
  - **Evidence:**
    ```tsx
    async function handleImport(file: File | null) {
      if (!file) return;
      setImportMessage(null);
      const rows = await readSpreadsheetRows(file);
      const miners = rows.map(mapImportRow).filter(...);
      if (miners.length === 0) {
        throw new Error("No importable miners were found. ...");
      }
      importMutation.mutate(miners);
    }
    ```
    ```tsx
    handleImport(event.target.files?.[0] ?? null).catch((importError) => {
      setImportMessage(null);
      importMutation.reset();
      console.error(importError);
      alert(String(importError));
    });
    ```
  - **Risk:** A 500MB XLSX (which is well within Excel's spec) reads the whole file via `file.text()` and `readSheet` in one synchronous burst — DoS of the WebView and possible Tauri host OOM. The `unzipper@0.12.3` advisory noted in `dep-audit-report.md` compounds this: a maliciously crafted zip can hang or blow memory. `window.alert()` blocks the WebView thread; in a desktop shell that has no other way to dismiss it, a modal dialog can wedge the whole UI.
  - **Fix:** (1) In `MinersView.tsx`, before calling `handleImport`, check `file.size` against a sane cap (e.g. 25 MB) and reject with a status message, not `alert()`. (2) Wrap `parseDelimited` and `readSheet` in try/catch; on failure, set `importMessage` to a string and do not call `alert()`. (3) Optionally also cap the row count after `readSpreadsheetRows` returns (e.g. `rows.length > 50_000`).

- **[M-2] `import_miners` returns a count that overstates the number of unique miners upserted** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\commands\miners.rs:91-154`
  - **Issue:** The loop increments `imported += 1` for every non-empty `serial`, but the SQL is `ON CONFLICT(serial) DO UPDATE`. If a file contains the same `serial` twice (or 50 times), every duplicate row passes the `serial.trim().is_empty()` check, runs a real `INSERT ... ON CONFLICT ... DO UPDATE`, and increments the counter. The user is shown "Imported 100 miners" when only e.g. 50 unique serials were upserted.
  - **Evidence:**
    ```rust
    for miner in miners {
        if miner.serial.trim().is_empty() {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO miners ( ... )
            VALUES ( ... )
            ON CONFLICT(serial) DO UPDATE SET
                model = excluded.model,
                ...
            "#,
        )
        .bind(...)
        ...
        imported += 1;
    }
    ```
  - **Risk:** Operators re-import a facility export that contains overlap with the existing DB; they see "Imported 1,234 miners" and believe 1,234 new units were added. In reality only the duplicates were refreshed. This is a data-quality and trust issue that will silently mislead the dashboard's `unit_count` once the user notices it does not match the new total.
  - **Fix:** Track `INSERT` vs `UPDATE` separately. With SQLite, check the row count returned from `execute`: `result.rows_affected()` only tells you rows touched. The cleanest fix is to pre-deduplicate by serial in Rust (e.g. `let mut seen = HashSet::new(); miners.into_iter().filter(|m| seen.insert(m.serial.trim().to_string()))`), or change the return type to `{ inserted: i64, updated: i64, skipped: i64 }` and increment each bucket accordingly. Update `MinerImportResult` in `src-tauri/src/models.rs` and the TS type in `src/features/miners/minerApi.ts` in lockstep.

- **[M-3] Backend writes do not validate `serial` (empty string accepted as primary key) and do not validate `model` / `status` against the enum before round-tripping the SQL CHECK** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\commands\miners.rs:18-88, 156-165`
  - **Issue:** `CreateMiner` and `UpdateMiner` deserialise `serial: String` (non-Option) but the command never trims it or rejects `""`. The DB schema enforces `serial TEXT NOT NULL UNIQUE` but not non-empty. The frontend's `<input required>` hides this — but a direct `invoke("create_miner", ...)` from the WebView console (or any future IPC consumer) can submit `serial: ""` and silently create a row with an empty primary key, which then collides on the next empty-serial insert with a generic `UNIQUE constraint failed` error. Similarly, `model` and `status` are typed as `String` on the Rust side; only the SQL `CHECK` saves us, and the failure surface there is an opaque constraint error.
  - **Evidence:**
    ```rust
    #[derive(Debug, Deserialize)]
    pub struct CreateMiner {
        pub serial: String,
        pub model: String,
        ...
        pub status: String,
        ...
    }
    ```
  - **Risk:** A blank `serial` is a permanent garbage row that is hard to delete (the dashboard would show it, the user could not easily find it). The opaque CHECK failure is shown to the user as raw text, which is poor UX.
  - **Fix:** In `create_miner` and `update_miner`, before binding, return `Err("serial must not be empty".to_string())` if `input.serial.trim().is_empty()`. Optionally also validate `input.model` is in the known set and `input.status` likewise; the easiest way is to mirror the SQL CHECK as a `const &[&str]` slice and check membership in Rust. This is defense-in-depth — the SQL CHECK is still the source of truth.

- **[M-4] Form-driven write path stores `""` for blank optional fields, while import-driven write path stores `null`** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:14-32, 210, 292-312, 405-421`
  - **Issue:** `emptyForm` initialises every optional field as `""`; `minerToForm(miner)` also maps `null -> ""` via `miner.firmware ?? ""`. The `Miner` type in `src/types/db.ts` declares `firmware: string | null` (and similarly for the other 11 optional columns). The Rust `Miner` struct uses `Option<String>`, so `Some("")` is what gets stored. Meanwhile `mapImportRow` uses `nullable(value(...))`, which returns `null` for blank. Result: a hand-entered unit has `firmware = ""`; an imported unit has `firmware = null`. The dashboard and DataTable do not distinguish between the two (`row.original.firmware || "-"`), so the inconsistency is invisible until a query does.
  - **Evidence:**
    ```tsx
    const emptyForm: CreateMinerInput = {
      serial: "",
      model: "S21",
      firmware: "",
      client_name: "",
      ...
    };
    function minerToForm(miner: Miner): CreateMinerInput {
      return {
        ...
        firmware: miner.firmware ?? "",
        ...
      };
    }
    ```
  - **Risk:** When (eventually) a query wants to know "how many units have no firmware set", `firmware IS NULL` and `firmware = ''` would give different answers, and the form-created rows would be in the wrong bucket. Latent data-quality issue.
  - **Fix:** Pick one canonical form value for "blank" and apply it consistently. The simplest: in `minerToForm` and `emptyForm`, use `null` instead of `""` for optional fields, and convert the controlled inputs to handle `null` (the form's `value={form.firmware ?? ""}` already does this; just have `setForm` write `null` for the empty case, e.g. `event.target.value === "" ? null : event.target.value`). This makes the form path match the import path.

- **[M-5] `parseDelimited` splits on raw `;` with no SQL-comments awareness in the migration runner, and the `read-excel-file` path does not validate the parsed row shape** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\db.rs:71-79`
  - **Issue:** The custom migration runner does `sql.split(';').map(str::trim).filter(|s| !s.is_empty())` and runs each chunk. If a future migration contains a SQL line comment like `-- this; not a separator` or a string literal containing `;`, the splitter will mangle it. Today's three migrations (0001, 0003, 0004) are comment-free so this is latent — but the same problem also affects any migration that uses `BEGIN; ... COMMIT;` (SQLite would need a multi-statement block, not a single `query` call).
  - **Evidence:**
    ```rust
    async fn run_migration(pool: &DbPool, sql: &str) -> Result<(), String> {
        for statement in sql.split(';').map(str::trim).filter(|statement| !statement.is_empty()) {
            if let Err(error) = sqlx::query(statement).execute(pool).await {
                let message = error.to_string();
                if !message.contains("duplicate column name") {
                    return Err(format!("Failed to apply database migration: {message}"));
                }
            }
        }
        Ok(())
    }
    ```
  - **Risk:** A future migration with a `;` inside a string literal or a `BEGIN;...COMMIT;` block will be silently split. Combined with the "swallow `duplicate column name`" logic, the failure could be hidden under a partial-success state. Difficult to debug.
  - **Fix:** Either (a) replace the `split(';')` with a small SQL tokeniser that knows about `--` comments and `'`/`"` strings, or (b) require each migration file to contain exactly one statement and drop the splitter. Option (b) is simpler and matches what `0001` and `0003` already do — `0004` is the only multi-statement one. If you keep the splitter, document its limits in a comment.

- **[M-6] Dashboard issues 4 separate `pool.inner().fetch_*` calls instead of one query (and never uses `JOIN`)** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\commands\dashboard.rs:7-54`
  - **Issue:** `get_dashboard_summary` issues 4 round-trips to SQLite: `COUNT(*) FROM miners`, `COUNT(*) FROM parts`, `COUNT(*) FROM parts WHERE qty_on_hand <= reorder_threshold`, then a `SELECT status, COUNT(*) GROUP BY status`, then a 10-row `SELECT ... FROM parts WHERE qty_on_hand <= reorder_threshold`. The first three counts are independent and could be one `SELECT (SELECT COUNT(*) FROM miners), (SELECT COUNT(*) FROM parts), ...` query. The two parts queries (`COUNT(*)` and the 10-row list) are both filtered by the same predicate.
  - **Evidence:**
    ```rust
    let unit_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM miners").fetch_one(pool.inner()).await.map_err(|error| error.to_string())?;
    let part_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM parts").fetch_one(pool.inner()).await.map_err(|error| error.to_string())?;
    let low_stock_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM parts WHERE qty_on_hand <= reorder_threshold").fetch_one(pool.inner()).await.map_err(|error| error.to_string())?;
    let units_by_status = sqlx::query_as::<_, CountByStatus>("SELECT status, COUNT(*) AS count FROM miners GROUP BY status ORDER BY status").fetch_all(pool.inner()).await.map_err(|error| error.to_string())?;
    let low_stock_parts = sqlx::query_as::<_, Part>("SELECT ... FROM parts WHERE qty_on_hand <= reorder_threshold ORDER BY qty_on_hand ASC, name ASC LIMIT 10").fetch_all(pool.inner()).await.map_err(|error| error.to_string())?;
    ```
  - **Risk:** Five queries on every dashboard load. For a single-user offline app this is fine, but the dashboard is a high-frequency surface and 4× the round-trips is wasteful. More importantly: if any one of the four `low_stock_count` style queries fails, the entire `get_dashboard_summary` returns an error and the dashboard shows "not available" — even though three of the four numbers would be perfectly valid.
  - **Fix:** Collapse the three scalar counts into a single `SELECT (SELECT COUNT(*) FROM miners) AS unit_count, (SELECT COUNT(*) FROM parts) AS part_count, (SELECT COUNT(*) FROM parts WHERE qty_on_hand <= reorder_threshold) AS low_stock_count` and bind into three scalars or a small struct. Keep the two list queries separate (they return different shapes).

- **[M-7] `readSpreadsheetRows` blindly trusts the file extension and the XLSX parser for the entire import payload** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:314-324, 96-104`
  - **Issue:** `file.name.split(".").pop()?.toLowerCase()` is the only gate on which parser path is taken. There is no `Content-Type` check, no magic-bytes sniff, and no try/catch around the XLSX path. A `.xlsx` file that is actually corrupted (zip with bad central directory — the failure mode most associated with the `unzipper` advisory) will throw an unhandled exception inside `readSheet`, which the outer `.catch` does catch — but the user is then shown `alert(String(error))` which can include parser internals (file offsets, library names) that are noise to the user.
  - **Evidence:**
    ```tsx
    async function readSpreadsheetRows(file: File): Promise<ImportRow[]> {
      const extension = file.name.split(".").pop()?.toLowerCase();
      if (extension === "csv" || extension === "tsv") {
        const delimiter = extension === "tsv" ? "\t" : ",";
        return rowsToObjects(parseDelimited(await file.text(), delimiter));
      }
      const { readSheet } = await import("read-excel-file/browser");
      return rowsToObjects(await readSheet(file));
    }
    ```
  - **Risk:** A maliciously crafted `.xlsx` triggers a parser error or hang (the `unzipper` advisory upstream). Even a benign corrupted file produces a wall of text in `alert()`. Combined with M-1 (no size cap), this is a small surface but with a known-bad dependency chain.
  - **Fix:** (1) Wrap the `readSheet` call in try/catch and translate parser errors to a short message like "Could not read the spreadsheet. The file may be corrupt or in an unsupported format." (2) Validate the file's first 2 bytes against the ZIP magic `0x50 0x4B` before passing to `readSheet` (this is what `unzipper` will read first). (3) Set the size cap from M-1.

## Low findings

- **[L-1] `miners.rs:11` uses `format!` for a constant SQL concatenation** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\commands\miners.rs:7,11`. The `MINER_SELECT` is a `const &str` and `" ORDER BY serial"` is a literal; the `format!` macro does runtime allocation for no reason and is a misleading pattern (it implies string interpolation of user data). Fix: `const LIST_MINERS_SQL: &str = concat!(MINER_SELECT, " ORDER BY serial");` and use that.

- **[L-2] `Cargo.toml` declares `chrono` and `thiserror` as direct dependencies that are never imported** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\Cargo.toml:21-22`. `chrono` and `thiserror` are listed but no `.rs` file in `src-tauri/src/` does `use chrono` or `use thiserror`. Drop them. The `chrono` feature on `sqlx` is a separate finding already covered by `dep-audit-report.md` M-1.

- **[L-3] `MinersView.tsx` calls `value(row, "status")` twice for the same row** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:399,418`. The result of `value(row, "status")` is needed once for the `rawStatus` fallback and again as the second argument of `normalizeStatus`. Cache it: `const statusValue = value(row, "status");` then use `statusValue` in both places. Pure cleanup, no behavior change.

- **[L-4] `value()` does an O(n) `Object.keys(row).find(...)` on every call, and the same `normalizeKey` target is recomputed for every row** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:424-432`. For a 10,000-row import with ~25 fields, that's 250,000 key normalisations. Build a `Map<normalizedKey, originalKey>` once per row at the top of `mapImportRow`. No correctness impact, but the import is noticeably slower than necessary.

- **[L-5] `normalizeDate` parameter naming is wrong (US `mm/dd/yyyy` is being interpreted as `dd/mm/yyyy`)** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:461-471`. The destructured names are `month, day, year`, but the final format is `${fullYear}-${month.padStart(2,"0")}-${day.padStart(2,"0")}`. For input `"1/2/2024"`, the function returns `"2024-01-02"` (interpreted as Jan 2, 2024 in `YYYY-MM-DD`). The regex matches US format (`mm/dd/yyyy`) but the output uses the captures as if they were `dd/mm/yyyy` — the destructured name `month` is a lie. Either swap the order or rename the destructures. Out-of-range values like `13/45/2024` are not rejected and will be stored as a nonsensical date string.

- **[L-6] `parseDelimited` is character-by-character with no encoding handling and will mis-parse UTF-16 / Windows-1252 files** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:344-390`. The function reads `await file.text()`, which the WebView decodes as UTF-8. A facility export from older Windows tooling may be UTF-16-LE or cp1252; the result is mojibake. Low priority for an internal Antminer fleet tool, but worth documenting that the import expects UTF-8.

- **[L-7] No file-size or row-count cap means a 500MB XLSX is read into memory in one go** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:96, 314-323`. Cross-references M-1 and M-7. Listed as Low here because the medium findings already capture the security and stability angle; this is the more granular "the function will literally read a 1GB file into a JS string" reminder.

- **[L-8] `tauri-plugin-sql` is registered in `lib.rs:32-36` but no `sql:default` capability is granted and no Rust code uses the plugin's runtime** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\lib.rs:32-36` vs `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\capabilities\default.json:6`. Already called out by `env-report.md` M-2. The dual-registration quirk (this plugin + the custom `sqlx` pool in `db.rs`) is documented in `CLAUDE.md`. If the plugin stays, annotate the capability file with a comment explaining the deliberate omission. If you want to drop it, also remove the `tauri-plugin-sql = { version = "2", features = ["sqlite"] }` line and the `tauri_plugin_sql::Builder` block; `init_pool` already does the real work.

- **[L-9] `tauri::Builder` is missing `tauri::generate_context!()` arguments and a CSP-relevant `default` capability match check** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\lib.rs:58`. `generate_context!` is invoked with no path; the default is the `tauri.conf.json` next to `Cargo.toml`, which is what we want. The `windows: ["main"]` in `default.json` matches the single window in `tauri.conf.json`. The CSP allows `data:` in `img-src`; the app does not currently render any user-provided images, so this is theoretical, but it is an XSS-enabling lever if a future feature adds profile images. Tighten to `img-src 'self' asset:` unless `data:` is needed.

- **[L-10] `readSheet` is dynamically imported but the path is a bare string, with no type guarantee that the `read-excel-file/browser` subpath exists in the installed version** — `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\features\miners\MinersView.tsx:322`. The `package-lock.json:2693-2706` resolves `read-excel-file@9.0.10` whose `exports` map is not visible in the lockfile, only in the published `package.json` on npm. If a future lockfile update changes the package to a version that renames the subpath, the dynamic import silently fails at runtime. Pin the version exactly (`9.0.10` not `^9.0.10`) in `package.json` if you cannot validate the subpath. (Touches `dep-audit-report.md` M-3 too.)

## Cross-reference to upstream reports

- **`dep-audit-report.md`:**
  - **HIGH (unzipper 0.12.3 transitive via `read-excel-file`)** — directly compounds this audit's **M-1** (no file-size cap) and **M-7** (no error wrapping around the XLSX path). A malicious or malformed `.xlsx` is the most likely real-world trigger of an `unzipper` failure. The fixes in M-1/M-7 reduce the blast radius even if the dep stays.
  - **MED (sqlx `chrono` feature unused)** — confirmed. Additionally, this audit finds **L-2** (the direct `chrono = "0.4"` dependency in `Cargo.toml` is also unused — no `use chrono::` anywhere in `src-tauri/src/`). Drop both.
  - **MED (tauri-plugin-sql 2.4.0 behind 2.11.x)** — confirmed; the plugin is registered in `lib.rs:32-36` but never used at runtime. Bumping the version is moot if the plugin is removed (see **L-8**). If kept, bump.
  - **MED (`read-excel-file` unmaintained)** — see **L-10**; the dynamic subpath import is fragile.
  - **MED (caret-pinned frontend deps)** — confirmed; not changed in this audit.
  - **LOW (lucide-react churn, rolldown pre-release)** — no code impact found.

- **`env-report.md`:**
  - **M-1 (path-alias form mismatch Vite vs tsconfig)** — confirmed in this audit: `vite.config.ts:20` uses `"/src"`, `tsconfig.json:20` uses `"src/*"`. All `@/...` imports in `src/` are consistent with both forms, so this is a config-only style drift. No code change required, but the fix proposed in `env-report.md` is sound (`vite-tsconfig-paths` to derive Vite's alias from tsconfig).
  - **M-2 (tauri-plugin-sql registered but no capability granted)** — see **L-8** above. No code uses the plugin; recommend either removal or annotation.
  - **L-1 (empty `src/data/`)** — confirmed; still empty, can be deleted.
  - **L-2 (Tailwind `require()` in ESM config)** — confirmed at `tailwind.config.ts:20`; replace `require("tailwindcss-animate")` with `import animate from "tailwindcss-animate"` and reference `animate` in the `plugins` array.
  - **L-3 (`index.html` lacks `__TAURI__` global)** — confirmed correct for Tauri v2; no action.

## Recommended fix order

1. **M-1 + M-7** (file-size cap, error wrapping, drop `alert()`). One cluster: tighten the import boundary so a bad XLSX cannot DoS the WebView and cannot wedge the UI with a blocking dialog. High value, small diff in `MinersView.tsx`.
2. **M-2** (return insert/update breakdown from `import_miners`). Aligns the operator-visible count with reality; touches `models.rs` and `minerApi.ts` in addition to `miners.rs`.
3. **M-3** (validate `serial` and the enum fields in `create_miner` / `update_miner`). Cheap defense-in-depth; prevents the "ghost unit" failure mode.
4. **M-4** (form should write `null`, not `""`, for blank optional fields). One-file refactor in `MinersView.tsx`; makes future reporting queries correct.
5. **M-6** (collapse the dashboard's 4 round-trips into fewer queries). Performance and partial-failure resilience; also touches `dashboard.rs` and the `DashboardSummary` struct.
6. **M-5** (migration runner's `;` split) and **L-1** (`format!` for const SQL concat) — clean-up; fix together because they are both in `db.rs` / `miners.rs`.
7. **L-2, L-3, L-4, L-5, L-6, L-10** — code-quality pass. Do as a single PR after the medium fixes land.
8. **L-8, L-9** (tauri-plugin-sql removal-or-annotation, CSP tightening) — small; bundle with the dependency bump from `dep-audit-report.md` MED-2.

## Out of scope (do NOT add to the list)
- Adding tests, linter, or formatter (per CLAUDE.md scope rule).
- Re-introducing ticketing / technician / repair_parts tables (per CLAUDE.md scope rule; the drops are in `0003`).
- Moving edit forms back into the list view (per CLAUDE.md UX rule; the `MinerDetailView` is the correct shape).
- License-compliance-for-distribution concerns (internal app; dep-audit confirmed no GPL family).
- "Use `sqlx::query!` compile-time-checked macros" — the schema is created at runtime from a single `fleet.db` file; compile-time SQL checking requires `cargo sqlx prepare` and a checked-in `.sqlx` directory, which is a much larger change than any finding here.
