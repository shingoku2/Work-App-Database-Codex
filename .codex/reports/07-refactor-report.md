# Refactor Report

## Refactor Plan
- Targets: repeated React Query cache invalidation after miner and part mutations.
- Files expected to change: `src/lib/queryInvalidation.ts`, `src/features/miners/MinersView.tsx`, and `src/features/inventory/InventoryView.tsx`.
- Risk level: low.
- Test coverage: miner API/import tests, inventory component tests, full Vitest suite, and the TypeScript production build.
- Refactor types: remove repeated private cache-maintenance logic and give it one descriptive internal helper.

## Changes Made

### Duplication Removal Consolidate fleet-data cache invalidation
- File: `src/lib/queryInvalidation.ts`, `src/features/miners/MinersView.tsx`, `src/features/inventory/InventoryView.tsx`.
- What changed: extracted the repeated sequential invalidation of a domain query (`miners` or `parts`) followed by `dashboard` into `invalidateFleetData`.
- Why: the same two-step operation appeared in five mutation success handlers. Keeping the query keys and ordering in one private helper makes the cache relationship explicit and prevents the handlers from drifting apart.
- Behavior preserved: each caller still awaits the domain invalidation first and the dashboard invalidation second. Mutation timing, query keys, UI state changes, API calls, error text, and public interfaces are unchanged.
- Validation: targeted Vitest tests, the full Vitest suite, and the production TypeScript/Vite build passed.
- Test impact: no tests were changed for this refactor.

## Skipped Candidates
- Candidate: split `ConnectionGate.tsx` into additional files or introduce a connection-state hook.
- Reason skipped: the component is readable, and state extraction would touch the authentication and certificate-repair flow that report 06 identifies as lacking real Tauri/keyring/HTTPS integration coverage.
- Required follow-up: consider only after end-to-end connection tests exist.

- Candidate: reorganize server TLS verification, authentication/session code, or login limiting.
- Reason skipped: these areas enforce security behavior and have limited integration coverage. Structural changes would create risk without a stage-7 benefit.
- Required follow-up: retain current structure until live HTTPS and authentication integration tests are available.

- Candidate: extract or restructure PostgreSQL transaction, final-administrator, bulk import, or SQLite migration logic.
- Reason skipped: report 06 identifies missing live PostgreSQL concurrency and import-policy tests. These areas are explicitly outside the conservative refactor scope.
- Required follow-up: add disposable PostgreSQL integration coverage before considering internal restructuring.

- Candidate: split `MinersView.tsx` into more components.
- Reason skipped: the file is long, but the current view-mode branches are cohesive and understandable. A broader split would move state and callbacks across component boundaries without enough behavior-preserving value.
- Required follow-up: revisit only if the miner workflow grows or focused component coverage is added.

## Validation Summary
- Command: `npm test -- src/test/InventoryView.test.tsx src/test/minerApi.test.ts src/test/import.test.ts`
- Result: passed, 70 tests across 3 files.
- Notes: targeted coverage for the two affected feature areas.

- Command: `npm test`
- Result: passed, 87 tests across 8 files.
- Notes: full frontend regression suite.

- Command: `npm run build`
- Result: passed.
- Notes: TypeScript compilation and Vite production build completed successfully.

## Summary
- Refactors applied: one private duplication-removal refactor.
- Files changed: three source files plus this report.
- Remaining technical debt: long frontend view files and weak live-infrastructure coverage around TLS, keyring authentication, PostgreSQL concurrency, and import policies.
- Recommended follow-up: preserve the current security/database structure until the integration gaps in report 06 are covered.
