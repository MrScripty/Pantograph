# scripts

## Purpose
This directory contains developer-facing validation and smoke-test scripts used
to verify Pantograph build, runtime, and model-integration behavior outside the
main app entrypoint.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `check-runtime-redistributables-smoke.sh` | Verifies a built Pantograph release artifact exists, then runs the bounded managed-runtime contract smoke that covers runtime-manager view projection, workflow preflight blocking, and diagnostics projection. |
| `check-no-python-linkage.sh` | Verifies the runtime-separation guarantee that Pantograph no longer links Python in-process. |
| `check-packaged-csharp-quickstart.sh` | Compiles the artifact-staged C# quickstart against the generated binding with Roslyn and .NET reference assemblies, then runs the authoring path against the packaged native library; does not restore NuGet packages. |
| `check-uniffi-csharp-diffusion-smoke.sh` | Opt-in generated-C#/native-runtime session diffusion smoke; requires a local diffusers model directory and Python environment. |
| `check-uniffi-csharp-smoke.sh` | Builds the Pantograph headless native library via `pantograph-uniffi`, generates C# into `target/`, compiles a small C# smoke harness, and runs a session-first harness against the direct embedded runtime. |
| `check-uniffi-embedded-runtime-surface.sh` | Builds `pantograph-uniffi`, extracts UniFFI metadata, and verifies the direct embedded runtime object plus workflow/session methods are exported. |
| `diffusion_cli_smoketest.py` | Loads the Pantograph diffusion worker directly against a local diffusers bundle such as tiny-sd-turbo. |
| `package-uniffi-csharp-artifacts.sh` | Builds the Pantograph headless native library, generates C#, stages docs/examples, and writes separate C# binding and native-library zip artifacts under `target/bindings-package/artifacts/`. |
| `trado_cli_smoketest.py` | Exercises the local TraDo/dLLM path outside the app runtime. |
| `validate-lint.mjs` | Runs or scopes lint validation helpers. |
| `validate-svelte.mjs` | Checks Svelte-specific build and validation expectations. |

## Problem
Some failures are easiest to isolate outside the desktop app itself, especially
runtime-boundary issues such as Python worker loading, local model compatibility,
or targeted validation script behavior.

## Constraints
- Scripts must be safe to run from the repository root.
- Smoke tests should exercise the same worker/runtime paths the app uses rather
  than introducing alternate execution logic.
- Validation scripts should stay focused and composable so launcher and CI flows
  can call them predictably.

## Decision
Keep one-off validation and smoke-test utilities here, separate from product
runtime code. The diffusion smoke test intentionally imports the same
`crates/inference/torch/worker.py` module Pantograph uses so local model issues
can be debugged without the full app UI in the loop.

## Alternatives Rejected
- Hide all runtime verification behind the desktop app only.
  Rejected because worker/runtime failures are harder to isolate that way.
- Put smoke tests in ad hoc shell snippets or wiki docs.
  Rejected because checked-in scripts are easier to review and rerun.

## Invariants
- Scripts run relative to the repository root.
- Smoke tests target real Pantograph worker/runtime modules, not forks of that
  logic.
- C# runtime execution smokes create workflow sessions before submitting runs.
- Validation scripts remain developer tools, not product runtime entrypoints.

## Revisit Triggers
- Scripts gain enough shared structure to justify a dedicated test harness.
- Operators begin depending on script output as a stable external interface.

## Dependencies
**Internal:** worker modules under `crates/inference/`, launcher/runtime docs,
and repo-local build configuration.

**External:** Bash, Node.js, Python, and any runtime libraries required by the
specific script being executed.

## Usage Examples
```bash
python3 -m py_compile scripts/diffusion_cli_smoketest.py
./.venv/bin/python scripts/diffusion_cli_smoketest.py --model-path /path/to/tiny-sd-turbo
./scripts/check-no-python-linkage.sh
./scripts/check-runtime-redistributables-smoke.sh
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH=/path/to/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-uniffi-csharp-diffusion-smoke.sh
```

## API Consumer Contract
None.
Reason: these scripts are internal developer/operator utilities, not a stable
public API surface.
Revisit trigger: external tooling starts depending on script arguments or output
schemas as a supported interface.

## Structured Producer Contract
None.
Reason: script stdout/stderr is diagnostic and may change unless a future script
is explicitly documented as machine-consumed.
Revisit trigger: CI, external tooling, or another repo begins parsing a script's
output structurally.
