# Dependency Audit Report

## Source Assessment
- Repository path: `C:\Users\deped\Documents\GitHub\Work-App-Database-Codex`
- Assessment date: 2026-06-08
- Ecosystems detected: JavaScript/TypeScript (npm), Rust (Cargo), Debian package metadata/shell packaging
- Package managers detected: npm, Cargo, dpkg-deb packaging script
- Lockfiles found: `package-lock.json` (lockfile version 3), root `Cargo.lock`
- Commands run:
  - `npm audit --json` - passed; 0 vulnerabilities across 341 installed dependencies.
  - `npm audit --omit=dev --json` - first attempt failed at the registry endpoint in the sandbox; escalated retry passed with 0 vulnerabilities.
  - `npm outdated --json` - completed with the expected exit code 1 because updates are available.
  - `npm ls --all --json` and `npm ls --depth=0` - passed; no missing, invalid, or extraneous dependency errors.
  - `npm explain class-variance-authority`, `npm explain clsx`, `npm explain tailwind-merge`, and `npm explain @vitest/ui` - passed.
  - `cargo metadata --format-version 1 --locked --no-deps` - passed.
  - `cargo metadata --format-version 1 --locked` - sandbox attempt failed while populating the Cargo registry cache; escalated retry passed.
  - `cargo tree --workspace --locked --duplicates` - passed.
  - Manifest, lockfile, license, package-source, source-usage, Debian control, and packaging-script inspections - passed.
- Commands not run and why:
  - `cargo audit`: `cargo-audit` is not installed; it was not installed because this stage is audit-only.
  - `cargo outdated`: `cargo-outdated` is not installed; it was not installed because this stage is audit-only.
  - `cargo deny`: `cargo-deny` is not installed; it was not installed because this stage is audit-only.
  - Debian package dependency resolution/build checks: current host is Windows, so `dpkg-deb` and Debian repository resolution were not validated.
- Network/live lookup available: partial; npm advisory and outdated lookups succeeded, but no RustSec or live crates.io staleness tool was available.

## Dependency Inventory
- Direct production dependencies:
  - npm: 11.
  - Cargo: 26 unique registry crates across three workspace packages, plus two workspace-local path edges to `fleet-shared`.
- Direct development dependencies:
  - npm: 15.
  - Cargo: one build dependency (`tauri-build`); no Rust dev-dependencies declared.
- Transitive dependency count, if available:
  - npm: 341 total lockfile/install entries (105 production, 236 development; npm also reports 63 optional and 8 peer relationships).
  - Cargo: 605 total packages, comprising 3 workspace packages and 602 registry packages across all supported targets.
- Container/base image dependencies: none found.
- OS package dependencies: Debian control declares the unversioned distro metapackage `postgresql`.
- Private or non-registry dependencies:
  - npm: none.
  - Cargo external dependencies: none; `fleet-shared` is a repository-local workspace path dependency.
- Package-source configuration: no repository `.npmrc` or `.cargo/config.toml`; npm packages resolve from `registry.npmjs.org`, and Cargo packages resolve from crates.io.

## Findings

### [HIGH] Workspace packages @ 0.3.0 - No project license blocks external distribution
- Manifest: `package.json`, `crates/fleet-shared/Cargo.toml`, `server/Cargo.toml`, `src-tauri/Cargo.toml`
- Scope: direct/project licensing
- Issue: The root npm package and all three Rust workspace packages omit license metadata, and no root `LICENSE` file exists.
- Evidence: Cargo metadata reports the three workspace packages as the only packages without license metadata. `README.md` explicitly states that the repository has no project license and that builds must be treated as internal-use artifacts. The repository also contains a Debian package builder, so distributable output is contemplated.
- Risk: External users or operators receive no permission grant to copy, install, modify, or redistribute the project. This is a release/distribution blocker even though third-party packages may individually permit distribution.
- Recommendation: Select and add an approved project license, add matching npm/Cargo manifest metadata, and review whether server/client distribution terms need to differ.
- Verification command: `cargo metadata --format-version 1 --locked --no-deps` and `npm pkg get license`

