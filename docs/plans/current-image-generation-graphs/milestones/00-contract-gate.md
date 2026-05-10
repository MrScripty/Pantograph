# Milestone 0: Contract Gate

**Goal:** Freeze the small set of cross-layer decisions that prevent duplicated
validation, backend-key drift, hidden compatibility behavior, and large media
duplication before touching implementation code.

**Tasks:**

- [x] Document that `diffusers` is a dependency/capability label that resolves
  to PyTorch execution for this plan.
- [x] Identify the single backend graph diagnostic DTO used by graph editor,
  IO inspector saved-graph mode, submit/admission, and run inspection.
- [x] Identify the shared graph inspection projection that both run inspection
  and saved-graph inspection can use without leaking run-only naming into
  saved-graph behavior.
- [x] Decide the current app-path behavior for retired node types: report stale
  diagnostics and block executable submission, with no silent rewrite.
- [x] Define the canonicalization split: current normalization remains
  available to app load/save/session paths; retired-node handling becomes
  stale diagnostic classification only.
- [x] Identify compatibility helpers, migration tests, fixtures, or docs that
  should be deleted or rewritten when touched because Pantograph does not
  preserve old graph shapes.
- [x] Define IO inspector saved-graph inspection mode for graphs that do not
  have a run snapshot.
- [x] Define generated-image output retention semantics: one retained image
  body plus descriptors/metadata elsewhere.
- [x] Define the public facade preservation boundary. Existing app-facing
  commands and crate facades should remain stable where they describe current
  concepts, while retired graph shapes and stale producers are removed instead
  of kept through compatibility branches.
- [x] Identify the single execution-selection backend-key normalization
  boundary that maps `diffusers` to PyTorch execution for this slice without
  changing runtime/dependency display identity.
- [x] Define that normalization boundary as task/artifact-aware: `diffusers`
  may resolve to PyTorch execution for `image_generation` +
  `diffusers_bundle`, but must remain a factual dependency/capability label in
  runtime, diagnostics, and model-library projections.
- [x] Define the model-family acceptance rule: Pumas package facts and
  dependency metadata select behavior; saved workflow names and model display
  names do not.
- [x] Define the no-fallback execution rule: a run gets one planned backend,
  one planned dependency environment, one planned device policy, one planned
  runtime variant, one planned pipeline family, and one planned scheduler
  choice.
- [x] Define the old-path removal rule: fallback or legacy behavior discovered
  during implementation is either deleted or replaced by the canonical
  contract. It must not remain reachable execution behavior behind a
  compatibility branch.
- [x] Identify the concrete fallback/legacy execution paths to remove before
  implementation expands: executable `ConservativeFallback`, override-fallback
  candidate synthesis, raw-device defaults, frontend synthetic device options,
  raw `runtime_hint` backend routing, and generic worker-side auto-selection.
- [x] Define required planning diagnostics for missing package facts,
  unsupported pipeline family, unsupported component layout, unsupported
  workflow option, incompatible scheduler, incompatible dimensions,
  unavailable dependency environment, unavailable device, and unacceptable
  resource estimate.
- [x] Define missing-facts diagnostic detail fields so implementation can tell
  Pumas exactly which fact was absent or ambiguous: package field path,
  expected evidence, observed evidence summary, affected family, and whether
  the fact blocks all execution or only a family/variant.
- [x] Record the reference-repo guidance to use during implementation:
  Transformers naming/config semantics; ComfyUI model-family detection and
  component taxonomy; InvokeAI model-family loader and validation boundaries.
- [x] Freeze `ImageGenerationFamily`, `ImageGenerationFamilyVariant`,
  `ImageGenerationComponentRole`, `ImageGenerationFamilyRequirements`,
  `ImageGenerationFamilyAdapter`, `PyTorchDiffusersImageGenerationPlan`, and
  `ImageGenerationPlanDiagnostic` before implementing worker execution.
- [x] Define minimum Pumas package fact requirements for image generation,
  including artifact kind/root, component roles, pipeline class/family
  evidence, generation defaults, custom-code policy, task modalities, and
  diffusers backend hint evidence.
- [x] Define how model-provided generation defaults are merged with graph/user
  request values. User request values win; missing optional values may use
  model defaults; unsupported values fail during planning.
