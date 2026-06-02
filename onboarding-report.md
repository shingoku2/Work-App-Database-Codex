# Onboarding Report

## Summary

A new dev with prior React + Rust + Tauri experience can get to a running app in roughly 20-40 minutes (best case) on Windows. The single biggest missing thing is a normal `README.md` — the only top-level onboarding doc is `CLAUDE.md`, which opens with the line *"This file provides guidance to Claude Code (claude.ai/code)"*, which is a strong signal that it is for the AI tool, not the human. The other top-level docs (`AGENTS.md`, `GEMINI.md`) are explicitly for other AI tools and named as such. There is no human-facing README, no `docs/`, no `CONTRIBUTING.md`. The remaining onboarding experience is solid: quirks are called out, build/test commands are present, and the dual-registration oddity is explained inline.

Worst pain points: (1) no stated prerequisites, so a fresh dev may not know they need the MSVC Build Tools / WebView2 / NSIS toolchain on Windows, or webkit2gtk on Linux, before `cargo check` will succeed; (2) no troubleshooting guidance if the first `cargo check` fails; (3) the apparent test-location contradiction between CLAUDE.md and `vitest.config.ts`.

## What works well

- `CLAUDE.md` documents the genuinely non-obvious quirks (dual migration registration in `lib.rs` + `db.rs`, dual pool, non-contiguous migration numbers, custom `;` splitter limits, list-first unit registry, schema CHECK enums vs TS unions). Anyone reading it cold will not accidentally re-introduce ticketing or renumber migrations.
- Path alias and feature slicing rules are explicit (`@/*` → `src/*`, `src/features/{dashboard,inventory,miners}`).
- The "no test runner / no linter / no formatter" / "no ticketing" / "no `xlsx` package" rules are stated as hard scope rules with reasons, not as guidelines.
- The package.json `scripts` match the doc claims exactly: `dev`, `build`, `preview`, `tauri`, `tauri:dev`, `tauri:build`, `test`, `test:watch`. No drift.
- `@tauri-apps/cli` is in `devDependencies`, so `npm run tauri:dev` works without a separate `cargo install tauri-cli`. Most projects forget this.
- `sqlx::query` is used everywhere (not `sqlx::query!`), so a new dev does **not** need `sqlx-cli` or a live database to compile. Big plus.
- `read-excel-file` is pinned to an exact version (`9.0.10`) and the doc calls out the rationale (unaddressed advisory). The "do not widen to a caret range" rule is good discipline.
- `tauri-plugin-sql` is registered but explicitly **not** granted in `capabilities/default.json`, and that file has a comment pointing the reader back to CLAUDE.md for the rationale. The same quirk is documented in two places, which is what a new dev needs.
- `Cargo.lock` and `package-lock.json` are both committed (lockfile pinning is real).

## Pain points (ranked)

