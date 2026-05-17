# Proposal: Pumas Artifact Load Target Resolution For Pantograph

## Context

Pantograph uses Pumas Library as the canonical owner of model identity, model
storage, selected artifact metadata, package facts, and library availability.
Pantograph does not own model files, model-library roots, or external-reference
asset resolution. Pantograph should trust Pumas-provided artifact references
instead of rediscovering, inferring, or validating Pumas storage layout from
local paths.

Pantograph's image-generation execution path now requires a clean handoff from
Pumas package facts to a PyTorch/Diffusers worker load target:

- Scheduler/admission selects backend, runtime variant, device, and model.
- Inference planning consumes scheduler-selected decisions and Pumas package
  facts.
- The PyTorch worker can load a Diffusers pipeline only from a concrete local
  directory.

The missing contract is a Pumas-owned API that turns a selected model artifact
reference into an execution-ready local load target, or returns typed
unavailability diagnostics.

## Problem

Pantograph currently has enough information to know which Pumas artifact was
selected, but not enough to safely load it without taking ownership of Pumas
storage semantics.

A root-relative artifact entry such as:

```text
image/stable-diffusion/tiny-sd
```

is useful for stable identity and diagnostics, but it does not by itself answer:

- whether the artifact is currently materialized locally;
- which Pumas library root or external-reference record owns it;
- whether the artifact is a loadable directory or a pending/missing artifact;
- whether the selected artifact id/path still matches the current model record;
- what exact local path should be passed to a runtime worker;
- whether the artifact is valid for the requested runtime family, such as a
  Diffusers bundle.

Pantograph should not solve this by configuring Pumas roots and joining paths.
That would duplicate Pumas ownership and would not handle Pumas-supported
external-reference assets correctly. Pantograph should also not pass a Pumas
handle to the Python worker and let the worker resolve it. That would move
library resolution and diagnostics into the wrong layer.

## Goal

Add a Pumas-owned artifact load-target resolver API.

The API should let Pantograph give Pumas a selected artifact reference and
receive one of:

- an execution-ready local load target for the exact selected artifact; or
- typed diagnostics explaining why the artifact cannot currently be loaded.

Pantograph will treat this Pumas response as authoritative.

## Non-Goals

Pumas should not own Pantograph scheduler policy, runtime ranking, worker
execution, lifecycle events, retry policy, or workflow diagnostics formatting.

Pantograph should not infer executable backend selection from Pumas package
hints. Pumas facts remain factual model/package evidence. Pantograph's scheduler
still decides runtime selection.

The Python worker should not call Pumas, inspect model library roots, or repair
missing paths. It should receive an already-approved local load target from Rust.

## Proposed API

Add an artifact materialization/load-target API to Pumas Library:

```rust
pub async fn resolve_model_artifact_load_target(
    &self,
    request: ResolveModelArtifactLoadTargetRequest,
) -> Result<ResolveModelArtifactLoadTargetResponse>;
```

Proposed request shape, using existing Pumas contracts:

```rust
pub struct ResolveModelArtifactLoadTargetRequest {
    pub model_ref: PumasModelRef,
    pub expected_artifact_kind: Option<PackageArtifactKind>,
    pub caller_observed_entry_path: Option<String>,
    pub caller_observed_package_facts_contract_version: Option<u32>,
    pub consumer: PumasArtifactConsumer,
}

pub struct PumasArtifactConsumer {
    pub consumer_name: String,
    pub task_kind: Option<String>,
    pub runtime_family: Option<String>,
}
```

Exact names can change. The important part is that Pantograph sends exactly one
authoritative selected-artifact reference: `PumasModelRef`, including its
existing `selected_artifact_id` and `selected_artifact_path` fields when a
specific artifact was selected. `caller_observed_entry_path` and
`caller_observed_package_facts_contract_version` are stale-check inputs only;
they must not override the selected artifact encoded in `PumasModelRef`.

This proposal intentionally does not introduce parallel artifact DTOs such as a
new `PumasArtifactKind` or `PumasArtifactRef`. Pumas should reuse
`PackageArtifactKind`, `PumasModelRef`, and existing model-library selector
state contracts unless a new type has deliberately different semantics.

## Proposed Response Shape

