# Antminer Fleet Manager

Offline desktop tool for tracking Antminer ASIC units (registry, lifecycle status, location, firmware) and replacement parts inventory. Tauri v2 + React 19 + Rust + SQLite. Single-user, local-only — the database is a `fleet.db` file in the OS app-data directory.

For project rules, architecture, and the dual-migration / dual-pool quirk, see [CLAUDE.md](./CLAUDE.md). For release history, see [CHANGELOG.md](./CHANGELOG.md).

## Prerequisites

- **Node.js 20 or newer** (Vite 7, React 19, vitest 3 all require it).
- **Rust stable** (install via [rustup](https://rustup.rs/); `rustup default stable` is enough).
- **Tauri v2 platform dependencies** — see <https://v2.tauri.app/start/prerequisites/> for the canonical list. In short:
  - **Windows:** [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (the "Desktop development with C++" workload) plus the [WebView2 runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (preinstalled on Windows 11). `npm run tauri:build` additionally needs [NSIS](https://nsis.sourceforge.io/) in PATH.
  - **Linux:** `webkit2gtk-4.1`, `libssl-dev`, `libgtk-3-dev`, `librsvg2-dev`, plus `appmenu-gtk3-module` on some distros.
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`).

## First build

```bash
npm ci                 # install exact audited deps (uses committed package-lock.json)
npm run tauri:dev      # boots Vite + the Tauri shell; the app window opens
```

`npm run tauri:dev` starts both processes: Vite on `127.0.0.1:1420` (via `beforeDevCommand` in `src-tauri/tauri.conf.json`) and the Tauri shell that loads the WebView. The first build can take 5-15 minutes while cargo fetches and compiles dependencies; subsequent runs use the cache and are much faster.

On first launch the app creates `fleet.db` in the platform app-data directory (Windows: `%APPDATA%\com.local.antminerfleet\`).

## Verification

Run from the repo root:

```bash
npm run build          # tsc + vite build
npm test               # vitest run (frontend, jsdom)
npm audit --omit=dev   # dependency audit
```

Run from `src-tauri/`:

```bash
cargo check            # backend type check
cargo test             # backend test suite
```

## Production bundle

```bash
npm run tauri:build    # NSIS installer (current-user install) into src-tauri/target/release/bundle/
```

## Where things live

- `src/features/{dashboard,inventory,miners}/` — feature-sliced UI, each owns its `*Api.ts` and view.
- `src/components/ui/` — shared UI (`DataTable`, `Panel`, `StatusBadge`).
- `src/types/db.ts` — TypeScript interfaces mirroring the Rust models in `src-tauri/src/models.rs`.
- `src-tauri/src/commands/` — one file per backend domain (`miners.rs`, `parts.rs`, `dashboard.rs`); each `#[tauri::command]` is registered in `src-tauri/src/lib.rs`.
- `src-tauri/migrations/` — numbered SQL files, applied in order.

See [CLAUDE.md](./CLAUDE.md) for the full architecture writeup, including the **dual-registration quirk** (migrations and the SQLite pool are registered in both `lib.rs` and `db.rs` and must stay in sync) and the **list-first unit registry** rule.

## Common first-build failures

- **Windows: `linker 'link.exe' not found`** — install the "Desktop development with C++" workload via the Visual Studio Build Tools installer. Rust on Windows uses the MSVC toolchain by default.
- **Windows: WebView2 loader errors** — install the [WebView2 Evergreen Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). Windows 11 has it preinstalled; Windows 10 may need an explicit install.
- **Linux: `pkg-config` can't find `webkit2gtk-4.1`** — install the package listed in the Tauri v2 prerequisites for your distro. The package name differs between Debian/Ubuntu (`libwebkit2gtk-4.1-dev`), Fedora (`webkit2gtk4.1-devel`), and Arch (`webkit2gtk-4.1`).
- **Linux: `failed to run custom build command for openssl-sys`** — install `libssl-dev` (or the distro equivalent).
- **`tauri:build` fails with "NSIS not found"** — install [NSIS 3](https://nsis.sourceforge.io/) and ensure `makensis` is on PATH. `tauri:dev` does not need it.

## Scope rules (summary)

- No ticketing or technician workflow (removed in migration `0003`).
- Unit Registry is list-first; the edit form lives on a dedicated detail page.
- Excel parsing uses `read-excel-file` (pinned to exact `9.0.10`); the `xlsx` npm package is forbidden due to an unaddressed security advisory.
- No linter or formatter is configured — keep it that way unless asked.
