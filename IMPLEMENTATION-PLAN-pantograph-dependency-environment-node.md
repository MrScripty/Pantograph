# Pantograph Plan: Dependency Environment Node (Auto + Manual, Persistent Envs)

## Status
In progress

## Current Source-of-Truth Summary

This is the active source of truth for the remaining dependency execution work
in Pantograph.

Completed baseline work now lives in:
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-resolve-only-hard-cutover.md`
  for the resolve-only dependency contract cutover
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-python-runtime-separation.md`
  for process-based Python runtime separation

This plan now owns the remaining dependency-environment lane: explicit
`dependency-environment` workflow steps, manual override patches, reusable
`environment_ref` outputs, deterministic environment reuse, and the remaining
backend/GUI lifecycle behavior that is broader than the completed resolve-only
baseline.

The narrower additive pin-awareness follow-on remains tracked in
`IMPLEMENTATION-PLAN-pantograph-pumas-pinning-update.md`. Older umbrella and
cutover planning documents now live under `docs/historical-plans/`.

## 1. Scope and Hard Cutover

- `Puma-Lib` responsibility is strictly limited to:
  - model selection metadata
  - resolver payload (`dependency_requirements`) display/passthrough
  - inference settings output
- `Puma-Lib` must not own dependency check/install lifecycle actions.
- New `Dependency Environment` node owns all dependency lifecycle behavior.
- Backward compatibility and migration compatibility layers are intentionally out of scope.

## 2. Contract Freeze (Implement First, Then Freeze)

Define and freeze these contracts before parallel implementation:

1. `DependencyRequirementsV1` (input contract to dependency node)
2. `DependencyOverridePatchV1` (manual override layer)
3. `EnvironmentRefV1` (output contract consumed by inference nodes)
4. `DependencyEnvironmentStatusV1` (node runtime status/diagnostics)

Required fields:

- `EnvironmentRefV1`:
  - `contract_version`
  - `environment_key`
  - `environment_kind`
  - `env_id`
  - `python_executable` (for python-family kinds)
  - `state`
  - `requirements_fingerprint`
  - `platform_key`
  - `backend_key`
  - `manifest_path`
- `DependencyOverridePatchV1`:
  - `binding_id`
  - `scope` (`binding` | `requirement`)
  - `fields` (supported override fields only)
  - `source` (`user`)
  - `updated_at`

Rules:

- Contract is append-only after freeze.
- All cross-layer and cross-language serialization must use the frozen schema.

## 3. Architecture and Layering

Implement with strict layering:

- Presentation: Svelte node UI only.
- Application: workflow commands/task orchestration.
- Domain: dependency environment service, state machine, fingerprint logic.
- Infrastructure: filesystem env store, pip/venv command runner, python resolver.

Dependency direction must point inward only.

## 4. Dependency Environment Node Behavior

### 4.1 Inputs

- `dependency_requirements` (required)
- `selected_binding_ids` (optional)
- `mode` (`auto` | `manual`)
- optional manual overrides from backend state

### 4.2 Outputs

- `environment_ref` (`EnvironmentRefV1`)
- `resolved_dependency_requirements` (base + override effective view)
- `status` (`DependencyEnvironmentStatusV1`)

### 4.3 State Machine

- `unresolved`
- `needs_user_input`
- `checking`
- `missing`
- `installing`
- `ready`
- `failed`

Transitions:

- Missing required install metadata in auto mode -> `needs_user_input`.
- Successful check with exact pins -> `ready`.
- Unsupported environment kind or failed install/check -> `failed`.

## 5. Auto + Manual Resolution Model

### 5.1 Auto Mode

- Use only Pumas declarative requirements plus known Pantograph defaults.
- No command execution hints from Pumas are trusted/executed.
- If inputs are insufficient (index/path/python unresolved), emit structured `needs_user_input` errors.

### 5.2 Manual Mode

- User can patch missing values in dependency node UI.
- Patch is stored as backend-owned override data, never as frontend-only source of truth.
- UI must show:
  - base value from Pumas
  - override value from Pantograph
  - effective merged value

Supported manual overrides (v1):

- `python_executable`
- `index_url`
- `extra_index_urls`
- `wheel_source_path`
- `package_source_override` (per package)

## 6. Persistent Environment Store (No Reinstall Every Run)

### 6.1 Environment Key

`environment_key` must be deterministic from:

- contract version
- `env_id`
- normalized sorted exact requirements
- normalized applied overrides
- `platform_key`
- python major/minor
- installer version

### 6.2 Storage

- Persist under Pantograph data dir (`.../envs/python/<environment_key>/`).
- Write `manifest.json` containing:
  - effective requirements
  - fingerprints
  - install metadata
  - created/last_used timestamps
  - validation result history

### 6.3 Reuse Rules

- On run: resolve key -> if env exists and validates, reuse.
- Install only on cache miss or failed validation.
- Environments are immutable; changed fingerprint creates a new env.

## 7. Security and Boundary Validation

Centralize validation in dedicated modules (no inline ad-hoc validation):

- path validation for local sources (`wheel_source_path`, local index paths)
- URL scheme/format validation for remote indexes
- python executable path validation
- strict payload shape validation at command boundary

Rules:

