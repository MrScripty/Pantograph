#!/usr/bin/env bash
set -euo pipefail

SOURCE_ROOTS="${TRACEABILITY_SOURCE_ROOTS:-src,src-tauri/src,crates,packages/svelte-graph/src,scripts,.pantograph}"
ADR_DIR="${ADR_DIR:-docs/adr}"
BASE_REF="${TRACEABILITY_BASE_REF:-origin/main}"
STAGED_ONLY="${TRACEABILITY_STAGED_ONLY:-0}"
HOST_FACING_DIRS="${TRACEABILITY_HOST_FACING_DIRS:-src,src-tauri/src/workflow,src-tauri/src/llm/commands,src-tauri/src/llm/commands/registry,crates/pantograph-workflow-service,crates/pantograph-workflow-service/src,crates/pantograph-workflow-service/src/workflow,crates/pantograph-workflow-service/tests,crates/pantograph-workflow-service/examples,crates/pantograph-embedded-runtime,crates/pantograph-uniffi,crates/pantograph-uniffi/src,crates/pantograph-uniffi/src/bin,crates/pantograph-rustler,crates/pantograph-rustler/src,crates/pantograph-frontend-http-adapter,crates/inference/torch,crates/inference/depth,crates/inference/audio}"
STRUCTURED_PRODUCER_DIRS="${TRACEABILITY_STRUCTURED_PRODUCER_DIRS:-src,src/generated,src-tauri/src/workflow,crates/pantograph-workflow-service,crates/pantograph-workflow-service/src,crates/pantograph-workflow-service/src/workflow,crates/pantograph-workflow-service/tests,crates/pantograph-embedded-runtime,crates/pantograph-uniffi,crates/pantograph-uniffi/src,crates/pantograph-uniffi/src/bin,crates/pantograph-rustler,crates/pantograph-rustler/src,crates/inference/torch,crates/inference/depth,crates/inference/audio,crates/inference/src/managed_runtime,crates/inference/src/managed_runtime/llama_cpp_platform,crates/inference/src/managed_runtime/ollama_platform,.pantograph,.pantograph/workflows,.pantograph/orchestrations}"

required_headers=(
  "## Purpose"
  "## Contents"
  "## Problem"
  "## Constraints"
  "## Decision"
  "## Alternatives Rejected"
  "## Invariants"
  "## Revisit Triggers"
  "## Dependencies"
  "## Related ADRs"
  "## Usage Examples"
)

host_facing_headers=("## API Consumer Contract")
structured_producer_headers=("## Structured Producer Contract")

banned_placeholders=(
  "Source file used by modules in this directory."
  "Subdirectory containing related implementation details."
  "Keep files in this directory scoped to a single responsibility boundary."
  "import { value } from './module';"
)

trim_dir() {
  local dir_path="$1"
  dir_path="${dir_path#./}"
  dir_path="${dir_path%/}"
  printf '%s\n' "$dir_path"
}

csv_contains_dir() {
  local needle
  local haystack="$2"
  local item
  needle="$(trim_dir "$1")"

  [ -n "$haystack" ] || return 1

  IFS=',' read -ra items <<< "$haystack"
  for item in "${items[@]}"; do
    if [ "$(trim_dir "$item")" = "$needle" ]; then
      return 0
    fi
  done

  return 1
}

extract_section_body() {
  local header="$1"
  local file="$2"

  awk -v header="$header" '
    $0 == header { in_section = 1; next }
    in_section && /^## / { exit }
    in_section { print }
  ' "$file"
}

file_in_source_root() {
  local file="$1"
  local root

  IFS=',' read -ra roots <<< "$SOURCE_ROOTS"
  for root in "${roots[@]}"; do
    root="$(trim_dir "$root")"
    if [ "$file" = "$root" ] || [[ "$file" == "$root/"* ]]; then
      return 0
    fi
  done

  return 1
}

resolve_diff_range() {
  if git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
    printf '%s...HEAD\n' "$BASE_REF"
  elif git rev-parse --verify "origin/master" >/dev/null 2>&1; then
    printf 'origin/master...HEAD\n'
  elif git rev-parse --verify "main" >/dev/null 2>&1; then
    printf 'main...HEAD\n'
  elif git rev-parse --verify "master" >/dev/null 2>&1; then
    printf 'master...HEAD\n'
  elif git rev-parse --verify "HEAD~1" >/dev/null 2>&1; then
    printf 'HEAD~1...HEAD\n'
  else
    return 1
  fi
}