### [MEDIUM] Third-party dependency set - Attribution and MPL-2.0 obligations are not packaged
- Manifest: `package-lock.json`, `Cargo.lock`, `server/scripts/build-deb.sh`
- Scope: transitive/license compliance
- Issue: No third-party attribution or license bundle is generated for npm or Cargo dependencies.
- Evidence: The lockfiles contain 341 npm entries and 602 external Cargo packages. Cargo metadata identifies five MPL-2.0 packages: `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, and `selectors`. The Debian build script installs only the binary, systemd unit, example configuration, and package control scripts.
- Risk: Binary or installer distribution may omit required copyright notices, license text, and MPL source/modification disclosures. Exact obligations depend on how each dependency is incorporated and whether it is modified.
- Recommendation: Generate a reviewed third-party notices bundle for both ecosystems, include required license texts in desktop and Debian artifacts, and document MPL-2.0 source-availability handling.
- Verification command: `cargo deny check licenses` plus an npm license inventory tool such as `npx license-checker --production --summary`

### [MEDIUM] Cargo dependency tree - Current Rust vulnerability status is unverified
- Manifest: `Cargo.lock`
- Scope: direct and transitive security
- Issue: No RustSec advisory scan could be performed because `cargo-audit` is not installed.
- Evidence: Cargo resolves 602 external packages. Metadata and duplicate-version inspection succeeded, but those commands do not check security advisories.
- Risk: Known vulnerabilities or withdrawn crates could exist without being detected by this stage. No CVE or RustSec advisory is asserted without verification.
- Recommendation: Run `cargo audit --locked` in CI and before release; consider `cargo deny check advisories` as a policy gate.
- Verification command: `cargo audit --locked`

### [LOW] class-variance-authority, clsx, tailwind-merge, @vitest/ui - Apparently unused direct dependencies
- Manifest: `package.json`
- Scope: direct production/tooling maintenance
- Issue: Repository source/config search found no imports or runtime references for `class-variance-authority`, `clsx`, or `tailwind-merge`, and no UI test script or source reference for `@vitest/ui`.
- Evidence: `npm explain` shows each package is installed from the root declaration. `clsx` is also transitive through `class-variance-authority`. `tailwindcss-animate` was checked separately and is used by `tailwind.config.ts`.
- Risk: Unused direct dependencies expand install size and supply-chain surface and create unnecessary update work.
- Recommendation: Confirm no planned or generated usage depends on them, then remove unused declarations in a fixer stage.
- Verification command: `npm explain <package>` plus repository import/config search

### [INFO] npm direct dependencies - Routine updates and major-version migrations are available
- Manifest: `package.json`, `package-lock.json`
- Scope: direct production/tooling maintenance
- Issue: `npm outdated` reports patch updates for several packages and newer major lines for development tooling and selected UI packages.
- Evidence: Patch updates include React Query, Tauri CLI, React types, PostCSS, React, and React DOM. Major lines are available for Vite, Vitest/UI, TypeScript, Tailwind CSS, jsdom, `@vitejs/plugin-react`, and `lucide-react`. `read-excel-file` has a newer minor release.
- Risk: No security risk was identified by npm audit. Major upgrades may require migration work; deferring routine patches increases future update size.
- Recommendation: Apply compatible patch/minor updates with tests, and schedule major upgrades separately after compatibility review.
- Verification command: `npm outdated`

### [INFO] Cargo dependency graph - Multiple versions increase graph size but no direct conflict was proven
- Manifest: `Cargo.lock`
- Scope: transitive maintenance
- Issue: `cargo tree --duplicates` reports parallel versions for several crates, including `bitflags`, `getrandom`, `hashbrown`, `indexmap`, `rand`, `thiserror`, `toml`, and Windows support crates.
- Evidence: The duplicate tree is primarily introduced by Tauri, SQLx, TLS, platform, and build-tool chains.
- Risk: Larger binaries, longer builds, and a wider maintenance surface. No functional or security defect was established.
- Recommendation: Recheck after normal direct-dependency updates; do not force transitive convergence without upstream compatibility evidence.
- Verification command: `cargo tree --workspace --locked --duplicates`

## License Summary
- Project license: none declared; no root `LICENSE` file.
- Assumed distribution model if unstated: internal-use only, based on the explicit `README.md` distribution warning.
- License types found:
  - npm: MIT, ISC, Apache-2.0, BSD-2-Clause, BSD-3-Clause, CC-BY-4.0, and MIT-0; no unknown lockfile licenses.
  - Cargo external crates: primarily MIT/Apache-2.0 combinations, plus MPL-2.0, Unicode-3.0, BSD, ISC, Zlib, CDLA-Permissive-2.0, CC0-1.0, BSL-1.0, and other permissive alternatives; no external package lacked license metadata.
- Potential conflicts: No confirmed incompatible dependency license was found. Five MPL-2.0 transitive crates impose file-level copyleft/notice obligations. Two `r-efi` versions offer permissive MIT or Apache-2.0 alternatives in addition to LGPL, so LGPL need not be selected.
- Attribution/NOTICE obligations: Not currently collected or shipped. CC-BY-4.0, Apache-2.0, MPL-2.0, and standard copyright/license notices require review before distribution.

## Summary Metrics
- Critical findings: 0
- High findings: 1
- Medium findings: 2
- Low findings: 1
- Info findings: 2
- npm advisory vulnerabilities: 0
- Rust advisory vulnerabilities: unknown; scan blocked by missing tool
- License conflicts: 1 project-level distribution blocker; 0 confirmed third-party incompatibilities
- Unknown licenses: 3 workspace packages; 0 external npm/Cargo packages
- Unused dependency candidates: 4 direct npm packages

## Verdict
BLOCKED

The dependency trees are locked, registry-sourced, and npm's live audit is clean. Release or external distribution remains blocked by the missing project license and missing third-party attribution process. Rust vulnerability status must also be verified before release.

## Next Actions
1. Choose and apply a project license, including npm and Cargo metadata.
2. Add automated third-party license/notice generation to desktop and Debian packaging.
3. Install and run `cargo audit --locked` in CI or a controlled audit environment.
4. Confirm and remove the four apparently unused npm direct dependencies in a fixer stage.
5. Apply compatible npm patch/minor updates, then evaluate major upgrades separately.
