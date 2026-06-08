# Documentation Report

## Documentation Gap Assessment
- Existing docs inspected: `README.md`, `CLAUDE.md`, `CHANGELOG.md`, `README_CODEX_PIPELINE.md`, `AGENTS.md`
- Existing doc quality: detailed but partially outdated
- Source files inspected: package scripts/config, Tauri config/capabilities, Rust commands/models/migrations, TypeScript API/import code
- Commands verified: `npm run build`, `npm test`, `cargo check`, `cargo test`
- Highest-priority gaps: distribution licensing status, stale import implementation description, and new command-boundary behavior

## Documentation Written or Updated

### File: `README.md`
- What changed: added distribution status and data-integrity behavior sections.
- Why: the license audit blocks redistribution, and the fixer changed verified create/update/import semantics.
- Source of truth used: reports 02, 04, 05, and current Rust commands.
- Accuracy notes: no license was selected or implied.

### File: `CLAUDE.md`
- What changed: replaced the inaccurate `INSERT ... ON CONFLICT` claim with the actual transactional lookup/update-or-insert flow and documented shared validation.
- Why: contributors need the real persistence behavior before changing import logic.
- Source of truth used: `src-tauri/src/commands/miners.rs`.
- Accuracy notes: the dual migration-registration guidance remains unchanged because that architecture was not modified.

## Inline Docstrings or Comments
- File: none
- Symbol: none
- Reason: current source comments are sufficient for the changed behavior.

## API Documentation
- API surface: Tauri miner and part commands
- Docs added/updated: user-visible normalization, validation, and missing-record behavior summarized in README
- Examples verified: no new command examples required

## Accuracy Flags
- Unverified behavior: packaged NSIS installer was not built or smoke-tested
- Ambiguous config: none
- Human input needed: project license selection and third-party attribution policy

## Summary
- Documents created: 1 report
- Documents updated: `README.md`, `CLAUDE.md`
- Docstrings/comments added: 0
- Remaining gaps: database migration compatibility guidance needs tests before documentation can be simplified