```rust
pub struct ResolveModelArtifactLoadTargetResponse {
    pub artifact_state: ModelArtifactState,
    pub entry_path_state: ModelEntryPathState,
    pub target: Option<PumasArtifactLoadTarget>,
    pub diagnostics: Vec<PumasArtifactLoadTargetDiagnostic>,
}

pub struct PumasArtifactLoadTarget {
    pub model_ref: PumasModelRef,
    pub artifact_kind: PackageArtifactKind,
    pub local_load_path: String,
    pub load_path_kind: PumasArtifactLoadPathKind,
    pub library_root_id: Option<String>,
    pub storage_kind: StorageKind,
    pub validation_state: AssetValidationState,
    pub content_fingerprint: Option<String>,
    pub package_facts_contract_version: Option<u32>,
}

pub enum PumasArtifactLoadPathKind {
    Directory,
    File,
}

pub struct PumasArtifactLoadTargetDiagnostic {
    pub code: PumasArtifactLoadTargetDiagnosticCode,
    pub field_path: Option<String>,
    pub message: String,
}

pub enum PumasArtifactLoadTargetDiagnosticCode {
    MissingModel,
    MissingSelectedArtifact,
    SelectedArtifactMismatch,
    ArtifactMissing,
    ArtifactPartial,
    ArtifactNeedsDetail,
    ArtifactPathMissing,
    ArtifactPathNotLoadable,
    ArtifactKindMismatch,
    InvalidArtifact,
    InvalidPackageFacts,
    StalePackageFacts,
    LibraryUnavailable,
}
```

The response should be serde-stable and append-only. Pantograph can map
`ModelArtifactState`, `ModelEntryPathState`, and these diagnostics into its
scheduler/readiness/planner diagnostics without parsing message text.

If Pumas wants a convenience status field in addition to the existing state
fields, it should be derived from `ModelArtifactState` and
`ModelEntryPathState`, not replace them. Pantograph needs the original state
fidelity for partial downloads, stale facts, ambiguous artifacts, and
needs-detail cases.

## Contract Rules

- Pumas owns resolving model refs, selected artifact refs, library roots,
  external-reference asset records, and local filesystem load paths.
- Pumas owns whether the selected artifact is currently loadable.
- Pumas owns validating that `local_load_path` is a Pumas-approved local path
  for the selected artifact. That path may be inside Pumas-managed storage or
  may be an approved external-reference asset. The ready target should expose
  typed `StorageKind` and `AssetValidationState` values so consumers do not
  assume all load targets live under one library root.
- Pumas owns checking whether the artifact kind matches the caller's expected
  artifact kind.
- Pumas should return typed unavailable states instead of throwing opaque
  string errors for normal missing/not-downloaded/stale cases.
- Pantograph should not join root paths, scan directories, infer from file
  names, or repair selected artifact refs.
- Pantograph should pass the resolved `local_load_path` to runtime workers only
  after Pumas returns ready artifact and entry-path states with a target.
- Runtime workers should not receive Pumas roots or call Pumas directly.

## Diffusers Image Generation Use Case

For Pantograph image generation, the request would look conceptually like:

```rust
ResolveModelArtifactLoadTargetRequest {
    model_ref: scheduler_selected_model_ref_with_selected_artifact,
    expected_artifact_kind: Some(PackageArtifactKind::DiffusersBundle),
    caller_observed_entry_path: Some(package_facts.artifact.entry_path),
    caller_observed_package_facts_contract_version: Some(
        package_facts.package_facts_contract_version,
    ),
    consumer: PumasArtifactConsumer {
        consumer_name: "pantograph".to_string(),
        task_kind: Some("image_generation".to_string()),
        runtime_family: Some("pytorch.diffusers".to_string()),
    },
}
```

If ready, Pumas returns:

```rust
PumasArtifactLoadTarget {
    artifact_kind: PackageArtifactKind::DiffusersBundle,
    local_load_path: "/.../Pumas-Library/shared-resources/models/image/...".to_string(),
    load_path_kind: PumasArtifactLoadPathKind::Directory,
    storage_kind: StorageKind::LibraryManaged,
    validation_state: AssetValidationState::Valid,
    ...
}
```

Pantograph then includes this resolved directory in the Rust-owned worker
envelope. The Python worker loads that directory and does not resolve Pumas
state.

## Availability Semantics

The resolver should map directly to existing Pumas model-library states:

- `ModelArtifactState::Ready` and a ready/loadable entry path means the target
  is currently loadable as requested. For `PackageArtifactKind::DiffusersBundle`,
  that means the target path is a Pumas-approved local directory and Pumas
  recognizes it as the selected Diffusers artifact.
- `ModelArtifactState::Missing` means the model or selected artifact is known
  but not locally materialized, or the selected artifact cannot be found.
- `ModelArtifactState::Partial` means some required artifact content is present
  but incomplete. Pantograph should treat this as not ready and surface the
  diagnostic without attempting a partial load.
- `ModelArtifactState::Invalid` means Pumas can locate the artifact but package
  validation does not consider it loadable.
- `ModelArtifactState::Ambiguous` means Pumas cannot identify one exact selected
  artifact. Pantograph should fail the workflow and refresh or ask the user to
  select an artifact explicitly.
- `ModelArtifactState::NeedsDetail` means Pumas has only summary/index state and
  must perform or schedule detail resolution before it can return a load target.
- `ModelArtifactState::Stale` means the caller's observed facts or selected
  artifact no longer match Pumas' current model record.

`ModelEntryPathState` should be returned alongside `ModelArtifactState` so
Pantograph can distinguish a missing artifact from a missing, stale, ambiguous,
or invalid load path.

