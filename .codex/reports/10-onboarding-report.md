# Onboarding Report

## First Impressions
The repository now explains the two-install architecture clearly: a Linux/PostgreSQL server and an online-required Tauri desktop client. A developer can build and test the client on Windows from the README, while a first server operator has complete commands but must validate them on disposable Debian/PostgreSQL infrastructure before production.

## Source Assessment
- Docs inspected in required order: `README.md`; no `CONTRIBUTING.md`; `server/README.md`; `docs/OPERATIONS.md`; `docs/INTERNAL_COMPLIANCE.md`; root, server, shared, and Tauri manifests; package scripts; server example config; packaging files; server/client entrypoints
- Commands attempted during the pipeline: `npm ci`/installed dependency checks, frontend build and tests, Cargo workspace check/tests/formatting, npm audit, server CLI help, and documented CLI subcommand inspection
- Commands not run and why: Debian package installation, systemd startup, live PostgreSQL migration/concurrency, PostgreSQL restore, and packaged Tauri/keyring end-to-end flows require target infrastructure unavailable on this Windows host
- Sandbox limitations: Linux service/package tools and a disposable PostgreSQL deployment were unavailable

## Phase Reports

### Phase 1: Understanding what this is
- Overall: Smooth
- Passed: purpose, internal audience, two separately installed components, online requirement, supported fleet scope, and repository layout are stated at the top of the README.
- Issues found: none.

### Phase 2: Prerequisites
- Overall: Rough
- Passed: desktop Node/Rust/Tauri prerequisites and Debian/Ubuntu server target are identified.
- Issues found:

#### [MEDIUM] Server build prerequisites are not version-pinned
- Where: `server/README.md`
- Problem: Rust and `dpkg-deb` are named, but supported Rust/PostgreSQL/Debian version ranges are not recorded.
- Impact: a future operator may use a materially different toolchain without knowing the tested baseline.
- Fix: record the versions used for the first successful staging deployment.

### Phase 3: Installation
- Overall: Rough
- Passed: desktop install/build commands exist; Debian package script, package metadata, service unit, and installation commands are documented.
- Issues found:

#### [HIGH] Debian package flow is source-reviewed but not executed
- Where: `server/scripts/build-deb.sh`, `server/packaging`, `server/README.md`
- Problem: this host cannot run `dpkg-deb`, install the package, or validate systemd behavior.
- Impact: first production installation could expose path, permission, dependency, or service-hardening defects.
- Fix: build and install the package on disposable Debian/Ubuntu amd64 before production.

### Phase 4: Configuration
- Overall: Smooth
- Passed: PostgreSQL URL, pool range, listen address, session range, TLS paths, placeholder rejection, file permissions, hidden/stdin password handling, logging, and client fingerprint pairing are documented and validated in code/CLI tests.
- Issues found:

#### [INFO] Secrets require an organizational provisioning choice
- Where: `/etc/antminer-fleet/server.toml`
- Problem: the runbook secures the file but does not prescribe a single secret-management product.
- Impact: operators must follow existing organizational policy for database credential delivery.
- Fix: select and document the internal secret-management mechanism during deployment.

### Phase 5: Running the application
- Overall: Rough
- Passed: server validation, migration, first-admin, service, health-check, client pairing, and login commands are all present.
- Issues found:

#### [HIGH] Complete server-to-client smoke test remains unverified
- Where: deployment sequence
- Problem: no real Linux server, PostgreSQL database, certificate, system keyring, and packaged client were connected during this pipeline.
- Impact: cross-component assumptions are compiled and unit-tested but not proven in the target environment.
- Fix: execute the staged smoke test listed in `docs/OPERATIONS.md`.

### Phase 6: Development workflow
- Overall: Smooth
- Passed: install, build, frontend tests, Rust checks/tests/formatting, security audit, project structure, test limitations, and operating docs are documented.
- Issues found:

#### [LOW] No automated CI or contribution guide
- Where: repository root
- Problem: validation commands are documented but not enforced by repository CI, and review conventions are not recorded.
- Impact: multi-developer changes can omit checks or use inconsistent review practices.
- Fix: add internal CI and `CONTRIBUTING.md` when more contributors begin working in the repository.

### Phase 7: First-task simulation
- Overall: Smooth
- Passed: server API/auth/import/config, shared contracts, Tauri networking, React features, tests, migrations, packaging, and operations documents have clear ownership boundaries.
- Issues found: changes crossing shared contracts still require coordinated Rust and TypeScript updates, which the README now calls out through the repository map and validation commands.

## Time Estimate
- Desktop developer, prerequisites installed: 15-30 minutes plus first Cargo compilation
- Server operator on prepared Debian/PostgreSQL staging: 45-90 minutes
- First organizational production rollout including firewall, DNS, TLS verification, backup/restore, and client smoke tests: half a day to one day

## Blocker List
- No source-code blocker was found in the current automated suites.
- Production rollout is blocked until the documented disposable Debian/PostgreSQL and packaged-client smoke test passes.

## Friction List
- Server dependency versions need a recorded tested baseline.
- No CI workflow or contributor guide exists.
- Internal secret provisioning is organization-specific.
- Third-party attribution records must be retained for internal compliance.

## Overall Onboarding Verdict
NEEDS WORK

Desktop development onboarding is smooth. Production server onboarding is documented but not yet proven on its target operating system and database.

## Recommended Documentation Fix Order
1. Record the exact tested Debian, PostgreSQL, Rust, and package-tool versions after staging validation.
2. Add the staging smoke-test result and any corrections to `docs/OPERATIONS.md`.
3. Add internal CI and contribution conventions when the repository has multiple active contributors.
