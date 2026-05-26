# crates/workflow-nodes/src/input

## Purpose
This directory contains workflow input-node descriptors and host-bridge stubs.
These nodes define the graph-facing contracts for user input, model selection,
and library-provided metadata before host-specific executors take over.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `puma_lib.rs` | Host-bridged model selector that publishes routing and dependency metadata from Pumas into workflow graphs. |
| `model_provider.rs` | Generic model selector descriptor/task used when the workflow is not backed by the Pantograph/Pumas library path. It does not own a separate `NodeExecutor` implementation. |
| `text_input.rs` | Freeform text source for prompts and other string inputs. |
| `number_input.rs` | Numeric source node that adopts downstream defaults and constraints. |
| `boolean_input.rs` | Boolean source node for true/false workflow settings. |
| `image_input.rs` | Image payload source for image-consuming workflows. |
| `audio_input.rs` | Audio payload source for audio-consuming workflows. |

## Problem
Workflow graphs need stable input contracts that can be shared across hosts.
Model-selection nodes in particular must emit enough metadata for downstream
routing without hardcoding runtime choices into the UI or executor.

## Constraints
- Input descriptors must stay host-agnostic.
- Host-owned nodes such as `puma-lib` still need discoverable metadata for the
  frontend and dependency preflight.
- Runtime-executable artifact paths must come from the Pumas artifact
  load-target contract at the host/planning boundary, not from graph-visible
  input-node outputs.
- Model-list/package-fact summary details must come from Pumas summary snapshot
  and summary resolution APIs, not Pumas storage internals.
- Model selector rows must come from Pumas `ModelLibrarySelectorSnapshot`
  first. Selected or expanded model details may use batch/package APIs after a
  row has been chosen.
- Pumas selector access must be explicit: owner API, local client, or read-only
  library. Read-only selector access is a local indexed read and must not start
  reconciliation, downloads, watchers, or hidden lifecycle work.
- Pumas selector unavailable states must be projected as typed unavailable
  options or diagnostics. They must not be hidden behind empty successful
  option lists or path-shaped fallback values.

## Decision
Keep input nodes as descriptor-first modules. `puma_lib.rs` emits the selected
`pumas_model_ref`, task metadata, recommended backend display metadata,
dependency requirements, and inference settings so downstream routing can
distinguish text, audio, and diffusion flows without accepting local paths as
model identity. The graph-facing executable selection value is
`pumas_model_ref`; raw artifact paths and backend keys are not current
execution outputs. The model option provider populates rows from Pumas
`ModelLibrarySelectorSnapshot`, captures package-fact summary status, summary
payload, readiness state, storage/validation state, and the producer cursor for
the populated page so UI/model-list consumers can refresh from Pumas update
feeds without inspecting Pumas storage.

## Alternatives Rejected
- Keep an unregistered `model-provider` `NodeExecutor` in this crate.
  Rejected because active model-provider execution is already owned by
  `node-engine` core executor handlers, leaving the workflow-nodes executor as
  dead code.
- Move Pumas-backed model selection into generic `model-provider`.
  Rejected because `puma-lib` owns the Pumas-specific workflow metadata and
  dependency contract.

## Invariants
- Input nodes do not own runtime execution side effects.
- Generic `model-provider` remains a descriptor/task contract; active runtime
  projection for model-provider nodes is owned by `node-engine` core executor
  handlers rather than an unregistered workflow-nodes executor.
- `puma-lib` metadata is the primary workflow-facing bridge from Pumas-Library
  into Pantograph routing.
- `puma-lib` option metadata may cache bounded Pumas package summaries for the
  listed page, but Pumas remains the source of truth and update cursor producer.
- `puma-lib` model options query the explicit selector-access role and must not
  require full owner `PumasApi` when a read-only selector snapshot is available.
- `puma-lib` summary-cache population applies the Pumas snapshot cursor before
  resolving sparse rows, consumes the update feed for that cursor, regenerates
  bounded missing summaries, then polls the update feed again so updates that
  arrive during summary regeneration invalidate only affected rows before those
  affected rows are refilled against the newest cursor.
- Pantograph must not infer Pumas runtime bundle semantics from projected
  display/debug metadata. Runtime execution consumes Pumas-approved load
  targets at the host/planning boundary.
- Task inference from selector metadata must remain conservative and
  deterministic.
- Stored Pumas inference settings are reused only when they are non-empty
  arrays; otherwise the node falls back to descriptor/API defaults so empty
  metadata does not masquerade as an executable settings contract.

## Revisit Triggers
- Model-selection metadata grows beyond what one node should emit directly.
- `image_input.rs` becomes part of first-class img2img or inpaint execution and
  needs a richer contract.

## Dependencies
**Internal:** `node_engine` task metadata and inventory registration.

**External:** optional `pumas_library` APIs behind the `model-library` feature.

## Related ADRs
- None identified as of 2026-04-21.
- Reason: the model-provider executor cleanup removes an inactive
  implementation and preserves the existing crate ownership boundary.
- Revisit trigger: generic model-provider execution becomes a public,
  host-bridged contract separate from `node-engine` core executor handling.

## Usage Examples
```rust
let metadata = ModelProviderTask::descriptor();
assert_eq!(metadata.node_type, "model-provider");
```

## API Consumer Contract
- Consumers should treat these modules as node descriptor sources, not direct
  execution APIs.
- `puma-lib` outputs are workflow metadata contracts centered on
  `pumas_model_ref`.
- Consumers must not assume Pantograph inferred runtime-executable paths from
  `metadata.json`, selector rows, or option values. Execution paths are resolved
  later through Pumas-owned artifact load targets.
- Consumers must treat selector display paths as display/debug data only. They
  are not executable graph values.
- Consumers must treat selected-model detail as role-dependent. Owner and
  local-client Pumas access may provide batch execution descriptors, package
  summaries, and inference settings; read-only access may provide only the
  bounded selector row and empty inference settings.

## Structured Producer Contract
- `puma-lib` emits only `pumas_model_ref` and display identity such as
  `model_id`. Model type, task kind, runtime hints, dependency requirements,
  inference settings, package facts, load targets, and executable paths are
  resolved by backend validation, dependency planning, scheduler, and
  runtime-host systems after graph authoring.
- `puma-lib` option values are typed Pumas model-reference payloads. Option
  metadata may include display/debug paths, readiness state, storage kind,
  validation state, and package summary facts; those fields are not executable
  graph inputs.
- Missing, stale, invalid, partial, ambiguous, or not-yet-detailed selector
  states are represented as disabled typed unavailable options or diagnostics.
  They are not converted to executable paths, backend choices, or successful
  empty fallback results.
- Selected `puma-lib` hydration should prefer the explicit selector-access role
  and must not read Pumas storage internals or synthesize runtime policy. The
  producer may project read-only selector-row facts when batch detail is not
  available.
- Metadata fields such as storage kind, validation state, and display paths are
  selector/debug evidence only. They are not the authoritative runtime contract
  for executable model selection.
- Diffusion models should resolve to canonical image-generation graph intent
  when explicit metadata is missing but `model_type == diffusion`. External
  package facts may still preserve ecosystem labels such as `text-to-image`,
  but starter graphs and saved workflows must route through
  `llm-inference` with `task_kind = image_generation`.
