# License Audit Report

This report identifies license risk signals and is not legal advice.

## Source Assessment
- Assessment date: 2026-06-08.
- Project files inspected: `README.md`, `README_CODEX_PIPELINE.md`, `CHANGELOG.md`, `CLAUDE.md`, `package.json`, root `Cargo.toml`, all workspace `Cargo.toml` files, Debian control/scripts, server packaging scripts, and generated artifact directories.
- Legal files inspected: no `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING`, `NOTICE`, `THIRD_PARTY_LICENSES`, or `COPYRIGHT` file exists at the repository root. No equivalent file was found in the desktop or server package inputs.
- Dependency files inspected: `package-lock.json`, root `Cargo.lock`, installed npm package metadata, and complete Cargo metadata.
- Vendored/generated content inspected: no repository `vendor/`, `third_party/`, `externals/`, or `licenses/` tree was found. Source-header search found no copied third-party attribution headers. Existing `dist/` and `server/package/` content contains no license or notice bundle.
- Commands run:
  - `git status --short` - passed; confirmed a dirty working tree and that this stage must not alter existing work.
  - Legal-file, README/docs, source-header, vendored-directory, packaging, and generated-artifact PowerShell inspections - passed except for one initially incorrect `server/debian/control` path; the corrected `server/packaging/debian/control` read passed.
  - `npm pkg get license` - passed; returned `{}`, confirming no npm project license metadata.
  - Node-based `package-lock.json` inventory - passed; 338 npm dependency entries had license metadata and none were unknown.
  - Node-based direct npm dependency inventory - passed.
  - `cargo metadata --format-version 1 --locked` - passed; provided full transitive Cargo license metadata.
  - `cargo metadata --format-version 1 --locked --no-deps` - passed; all three workspace packages had empty `license` and `license_file` fields.
  - `cargo tree --workspace --locked -i <crate>` for all MPL-2.0 crates - passed; established their Tauri dependency paths.
  - npm and Cargo `NOTICE` file searches - passed; no npm notice file was found, and Cargo package `cfg_aliases 0.2.1` contains `NOTICES.md`.
- Commands not run or not completed:
  - `npx --no-install license-checker --production --json` - timed out after 30 seconds; `node_modules/.bin/license-checker` is absent. No package was installed because this stage is audit-only.
  - `cargo license --version` - failed because `cargo-license` is not installed. No tool was installed because this stage is audit-only.
  - Debian package build/contents inspection - not run because the host is Windows and the packaging script requires a Debian environment and `dpkg-deb`.
- Transitive license visibility: full metadata visibility for locked npm and Cargo dependencies; partial artifact-obligation visibility because no production Debian or desktop installer was built and inspected on its target platform.

## Project License
- Project license: none declared; absent from npm metadata, all three Rust workspace manifests, and repository legal files.
- Distribution model stated: `README.md` says builds must be treated as internal-use artifacts until licensing and attribution are resolved.
- Distribution model assessed: current posture is `Internal Use Only`. The repository also documents separately installed desktop and Debian server artifacts, so any external release is conservatively assessed as `Commercial Closed-Source` because no open-source or other release model is stated.
- Project license risk: critical for external distribution. The repository provides no recipient permission grant or proprietary distribution terms. Internal use by the copyright owner is a narrower posture and does not clear release artifacts for third parties.

## Dependency License Inventory

The table lists every direct production dependency, every workspace package, and all transitive packages with heightened obligations. The complete locked transitive inventories were checked by command rather than expanded into hundreds of low-risk rows.

