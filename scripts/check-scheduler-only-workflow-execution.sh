#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

source_roots=(
  "src"
  "src-tauri/src"
  "packages/svelte-graph/src"
  "crates/pantograph-workflow-service/src"
  "crates/pantograph-embedded-runtime/src"
  "crates/pantograph-uniffi/src"
  "crates/pantograph-rustler/src"
  "crates/pantograph-frontend-http-adapter/src"
)

for root in "${source_roots[@]}"; do
  if [[ ! -e "$root" ]]; then
    echo "Scheduler-only workflow execution check expected source root '$root'" >&2
    exit 1
  fi
done

scan_forbidden() {
  local pattern="$1"
  local description="$2"
  local matches
  matches="$(
    rg -n \
      --glob '*.rs' \
      --glob '*.ts' \
      --glob '*.svelte' \
      "$pattern" \
      "${source_roots[@]}" || true
  )"
  if [[ -n "$matches" ]]; then
    echo "Forbidden direct workflow execution surface found: $description" >&2
    echo "$matches" >&2
    exit 1
  fi
}

scan_forbidden 'pub[[:space:]]+async[[:space:]]+fn[[:space:]]+workflow_run\b' \
  'public Rust workflow_run method'
scan_forbidden 'pub[[:space:]]+fn[[:space:]]+workflow_run\b' \
  'public Rust workflow_run function'
scan_forbidden 'pub\(crate\)[[:space:]]+fn[[:space:]]+workflow_run\b' \
  'crate-visible binding workflow_run function'
scan_forbidden 'fn[[:space:]]+frontend_http_workflow_run\b' \
  'Rustler frontend_http_workflow_run NIF'
scan_forbidden 'frontend_http_workflow_run\b' \
  'frontend HTTP direct workflow_run binding export'
scan_forbidden 'workflow_run_attributed\b' \
  'direct attributed workflow_run entrypoint'
scan_forbidden 'execute_workflow_v2\b' \
  'Tauri raw graph execute_workflow_v2 command'
scan_forbidden 'executeWorkflow[[:space:]]*\(' \
  'frontend raw graph executeWorkflow API'

echo "Verified scheduler-only workflow execution public surfaces."
