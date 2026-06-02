# Environment & Config Validation Report

## Summary
Configuration layer is internally consistent. Frontend, backend, and Tauri shell agree on port 1420, DB path, command set, migration set, and bundle target. No secrets, no `.env` files, no hardcoded paths. One minor style nit on the path-alias form. Migration list (lib.rs) and custom runner (db.rs) are in sync. All frontend `command(...)` calls map 1:1 to `#[tauri::command]` entries in `invoke_handler!`. The deliberate absences called out in CLAUDE.md are respected (no test runner, no linter/formatter, non-contiguous migration versions).

Severity counts: Critical 0, High 0, Medium 2, Low 3.

---

## Critical findings (blockers)
None.

---

## High findings
None.

---

## Medium findings

**M-1 — Path alias form differs between Vite and tsconfig**
- **Location:** `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\vite.config.ts` line 20 vs `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\tsconfig.json` line 20.
- **Issue:** Vite uses `alias: { "@": "/src" }` (absolute form), TypeScript uses `"@/*": ["src/*"]` (glob form). Both resolve correctly so this is a style inconsistency, not a defect.
- **Impact:** Risk of confusion when extending the alias; one config breaks while the other still type-checks. No runtime impact today.
- **Fix:** Pick one form. Recommend matching the tsconfig glob in Vite: `alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) }` or use `vite-tsconfig-paths` to derive Vite aliases from tsconfig.

**M-2 — `tauri-plugin-sql` is registered but capability set does not grant it**
- **Location:** `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\src\lib.rs` line 32 (plugin registered) vs `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src-tauri\capabilities\default.json` line 6 (`permissions: ["core:default"]`).
- **Issue:** `tauri-plugin-sql` is added as a plugin and migrations are registered against `sqlite:fleet.db`, but the capability only allows `core:default`. The frontend does not call into `tauri-plugin-sql` (it goes through custom Rust commands using its own `sqlx::SqlitePool`), so there is no live bug — the plugin is effectively registered but unused. This is a coupling risk: if the plugin stays registered with no capability, future maintainers may assume the frontend can use `@tauri-apps/plugin-sql` and silently break.
- **Impact:** Maintenance hazard, not a runtime failure. The double-registered migration path (plugin + db.rs) is also a documented quirk — the plugin's pool is never actually connected to from JS.
- **Fix:** Either (a) drop `tauri-plugin-sql` from `Cargo.toml` and `lib.rs` since the app uses its own `sqlx` pool exclusively, or (b) leave the plugin and add a comment in `capabilities/default.json` noting that SQL access from the frontend is intentionally disabled. CLAUDE.md already documents the dual-registration as intentional, so this is a tracking-only item.

---

## Low findings / observations

**L-1 — `src/data/` directory is empty**
- **Location:** `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\src\data\`
- **Issue:** Empty directory, no contents.
- **Impact:** None — likely a placeholder for future use.
- **Fix:** Remove if unused, or seed with a `.gitkeep`.

**L-2 — Tailwind plugin loaded via CJS `require()` in an ESM module**
- **Location:** `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\tailwind.config.ts` line 20.
- **Issue:** `package.json` declares `"type": "module"`, but `tailwind.config.ts` uses `require("tailwindcss-animate")`. Tailwind's own config loader tolerates this, but the use of `require` in an ESM project is a small style smell.
- **Impact:** Works today. Will break if Tailwind is ever loaded through a strict ESM pipeline.
- **Fix:** `import animate from "tailwindcss-animate"` and reference `animate` in `plugins: [animate]`.

**L-3 — `index.html` does not reference the Tauri global**
- **Location:** `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex\index.html`.
- **Issue:** The audit prompt asked for a Tauri-injected `__TAURI__` script tag. In Tauri v2 this is no longer the default — the `@tauri-apps/api` ESM imports (used via `src/lib/tauri.ts`) work without the global. No `withGlobalTauri` is set, so the omission is correct for v2.
- **Impact:** None.
- **Fix:** No action.

---

## Intentional absences (do NOT flag)
- No test runner, no linter, no formatter — explicit CLAUDE.md scope rule.
- No `.env*` files and no `.env.example` — single-user offline workstation with no runtime configuration knobs. No secrets in the bundle. This is correct for the product shape.
- Non-contiguous migration versions 1, 3, 4 (0002 was removed) — documented in CLAUDE.md. Both registration sites (`lib.rs` and `db.rs`) agree on the same set, so the gap is intentional and consistent.
- `dist/` is correctly ignored via `.gitignore`. Build output is not committed.
- `node_modules/`, `src-tauri/target/`, `src-tauri/gen/` are ignored.

---

## Recommended actions
1. (Optional, M-1) Align the Vite and tsconfig path-alias forms so future maintainers do not need to keep them in mental sync.
2. (Optional, M-2) Decide whether `tauri-plugin-sql` is staying in the binary. The Rust backend uses its own `sqlx` pool exclusively; removing the plugin would shrink the bundle and remove the dual-migration surface entirely. If it stays, annotate `capabilities/default.json` so the next person does not assume frontend SQL access is allowed.
3. (Cosmetic, L-2) Migrate `tailwind.config.ts` to an ESM `import` for the animate plugin.
4. (Cosmetic, L-1) Remove the empty `src/data/` directory or commit a `.gitkeep`.
5. (None) No security action items — no secrets found, no hardcoded credentials, capability set is the v2 default and matches the actual surface area used.

---

## Verdict

**ISSUES FOUND** — but only at Medium and Low severity. The configuration layer is internally consistent, secrets-free, and the app will start and run. The two Medium items are maintenance/clarity concerns, not deploy blockers. No Critical, no High.
