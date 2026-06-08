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
