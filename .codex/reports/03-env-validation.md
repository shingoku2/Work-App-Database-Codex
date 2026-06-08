# Environment and Config Validation Report

## Source Assessment
- Assessment date: 2026-06-08.
- Prior reports read first: `.codex/reports/01-dependency-audit.md` and `.codex/reports/02-license-audit.md`. Their distribution blockers remain separate from this configuration verdict.
- Config files inspected: `server/config/server.example.toml`, `server/src/config.rs`, `server/src/main.rs`, `server/src/api.rs`, `server/src/auth.rs`, `src-tauri/src/client.rs`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, root Cargo/npm/TypeScript JSON and TOML files.
- Deployment files inspected: `server/packaging/antminer-fleet-server.service`, `server/packaging/debian/control`, `server/packaging/debian/postinst`, `server/packaging/debian/prerm`, and `server/scripts/build-deb.sh`.
- Documentation inspected: `README.md`, `server/README.md`, `CLAUDE.md`, and `CHANGELOG.md`.
- Source search patterns used: standard environment APIs, `DATABASE_URL`, `RUST_LOG`, server TOML keys, persisted client config names, credential storage references, and secret/key patterns. Generated, dependency, build, package, Git, and pipeline-report directories were excluded where appropriate.
- Commands run:
  - `git status --short` - passed; confirmed a dirty working tree and pre-existing implementation changes.
  - Targeted `Get-ChildItem`, `Get-Content`, and `Select-String` inventory/search commands - passed.
  - `cargo run -p antminer-fleet-server -- --config server/config/server.example.toml validate-config` - passed and reported `configuration is valid`.
  - `cargo run -p antminer-fleet-server -- --help` - passed.
  - `target\debug\antminer-fleet-server.exe create-admin --help`, `reset-password --help`, and `generate-tls --help` - passed.
  - A temporary out-of-repository TOML containing a non-PostgreSQL URL, `max_connections = 0`, and empty TLS paths was checked with `validate-config` - unexpectedly passed with exit code 0.
  - PowerShell JSON parsing for `package.json`, `tsconfig.json`, `src-tauri/tauri.conf.json`, and `src-tauri/capabilities/default.json` - passed.
  - Targeted secret-pattern scan - passed; no committed real secret or private key was identified. Matches were dependency/test terminology, not deployable credentials.
  - Two initial PowerShell inspection commands failed because `$variable:` was parsed as an invalid variable reference; corrected commands passed.
  - `bash -n server/scripts/build-deb.sh server/packaging/debian/postinst server/packaging/debian/prerm` - failed because Windows Subsystem for Linux has no installed distribution.
- Commands not run and why:
  - `systemd-analyze verify` - unavailable on the Windows host.
  - `dpkg-deb` package build/inspection - unavailable on the Windows host.
  - Live `serve`, migration, database, and TLS-file checks - no disposable PostgreSQL deployment or Linux service environment was provided, and this stage must not modify deployment state.

## Config Inventory
- Config mechanisms in use:
  - Server TOML selected by `--config`, defaulting to `/etc/antminer-fleet/server.toml`.
  - Optional `RUST_LOG` filtering through `tracing_subscriber::EnvFilter::try_from_default_env()`.
  - Desktop `server.json` under Tauri's platform app-data directory.
  - Desktop session token in the operating-system credential manager under a fixed service/account name.
  - Tauri build, bundle, CSP, and capability JSON.
- Unique environment variables read in source: `RUST_LOG` (implicit default used by `EnvFilter::try_from_default_env()`).
- Variables documented in examples/docs: `listen`, `session_days`, `database.url`, `database.max_connections`, `tls.certificate`, `tls.private_key`, and `DATABASE_URL`.
- Environment differentiation: partial. Operators can select another TOML with `--config`, but there are no named development/test/production profiles or documented override conventions.
- Startup validation: partial. TOML syntax, required structural fields, non-empty `database.url`, and `session_days >= 1` are checked. Database URL semantics, pool size, TLS paths/files, certificate/key pairing, placeholder credentials, and client persisted-config recovery are not validated by the dedicated validation path.
- Secret management pattern: the PostgreSQL credential is embedded in a root/service-readable TOML; the TLS private key is stored under `/etc/antminer-fleet/tls`; desktop bearer tokens use the OS credential manager; user passwords are not persisted by the client. Administrative password flags create an avoidable process-argument exposure path.