1. **[high] No `README.md` and no `docs/`** — `ls` at the repo root shows `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `CHANGELOG.md`, plus four `*-report.md` files. CLAUDE.md's own opening line scopes it to "Claude Code", so a new dev has to decide whether it applies to them. A normal `README.md` is the universal entry point and is missing.
   - Fix: add `README.md` that links to `CLAUDE.md` and `CHANGELOG.md` for deeper context, and restates the prerequisites and first-run commands in one place.

2. **[high] No prerequisite list anywhere** — A new dev on a clean machine will not know they need:
   - Node.js (no version specified; `package.json` engines field is absent; Vite 7, React 19, vitest 3 all need Node 20+ but this is not stated).
   - Rust toolchain (implied by `cargo check` and `cargo test` in the commands, but a new dev doesn't know if a specific version is required).
   - On Windows: MSVC Build Tools (C++ workload), WebView2 runtime (Tauri v2 prerequisite), and NSIS if they want to run `npm run tauri:build` (since the bundle target is `nsis`).
   - On Linux: `webkit2gtk-4.1`, `libssl-dev`, `libgtk-3-dev`, `librsvg2-dev`, plus `appmenu-gtk3-module` for some distros.
   - On macOS: Xcode Command Line Tools.
   - Fix: a "Prerequisites" section in README.md (or CLAUDE.md) that links to the canonical Tauri v2 prerequisites page (https://v2.tauri.app/start/prerequisites/) and notes the platform-specific gotchas.

3. **[high] No troubleshooting section for first `cargo check` failure** — Tauri v2 first builds on Windows routinely fail with linker errors, missing WebView2 SDK headers, or `sqlx` not finding sqlite. The current docs say "if it works, you're good" with no fallback. A new dev hits a cryptic Rust error, looks at CLAUDE.md, finds nothing, and goes to a senior dev (or gives up).
   - Fix: add a "Common first-build failures" section with the three or four most likely errors and what they mean (linker, WebView2, sqlite, sqlx).

4. **[medium] Test-location contradiction** — CLAUDE.md (line 32) says "Frontend tests live under `src/test/`". `vitest.config.ts` line 15 says `include: ["src/**/*.test.{ts,tsx}"]`. Both are in the repo, so a new dev can put tests next to the code they exercise (the modern convention), or in `src/test/`, and both will work. But the doc and the config disagree on which is the rule.
   - Fix: pick one. Either narrow the `include` glob to `src/test/**` to match the doc, or relax the doc to "frontend tests are configured by vitest with `include: ['src/**/*.test.{ts,tsx}']`; existing tests are under `src/test/`". The first is less surprising.

5. **[medium] Command syntax is PowerShell-only** — Every fenced block in CLAUDE.md and CHANGELOG.md is labelled `powershell`, but the commands inside (`npm run build`, `cargo check`) are also valid in bash/zsh. A Mac/Linux dev is left to figure out they can copy-paste unchanged. This is mostly cosmetic but the explicit `powershell` label will make a Linux dev pause.
   - Fix: change the fence info string to `bash` (or omit it) since none of the commands are PowerShell-specific. Keep PowerShell only for genuinely shell-specific lines.

6. **[medium] Dev port `127.0.0.1:1420` is mentioned but not explained** — The tail of CLAUDE.md (line 29) notes that `npm run dev`/`npm run preview` serve on that port, and that the app is non-functional without the Tauri shell. It does not say that `npm run tauri:dev` starts the Vite dev server automatically (via `beforeDevCommand` in `tauri.conf.json`) and connects Tauri to it. A new dev running `npm run tauri:dev` for the first time will see two processes start and may not know why.
   - Fix: one-line note that `tauri:dev` boots Vite and the Tauri shell together, and the port exists because of `tauri.conf.json`'s `devUrl` / `beforeDevCommand`.

7. **[medium] `CHANGELOG.md` Verification section uses `npm ci` but CLAUDE.md does not** — CHANGELOG.md says `npm ci` is the install command (line 56). CLAUDE.md does not say anything about installing dependencies at all. A new dev who reads CLAUDE.md first will wonder whether to run `npm install` or `npm ci`. The lockfile is committed, so `npm ci` is the right answer for reproducibility.
   - Fix: add `npm ci` to the CLAUDE.md commands section as the install step, and add an `engines` field to `package.json` for the Node version.

8. **[medium] Path-alias form inconsistency is not in CLAUDE.md** — `vite.config.ts` uses `alias: { "@": "/src" }` (absolute form), `tsconfig.json` uses `"@/*": ["src/*"]` (glob form). Both work today. `env-report.md` flags this as a maintenance hazard, but env-report is a pipeline output a new dev will not read. CLAUDE.md's "Path alias" line says the alias is "configured in both `tsconfig.json` and `vite.config.ts`" without warning that the two forms differ.
   - Fix: align both configs to the same form (the `env-report.md` M-1 fix), and remove the discrepancy so CLAUDE.md stays accurate.

9. **[low] No description of what the app actually does at the top of CLAUDE.md** — A new dev opening the file learns "Tauri v2 desktop app. React 19 + TypeScript + Vite frontend talks to a Rust backend over Tauri commands; SQLite is the only persistence layer (local file `fleet.db`)." That tells them the stack, not the product. The product is hinted at in `Cargo.toml` ("Offline Antminer repair and inventory manager") and `tauri.conf.json` (`"Antminer Fleet Manager"`) but those are not in the onboarding path.
   - Fix: add a one-line "What this is" sentence to CLAUDE.md (or the new README) — "Offline desktop tool for tracking Antminer ASIC repair jobs and replacement parts."