- [x] Define gateway option diagnostics as informational lifecycle metadata.
  Family adapters own final option support decisions and must produce the
  accepted/rejected/ignored option diagnostics used for readiness/admission.
- [x] Define the initial family requirements table for SD/SDXL, FLUX, FLUX.2,
  Qwen Image, Lumina Image, GLM Image, and Z-Image.
- [x] Define validated Rust value types or enums for planner backend,
  dependency environment, device policy, runtime variant, selected device id,
  pipeline family, scheduler, dimensions, and option-support decisions where
  raw strings would cross module boundaries.
- [x] Identify public/cross-layer DTOs requiring serde round-trip tests and
  matching frontend TypeScript type updates.
- [x] Define the exact wire-format attributes for those DTOs, including
  serde tagging/casing, optional-field behavior, bounded payload fields, and
  whether unknown fields are rejected at persisted or IPC boundaries.
- [x] Define executable boundary contract fixtures for saved workflow JSON,
  stale graph diagnostics, Tauri IPC payloads, Pumas package fact snapshots,
  Python worker image-generation request/response envelopes, and artifact
  descriptors.
- [x] Identify the first failing acceptance test or fixture for each vertical
  slice before implementation starts: saved Juggernaut graph, retired-node
  rejection, stale graph projection, IO inspector stale graph view,
  PyTorch/diffusers planning, worker envelope, and single-body artifact
  retention.
- [x] Define isolated test roots for Pumas fixtures, saved workflow fixtures,
  runtime state, artifact store data, and run projection data so tests do not
  read or mutate developer-local state.
- [x] Record decomposition decisions for large touched modules before adding
  implementation code.

## Frozen Contract Decisions

Milestone 0 is a documentation and contract-freeze slice. It does not change
production behavior. Its allowed write set is this milestone file and
`05-execution-management.md`; later source/test slices must implement the
contracts below without reintroducing fallback or legacy execution behavior.

### Graph Diagnostics And Inspection

- The single planned graph diagnostic DTO is
  `WorkflowGraphDiagnostic`, owned by
  `crates/pantograph-workflow-service/src/graph/diagnostics.rs` and exported
  through the graph facade. It carries `code`, `severity`, `node_id`,
  `node_type`, `message`, `blocking_submission`, and bounded `details`.
- The shared graph inspection projection is
  `WorkflowGraphInspectionProjection`, also backend-owned. It contains the
  graph snapshot, selected node details, `Vec<WorkflowGraphDiagnostic>`, and an
  optional run context. Run inspection may wrap this projection with run,
  status, and artifact records; saved-graph inspection uses the same projection
  with no run context.
- The IO inspector saved-graph mode consumes
  `WorkflowGraphInspectionProjection` from the backend. The frontend may render
  stale markers and selected-node details, but it must not infer stale graph
  diagnostics from node names or local registry misses.
- Retired node types are stale diagnostics only. App load, save, edit-session,
  submit, admission, and run-inspection paths must not silently rewrite
  `diffusion-inference` into executable current graphs.
- Current graph canonicalization remains available for current schema
  normalization, definition overlays, edge normalization, and display metadata.
  Retired-node handling is split out as diagnostic classification, not
  executable migration.

### Execution Selection

- The single execution backend-key normalization boundary is the future
  inference planner entry point, scoped by task and artifact. `runtime_hint =
  "diffusers"` resolves to PyTorch execution only when the task is
  `image_generation` and the selected artifact kind is `diffusers_bundle`.
- `diffusers` remains a dependency/capability label in runtime capabilities,
  model-library facts, diagnostics, and Pumas evidence. It is not a separate
  Pantograph execution backend in this plan.
- A run has exactly one planned backend, one dependency environment, one device
  policy, one runtime variant, one pipeline family, and one scheduler decision.
  Planning failure is terminal for readiness/admission and returns typed
  diagnostics.
- Auto device mode is a canonical scheduler policy. If auto cannot produce a
  valid candidate it fails with a typed diagnostic; it must not call old raw
  device defaults as backup.

### Image Family Planner

- Family selection requires explicit Pumas facts: artifact kind/root,
  selected-files or manifest evidence, component roles, pipeline
  class/family evidence, task modalities, generation defaults, custom-code
  policy, and backend-hint evidence.
- Saved workflow names, display names, directory names, and file names are not
  acceptable evidence for model family, family variant, or component layout.
