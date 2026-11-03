set -euo pipefail
export ORT_SKIP_DOWNLOAD=1
tests=(
  core_executor::tests::inference_tests::test_canonical_llm_embedding_uses_typed_gateway_boundary
  core_executor::tests::inference_tests::test_canonical_llm_embedding_with_package_facts_emits_compatibility_lifecycle
  core_executor::tests::inference_tests::test_execute_llm_inference_non_streaming_uses_typed_gateway_boundary
)
cargo test -p node-engine --features inference-nodes --lib core_executor::tests::inference_tests:: -- --list > "$RUNNER_TEMP/node-typed-capture-tests.list"
for test_name in "${tests[@]}"; do
  python3 -c 'import pathlib, sys; lines = pathlib.Path(sys.argv[1]).read_text().splitlines(); sys.exit(0 if sys.argv[2] + ": test" in lines else 1)' "$RUNNER_TEMP/node-typed-capture-tests.list" "$test_name"
  result_path="$RUNNER_TEMP/node-typed-capture-${test_name##*::}.log"
  cargo test -p node-engine --features inference-nodes --lib "$test_name" -- --exact 2>&1 | tee "$result_path"
  python3 -c 'import pathlib, re, sys; text = pathlib.Path(sys.argv[1]).read_text(); sys.exit(0 if re.search(r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", text, re.M) else 1)' "$result_path"
done