| Package | Version | Direct/Transitive | License | Risk | Notes |
|---|---:|---|---|---|---|
| `antminer-fleet-manager` | 0.3.0 | Project | None | Critical | Desktop distributable has no license metadata or legal file. |
| `antminer-fleet-server` | 0.3.0 | Project | None | Critical | Debian/server distributable has no license metadata or legal file. |
| `fleet-shared` | 0.3.0 | Project | None | Critical | Local dependency has no license metadata. |
| `@tanstack/react-query` | 5.100.10 | Direct npm | MIT | Medium | Attribution/license-text obligation. |
| `@tanstack/react-table` | 8.21.3 | Direct npm | MIT | Medium | Attribution/license-text obligation. |
| `@tauri-apps/api` | 2.11.0 | Direct npm | Apache-2.0 OR MIT | Medium | Select and preserve a permitted license path. |
| `class-variance-authority` | 0.7.1 | Direct npm | Apache-2.0 | Medium | Preserve license and applicable notices. |
| `clsx` | 2.1.1 | Direct npm | MIT | Medium | Attribution/license-text obligation. |
| `lucide-react` | 0.555.0 | Direct npm | ISC | Medium | Attribution/license-text obligation. |
| `react` | 19.2.6 | Direct npm | MIT | Medium | Attribution/license-text obligation. |
| `react-dom` | 19.2.6 | Direct npm | MIT | Medium | Attribution/license-text obligation. |
| `read-excel-file` | 9.0.10 | Direct npm | MIT | Medium | Attribution/license-text obligation. |
| `tailwind-merge` | 3.6.0 | Direct npm | MIT | Medium | Attribution/license-text obligation. |
| `tailwindcss-animate` | 1.0.7 | Direct npm | MIT | Medium | Attribution/license-text obligation. |
| `argon2` | 0.5.3 | Direct Cargo | MIT OR Apache-2.0 | Medium | Server dependency; select and preserve a permitted license path. |
| `axum` | 0.8.9 | Direct Cargo | MIT | Medium | Server dependency. |
| `axum-server` | 0.8.0 | Direct Cargo | MIT | Medium | Server dependency. |
| `chrono` | 0.4.45 | Direct Cargo | MIT OR Apache-2.0 | Medium | Server dependency. |
| `clap` | 4.6.1 | Direct Cargo | MIT OR Apache-2.0 | Medium | Server dependency. |
| `rand` | 0.9.4 | Direct Cargo | MIT OR Apache-2.0 | Medium | Server dependency. |
| `rcgen` | 0.14.8 | Direct Cargo | MIT OR Apache-2.0 | Medium | Server dependency. |
| `rpassword` | 7.5.4 | Direct Cargo | Apache-2.0 | Medium | Server dependency; preserve license/notices. |
| `sqlx` | 0.8.6 | Direct Cargo | MIT OR Apache-2.0 | Medium | Server dependency. |
| `toml` | 0.9.12+spec-1.1.0 | Direct Cargo | MIT OR Apache-2.0 | Medium | Server dependency. |
| `tower-http` | 0.6.11 | Direct Cargo | MIT | Medium | Server dependency. |
| `tracing` | 0.1.44 | Direct Cargo | MIT | Medium | Server dependency. |
| `tracing-subscriber` | 0.3.23 | Direct Cargo | MIT | Medium | Server dependency. |
| `keyring` | 3.6.3 | Direct Cargo | MIT OR Apache-2.0 | Medium | Desktop dependency. |
| `reqwest` | 0.12.28 | Direct Cargo | MIT OR Apache-2.0 | Medium | Desktop dependency. |
| `tauri` | 2.11.2 | Direct Cargo | Apache-2.0 OR MIT | Medium | Desktop dependency. |
| `tauri-build` | 2.6.2 | Direct build Cargo | Apache-2.0 OR MIT | Medium | Desktop build dependency. |
| `tokio` | 1.52.3 | Direct Cargo | MIT | Medium | Shared server/client dependency. |
| `serde` | 1.0.228 | Direct Cargo | MIT OR Apache-2.0 | Medium | Shared server/client dependency. |
| `serde_json` | 1.0.150 | Direct Cargo | MIT OR Apache-2.0 | Medium | Shared server/client dependency. |
| `sha2` | 0.10.9 | Direct Cargo | MIT OR Apache-2.0 | Medium | Shared server/client dependency. |
| `thiserror` | 2.0.18 | Direct Cargo | MIT OR Apache-2.0 | Medium | Server/shared dependency. |
| `uuid` | 1.23.2 | Direct Cargo | Apache-2.0 OR MIT | Medium | Shared server/client dependency. |
| `rustls-pemfile` | 2.2.0 | Direct Cargo | Apache-2.0 OR ISC OR MIT | Medium | Server/client dependency. |
| `url` | 2.5.8 | Direct Cargo | MIT OR Apache-2.0 | Medium | Desktop dependency. |
| `urlencoding` | 2.1.3 | Direct Cargo | MIT | Medium | Desktop dependency. |
| `cssparser` | 0.36.0 | Transitive Cargo | MPL-2.0 | High | Reached through Tauri utilities/code generation. |
| `cssparser-macros` | 0.6.1 | Transitive Cargo | MPL-2.0 | High | Procedural macro reached through `cssparser`. |
| `dtoa-short` | 0.3.5 | Transitive Cargo | MPL-2.0 | High | Reached through `cssparser`. |
| `option-ext` | 0.2.0 | Transitive Cargo | MPL-2.0 | High | Reached through `dirs` and Tauri, including the desktop runtime path. |
| `selectors` | 0.36.1 | Transitive Cargo | MPL-2.0 | High | Reached through Tauri utilities/code generation. |
| `caniuse-lite` | 1.0.30001792 | Transitive npm dev | CC-BY-4.0 | High | Development/build data; verify whether generated output incorporates attributable content. |
| `r-efi` | 5.3.0, 6.0.0 | Transitive Cargo | MIT OR Apache-2.0 OR LGPL-2.1-or-later | Low | A permissive MIT or Apache-2.0 option is available; document the selected path. |
| Remaining npm packages | 337 locked entries | Direct dev/transitive | MIT, ISC, Apache-2.0, BSD-2-Clause, BSD-3-Clause, MIT-0 | Medium | No unknown metadata; attribution bundle still required. |
| Remaining Cargo crates | 597 registry packages | Direct/transitive | Primarily MIT/Apache-2.0 plus permissive BSD, ISC, Unicode-3.0, Zlib, BSL-1.0, CC0, and CDLA alternatives | Medium | No unknown external metadata; exact license texts must be collected for shipped components. |