- Missing fact diagnostics include package field path, expected evidence,
  observed evidence summary, affected family, and whether the missing fact
  blocks all execution or only a family/variant.
- User request values override model defaults. Missing optional request values
  may use model defaults. Unsupported model or user values fail during
  planning rather than being ignored or coerced by the worker.
- Gateway option diagnostics remain informational lifecycle metadata. Family
  adapters own final accepted/rejected/ignored option decisions.
- PyTorch worker image execution consumes a validated
  `PyTorchDiffusersImageGenerationPlan`; it must not run generic pipeline
  discovery, unconditional `trust_remote_code=True`, or implicit fallback.

### Wire Format And Fixtures

- Public/cross-layer DTOs introduced by this plan use explicit
  `snake_case` serde wire names, optional fields with
  `skip_serializing_if = "Option::is_none"`, bounded `details` maps, and enum
  values modeled as Rust enums/newtypes before crossing crate, IPC, or worker
  boundaries.
- Unknown persisted workflow fields may remain tolerated where existing graph
  persistence already preserves additive metadata. New IPC and Python worker
  envelopes reject unknown required contract fields by shape validation at the
  boundary.
- Required executable fixtures:
  `crates/pantograph-workflow-service/src/workflow/tests/fixtures/current_juggernaut_graph.json`,
  `crates/pantograph-workflow-service/src/graph/tests/fixtures/stale_diffusion_inference_graph.json`,
  `crates/pantograph-workflow-service/tests/fixtures/graph_inspection_stale_diagnostics.json`,
  `src/services/workflow/fixtures/graph_inspection_stale_diagnostics.json`,
  `crates/inference/tests/fixtures/inference_package_facts/diffusers_bundle_package_facts.json`,
  `crates/inference/tests/fixtures/image_generation_worker_request.json`,
  `crates/inference/tests/fixtures/image_generation_worker_response.json`, and
  `crates/pantograph-workflow-service/tests/fixtures/single_body_image_artifact_descriptor.json`.
- Isolated test roots: Pumas package facts use per-test temp package roots,
  saved workflow tests use per-test temp `.pantograph/workflows`, runtime state
  uses per-test temp runtime roots, artifact tests use per-test temp artifact
  stores, and run projection tests use in-memory or per-test SQLite ledgers.

### Code Search Findings

Code search on 2026-05-10 found these planned removal or replacement targets:

