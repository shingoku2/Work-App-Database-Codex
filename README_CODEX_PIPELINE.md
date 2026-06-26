# Codex Agent Pipeline

This folder contains Codex CLI-ready agent prompts converted from the original Claude-oriented agent Markdown files.

## Suggested Layout

```text
AGENTS.md
.codex/
  pipeline/
    CODEX_DEP_AUDITOR.md
    CODEX_LICENSE.md
    CODEX_ENV_VALIDATOR.md
    CODEX_AUDITOR.md
    CODEX_FIXER.md
    CODEX_TEST_WRITER.md
    CODEX_REFACTOR.md
    CODEX_DOCS_WRITER.md
    CODEX_CHANGELOG.md
    CODEX_ONBOARDING.md
  reports/
```

## Running One Stage

From the repo root:

```bash
cat .codex/pipeline/CODEX_AUDITOR.md | codex exec --sandbox workspace-write -
```

Use `workspace-write` when a stage needs to write `.codex/reports/`. The auditor-style prompts are instructed not to modify application source even when workspace writing is available.

## Running the Pipeline Manually

Run stages in order:

```text
1. Dependency audit
2. License audit
3. Environment/config validation
4. Code audit
5. Fixes
6. Tests
7. Refactor
8. Docs
9. Changelog
10. Onboarding simulation
```

Do not run fixer/refactor stages blindly against production branches. Let Codex create a diff, review it like it came from a suspicious intern with root access, then merge intentionally.

## Current Pipeline Status

The 0.3.0 cleanup pass removed stale root-level pipeline reports and migration
scratch files. New pipeline output still belongs under `.codex/reports/`; do
not recreate root-level report files.

Latest dependency-update sweep report:
`.codex/reports/2026-06-26_dependency-update-bug-sweep.md`.

Current frontend tooling requires Node.js 20.19.0+ on the Node 20 line. Keep
context files and package engine metadata aligned when dependency updates change
toolchain requirements.