10. **[low] No branch / commit / PR conventions** — There is no `CONTRIBUTING.md`, no mention of branch naming, commit-message format, or PR review process. For an internal tool this may be intentional, but a new dev has no way to know whether to commit to `main`, open a PR, or use conventional commits.
    - Fix: one short paragraph in CONTRIBUTING.md or README.md. "Commit to feature branches, open a PR into `main`, no formal commit message convention required."

11. **[low] `read-excel-file` rule is in CLAUDE.md but not enforced visibly** — CLAUDE.md says "do not widen to a caret range" on `read-excel-file`, and the package.json does pin it exactly. Good. But there's no equivalent "pinned, do not widen" note for the `@tauri-apps/*` packages, even though those have had their own advisories in the past. Not a current defect, just a missed opportunity for a clear "these are pinned for a reason" list.
    - Fix: leave alone unless the project standardizes pinning.

## Specific doc changes recommended

- `C:\Users\deped\Documents\GitHub\Work-App-Codex\README.md` — **create** (does not exist). Sections: 1) What this is (one sentence), 2) Prerequisites (Node 20+, Rust stable, platform Tauri deps with link), 3) First build (`npm ci` from root, then `npm run tauri:dev`), 4) Verification (`npm run build`, `npm test`, `cd src-tauri && cargo check && cargo test`), 5) Where things live (link to CLAUDE.md architecture section), 6) Common first-build failures, 7) Pointer to CHANGELOG.md.
- `C:\Users\deped\Documents\GitHub\Work-App-Codex\CLAUDE.md` line 7-15 (Commands block) — change fence info string from `powershell` to `bash` (or drop it). Add `npm ci` as the install step. The current `npm run build` / `cargo check` / `npm test` / `cargo test` list does not say how to get the dependencies in the first place.
- `C:\Users\deped\Documents\GitHub\Work-App-Codex\CLAUDE.md` line 32 (test location) — pick one of: (a) narrow `vitest.config.ts` `include` to `src/test/**`, or (b) reword the doc to match the glob.
- `C:\Users\deped\Documents\GitHub\Work-App-Codex\CLAUDE.md` line 37 (Architecture opening) — add a one-sentence product description so a new dev knows what "Tauri v2 desktop app" is for before they read the stack breakdown.
- `C:\Users\deped\Documents\GitHub\Work-App-Codex\CLAUDE.md` line 29 (dev port note) — add one sentence clarifying that `npm run tauri:dev` boots Vite (via `beforeDevCommand`) and the Tauri shell together, so the port and the shell are not two separate things to start.
- `C:\Users\deped\Documents\GitHub\Work-App-Codex\CLAUDE.md` after the "Dependency rules" section — add a "Common first-build failures" subsection with: missing MSVC Build Tools on Windows; missing webkit2gtk on Linux; sqlx needing `runtime-tokio` (already enabled, so the failure mode is "offline build" / `SQLX_OFFLINE` if a query! macro is ever added); WebView2 not present.
- `C:\Users\deped\Documents\GitHub\Work-App-Codex\vite.config.ts` line 20 — change `alias: { "@": "/src" }` to `alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) }` to match the tsconfig glob form. (env-report.md M-1 already recommends this.)
- `C:\Users\deped\Documents\GitHub\Work-App-Codex\package.json` line 3 (after `"version"`) — add `"engines": { "node": ">=20" }` to state the Node prerequisite in a machine-readable form.

## CLAUDE.md content that was unclear or missing