- Executable technical-fit fallback:
  `crates/pantograph-runtime-registry/src/technical_fit.rs` and mirrored
  workflow/embedded runtime mapping in
  `crates/pantograph-workflow-service/src/technical_fit.rs` and
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`.
- Override fallback candidate synthesis:
  `crates/pantograph-runtime-registry/src/technical_fit.rs`.
- Raw llama.cpp device auto/default behavior:
  `crates/inference/src/device.rs`, `crates/inference/src/server.rs`, and
  `crates/pantograph-embedded-runtime/src/embedded_workflow_host_helpers.rs`.
- Raw `runtime_hint` routing and dependency preflight:
  `crates/node-engine/src/core_executor/inference_nodes.rs`,
  `crates/node-engine/src/core_executor/dependency_preflight.rs`,
  `crates/pantograph-workflow-service/src/capabilities.rs`, and
  `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`.
- Retired graph producer/template/test paths:
  `src/templates/workflows/tiny-sd-turbo-text-to-image.json`,
  `src/templates/workflows/README.md`,
  `src/services/workflow/templateService.test.ts`, and
  `crates/pantograph-workflow-service/src/graph/canonicalization_legacy_migration.rs`.
- Frontend synthetic or local fallback presentation targets:
  `packages/svelte-graph/src/utils/buildRegistry.ts`,
  `src/services/workflow/mocks.ts`, and
  `packages/svelte-graph/src/backends/MockWorkflowBackend.ts`.
- Artifact retention paths already provide a canonical single-body direction in
  `crates/pantograph-workflow-service/src/workflow/artifact_output_conversion.rs`
  and `crates/pantograph-workflow-service/src/workflow/artifact_store/`.
  Milestone 6 must prove generated image execution writes one retained body and
  returns descriptors/metadata elsewhere.

### Decomposition Decisions

- `crates/pantograph-workflow-service/src/graph/canonicalization_legacy_migration.rs`
  is already focused on legacy migration. The next graph slice should remove or
  bypass retired image-generation migration there instead of adding another
  compatibility branch.
- New graph diagnostics belong in a new focused graph diagnostics module,
  keeping `canonicalization.rs`, `session.rs`, and workflow diagnostics APIs
  from growing.
- Image-family planner code belongs in focused inference planner modules rather
  than expanding `crates/inference/src/backend/compatibility.rs` or
  `crates/inference/src/types.rs`.
- Device/runtime contracts should be added as canonical contract modules before
  modifying llama.cpp adapter code in `crates/inference/src/device.rs`.
- `src/components/workbench/IoInspectorPage.svelte` is already large; frontend
  stale saved-graph work should first add presenter tests and presenter helpers
  rather than adding inference logic directly to the component.
- `artifact_output_conversion.rs` may remain intact for the first single-body
  image artifact acceptance test. If generated-image conversion expands beyond
  port/body mapping, extract an image artifact conversion helper before adding
  broad media-family logic.

### First Acceptance Checks

- Milestone 1: backend or frontend fixture proves
  `.pantograph/workflows/juggernaut-x-v10-sdxl.json` uses
  `puma-lib -> llm-inference -> image-output`.
- Milestone 2: graph/service test proves `diffusion-inference` is reported as
  stale and not rewritten into an executable graph.
- Milestone 3: workflow-service serde/behavior test proves stale graph
  diagnostics flow through `WorkflowGraphDiagnostic`.
- Milestone 4: frontend presenter/component test consumes backend-produced
  stale graph diagnostics without local inference.
- Milestone 5: inference contract tests prove explicit unavailable device
  requests fail and auto records the selected runtime/device.
- Milestone 6: inference/worker envelope tests prove a validated
  `PyTorchDiffusersImageGenerationPlan` is required, and artifact tests prove
  one retained generated-image body.
- Milestone 7: inference/runtime test proves Candle image generation returns a
  typed unavailable diagnostic, not a fallback candidate.
- Milestone 8: release verification records affected Rust/frontend checks and
  manual saved-graph/user validation.

## Verification

- [x] Contract notes are present in this plan or touched module README before
  implementation slices begin.
- [x] Tests to be added in later milestones can trace to a single contract
  decision instead of page-specific or backend-specific behavior.
- [x] Code search confirms planned implementation has one diagnostic DTO, one
  execution backend-key normalization boundary, and one retained media body
  contract.
- [x] Contract notes distinguish execution backend normalization from runtime and
  dependency labels so diagnostics can still say `diffusers` where that is the
  factual dependency surface.
- [x] Contract notes state that planning failure is a terminal readiness/admission
  problem, not a trigger for fallback.
- [x] Contract notes state that auto mode is a canonical scheduler policy and fails
  with typed diagnostics when no valid candidate exists.
- [x] Contract notes state that old Pantograph graph shapes are not supported by
  migration or compatibility paths.
- [x] Contract notes list the removed or replaced fallback/legacy execution paths
  and verify none remain reachable from submit/admission/execution.
- [x] Contract notes cite the exact reference repo areas used for naming,
  taxonomy, and family-specific validation guidance.
- [x] Contract notes define the family adapter trait as sync and side-effect free,
  with async work isolated to Pumas/runtime/worker shells.
- [x] Contract notes state that family selection requires explicit Pumas facts and
  cannot use workflow names, display names, directory names, or file names.
- [x] Contract notes state that PyTorch worker image execution consumes a validated
  plan and must not use generic pipeline discovery, implicit fallback, or
  unconditional `trust_remote_code=True`.
- [x] Contract tests are listed for Rust serde round trips and frontend type
  consumption before implementation starts.
- [x] Executable boundary fixtures are listed for saved workflow JSON, stale graph
  diagnostics, Tauri IPC payloads, Pumas package fact snapshots, Python worker
  envelopes, and artifact descriptors.
- [x] Each implementation milestone has at least one acceptance test or fixture
  named before code changes begin.
- [x] Test isolation roots are documented for Pumas, workflow, artifact, runtime,
  and run-projection data.
- [x] Decomposition review produces either extraction tasks or an explicit reason
  for each large touched file to remain intact for this plan.

**Status:** Completed on 2026-05-10