- Validate once at boundary; internal code assumes validated data.
- Reject unsupported URL schemes and invalid paths.
- Never execute raw user-provided shell strings; only structured command args.

## 8. Cross-Platform Strategy

Introduce platform abstractions (no inline OS branching in domain logic):

- `EnvironmentProvisioner` trait/interface
- `PythonRuntimeLocator` trait/interface
- platform-specific implementations selected by factory

Must support Linux + Windows required paths and macOS best-effort.
All filesystem operations must handle spaces safely.

## 9. Concurrency and Lifecycle

- Per-`environment_key` install lock to prevent duplicate concurrent installs.
- Keep related shared state under one lock.
- Do not hold async locks across blocking process/file operations.
- Track spawned tasks and guarantee shutdown cleanup.

## 10. Inference Integration

- Inference nodes consume `environment_ref` explicitly.
- If `environment_ref` missing or not `ready`, fail fast with actionable error.
- Remove hidden dependency lifecycle controls from inference/Puma-Lib surfaces.

## 11. Frontend UX Changes

- Remove Resolve/Check/Install actions from `Puma-Lib` node UI.
- Add `Dependency Environment` node UI with:
  - mode toggle (`auto/manual`)
  - base vs override vs effective requirements
  - missing-fields prompts in `needs_user_input`
  - status timeline and diagnostics

Frontend rules:

- Declarative rendering only.
- Event-driven state sync preferred; no global polling loops.
- If polling is required, timer lifecycle and cleanup tests are mandatory.

## 12. Testing Plan

### 12.1 Unit

- environment key determinism
- requirement normalization
- override merge logic
- state machine transitions
- validation module behavior

### 12.2 Integration

- first run installs env
- second run reuses env with no install
- manual-required flow (`needs_user_input` -> override -> ready)
- concurrent requests for same env result in single install

### 12.3 E2E

- graph path: `Puma-Lib -> Dependency Environment -> Audio/PyTorch Inference`
- stable-audio resolves to ready env and executes
- missing metadata scenario prompts for manual fields and succeeds after input

### 12.4 Acceptance

- No dependency lifecycle action remains in `Puma-Lib`.
- `environment_kind` supports `python` and `python-venv`.
- No reinstall on unchanged fingerprint.
- Explicit `needs_user_input` state appears for incomplete auto resolution.

## 13. Documentation and Directory Requirements

For any new `src` directories created during implementation:

- add/update directory `README.md`
- document public interfaces and contract schemas
- document operational behavior of env store and GC policy

## 14. Task Breakdown and Commit Instructions

Apply one commit per completed task. No commit should mix unrelated concerns.

### Task 1: Freeze Contracts and Status Model

Deliverables:

- frozen contract types and serialization tests
- version-gate enforcement

Verification before commit:

- `cargo check --manifest-path src-tauri/Cargo.toml`
- targeted contract tests

Commit:

- `git add <contract files> <tests>`
- `git commit -m "feat(workflow): add dependency environment v1 contracts"`

### Task 2: Domain Service + Env Key + Manifest

Deliverables:

- dependency environment service
- deterministic key/fingerprint
- manifest writer/reader

Verification before commit:

- unit tests for determinism and manifest roundtrip

Commit:

- `git add <domain files> <tests>`
- `git commit -m "feat(workflow): add dependency environment persistence and fingerprinting"`

### Task 3: Auto/Manual Merge + Validation Boundary

Deliverables:

- centralized validators
- override patch store and merge
- `needs_user_input` generation

Verification before commit:

- validation + merge tests

Commit:

- `git add <validation/merge files> <tests>`
- `git commit -m "feat(workflow): add manual override patching and boundary validation"`

### Task 4: Provisioner + Installer + Concurrency Controls

Deliverables:

- platform provisioner abstraction + factory
- install/check engine with per-env lock
- graceful task lifecycle wiring

Verification before commit:

- integration tests (single install under concurrency)

Commit:

- `git add <provisioner/install files> <tests>`
- `git commit -m "feat(workflow): add dependency environment installer and locking"`

### Task 5: New Dependency Environment Node + UI

Deliverables:

- node registration, commands, Svelte node
- auto/manual controls and diagnostics rendering

Verification before commit:

- `npm run typecheck`
- frontend tests for state rendering and interactions

Commit:

- `git add <node/ui files> <tests>`
- `git commit -m "feat(ui): add dependency environment node with auto/manual controls"`

### Task 6: Inference Wiring and Puma-Lib Cleanup

Deliverables:

- inference consumes `environment_ref`
- remove dependency lifecycle actions from `Puma-Lib`

Verification before commit:

- workflow integration tests
- stable-audio smoke run

Commit:

- `git add <inference + puma-lib files> <tests>`
- `git commit -m "refactor(workflow): route dependency lifecycle through dependency environment node"`

### Task 7: Docs + Final Acceptance Suite

Deliverables:

- updated docs/readmes
- acceptance test coverage summary

Verification before commit:

- full required checks for changed scope

Commit:

- `git add <docs + test artifacts>`
- `git commit -m "docs(workflow): document dependency environment architecture and operations"`

## 15. Quality Gates Per Task

Before each task commit:

1. Run relevant lint/typecheck/tests for touched layers.
2. Review staged diff only (`git diff --cached`).
3. Use conventional commit format with accurate scope.
4. Do not bypass failing blocking gates.