## Findings

### [HIGH] Server TOML validation - Undeployable configurations are accepted as valid
- Location: `server/src/config.rs`, `server/src/main.rs`, `server/config/server.example.toml`.
- Issue: `validate-config` checks only that `database.url` is non-empty and `session_days` is positive. It does not parse or restrict the database URL to PostgreSQL, reject the shipped `CHANGE_ME` placeholder, require `max_connections > 0`, reject empty TLS paths, or verify certificate/private-key existence, readability, parseability, and matching identity.
- Impact: the Debian-installed default and materially invalid operator configurations can pass the advertised validation command, then fail later during database connection or HTTPS startup. The shipped example passed validation without a usable database credential; a deliberately invalid profile also passed.
- Fix: make validation reject placeholders and invalid PostgreSQL URLs, enforce numeric bounds, validate TLS paths and certificate/key material for `serve`, and distinguish pre-TLS generation validation from deploy-ready validation.
- Secret exposure: no

### [HIGH] Desktop persisted config - A malformed `server.json` aborts application startup
- Location: `src-tauri/src/client.rs:39-55`, `src-tauri/src/lib.rs`.
- Issue: existing persisted JSON is read and deserialized during Tauri setup, and any read/parse/schema error is propagated out of setup. The file is written directly rather than atomically, has no schema version, and has no quarantine/reset recovery path.
- Impact: truncation, manual damage, incompatible future schema changes, or filesystem errors can prevent the client from opening, so the user cannot reach the built-in "Forget Server" recovery action.
- Fix: use an atomic replace, version the persisted schema, validate the loaded URL/certificate/fingerprint, and recover to an explicit repair/reset screen rather than terminating setup.
- Secret exposure: no

### [HIGH] Administrative CLI passwords - Optional plaintext process arguments are supported
- Location: `server/src/main.rs:25-35`, `server/src/auth.rs`.
- Issue: `create-admin` and `reset-password` accept `--password <PASSWORD>`. Non-interactive execution explicitly requires that flag.
- Impact: plaintext passwords can be retained in shell history, process listings, service logs, automation transcripts, or job metadata.
- Fix: remove the plaintext argument or replace it with protected stdin/file-descriptor input and document a secret-safe automation method.
- Secret exposure: no committed secret; runtime exposure path exists

### [HIGH] `DATABASE_URL` - Documentation declares a variable that current code/tests do not read
- Location: `README.md:79`; repository source and test configuration.
- Issue: the README states PostgreSQL integration tests require `DATABASE_URL`, but no current source, test, script, or configuration reads it.
- Impact: operators and CI maintainers can supply a disposable database URL and incorrectly believe integration coverage is active.
- Fix: either implement and gate the documented integration tests on `DATABASE_URL`, or remove the statement until that test path exists.
- Secret exposure: no

### [MEDIUM] `RUST_LOG` - Runtime log control is undocumented
- Location: `server/src/main.rs:53-57`, server operations documentation, systemd unit.
- Issue: the server reads the standard `RUST_LOG` environment variable, but supported values, the default filter, and the systemd override method are not documented.
- Impact: operators lack a defined method to tune diagnostics and may enable excessively verbose dependency logging without understanding the operational or data-exposure implications.
- Fix: document the default, safe examples, and a systemd drop-in method; caution against trace logging in production.
- Secret exposure: no

### [MEDIUM] Debian PostgreSQL dependency - Package behavior conflicts with remote-database documentation
- Location: `server/packaging/debian/control`, `server/packaging/antminer-fleet-server.service`, `README.md:21`.
- Issue: documentation permits PostgreSQL on another operator-controlled host, but the Debian package unconditionally depends on the `postgresql` metapackage and the unit orders itself after `postgresql.service`.
- Impact: remote-database deployments install and couple to an unnecessary local database service, increasing resource use and operational ambiguity.
- Fix: depend on the required client/runtime libraries only, and make local PostgreSQL an optional/recommended deployment choice; remove hard ordering on the local service if remote PostgreSQL is supported.
- Secret exposure: no

