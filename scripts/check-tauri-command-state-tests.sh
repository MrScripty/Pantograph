#!/usr/bin/env bash
set -euo pipefail

log_dir="${RUNNER_TEMP:-target}/tauri-command-state-tests"
mkdir -p "$log_dir"
cargo_args=(test -p pantograph --bin pantograph --no-default-features --features backend-llamacpp)
tests=(
  workflow::command_state::tests::run_bundle_preserves_managed_identity_without_payload_state
  workflow::command_state::tests::run_bundle_preserves_missing_state_order_and_original_camel_case_keys
  workflow::command_state::tests::query_bundle_preserves_managed_identity_without_payload_state
  workflow::command_state::tests::query_command_preserves_missing_state_order_and_original_keys
  workflow::command_state::tests::query_command_preserves_flat_required_optional_and_context_payloads
)

cargo "${cargo_args[@]}" workflow::command_state::tests:: -- --list > "$log_dir/tests.list"
for test_name in "${tests[@]}"; do
  python3 -c 'import pathlib, sys; lines = pathlib.Path(sys.argv[1]).read_text().splitlines(); sys.exit(0 if sys.argv[2] + ": test" in lines else 1)' "$log_dir/tests.list" "$test_name"
  result_path="$log_dir/${test_name##*::}.log"
  cargo "${cargo_args[@]}" "$test_name" -- --exact 2>&1 | tee "$result_path"
  python3 -c 'import pathlib, re, sys; text = pathlib.Path(sys.argv[1]).read_text(); sys.exit(0 if re.search(r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", text, re.M) else 1)' "$result_path"
done
