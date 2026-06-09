# Internal Dependency and Attribution Record

This application is intended for internal organizational use. This document
records the dependency-license review performed on June 8, 2026 and the
follow-up needed to keep an internal attribution file current. It is not legal
advice and is not a complete artifact-level third-party notices bundle.

## Project posture

- The repository does not currently declare a project license.
- The current operating scope is internal use by the organization controlling
  the source and deployments.
- Do not provide source, desktop installers, server packages, or hosted access
  to third parties without an ownership/terms review and an artifact-specific
  license and notice bundle.

## Dependency records

The authoritative version records are:

- `package-lock.json` for JavaScript/TypeScript dependencies.
- `Cargo.lock` for Rust dependencies.
- `package.json` and the workspace `Cargo.toml` files for direct dependencies.

The June 8 audit found:

- no unknown external license metadata in the npm or Cargo lockfiles;
- primarily MIT, Apache-2.0, ISC, BSD, and other permissive terms;
- five MPL-2.0 transitive Rust crates: `cssparser`,
  `cssparser-macros`, `dtoa-short`, `option-ext`, and `selectors`;
- development/build data from `caniuse-lite` under CC-BY-4.0; and
- an upstream `NOTICES.md` in `cfg_aliases 0.2.1`.

No repository modification of the MPL-2.0 crates was found. Whether each
build-time package contributes material to a shipped desktop artifact remains
unverified because final target artifacts were not inspected.

## Internal attribution practice

For every internally deployed version:

1. Retain the matching `package-lock.json` and `Cargo.lock`.
2. Record the server package and desktop installer versions deployed.
3. Retain the upstream license texts and applicable NOTICE files for components
   included in those artifacts.
4. Record whether any third-party dependency was patched or vendored.
5. Re-run the inventory after dependency or lockfile changes.
6. Escalate MPL-2.0, CC-BY-4.0, Apache NOTICE, or new/unknown license results for
   organizational review.

The June 8 reports under `.codex/reports/01-dependency-audit.md` and
`.codex/reports/02-license-audit.md` contain the detailed reviewed inventory
and dependency paths. Those pipeline reports are internal engineering records,
not files currently included in the Debian or desktop packages.

## Inventory commands

Run from the repository root:

```bash
npm audit --omit=dev
npm ls --all --json
cargo metadata --format-version 1 --locked
cargo tree --workspace --locked
```

For vulnerability and license policy checks, install approved versions of the
following tools in CI or a controlled audit environment:

```bash
cargo audit --locked
cargo deny check advisories licenses
npx license-checker --production --summary
```

`cargo-audit`, `cargo-deny`, `cargo-license`, and `license-checker` were not
available for the June 8 local audit. The npm advisory check completed with
zero reported vulnerabilities; current RustSec status remains unverified until
`cargo audit --locked` is run.

## Packaging gap

The current Debian build script installs the server binary, systemd unit,
example configuration, and maintainer scripts. The Tauri configuration builds
an NSIS installer. Neither packaging path currently generates or installs a
third-party notices bundle.

For continued internal-only operation, retain this document and the locked
inventories with deployment records. Before any distribution outside the
organization, generate and inspect artifact-specific software bills of
materials and notice bundles, determine which MPL/CC-BY material is included,
and establish project distribution terms.
