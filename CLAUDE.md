# CLAUDE.md

This file documents the project for both human contributors and the Claude Code CLI. The rules below are binding for code changes.

**What this is:** offline desktop tool for tracking Antminer ASIC units (registry, lifecycle status, location, firmware) and replacement parts inventory. See [README.md](./README.md) for prerequisites and first-build steps; see [CHANGELOG.md](./CHANGELOG.md) for release history.

## Commands

Install exact audited dependencies (lockfile is committed):

```bash
npm ci
```

Verification after code changes (run from the repo root):

```bash
npm run build      # tsc + vite build
npm test           # vitest run (frontend, jsdom)
npm audit --omit=dev
cd src-tauri
cargo check        # backend type check
cargo test         # backend test suite
```

Launch the desktop app (boots Vite and the Tauri shell together via `beforeDevCommand`):

```bash
npm run tauri:dev
```

Production bundle (NSIS installer, current-user install):

```bash
npm run tauri:build
```

`npm run dev` / `npm run preview` only serve the Vite frontend on `127.0.0.1:1420`; the app is non-functional without the Tauri shell because all data access goes through `invoke`. `tauri:dev` is the normal entry point — it spawns Vite on that port and opens the Tauri window, so the dev only ever runs one command.

There is no linter or formatter configured. Do not add one without being asked.

Frontend tests live under `src/test/` and are picked up by vitest's `include` glob `src/test/**/*.test.{ts,tsx}`. JSDOM 25 is missing `Blob.arrayBuffer()` and `Blob.text()` so `src/test/setup.ts` polyfills them via `FileReader`. The `tauri-plugin-sql` plugin is registered in `lib.rs` but is intentionally not granted in `capabilities/default.json` — all DB access still flows through the custom Rust commands.

## Architecture

Tauri v2 desktop app. React 19 + TypeScript + Vite frontend talks to a Rust backend over Tauri commands; SQLite is the only persistence layer (local file `fleet.db` in the OS app-data directory).

### Frontend → backend boundary

- All backend calls go through `command<T>(name, args)` in `src/lib/tauri.ts` (thin wrapper around `@tauri-apps/api/core` `invoke`). Do not call `invoke` directly elsewhere.
- All data fetching/mutation uses TanStack Query. The shared `QueryClient` lives in `src/lib/queryClient.ts`.
- Path alias `@/*` → `src/*` (configured in both `tsconfig.json` and `vite.config.ts`).
- Frontend is feature-sliced under `src/features/{dashboard,inventory,miners}`. Each feature owns its `*Api.ts` (TanStack-friendly functions wrapping `command()`) and its view component. Larger features split non-view helpers into sibling modules (e.g. `src/features/miners/import.ts` holds the CSV/TSV/XLSX import helpers extracted from `MinersView.tsx`). Shared UI lives in `src/components/`.
- The shared `DataTable` (`src/components/ui/DataTable.tsx`) handles filtering, sorting, page size, page jump, first/prev/next/last, and optional row-click — reuse it rather than building new tables.

### Database layer (important quirk)

Migrations are registered in **two places** and both must be updated when adding a new one:

1. `src-tauri/src/lib.rs` — registers the `tauri-plugin-sql` migration list for `sqlite:fleet.db`.
2. `src-tauri/src/db.rs` — `init_pool` opens its own `sqlx::SqlitePool` against `<app_data_dir>/fleet.db` and runs the same migrations through a custom `schema_migrations` table. This is the pool that backend commands actually use (managed via `handle.manage(pool)` and injected as `State<'_, DbPool>`).

The custom runner in `db.rs` swallows `duplicate column name` errors so re-running `ALTER TABLE … ADD COLUMN` migrations is safe, but other failures propagate. Migration versions are non-contiguous (1, 3, 4) because `0002` was removed; don't renumber, just append.

### Backend commands

Every frontend operation maps to a `#[tauri::command]` in `src-tauri/src/commands/{miners,parts,dashboard}.rs`, registered in `src-tauri/src/lib.rs`'s `invoke_handler!`. Adding a command requires both the function and the handler registration. Rust models in `src-tauri/src/models.rs` mirror TypeScript interfaces in `src/types/db.ts` — keep them in sync.

### Schema constraints

`miners.model` and `miners.status` are `CHECK`-constrained enums (see `0001_initial_schema.sql`); the TypeScript `MinerModel` / `MinerStatus` / `PartCategory` unions in `src/types/db.ts` mirror them. Widening the enum requires changing the SQL CHECK, the Rust model, and the TS type together.

`miner_serial` is the import upsert key — `import_miners` does `INSERT … ON CONFLICT(serial) DO UPDATE`, so re-importing a facility export refreshes existing rows.

## Scope and product rules

- This app intentionally has **no ticketing or technician workflow**. Migration `0003_remove_ticketing.sql` drops the legacy tables. Do not reintroduce ticket/technician/repair_parts tables unless explicitly asked.
- Unit Registry is **list-first**: clicking a miner row (or "add new") opens a dedicated detail/edit page. Do not move the full edit form back into the list view.
- Miner import supports `.csv`, `.tsv`, `.xlsx`. Expected columns: `client_name`, `miner_type`, `miner_ip`, `miner_mac`, `miner_serial`, `firmware_version`, `pickaxe`, `miner_state`, `miner_row`, `miner_index`, `miner_rack`, `miner_rack_group`. Extra columns (miner id, miner name, raw status, tags, PSU serial, control board, wattage, hash rate, max temp, last update) are folded into `notes`.

## Dependency rules

- Do **not** add the `xlsx` npm package — it has an unaddressed security advisory. Excel parsing uses `read-excel-file` (pinned to exact `9.0.10` in `package.json`; do not widen to a caret range); CSV/TSV parsing is implemented locally.
- Tailwind for styling; prefer `clsx` + `tailwind-merge` for conditional classes.
