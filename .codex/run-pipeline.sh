#!/usr/bin/env bash
set -euo pipefail

mkdir -p .codex/reports

run_stage() {
  local prompt_file="$1"
  echo "=== Running ${prompt_file} ==="
  codex exec --sandbox workspace-write - < "$prompt_file"
}

run_stage .codex/pipeline/CODEX_DEP_AUDITOR.md
run_stage .codex/pipeline/CODEX_LICENSE.md
run_stage .codex/pipeline/CODEX_ENV_VALIDATOR.md
run_stage .codex/pipeline/CODEX_AUDITOR.md
run_stage .codex/pipeline/CODEX_FIXER.md
run_stage .codex/pipeline/CODEX_TEST_WRITER.md
run_stage .codex/pipeline/CODEX_REFACTOR.md
run_stage .codex/pipeline/CODEX_DOCS_WRITER.md
run_stage .codex/pipeline/CODEX_CHANGELOG.md
run_stage .codex/pipeline/CODEX_ONBOARDING.md

echo "Pipeline complete. Review .codex/reports/ and the working tree diff."