changed_files_for_mode() {
  local diff_range

  if [ "$STAGED_ONLY" = "1" ]; then
    git diff --cached --name-only --diff-filter=ACMR
    return 0
  fi

  diff_range="$(resolve_diff_range)" || return 1
  git diff --name-only --diff-filter=ACMR "$diff_range"
}

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Not inside a git repository."
  exit 1
fi

if ! command -v rg >/dev/null 2>&1; then
  echo "Missing required tool: rg"
  exit 1
fi

if ! mapfile -t changed_files < <(changed_files_for_mode); then
  echo "Skipping decision traceability check: unable to resolve changed files."
  exit 0
fi

if [ "${#changed_files[@]}" -eq 0 ]; then
  echo "No changed files detected for decision traceability check."
  exit 0
fi

declare -A changed_lookup=()
for file in "${changed_files[@]}"; do
  changed_lookup["$file"]=1
done

adr_changed=false
for file in "${changed_files[@]}"; do
  if [[ "$file" == "$ADR_DIR/"*.md ]]; then
    adr_changed=true
    break
  fi
done

declare -A changed_dirs=()
for file in "${changed_files[@]}"; do
  if file_in_source_root "$file"; then
    changed_dirs["$(dirname "$file")"]=1
  fi
done

if [ "${#changed_dirs[@]}" -eq 0 ]; then
  echo "No source-directory changes detected for decision traceability check."
  exit 0
fi

failures=0
for module_dir in "${!changed_dirs[@]}"; do
  readme_path="$module_dir/README.md"
  required_headers_for_dir=("${required_headers[@]}")

  if csv_contains_dir "$module_dir" "$HOST_FACING_DIRS"; then
    required_headers_for_dir+=("${host_facing_headers[@]}")
  fi
  if csv_contains_dir "$module_dir" "$STRUCTURED_PRODUCER_DIRS"; then
    required_headers_for_dir+=("${structured_producer_headers[@]}")
  fi

  if [ ! -f "$readme_path" ]; then
    echo "Missing README.md for changed directory: $module_dir"
    failures=$((failures + 1))
    continue
  fi

  missing_header=false
  for header in "${required_headers_for_dir[@]}"; do
    if ! rg -F -x -q "$header" "$readme_path"; then
      echo "Missing required heading in $readme_path: $header"
      missing_header=true
    fi
  done
  if [ "$missing_header" = true ]; then
    failures=$((failures + 1))
  fi

  none_format_invalid=false
  for header in "${required_headers_for_dir[@]}"; do
    section_body="$(extract_section_body "$header" "$readme_path")"
    if printf '%s\n' "$section_body" | rg -i -q '\bnone\b'; then
      if ! printf '%s\n' "$section_body" | rg -i -q 'reason:'; then
        echo "Section with None is missing Reason in $readme_path: $header"
        none_format_invalid=true
      fi
      if ! printf '%s\n' "$section_body" | rg -i -q 'revisit trigger:'; then
        echo "Section with None is missing Revisit trigger in $readme_path: $header"
        none_format_invalid=true
      fi
    fi
  done
  if [ "$none_format_invalid" = true ]; then
    failures=$((failures + 1))
  fi

  placeholder_found=false
  for phrase in "${banned_placeholders[@]}"; do
    if rg -F -q "$phrase" "$readme_path"; then
      echo "Banned placeholder phrase in $readme_path: $phrase"
      placeholder_found=true
    fi
  done
  if [ "$placeholder_found" = true ]; then
    failures=$((failures + 1))
  fi

  if [ -z "${changed_lookup["$readme_path"]+set}" ] && [ "$adr_changed" = false ]; then
    echo "Changed directory without decision traceability update: $module_dir"
    echo "Update $readme_path or add/update an ADR under $ADR_DIR/."
    failures=$((failures + 1))
  fi
done

if [ "$failures" -gt 0 ]; then
  echo "Decision traceability check failed ($failures issue(s))."
  exit 1
fi

echo "Decision traceability check passed."
