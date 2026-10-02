# scripts

## Purpose
This directory contains developer-facing validation and smoke-test scripts used
to verify Pantograph build, runtime, and model-integration behavior outside the
main app entrypoint.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `check-runtime-redistributables-smoke.sh` | Verifies a built Pantograph release artifact exists, then runs the bounded release contract smoke that covers managed-runtime view projection, runtime diagnostics projection, current image workflow shape, Pumas resolution, stale graph diagnostics, and image artifact retention. |
| `check-current-image-workflow-smoke.mjs` | Validates the bundled current image workflow template and tracked Juggernaut workflow still use canonical `puma-lib -> llm-inference -> image-output` graph shape without retired executable inference nodes. |
| `check-decision-traceability.sh` | Runs the decision-to-guide traceability gate over explicit Git snapshots using the reviewed impact map. |
| `check-decision-traceability.mjs` | Validates mapped decision impacts, canonical guides, and local ADR references without per-directory documentation rules. |
| `check-decision-traceability.test.mjs` | Exercises Git snapshot isolation, impact ownership, missing inputs, broken references, and path transitions. |
| `check-no-python-linkage.sh` | Verifies the runtime-separation guarantee that Pantograph no longer links Python in-process. |
| `check-scheduler-only-workflow-execution.sh` | Fails when public Rust, Tauri, binding, or frontend source reintroduces direct workflow execution APIs outside scheduler session execution. |
| `check-rustler-beam-smoke.sh` | Builds `pantograph_rustler`, verifies the local BEAM toolchain exists, and runs the Mix smoke harness under `bindings/beam/pantograph_native_smoke/`. |
| `check-packaged-csharp-quickstart.sh` | Compiles the artifact-staged C# quickstart against the generated binding with Roslyn and .NET reference assemblies, then runs the authoring path against the packaged native library; does not restore NuGet packages. |
| `check-workflow-image-generation-real-smoke.sh` | Opt-in real image-generation lane smoke that requires a Pumas Diffusers model id, validates canonical workflow shape, verifies the desktop Tauri crate builds with `backend-pytorch`, runs the generated-C#/native-runtime Diffusers session smoke, and checks workflow-editor command projections. This is an intermediate real-backend gate, not the final desktop GUI proof. |
| `check-workflow-editor-image-generation-gui-smoke.sh` | Opt-in desktop GUI smoke wrapper for the workflow-editor image-generation path. It preflights Pumas model/artifact ids, a saved workflow id, Python, Linux WebDriver/display prerequisites, and WebdriverIO before launching the Tauri WebDriver harness. |
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
- Decision traceability checks only declared decision-source impacts and local
  references. It does not infer semantic contract changes from source-directory
  names, require fixed headings, or accept an unrelated ADR as evidence.
- The gate reads the selected Git snapshots, including the map and guides. It
  fails on unavailable or unreadable inputs rather than guessing a branch or
  treating a failed diff as an empty change.
- Smoke tests target real Pantograph worker/runtime modules, not forks of that
  logic.
- Release contract smoke is headless: it validates the built artifact and
  canonical workflow/runtime contracts, not a full desktop GUI or model
  execution session.
- C# runtime execution smokes create workflow sessions before submitting runs.
- Public workflow execution validation rejects direct run APIs; callers must use
  scheduler session create/run/close surfaces.
- Validation scripts remain developer tools, not product runtime entrypoints.

## Revisit Triggers
- Scripts gain enough shared structure to justify a dedicated test harness.
- Operators begin depending on script output as a stable external interface.

## Dependencies
**Internal:** worker modules under `crates/inference/`, launcher/runtime docs,
and repo-local build configuration.

**External:** Bash, Node.js, Python, and any runtime libraries required by the
specific script being executed. Decision traceability additionally uses the
locked development-only `commonmark` package (BSD-2-Clause); run `npm ci` before
invoking it. The package owns Markdown parsing, avoiding a parallel regex
grammar; it does not render or execute document content.

## Related ADRs
- `docs/adr/ADR-011-scheduler-only-workflow-execution.md`
- Reason: the scheduler-only guardrail script enforces the public workflow
  execution boundary frozen by the ADR.
- Revisit trigger: workflow execution exposes a new public transport or binding
  surface that must be covered by guardrail scans.

