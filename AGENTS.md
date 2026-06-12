# AGENTS.md

## Project Agent Instructions

This repository uses a staged Codex agent pipeline under `.codex/pipeline/`.

General rules for all Codex work in this repository:

- Read the task prompt and relevant repository instructions before acting.
- Make small, reviewable changes.
- Do not commit, push, deploy, publish, or rewrite history unless explicitly asked.
- Do not expose or print secret values.
- If a secret is found in source, report the file/key name and state that it must be rotated.
- Prefer the smallest relevant validation command before broad checks.
- Report commands run and whether they passed or failed.
- Do not hide test, build, lint, typecheck, audit, or sandbox failures.
- Preserve public APIs unless a task explicitly allows breaking changes.
- When writing reports, place them in `.codex/reports/`.

## Architecture Boundaries

- The production application is client/server. PostgreSQL is the only
  production database and is owned exclusively by `antminer-fleet-server`.
- The React frontend runs inside Tauri. Browser code must call the established
  Tauri command wrapper; do not add direct browser access to the server or
  database.
- The Tauri Rust layer performs HTTPS requests with Rustls and stores the
  bearer session token in the operating-system credential manager.
- The client is online-required. Do not add a local production database,
  offline write queue, or automatic upload of legacy `fleet.db` data.
- Shared Rust API/domain contracts live in `crates/fleet-shared`; matching
  TypeScript contracts live in `src/types/db.ts`. Keep field names and enum
  values synchronized.
- Preserve API prefix `/api/v1` and numeric optimistic-concurrency versions for
  miners, parts, and users unless a task explicitly changes the protocol.

## TLS and Pairing

- Clients pin the exact server leaf certificate. Do not replace this with
  normal CA-only trust or add an insecure certificate bypass.
- `/pairing` and `/health` must remain available without bearer
  authentication.
- Pairing retrieves the certificate, requires independent full SHA-256
  fingerprint confirmation, and verifies `/health` using the pinned
  certificate.
- Certificate replacement requires clients to forget and re-pair. Treat
  automatic certificate renewal as a breaking operational change.
- `VITE_FLEET_SERVER_URL` is public build configuration used only to prefill
  the pairing form. It must contain an HTTPS origin only, never credentials or
  private material.

## SSH Tunnel Topology

- The optional backend reverse tunnel publishes the server only on the SSH
  host's `127.0.0.1:8443`.
- Desktop clients use a local SSH forward from their own
  `127.0.0.1:8443` to the SSH host's `127.0.0.1:8443`; they must not depend on
  a container IP.
- Backend tunnel implementation files are:
  `server/scripts/run-reverse-tunnel.sh`,
  `server/config/tunnel.example.conf`, and
  `server/packaging/antminer-fleet-tunnel.service`.
- Windows tunnel helper files are under `scripts/`. Keep
  `scripts/fleet-tunnel.local.json` local and ignored by Git.
- Tunnel automation must use batch/key authentication, reject forwarding
  failures, use keepalives, and avoid exposing host port `8443` publicly.
- Never commit SSH private keys, `known_hosts` deployment files, local tunnel
  configuration, database passwords, TLS private keys, or bearer tokens.

## Validation by Area

- Frontend-only: `npm run build` and `npm test`.
- Rust/server-only: start with `cargo check -p antminer-fleet-server --locked`;
  add targeted server tests when behavior changes.
- Shared or cross-component Rust changes: `cargo check --workspace --locked`
  and the relevant workspace tests.
- Packaging/tunnel changes: run `sh -n` on changed shell scripts,
  `systemd-analyze verify` on changed units when available,
  `sh server/scripts/build-deb.sh`, and inspect the resulting package.
- Tunnel runtime changes: verify both `/health` and `/pairing` through the
  complete tunnel path. Test reconnect behavior when the change affects
  supervision or keepalives.
- Always run `git diff --check` before reporting completion.

## Pipeline Order

1. `CODEX_DEP_AUDITOR.md`
2. `CODEX_LICENSE.md`
3. `CODEX_ENV_VALIDATOR.md`
4. `CODEX_AUDITOR.md`
5. `CODEX_FIXER.md`
6. `CODEX_TEST_WRITER.md`
7. `CODEX_REFACTOR.md`
8. `CODEX_DOCS_WRITER.md`
9. `CODEX_CHANGELOG.md`
10. `CODEX_ONBOARDING.md`
