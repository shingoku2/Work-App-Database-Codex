# Antminer Fleet Manager

`antminer-fleet-manager` is a desktop application for offline Antminer repair and inventory management. It is built using the **Tauri v2** framework, combining a **React** frontend with a **Rust** backend.

## Project Overview

- **Purpose**: Manage ASIC miner assets and track spare parts inventory from a local offline database.
- **Frontend Stack**: React 19, TypeScript, Vite, TanStack Query (v5), TanStack Table (v8), Lucide React, Tailwind CSS.
- **Backend Stack**: Rust, Tauri v2, `sqlx` (SQLite), `tauri-plugin-sql`.
- **Database**: SQLite (local file: `fleet.db`).

## Project Structure

- `src/`: React frontend source code.
  - `features/`: Domain-specific logic, components, and API calls (e.g., `miners`, `inventory`, `dashboard`).
  - `components/`: Shared UI components (layout, shell, etc.).
  - `lib/`: Shared utilities (Tauri command wrapper, QueryClient).
  - `types/`: TypeScript definitions, including database models (`db.ts`).
- `src-tauri/`: Rust backend and Tauri configuration.
  - `src/`: Rust source code.
    - `commands/`: Implementation of Tauri commands invoked from the frontend.
    - `db/`: Database initialization and pool management.
    - `models/`: Rust structs representing database records.
    - `lib.rs`: Tauri app setup and command registration.
  - `migrations/`: SQL migration files for schema management.

## Building and Running

### Prerequisites
- Node.js (v18+)
- Rust (stable)

### Development
```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri:dev
```

### Production Build
```bash
# Build the application package
npm run tauri:build
```

## Development Conventions

- **Tauri Commands**: Frontend calls to the backend should use the `command` wrapper in `src/lib/tauri.ts`.
- **State Management**: Use **TanStack Query** for all data fetching and mutations.
- **Styling**: Use **Tailwind CSS**. Prefer using `clsx` and `tailwind-merge` for dynamic classes.
- **Type Safety**: Ensure all backend command inputs and outputs have corresponding TypeScript interfaces in `src/types/db.ts`.
- **Database**: All database interactions should be handled via Tauri commands in Rust. Do not attempt direct database access from the frontend.
- **Migrations**: New schema changes must be added as new migration files in `src-tauri/migrations` and registered in `src-tauri/src/lib.rs`.