### [MEDIUM] Server option semantics - Defaults and valid operating ranges are incomplete
- Location: `server/config/server.example.toml`, `server/src/config.rs`, `README.md`, `server/README.md`.
- Issue: the example supplies values but does not document defaults when keys are omitted, valid ranges, URL percent-encoding requirements, wildcard-listen exposure, session-lifetime tradeoffs, certificate rotation behavior on the server, or pool-sizing guidance.
- Impact: valid TOML can still encode poor or nonfunctional deployment choices, and operators lack enough information to distinguish required values from tunables.
- Fix: document every key, default, required status, accepted format/range, security impact, and restart/rotation behavior.
- Secret exposure: no

### [LOW] Local deployment artifacts - Secret-bearing filenames are not ignored
- Location: `.gitignore`.
- Issue: no repository ignore rule covers local `server.toml`, TLS private-key extensions, or similar operator-created deployment files.
- Impact: a developer who stages a local test deployment inside the repository could accidentally add a database credential or private key.
- Fix: ignore conventional local deployment config/private-key paths while keeping the checked-in example explicitly allowed.
- Secret exposure: no current secret found

## Variable Map
| Variable | Read in Source | Documented | Default | Required | Sensitive | Notes |
|---|---|---|---|---|---|---|
| `RUST_LOG` | yes | no | `antminer_fleet_server=info,tower_http=info` | no | potentially | Standard tracing filter; arbitrary verbose filters are accepted. |
| `DATABASE_URL` | no | yes | none | no in current code | yes | README claims integration-test use, but no reader exists. |
| `listen` | yes | example only | none | yes | no | Parsed as `SocketAddr`; example exposes all interfaces on port 8443. |
| `session_days` | yes | example only | `30` when omitted | no | no | Only lower bound `>= 1` is validated. |
| `database.url` | yes | partial | none | yes | yes | Stored inline in TOML; non-empty only validation. |
| `database.max_connections` | yes | example only | `10` when omitted | no | no | Zero and unreasonable values are not rejected by `validate-config`. |
| `tls.certificate` | yes | partial | none | yes | no | File existence/content is checked only during server startup. |
| `tls.private_key` | yes | partial | none | yes | yes | Path is required structurally; file protection is documented but not validated. |
| Desktop `url` | yes | partial | none | required after pairing | no | HTTPS/host validation occurs while pairing, not when persisted config is loaded. |
| Desktop `certificate_pem` | yes | partial | none | required after pairing | no | Public certificate persisted in app data and used as a trust root. |
| Desktop `fingerprint_sha256` | yes | partial | none | required after pairing | no | Recomputed during pairing, but not revalidated when persisted config loads. |
| Desktop credential token | yes | yes | none | required after login | yes | Stored in OS credential manager, not `server.json`. |

## Documentation Mismatches
- Code reads but docs omit: `RUST_LOG`; server TOML defaults and accepted ranges; persisted `server.json` failure/recovery behavior.
- Docs declare but code does not read: `DATABASE_URL`.
- Naming mismatches: none among the six server TOML keys.
- Deployment mismatch: remote PostgreSQL is documented as supported, while the Debian package requires and orders against local PostgreSQL.
- Client documentation accurately states that one URL/certificate profile is stored in app data, the token is stored in the OS credential manager, and passwords are not persisted.

## Summary Metrics
- Critical: 0
- High: 4
- Medium: 3
- Low: 1
- Undocumented environment variables: 1
- Unused documented environment variables: 1
- Hardcoded real secrets: 0

## Verdict
BLOCKED

The configuration layer is not deploy-ready. The package installs a required database credential placeholder that passes `validate-config`, and the validator also accepts invalid database, connection-pool, and TLS settings. A malformed persisted desktop profile can additionally prevent the client from starting. No real committed secret was found, but the administrative plaintext password option is unsafe for automation.

## Next Actions
1. Make `validate-config` prove deployability: reject placeholders, validate PostgreSQL URL and pool bounds, and verify TLS material with a mode appropriate to generation versus serving.
2. Add recoverable, atomic, versioned handling for desktop `server.json`.
3. Remove plaintext administrative password arguments and provide protected non-interactive input.
4. Align `DATABASE_URL`, `RUST_LOG`, remote PostgreSQL packaging, and every server TOML key with operator documentation.
5. Validate the systemd unit, maintainer scripts, and built Debian package on Debian/Ubuntu before release.