## Findings

### [CRITICAL] Workspace packages @ 0.3.0 - No project license
- Distribution conflict: external distribution of the desktop client, server binary, source archive, or Debian package has no documented recipient permission grant or proprietary terms.
- Scope: `antminer-fleet-manager`, `antminer-fleet-server`, and `fleet-shared`.
- Evidence: no project legal file; `npm pkg get license` returned `{}`; Cargo metadata reported empty `license` and `license_file` for every workspace package; `README.md` explicitly limits builds to internal use.
- Options:
  1. Replace with compatible alternative: not applicable to project-owned code.
  2. Obtain commercial license: establish approved proprietary distribution terms or an open-source license from the rights holder.
  3. Isolate/restructure usage: keep artifacts strictly internal until terms are approved and applied to all separately distributed components.
  4. Legal review needed: yes, before any third-party release.
- Urgency: release blocker.

### [HIGH] cssparser 0.36.0, cssparser-macros 0.6.1, dtoa-short 0.3.5, option-ext 0.2.0, selectors 0.36.1 - MPL-2.0
- Distribution conflict: MPL-2.0 permits larger-work distribution but requires preservation of notices and availability of Source Code Form for covered files when distributing executable form. No process or artifact currently satisfies those obligations.
- Scope: transitive Tauri desktop dependency graph. `option-ext` is on a Tauri runtime path; the other four appear through Tauri utilities/build/code-generation paths and need final-artifact confirmation.
- Evidence: locked Cargo metadata and inverse dependency trees; no vendored modifications were found, and no MPL source offer, license bundle, or notice is packaged.
- Options:
  1. Replace with compatible alternative: use an upstream dependency configuration that removes the MPL crates if practical.
  2. Obtain commercial license: investigate only if the upstream owners offer one.
  3. Isolate/restructure usage: determine which crates are build-only and exclude non-shipped tooling from artifact obligations; for shipped covered code, publish or point to the exact corresponding source and preserve MPL notices.
  4. Legal review needed: yes, to approve the compliance method.
- Urgency: required before desktop distribution.

