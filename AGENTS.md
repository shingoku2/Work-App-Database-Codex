# AGENTS.md

## Project Context

This is a Tauri v2 desktop app for offline Antminer asset management and inventory tracking. The frontend is React + TypeScript + Vite, with TanStack Query/Table and Tailwind styling. The backend is Rust with SQLite via `sqlx` and `tauri-plugin-sql`.

The app intentionally no longer includes a ticketing or technician workflow. Keep future work focused on:

- Miner/ASIC asset registry
- Miner detail pages and editable asset metadata
- CSV/TSV/XLSX miner import
- Parts inventory and reorder tracking
- Dashboard summaries for units and inventory

## Key Paths

- Frontend entry: `src/App.tsx`
- Shell/nav: `src/components/layout/AppShell.tsx`
- Shared table: `src/components/ui/DataTable.tsx`
- Miner UI: `src/features/miners/MinersView.tsx`
- Miner API: `src/features/miners/minerApi.ts`
- Inventory UI: `src/features/inventory/InventoryView.tsx`
- Shared types: `src/types/db.ts`
- Rust commands: `src-tauri/src/commands/`
- Rust models: `src-tauri/src/models.rs`
- Migrations: `src-tauri/migrations/`

## Current Miner Import Behavior

The Unit Registry supports importing `.csv`, `.tsv`, and `.xlsx` files. It maps rows into miner records and upserts by `miner_serial`, so re-importing an updated facility export refreshes existing assets instead of duplicating them.

Expected import columns include:

- `client_name`
- `miner_type`
- `miner_ip`
- `miner_mac`
- `miner_serial`
- `firmware_version`
- `pickaxe`
- `miner_state`
- `miner_row`
- `miner_index`
- `miner_rack`
- `miner_rack_group`

Extra export columns are allowed. Some are folded into `notes`, such as miner id, miner name, raw status, tags, PSU serial, control board, wattage, hash rate, max temp, and last update.

## Database Notes

The miner table has migration-added fields for imported facility metadata:

- `client_name`
- `miner_type`
- `ip_address`
- `mac_address`
- `pickaxe`
- `miner_state`
- `miner_row`
- `miner_index`
- `miner_rack`
- `miner_rack_group`

Migration `0003_remove_ticketing.sql` drops legacy ticket/technician tables from existing local databases. Do not reintroduce ticket tables unless explicitly requested.

## UX Notes

The Unit Registry is list-first. Clicking a miner row opens a dedicated detail/edit page. Adding a new miner also opens that same detail-style page. Avoid bringing the full edit form back into the registry list view.

The shared `DataTable` supports filtering, sorting, page size selection, direct page jump, first/previous/next/last controls, and optional row-click behavior.

## Verification

Use these checks after code changes:

```powershell
npm run build
cargo check
npm audit --omit=dev
```

Run `cargo check` from `src-tauri`.

To launch the desktop app:

```powershell
npm run tauri:dev
```

## Dependency Notes

Do not use the `xlsx` package unless there is a strong reason and the security advisory has been addressed. The current Excel import uses `read-excel-file`, and CSV/TSV parsing is implemented locally.