- The very first sentence ("This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository") is accurate but actively confusing for a new dev opening it cold. A line like "This file also documents the project for human contributors — the rules below are binding for code changes" would close that gap.
- No statement of the project name or product purpose at the top. The product name (`Antminer Fleet Manager`) only appears in `tauri.conf.json` and the CHANGELOG, not in the file a new dev reads first.
- No link between the "first run" commands and the platform prerequisites. The reader has to know independently that Tauri v2 needs the MSVC + WebView2 combo on Windows.
- No statement of what the database file is called or where it lives on disk. The `db.rs` file shows `<app_data_dir>/fleet.db` but a new dev reading CLAUDE.md does not learn that the DB is created on first launch and lives in the OS app-data directory.
- The "Adding a Tauri command" workflow is implied ("both the function and the handler registration") but no example is given. The current commands in `src-tauri/src/commands/{miners,parts,dashboard}.rs` are the only worked example. A new dev adding their first command will probably do this fine, but a one-line template would help.
- The "Adding a migration" workflow is also implied. The "register in both `lib.rs` and `db.rs`" rule is stated but the actual mechanics (the `Migration` struct shape in `lib.rs` vs the `(i64, &str)` tuple in `db.rs`) are not — the new dev has to diff both call sites against an existing migration.
- No "project structure" section. The Architecture section names the major folders but does not show the on-disk layout. A new dev who hasn't read `src/features/inventory/` yet doesn't know whether `src/data/` (currently empty, per env-report.md L-1) is meaningful.
- The "intentional absences" (no test runner, no linter, no ticketing) are now slightly out of date: tests were added (vitest is in package.json, `src/test/` exists, CHANGELOG 0.2.0 Added line says so) but CLAUDE.md still lists "no test runner" as an absence indirectly via the `npm test` command vs. the `no linter or formatter` line. A new dev reading carefully will not be misled, but a sloppy reader could be.
- The `capabilities/default.json` description (line 4 of that file) explicitly references CLAUDE.md's "dual-registration quirk". That phrase does not appear in CLAUDE.md — CLAUDE.md describes the quirk but never names it "dual registration" in those exact words. A new dev clicking the reference may not find what they expect.

## Out of scope (do NOT flag)

- Documentation aimed at non-technical end users of the Antminer Fleet Manager desktop tool — this is an internal app for the maintainer's own use, and the project description ("Offline Antminer repair and inventory manager") confirms it.
- API references for React 19, Tauri v2, sqlx, TanStack Query, or vitest — the docs reasonably assume familiarity with the stack.
- Anything that requires launching the app to discover (visual layout, runtime errors, performance characteristics).
- Re-doing the audit reports' findings — those are already on disk and the prompt says not to surface pipeline outputs as user-facing.

## Time estimate

- **Best case (Windows dev with Node 20+, Rust stable, MSVC Build Tools, WebView2 already installed, lockfile respected):** 15-25 minutes. `npm ci` (2-3 min), `npm run tauri:dev` triggers Vite install + Tauri build (5-15 min depending on cache), window opens, dashboard loads from a freshly-created `fleet.db`. Tests run in under 30 seconds.
- **Realistic case (one or two gaps):** 1-2 hours. Most likely delays: (a) discovering the `cargo` install on a fresh Windows box and getting `rustup default stable` right; (b) hitting a first `cargo build` failure on missing sqlite or linker; (c) reading the `db.rs` `;` splitter comment to understand why a future migration needs to avoid string semicolons. None of these are blocking, but they require code-reading to recover from.
- **Worst case (clean Windows, no Rust, no Node, no NSIS):** half a day to a day. Installing rustup with the MSVC toolchain is 5-10 minutes; installing the Tauri v2 Windows prerequisites (WebView2 SDK, WiX/NSIS if you want `tauri:build`) is 15-30 minutes plus restarts. A dev who tries `npm run tauri:build` first and not `tauri:dev` will hit a missing NSIS path and lose an hour chasing the wrong error.

## Overall Onboarding Verdict

**NEEDS WORK**

A competent dev can get to a running app, but the absence of a `README.md`, the missing prerequisite list, and the PowerShell-only command fences force the dev to discover the platform-specific Tauri prerequisites the hard way. The architectural and quirk documentation in `CLAUDE.md` is unusually thorough for an internal tool — that is the strong side — but it is framed as AI-tool guidance, which makes a new dev hesitate to treat it as authoritative. Fixing this is mostly a documentation-shape problem (add README, add prerequisites, re-label fences, add a one-sentence product description). The codebase itself is not the obstacle.
