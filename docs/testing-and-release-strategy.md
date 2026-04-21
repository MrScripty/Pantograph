# Testing and Release Strategy

## Purpose

This document defines Pantograph's test placement, acceptance paths, and release
smoke strategy. It turns the hybrid test layout into repo policy so local
commands, CI jobs, and release checks can evolve without changing what each
layer is responsible for proving.

## Test Placement

Pantograph uses a hybrid placement strategy by layer:

| Layer | Placement | Primary command |
| ----- | --------- | --------------- |
| Frontend and Svelte graph package | Colocated `*.test.ts` files beside source modules | `npm run test:frontend` |
| Rust crates | Crate-local unit tests and crate `tests/` integration tests when a shared harness is needed | `cargo test -p <crate> --lib` or `cargo test -p <crate>` |
| Runtime and binding smoke checks | Root `scripts/` entrypoints because they exercise packaged artifacts, generated bindings, or host runtimes | Script-specific check commands |
| Cross-layer acceptance | The smallest command that exercises the producer-to-consumer path through the real boundary | `./launcher.sh --test` plus targeted binding or smoke scripts |

Colocated tests are preferred for module-local frontend behavior and Rust helper
logic because discoverability matters more than a single central test tree.
Integration or smoke tests stay in `scripts/` or crate `tests/` when they need
shared fixtures, packaged artifacts, native library paths, or process/runtime
setup.

## Canonical Local Gate

`./launcher.sh --test` is the canonical local quality gate. It runs:

- `npm run lint:critical`
- `npm run typecheck`
- `npm run test:frontend`
- `cargo check --workspace --all-features`
- `cargo check --workspace --no-default-features`
- `cargo test -p node-engine --lib`
- `cargo test -p workflow-nodes --lib`

Use targeted commands in addition to the launcher gate when a change touches a
specialized surface. Examples:

- Runtime separation: `npm run test:runtime-separation`
- UniFFI C# surface: `./scripts/check-uniffi-csharp-smoke.sh`
- Packaged C# quickstart: `./scripts/check-packaged-csharp-quickstart.sh`
- Diffusion host smoke:
  `./scripts/check-uniffi-csharp-diffusion-smoke.sh` with the required
  `PANTOGRAPH_DIFFUSION_SMOKE_*` environment variables

## Acceptance Policy

Changes that cross a producer/consumer boundary need one acceptance path through
the real boundary. Typecheck, isolated unit tests, or wrapper-only tests are not
enough for those changes.

Required acceptance coverage by change type:

| Change type | Required acceptance path |
| ----------- | ------------------------ |
| Frontend graph contract or mutation response | Colocated frontend/package tests that exercise the serialized response shape consumed by the UI |
| Workflow service or embedded runtime contract | Native Rust tests for contract shaping plus the smallest host-facing binding smoke when exported behavior changes |
| Generated or packaged bindings | Native-side contract tests and a host-language smoke that loads the generated or packaged artifact from the same build |
| Durable runtime state, replay, recovery, or background worker behavior | Tests that use isolated temp roots or explicit serialization for process-global state |

Any test that mutates environment variables, temp roots, registry files, cache
roots, ports, or process-global state must either own isolated state for the
test or serialize the suite with a documented guard.

## Release Smoke Strategy

`./launcher.sh --release-smoke` is the local and CI entrypoint for release
artifact sanity checks. The current implementation verifies that a built release
artifact exists, then runs the runtime redistributables smoke script. It does
not yet launch the full GUI because Pantograph does not expose a headless
desktop startup probe.

When CI adds the full GUI release smoke, the job must own these constraints:

- Build the release artifact with `./launcher.sh --build-release` or download
  the artifact from the same workflow run.
- Run `./launcher.sh --release-smoke` from a clean checkout after dependency
  installation.
- Use a declared display server strategy on Linux, preferably `xvfb-run` around
  a dedicated bounded smoke script.
- Keep CI-only launch flags isolated to the release smoke path; normal
  interactive `--run` and `--run-release` behavior must not be weakened.
- Use a launcher-managed state root or another explicit temporary path so the
  smoke does not depend on or mutate operator desktop state.
- Declare GPU and sandbox behavior in the workflow. If software rendering or
  sandbox-disabling flags become necessary for CI, they belong only in the
  bounded smoke command.
- Declare shared-memory behavior explicitly. If the runner needs a larger
  `/dev/shm`, a temporary profile directory, or a tool-specific shared-memory
  flag, keep that setting inside the release smoke job.
- Bound startup with a timeout and fail if the process exits early, fails to
  create the expected ready signal, or needs undeclared runner state.
- Preserve the existing redistributables smoke checks before any GUI process
  launch so packaging regressions fail quickly.

Release CI should keep the GUI smoke separate from artifact packaging and
checksumming jobs. Packaging proves distributable shape; release smoke proves a
built artifact can satisfy the minimum runtime startup contract in a controlled
runner environment.
