#!/usr/bin/env bash
# Execute source-bound host/workflow tests and real CPU sampling evidence.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

expected_sha="${1:?Usage: qualify-workflow-text-controls.sh FULL_COMMIT_SHA [LOG_DIRECTORY]}"
[[ "$expected_sha" =~ ^[0-9a-f]{40}$ ]] || { echo 'A full lowercase commit SHA is required' >&2; exit 2; }
actual_sha="$(git rev-parse HEAD)"
[[ "$actual_sha" == "$expected_sha" ]] || { echo "Source mismatch: expected $expected_sha, found $actual_sha" >&2; exit 2; }
git diff --quiet
git diff --cached --quiet
[[ -z "$(git ls-files --others --exclude-standard -- crates scripts .github)" ]] || {
  echo 'Untracked source or qualification files prevent exact-head qualification' >&2; exit 2;
}

log_directory="${2:-$(mktemp -d "${TMPDIR:-/tmp}/pantograph-text-controls.XXXXXX")}"
mkdir -p "$log_directory"
log_directory="$(cd "$log_directory" && pwd)"
python_executable="${PANTOGRAPH_QUALIFICATION_PYTHON:-python3}"

{
  git show -s --format='commit=%H%ntree=%T%nparents=%P' HEAD
  cargo --version
  rustc --version
  "$python_executable" --version
  printf 'ORT_DYLIB_PATH=%s\n' "${ORT_DYLIB_PATH:-unset}"
} | tee "$log_directory/source-and-tools.log"

run_suite() {
  local label="$1" package="$2" filter="$3"
  shift 3
  local command=(cargo test --locked -p "$package" "$@" --lib "$filter")
  "${command[@]}" -- --list > "$log_directory/$label-discovery.log" 2>&1
  "$python_executable" - "$log_directory/$label-discovery.log" "$filter" <<'PY'
import pathlib, sys
names = [line for line in pathlib.Path(sys.argv[1]).read_text().splitlines()
         if line.startswith(sys.argv[2]) and line.endswith(": test")]
print(sys.argv[2], "tests discovered:", len(names))
if not names:
    raise SystemExit("Requested native suite contains no tests")
PY
  "${command[@]}" 2>&1 | tee "$log_directory/$label-tests.log"
  # A successful zero-test or all-ignored invocation is not qualification.
  "$python_executable" - "$log_directory/$label-tests.log" <<'PY'
import pathlib, re, sys
text = pathlib.Path(sys.argv[1]).read_text()
if not re.search(r"^test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;", text, re.M):
    raise SystemExit("Requested native suite did not execute all admitted tests")
PY
}

run_suite text-host pantograph-embedded-runtime runtime_host_text_execution::tests:: --features backend-pytorch
run_suite text-descriptor pantograph-embedded-runtime inference_interface_facts_provider::tests:: --features backend-pytorch
run_suite input-mapping pantograph-workflow-service workflow::runtime_host_task_input_mapping::tests::
run_suite authored-source pantograph-workflow-service workflow::external_input_materialization::tests::
run_suite public-session pantograph-workflow-service workflow::tests::session_execution::

cargo test --locked -p pantograph-runtime-host-contracts -p pantograph-inference-interface-contracts \
  2>&1 | tee "$log_directory/serialization-contract-tests.log"

HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 "$python_executable" - <<'PY'
import torch, transformers, unittest
print("torch", torch.__version__, "transformers", transformers.__version__)
if torch.version.cuda is not None:
    raise SystemExit("Qualification requires a CPU Torch build")
suite = unittest.defaultTestLoader.discover("crates/inference/torch/tests")
print("Sampler tests discovered:", suite.countTestCases())
if suite.countTestCases() < 6:
    raise SystemExit("Required real sampler tests were not discovered")
PY
HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 "$python_executable" -m unittest discover \
  -s crates/inference/torch/tests -v 2>&1 | tee "$log_directory/cpu-sampler-tests.log"

git diff --quiet
git diff --cached --quiet
[[ "$(git rev-parse HEAD)" == "$expected_sha" ]]
[[ -z "$(git ls-files --others --exclude-standard -- crates scripts .github)" ]]
(cd "$log_directory" && sha256sum *.log > SHA256SUMS)
printf 'Qualified host/workflow and CPU sampler source %s; logs: %s\n' "$expected_sha" "$log_directory"
