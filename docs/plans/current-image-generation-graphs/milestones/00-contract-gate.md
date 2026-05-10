# Milestone 0: Contract Gate

**Goal:** Freeze the small set of cross-layer decisions that prevent duplicated
validation, backend-key drift, hidden compatibility behavior, and large media
duplication before touching implementation code.

**Tasks:**

- [ ] Document that `diffusers` is a dependency/capability label that resolves
  to PyTorch execution for this plan.
- [ ] Identify the single backend graph diagnostic DTO used by graph editor,
  IO inspector saved-graph mode, submit/admission, and run inspection.
- [ ] Identify the shared graph inspection projection that both run inspection
  and saved-graph inspection can use without leaking run-only naming into
  saved-graph behavior.
- [ ] Decide the current app-path behavior for retired node types: report stale
  diagnostics and block executable submission, with no silent rewrite.
- [ ] Define the canonicalization split: current normalization remains
  available to app load/save/session paths; retired-node handling becomes
  stale diagnostic classification only.
- [ ] Identify compatibility helpers, migration tests, fixtures, or docs that
  should be deleted or rewritten when touched because Pantograph does not
  preserve old graph shapes.
- [ ] Define IO inspector saved-graph inspection mode for graphs that do not
  have a run snapshot.
- [ ] Define generated-image output retention semantics: one retained image
  body plus descriptors/metadata elsewhere.
- [ ] Define the public facade preservation boundary. Existing app-facing
  commands and crate facades should remain stable where they describe current
  concepts, while retired graph shapes and stale producers are removed instead
  of kept through compatibility branches.
- [ ] Identify the single execution-selection backend-key normalization
  boundary that maps `diffusers` to PyTorch execution for this slice without
  changing runtime/dependency display identity.
- [ ] Define that normalization boundary as task/artifact-aware: `diffusers`
  may resolve to PyTorch execution for `image_generation` +
  `diffusers_bundle`, but must remain a factual dependency/capability label in
  runtime, diagnostics, and model-library projections.
- [ ] Define the model-family acceptance rule: Pumas package facts and
  dependency metadata select behavior; saved workflow names and model display
  names do not.
- [ ] Define the no-fallback execution rule: a run gets one planned backend,
  one planned dependency environment, one planned device policy, one planned
  runtime variant, one planned pipeline family, and one planned scheduler
  choice.
- [ ] Define the old-path removal rule: fallback or legacy behavior discovered
  during implementation is either deleted or replaced by the canonical
  contract. It must not remain reachable execution behavior behind a
  compatibility branch.
- [ ] Identify the concrete fallback/legacy execution paths to remove before
  implementation expands: executable `ConservativeFallback`, override-fallback
  candidate synthesis, raw-device defaults, frontend synthetic device options,
  raw `runtime_hint` backend routing, and generic worker-side auto-selection.
- [ ] Define required planning diagnostics for missing package facts,
  unsupported pipeline family, unsupported component layout, unsupported
  workflow option, incompatible scheduler, incompatible dimensions,
  unavailable dependency environment, unavailable device, and unacceptable
  resource estimate.
- [ ] Define missing-facts diagnostic detail fields so implementation can tell
  Pumas exactly which fact was absent or ambiguous: package field path,
  expected evidence, observed evidence summary, affected family, and whether
  the fact blocks all execution or only a family/variant.
- [ ] Record the reference-repo guidance to use during implementation:
  Transformers naming/config semantics; ComfyUI model-family detection and
  component taxonomy; InvokeAI model-family loader and validation boundaries.
- [ ] Freeze `ImageGenerationFamily`, `ImageGenerationFamilyVariant`,
  `ImageGenerationComponentRole`, `ImageGenerationFamilyRequirements`,
  `ImageGenerationFamilyAdapter`, `PyTorchDiffusersImageGenerationPlan`, and
  `ImageGenerationPlanDiagnostic` before implementing worker execution.