### [HIGH] caniuse-lite @ 1.0.30001792 - CC-BY-4.0
- Distribution conflict: CC-BY-4.0 requires attribution when licensed material or adapted material is shared. The package is development-only, but the repository has not documented whether browser-target data is incorporated into generated CSS or other distributed output.
- Scope: transitive npm development/build dependency.
- Evidence: `package-lock.json` marks the package as dev-only and licensed CC-BY-4.0; no attribution bundle is generated.
- Options:
  1. Replace with compatible alternative: use a build path that does not incorporate CC-BY data if attribution cannot be supported.
  2. Obtain commercial license: investigate with the data rights holder if necessary.
  3. Isolate/restructure usage: establish whether the data is only consulted during builds; if attributable content is shipped, include required creator/source/license attribution.
  4. Legal review needed: yes if generated output incorporates the dataset.
- Urgency: resolve during release artifact review.

### [MEDIUM] Locked npm and Cargo dependency sets - License and notice material is not packaged
- Distribution conflict: permissive licenses generally allow closed-source distribution but still require copyright notices and license text. Apache-2.0 components may also require preservation of applicable NOTICE material and patent-license handling.
- Scope: desktop assets, desktop native binary, and Debian server package.
- Evidence: 338 npm entries and 602 external Cargo crates have license metadata, but no third-party bundle exists. The Debian builder installs only the server binary, service, configuration example, and control scripts. `cfg_aliases 0.2.1` includes `NOTICES.md`, while no generated artifact contains notice material.
- Options:
  1. Replace with compatible alternative: not generally needed for permissive dependencies.
  2. Obtain commercial license: not generally needed where existing terms are followed.
  3. Isolate/restructure usage: generate artifact-specific inventories so build-only packages are not represented as runtime components.
  4. Legal review needed: review the final attribution bundle and Apache-2.0 NOTICE/patent treatment.
- Urgency: required before any distribution.

## Compliance Obligations
- Attribution required: yes, for shipped MIT, ISC, BSD, Apache-2.0, MPL-2.0, CC-BY-4.0, and other notice-bearing material as applicable.
- License text distribution required: yes for shipped components under their applicable terms. No bundle currently exists.
- NOTICE file required: preserve upstream NOTICE content when the selected license requires it and the upstream notice applies. At minimum, `cfg_aliases 0.2.1` contains `NOTICES.md`; artifact-specific review is needed to determine whether it is shipped.
- Source availability obligations: MPL-2.0 covered source must be made available for any covered code distributed in executable form. No repository modification of those crates was found, but a corresponding-source method is still required for shipped MPL components.
- Patent/NOTICE considerations: Apache-2.0 patent grants, termination language, license text, modification notices, and applicable NOTICE preservation require documented handling. Dual-license dependencies should have a selected permissive path recorded.
- Project terms: add an approved license or proprietary EULA/installer terms covering each artifact and align npm/Cargo metadata with that decision.
- Release evidence: produce and retain an SBOM, dependency license inventory, source-offer/source-location record for MPL components, and final installer/package contents report.

## Unknown or Unverifiable Licenses
- Package: no external npm or Cargo package has unknown license metadata in the current lockfiles.
- Reason unknown: final target artifacts were not built and inspected, so it is unverified which build-time dependencies or licensed data are incorporated into the desktop and Debian outputs.
- Required follow-up: build on supported target platforms, generate an artifact-level SBOM/license report, inspect embedded assets and notices, and run `license-checker`/equivalent plus `cargo-license` or `cargo-deny` once those tools are approved and available.

## Summary
- Critical blockers: one project-wide blocker affecting all three workspace packages: no project license or proprietary recipient terms.
- High-risk items requiring legal review: five MPL-2.0 Cargo crates and one CC-BY-4.0 npm build-data package.
- Compliance obligations outstanding: project terms, third-party attribution/license bundle, MPL corresponding-source method, Apache NOTICE/patent review, and final artifact SBOM.
- Unknown licenses: zero external dependency metadata entries; final artifact inclusion remains unverified.

## Verdict
DO NOT DISTRIBUTE

The current internal-use-only posture is consistent with the repository warning. Do not provide source or built desktop/server artifacts to third parties until project distribution terms and the third-party compliance bundle are approved and included.
