# Stage 11 Coordination Ledger

Stage `11` completed `wave-01-preflight-contract-audit` and the ArtifactStore
backend portions of Wave `02`.

## Current Status

- Active stage plan:
  `docs/plans/pantograph-execution-platform/11-artifact-format-settings-and-managed-media-dependencies.md`.
- Required first wave: `wave-01-preflight-contract-audit`.
- Source and test edits for Stage `11` are currently limited to the Wave `01`
  backend contract freeze and Wave `02` ArtifactStore backend work in
  `crates/pantograph-workflow-service`.
- The current work completed stage-start preflight, source audit, shared
  contract freeze, and implementation-wave scaffolding.
- Existing unrelated dirty files before Stage `11`: deleted
  `.pantograph/workflows/tiny-sd-turbo-diffusion.json`, deleted image assets,
  untracked `.pantograph/workflow-diagnostics.sqlite`, and untracked assets.
  They remain outside the Stage `11` write set.

## Wave Status

| Wave | Status | Integration Notes |
| ---- | ------ | ----------------- |
| `wave-01-preflight-contract-audit` | Complete | Backend ArtifactStore descriptor, handle-based access, format settings, media capability, and managed redistributable DTOs are frozen with contract snapshots. |
| `wave-02-artifact-store-backend` | Complete | ArtifactStore core, private disk persistence, restart reconciliation, consume acknowledgement, cleanup, memory-cache enforcement, disk-budget enforcement, stream persistence, finalize lifecycle, service facade, and focused tests are implemented. Execution cutover and diagnostics descriptor linking remain assigned to Wave `04`. |
| `wave-03-managed-redistributables` | In progress | First worker owns the backend `inference` managed media redistributables module for tool/native dependency definitions, status projection, and tests. Existing sidecar runtime code remains read-only for this first split. |
| `wave-04-execution-diagnostics-cutover` | Pending | Depends on backend ArtifactStore and managed format capability contracts. |
| `wave-05-api-bindings-frontend-settings` | Pending | Depends on backend Settings/capability APIs and binary-safe body access contracts. |

## Required First Actions

1. Apply `08-stage-start-implementation-gate.md`.
2. Inspect current dirty files and confirm Stage `11` write-set safety.
3. Audit existing `base64`, `image_base64`, `audio_base64`, data-url, and
   oversized media JSON paths.
4. Freeze ArtifactStore, format settings, capability, and managed
   redistributable DTOs before source implementation begins.
5. Decide whether subsequent waves can run concurrently and record
   non-overlapping write sets before launching workers.

## Source Audit Snapshot

Initial local audit found these migration families:

| Area | Current Behavior | Stage `11` Migration Decision |
| ---- | ---------------- | ----------------------------- |
| Workflow nodes | `image-input`, `audio-input`, `image-output`, and `audio-output` nodes pass base64 strings through workflow context. | Replace media bodies with ArtifactStore descriptors while preserving graph-visible typed media ports. |
| Embedded runtime | Python bridge maps `image_base64` and `audio_base64` worker responses into workflow outputs and stream chunks. | Convert worker media bodies to ArtifactStore artifacts before workflow-output validation; stream chunks become artifact chunk metadata/revisions. |
| Python workers | Torch, ONNX, Stable Audio, and depth workers exchange base64 media in JSON. | Keep worker-local compatibility only behind backend conversion adapters; public Pantograph API must expose descriptors and binary-safe reads. |
| Frontend nodes | Svelte image/audio input and output nodes build data URLs and decode base64 in the browser. | Frontend consumes backend descriptors, read handles, stream handles, and blob/object URLs from binary-safe APIs; no persistent global settings outside Settings. |
| Binding smoke/examples | C# native smoke decodes image base64 for current compatibility checks. | Binding support must move to descriptor/read-handle DTO parity and host-language binary retrieval smoke. |
| Workflow value validation | `max_value_bytes` rejects large JSON payloads. | Keep bounded JSON validation; media payloads must be converted before the validation boundary. |

## Open Decisions

- Whether the managed redistributables boundary is generalized in place from
  `managed_runtime` or split into a new dependency/tool/library boundary that
  reuses lower-level helpers. Wave `01` froze DTO categories but did not choose
  implementation module ownership.
- Exact crate/module ownership for ArtifactStore physical payload storage.
  Wave `01` froze workflow-service public contracts but did not implement body
  storage.
- Exact crate/module ownership for OCIO safe wrapper and native-library
  loading.
- Exact persistent store owner for global artifact format defaults and
  ArtifactStore policy. Wave `01` froze settings/policy DTO shape but did not
  implement persistence.
- Which old Settings surfaces are embedded into the workbench Settings page and
  which are retired.

## Wave 03 Worker Split

| Owner | Scope | Primary Write Set | Forbidden Shared Files | Report |
| ----- | ----- | ----------------- | ---------------------- | ------ |
| Managed media redistributables worker | Add a backend-owned non-runtime redistributables module for ffmpeg, `ocioconvert`, `oiiotool`, and OpenColorIO definitions, status projection, activation metadata scaffolding, and tests. | `crates/inference/src/managed_redistributables.rs`, `crates/inference/src/managed_redistributables/**`, `crates/inference/src/lib.rs`, `crates/inference/tests/managed_redistributables.rs`, `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/reports/wave-03-worker-managed-media-redistributables.md` | Existing `crates/inference/src/managed_runtime/**` sidecar runtime implementation, frontend files, binding files, ArtifactStore files, generated output, `.pantograph/**`, and `assets/**`. | `reports/wave-03-worker-managed-media-redistributables.md` |

Integration rule: the worker may read existing managed runtime code for
patterns, but tool binaries and OpenColorIO must not be modeled with
runtime-only DTO names or host PATH discovery as the source of truth.

## Verification Ledger

- 2026-04-29 Wave `02` ArtifactStore core-slice verification passed:
  `cargo test -p pantograph-workflow-service --test artifact_store`,
  `cargo test -p pantograph-workflow-service --test artifact_contract`,
  `cargo test -p pantograph-workflow-service --test contract`,
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`,
  and `cargo fmt --all -- --check`.
- 2026-04-29 Wave `02` ArtifactStore stream/cache/disk-policy integration
  passed: `cargo test -p pantograph-workflow-service --test artifact_store`,
  `cargo test -p pantograph-workflow-service --test artifact_store_policy`,
  `cargo test -p pantograph-workflow-service --test artifact_contract`, and
  `cargo fmt --all -- --check`.
- 2026-04-29 Separate clippy cleanup commit cleared existing
  workflow-service lints discovered by the Wave `02` package clippy gate.
- 2026-04-29 Wave `01` verification passed:
  `cargo test -p pantograph-workflow-service --test artifact_contract`,
  `cargo test -p pantograph-workflow-service --test contract`, and
  `cargo fmt --all -- --check`.
- 2026-04-29 Stage-start preflight recorded dirty-file isolation, standards
  reviewed, source-audit families, required first wave, and expected first
  verification commands.