- [ ] Define minimum Pumas package fact requirements for image generation,
  including artifact kind/root, component roles, pipeline class/family
  evidence, generation defaults, custom-code policy, task modalities, and
  diffusers backend hint evidence.
- [ ] Define how model-provided generation defaults are merged with graph/user
  request values. User request values win; missing optional values may use
  model defaults; unsupported values fail during planning.
- [ ] Define gateway option diagnostics as informational lifecycle metadata.
  Family adapters own final option support decisions and must produce the
  accepted/rejected/ignored option diagnostics used for readiness/admission.
- [ ] Define the initial family requirements table for SD/SDXL, FLUX, FLUX.2,
  Qwen Image, Lumina Image, GLM Image, and Z-Image.
- [ ] Define validated Rust value types or enums for planner backend,
  dependency environment, device policy, runtime variant, selected device id,
  pipeline family, scheduler, dimensions, and option-support decisions where
  raw strings would cross module boundaries.
- [ ] Identify public/cross-layer DTOs requiring serde round-trip tests and
  matching frontend TypeScript type updates.
- [ ] Define the exact wire-format attributes for those DTOs, including
  serde tagging/casing, optional-field behavior, bounded payload fields, and
  whether unknown fields are rejected at persisted or IPC boundaries.
- [ ] Define executable boundary contract fixtures for saved workflow JSON,
  stale graph diagnostics, Tauri IPC payloads, Pumas package fact snapshots,
  Python worker image-generation request/response envelopes, and artifact
  descriptors.
- [ ] Identify the first failing acceptance test or fixture for each vertical
  slice before implementation starts: saved Juggernaut graph, retired-node
  rejection, stale graph projection, IO inspector stale graph view,
  PyTorch/diffusers planning, worker envelope, and single-body artifact
  retention.
- [ ] Define isolated test roots for Pumas fixtures, saved workflow fixtures,
  runtime state, artifact store data, and run projection data so tests do not
  read or mutate developer-local state.
- [ ] Record decomposition decisions for large touched modules before adding
  implementation code.

**Verification:**

- Contract notes are present in this plan or touched module README before
  implementation slices begin.
- Tests to be added in later milestones can trace to a single contract
  decision instead of page-specific or backend-specific behavior.
- Code search confirms planned implementation has one diagnostic DTO, one
  execution backend-key normalization boundary, and one retained media body
  contract.
- Contract notes distinguish execution backend normalization from runtime and
  dependency labels so diagnostics can still say `diffusers` where that is the
  factual dependency surface.
- Contract notes state that planning failure is a terminal readiness/admission
  problem, not a trigger for fallback.
- Contract notes state that auto mode is a canonical scheduler policy and fails
  with typed diagnostics when no valid candidate exists.
- Contract notes state that old Pantograph graph shapes are not supported by
  migration or compatibility paths.
- Contract notes list the removed or replaced fallback/legacy execution paths
  and verify none remain reachable from submit/admission/execution.
- Contract notes cite the exact reference repo areas used for naming,
  taxonomy, and family-specific validation guidance.
- Contract notes define the family adapter trait as sync and side-effect free,
  with async work isolated to Pumas/runtime/worker shells.
- Contract notes state that family selection requires explicit Pumas facts and
  cannot use workflow names, display names, directory names, or file names.
- Contract notes state that PyTorch worker image execution consumes a validated
  plan and must not use generic pipeline discovery, implicit fallback, or
  unconditional `trust_remote_code=True`.
- Contract tests are listed for Rust serde round trips and frontend type
  consumption before implementation starts.
- Executable boundary fixtures are listed for saved workflow JSON, stale graph
  diagnostics, Tauri IPC payloads, Pumas package fact snapshots, Python worker
  envelopes, and artifact descriptors.
- Each implementation milestone has at least one acceptance test or fixture
  named before code changes begin.
- Test isolation roots are documented for Pumas, workflow, artifact, runtime,
  and run-projection data.
- Decomposition review produces either extraction tasks or an explicit reason
  for each large touched file to remain intact for this plan.

**Status:** Not started