The resolver may need resolver-specific derivation for
`ModelArtifactState::Missing` and related states. The existing selector
projection may derive ready, partial, invalid, stale, ambiguous, and
needs-detail states from metadata and download fields without covering every
exact selected-artifact load-target case. The resolver implementation should
not assume the selector projection already covers every state needed at this
execution boundary.

An expected-kind mismatch is an additional diagnostic over the state fields:
the artifact may exist, but if Pantograph requested a Diffusers directory and
the selected artifact is GGUF, Pumas should return an
`ArtifactKindMismatch` diagnostic and no load target.

## Authority And Stale Checks

`PumasModelRef` is the authoritative selected-artifact reference in the request.
If `model_ref.selected_artifact_id` or `model_ref.selected_artifact_path` is
present, Pumas should resolve exactly that artifact or return typed diagnostics.
If both are absent, Pumas may resolve the model only when the model has exactly
one unambiguous loadable artifact for the requested kind. Missing selected
artifact fields should become `MissingSelectedArtifact` only when exact artifact
identity is required, when multiple artifacts could match, or when the
artifact-kind request cannot be resolved without ambiguity.

`caller_observed_entry_path` and
`caller_observed_package_facts_contract_version` are optional observations from
Pantograph's cached package facts. They exist only to help Pumas return precise
stale-facts diagnostics. They must not select a different artifact, repair the
request, or override `PumasModelRef`.

If the caller-observed entry path disagrees with the selected artifact resolved
from `PumasModelRef`, Pumas should return `ModelArtifactState::Stale` or a
specific `SelectedArtifactMismatch` diagnostic rather than silently switching to
either side.

## Staged Implementation

### Stage 1: Read-Only Resolver

Add the request/response DTOs and implement lookup from existing Pumas indexed
model records, selected artifact metadata, package facts, and external-reference
asset records. Do not trigger downloads or repairs from this API.

This implementation should not simply wrap `resolve_model_package_facts` or
`resolve_model_execution_descriptor`. Current executable APIs are model-level
and may choose a primary file or directory; this resolver needs an exact
selected-artifact contract. It also needs lower-level lookup or explicit error
translation so normal unavailable states such as missing, partial, invalid,
needs-detail, and stale return typed responses instead of opaque errors.

Expose the resolver through the surfaces Pantograph actually consumes:
`ModelLibrary`, the Pumas API/RPC state, `PumasLocalClient`, and
`PumasReadOnlyLibrary` if read-only consumers need load-target checks without
owning lifecycle.

### Stage 2: Pantograph Integration

Pantograph calls the resolver at the composition boundary before worker
dispatch. Inference planning remains side-effect free; the resolver call belongs
in the host/runtime integration layer that already has Pumas access.

### Stage 3: Materialization Hooks

If Pumas later supports managed download/materialization, the same API can
return the existing not-materialized state plus actionable metadata or a
separate materialization handle. Pantograph still should not perform path
repair.

## Test Expectations

Pumas should add tests for:

- valid Diffusers bundle returns `ModelArtifactState::Ready`, a ready entry
  path state, and a directory load target;
- valid GGUF artifact returns `ModelArtifactState::Ready`, a ready entry path
  state, and a file load target when requested as GGUF;
- requesting Diffusers for a GGUF artifact returns `ArtifactKindMismatch`;
- missing selected artifact id/path returns `MissingSelectedArtifact` when
  exact artifact identity is required or the model has ambiguous artifacts;
- stale selected artifact returns `ModelArtifactState::Stale` plus a precise
  stale/mismatch diagnostic;
- known but not downloaded model returns `ModelArtifactState::Missing` or the
  existing Pumas state that represents not materialized;
- partially downloaded artifacts return `ModelArtifactState::Partial`;
- summary-only artifacts return `ModelArtifactState::NeedsDetail`;
- invalid package facts return `InvalidPackageFacts` or `InvalidArtifact`;
- external-reference assets can return a Pumas-approved local load path with
  typed `StorageKind` and `AssetValidationState`; tests must not require all
  paths to live under the Pumas library root;
- serde fixtures round-trip the request and response shapes.

Pantograph should add tests after the API exists for:

- `Ready` Diffusers target reaches the PyTorch worker envelope;
- non-ready target becomes terminal readiness/planning diagnostics before
  worker dispatch;
- Python worker receives no Pumas handle and performs no path resolution.

## Acceptance Criteria

- Pantograph can request a load target for a selected Pumas artifact without
  knowing Pumas library roots.
- Pumas returns typed load-target readiness diagnostics while preserving
  existing `ModelArtifactState` and `ModelEntryPathState` fidelity.
- Pantograph passes only Pumas-approved local load targets to workers.
- No Pantograph code joins Pumas root paths or infers load paths from model ids,
  artifact names, package hints, or filesystem scans.
- The API is reusable for image generation, text generation, audio generation,
  ONNX, GGUF, and future model families.