## Usage Examples
```bash
python3 -m py_compile scripts/diffusion_cli_smoketest.py
./.venv/bin/python scripts/diffusion_cli_smoketest.py --model-path /path/to/tiny-sd-turbo
node scripts/check-current-image-workflow-smoke.mjs
TRACEABILITY_STAGED_ONLY=1 npm run lint:no-new
npm run lint:a11y
npm run format:check
npm run release:sbom -- 0.1.0
TRACEABILITY_STAGED_ONLY=1 ./scripts/check-decision-traceability.sh
node --test scripts/check-decision-traceability.test.mjs
./scripts/check-no-python-linkage.sh
./scripts/check-scheduler-only-workflow-execution.sh
./scripts/check-rustler-beam-smoke.sh
./scripts/generate-release-sbom.sh 0.1.0
./scripts/check-runtime-redistributables-smoke.sh
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID=diffusers \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-uniffi-csharp-diffusion-smoke.sh
PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID=diffusers \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-workflow-image-generation-real-smoke.sh
PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID=diffusers \
  PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID=tiny-sd-turbo-diffusion \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-workflow-editor-image-generation-gui-smoke.sh
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

## Decision Traceability Operation

The [impact map](decision-traceability-map.json) is owned by this guide. It maps
exact accepted decision documents to current documentation owners:

- ADR-001/011: host/service ownership and scheduler-session consumer lifecycle,
  owned by [headless integration](../docs/headless-workflow.md)
- ADR-002/003/007: runtime lifecycle, readiness, observability and recovery,
  owned by [runtime operations](../docs/runtime-operations.md)
- ADR-006/009: canonical/composed-node and migration responsibility,
  owned by [architecture](../ARCHITECTURE.md)
- ADR-017: contributor workspace policy and verification procedure,
  owned by [development](../docs/development.md)
- The map itself: the gate's declared coverage, owned by this guide

Changing a mapped decision or its mapping requires updating that boundary's
canonical guide. New or changed unmapped ADRs fail as unresolved coverage until
an owner and the affected knowledge are declared. Removing a row or moving a
trigger still evaluates its prior obligation. A moved guide must be replaced in
the current map and affected links; if the old guide remains, update it to
explain the migration. A retired row still requires its prior guide
to explain the change. The map admits exact decision-document paths only, not
whole source trees or mixed implementation files.

Staged mode (`TRACEABILITY_STAGED_ONLY=1`) compares HEAD with the index tree and
reads all content from those snapshots. Unstaged documents cannot repair the
index. Range mode requires both revisions explicitly:

```bash
TRACEABILITY_MODE=range TRACEABILITY_BASE_REF=<base-commit> \
  TRACEABILITY_HEAD_REF=<head-commit> npm run traceability
```

The gate reports the resolved input IDs. CI supplies event base/head commits
and fetches their history. Missing modes, refs, maps, owners or local targets
fail; unreadable Git state also fails. The old source-root/host/producer path
list overrides are rejected. The shell entrypoint explicitly selects this
repository's map; the Node implementation accepts `--map` for isolated tests or
an explicitly reviewed alternative.

Both prior and current maps are read. The sole migration case is the audited
pre-map commit `4938e405c7f656365eefdca492774ccae110c90d` with this exact map path.
At that base, current decision rows must also resolve in the prior snapshot;
there is no general missing-map opt-out. Remove that transition once supported
comparison bases all contain the map.

Local links in mapped guides and changed Markdown are checked, including ADR
paths in code spans. Unchanged readers are checked when an ADR or prior/current
canonical guide target changes, so moving/deleting owned documentation cannot
leave stale references. Untouched historical links are outside this change-scoped
gate. The development-only CommonMark parser owns Markdown syntax, including balanced
link destinations, used reference links and images. Fenced/indented examples
are not live links; inline code-span ADR paths remain the repository convention.
Tracked directory targets are valid. Object-key, boundary-row and trigger
ordering do not change mapped knowledge or create guide-edit obligations.
URL fragments and remote URLs are outside this file-existence check. A guide
edit proves traceability, not that its prose is substantively correct.

This gate does not prove semantic contract completeness. A code change that
alters consumer behavior, responsibility, invariants or procedures still needs
its canonical documentation under [documentation rules](../docs/README.md),
with review and applicable contract tests. Routine repairs under unchanged
contracts need no documentation churn. Existing host/binding, scheduler-only,
worker-protocol and structured-producer checks remain required; the removed
universal README headings never proved those contracts.
