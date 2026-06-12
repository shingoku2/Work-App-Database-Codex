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

## Current Architecture

- The product is an internal, online-required Tauri desktop client plus a
  separately hosted Linux/PostgreSQL server. Do not reintroduce local
  production SQLite ownership in the desktop.
- React calls typed Tauri commands; Rust performs authenticated HTTPS requests
  to the server and pins the exact paired leaf certificate.
- Preserve fingerprint confirmation, exact certificate pinning, named-user
  authentication, revocable bearer sessions, optimistic concurrency versions,
  and role enforcement.
- `VITE_FLEET_SERVER_URL` is public build configuration used only to prefill
  first-run pairing. It must contain an HTTPS origin, never credentials,
  tokens, certificates, keys, or other secrets.

## SSH Tunnel Deployment

- The packaged backend reverse tunnel publishes Fleet Server on the SSH host's
  `127.0.0.1:8443`. Windows clients create a local forward from
  `127.0.0.1:8443` to that host-loopback endpoint. Do not configure clients to
  route directly to a container IP.
- Windows tunnel automation lives in `scripts/fleet-tunnel.ps1` with example
  configuration in `scripts/fleet-tunnel.example.json`.
- `scripts/fleet-tunnel.local.json`, `.env.local`,
  `.env.production.local`, and SSH private keys are machine-specific and must
  remain ignored and uncommitted.
- Tunnel automation must use non-interactive public-key or agent
  authentication, strict host-key verification, `ExitOnForwardFailure`, and
  keepalives. Never add password storage or an insecure host-key bypass.
- Treat the certificate fingerprint as public verification data, but never
  expose the TLS private key, SSH private key, database credential, user
  password, or bearer token.
- When changing tunnel behavior, validate start, status, stop, duplicate-port
  rejection, `/health`, `/pairing`, API version compatibility, and the exact
  expected certificate fingerprint. Use a temporary local port when a working
  user tunnel must remain uninterrupted.

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
