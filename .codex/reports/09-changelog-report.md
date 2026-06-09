# Changelog Report

## Source Assessment
- Sources available: current `git status`, working-tree diff, recent commit
  history, reports 01-08, `CHANGELOG.md`, root and server READMEs, operations
  and internal-compliance documentation, manifests, and the exact-currency
  migration/import implementation.
- Source quality: strong for implemented behavior and local automated
  validation. Reports 04-08 provide finding-to-fix and test traceability.
- Date range covered: changes after the documented `0.2.0` release through the
  current unreleased working tree. No release date was assigned.
- Commands run:
  - `git status --short`
  - `git diff --stat`
  - targeted `git diff` for changelog and documentation
  - `git log --oneline --decorate -12`
  - reads of `.codex/pipeline/CODEX_CHANGELOG.md`, reports 01-08,
    `CHANGELOG.md`, `README.md`, `server/README.md`, and `docs/*.md`
  - targeted inspection of manifests, `server/migrations/0002_exact_currency.sql`,
    and currency conversion code
- Gaps affecting accuracy: no disposable Debian/systemd/PostgreSQL deployment,
  packaged Tauri/keyring environment, or RustSec tooling was available. Those
  limits are carried into the changelog rather than inferred away.

## Changelog Entry or Full Changelog

Updated the existing `[Unreleased]` entry in `CHANGELOG.md`.

The entry now documents only verified user/operator-facing behavior:

- the breaking move from desktop-owned SQLite to a central PostgreSQL server;
- mandatory server installation, TLS setup, migration, first-admin creation,
  pairing, and named login;
- the operator-driven SQLite dry-run/apply path and its three conflict
  policies;
- Admin/User authorization boundaries and session handling;
- fingerprint confirmation and exact leaf-certificate pinning;
- optimistic concurrency and final-administrator protection;
- exact integer-cent currency storage, including nearest-cent conversion from
  previous floating-point values;
- actionable upgrade instructions; and
- passed validation plus infrastructure that has not yet been exercised.

The prior security note implying a public-distribution readiness gate was
removed. The release record now reflects the confirmed internal-application
scope without claiming that internal use eliminates dependency-recordkeeping
or future external-distribution review.

## Release Notes

### Summary

This release changes Antminer Fleet Manager from a standalone desktop database
into an internal client/server system. A central Linux service owns the
PostgreSQL database and HTTPS API; desktop installations connect remotely and
do not keep a production inventory database.

### Breaking Changes

- Existing desktop installations cannot continue using local `fleet.db` as
  their live database.
- Operators must deploy the server and PostgreSQL before deploying the new
  client.
- Legacy data requires an explicit server-side SQLite import.
- Every client must confirm and pin the server certificate fingerprint, then
  authenticate with a named account.
- Part-cost integrations must use integer cents rather than floating-point
  dollar values.

### Operator Migration

Follow `server/README.md` and `docs/OPERATIONS.md` to install the package,
configure PostgreSQL and TLS, migrate the schema, create the first
administrator, and start the service. Preserve the legacy SQLite file, preview
it with `import-sqlite`, apply one reviewed conflict policy, verify the imported
records, and only then treat PostgreSQL as authoritative.

Existing floating-point part costs are multiplied by 100 and rounded to the
nearest cent when converted to `unit_cost_cents`. The PostgreSQL migration and
legacy SQLite importer use that same conversion rule.

### Authentication and Trust

The server uses named accounts, Admin/User roles, Argon2id password hashing,
and revocable opaque sessions. Account administration and spreadsheet import
require an administrator. Clients trust only the exact leaf certificate
accepted after the user independently verifies its complete SHA-256
fingerprint.

### Known Verification Boundary

Rust and frontend automated checks pass. The Debian package/systemd service,
live PostgreSQL migration and concurrency behavior, populated currency
migration, SQLite conflict races, backup/restore, certificate rotation, and a
packaged Tauri/keyring flow still require staging verification on target
infrastructure. RustSec advisory status is also pending because `cargo-audit`
was unavailable.

## Version Recommendation
- Recommended bump: `1.0.0` (major).
- Justification: the move from a self-contained SQLite desktop application to
  an online-required client/server deployment breaks the prior storage,
  installation, authentication, trust, and migration model. The manifests
  currently say `0.3.0`, but a major version communicates this operational
  boundary more accurately under the repository's stated Semantic Versioning
  policy. If the project intentionally treats all `0.x` releases as unstable,
  `0.3.0` is the minimum defensible alternative, but it is not the primary
  recommendation.

## Data Quality Flags
- Ambiguous change: none material for the documented user/operator behavior.
- Missing source: no target-infrastructure execution evidence for the items
  listed in the validation boundary.
- Human verification needed: run the deployment checklist on disposable
  Debian/PostgreSQL infrastructure, exercise a packaged client, and decide
  whether to align all manifests and package filenames with the recommended
  major version before assigning a release date.

## Files Updated
- `CHANGELOG.md`
- `.codex/reports/09-changelog-report.md`
