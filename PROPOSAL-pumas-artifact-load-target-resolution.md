# Proposal: Pumas Artifact Load Target Resolution For Pantograph

## Context

Pantograph uses Pumas Library as the canonical owner of model identity, model
storage, selected artifact metadata, package facts, and library availability.
Pantograph does not own model files or model-library roots. Pantograph should
trust Pumas-provided artifact references instead of rediscovering, inferring, or
validating Pumas storage layout from local paths.

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
- which Pumas library root owns it;
- whether the artifact is a loadable directory or a pending/missing artifact;
- whether the selected artifact id/path still matches the current model record;
- what exact local path should be passed to a runtime worker;
- whether the artifact is valid for the requested runtime family, such as a
  Diffusers bundle.

Pantograph should not solve this by configuring Pumas roots and joining paths.
That would duplicate Pumas ownership. It should also not pass a Pumas handle to
the Python worker and let the worker resolve it. That would move library
resolution and diagnostics into the wrong layer.

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

Proposed request shape:

```rust
pub struct ResolveModelArtifactLoadTargetRequest {
    pub model_ref: PumasModelRef,
    pub artifact_ref: PumasArtifactRef,
    pub expected_artifact_kind: Option<PumasArtifactKind>,
    pub consumer: PumasArtifactConsumer,
}

pub struct PumasArtifactRef {
    pub selected_artifact_id: Option<String>,
    pub selected_artifact_path: Option<String>,
    pub entry_path: Option<String>,
}

pub enum PumasArtifactKind {
    DiffusersBundle,
    HfCompatibleDirectory,
    Gguf,
    Onnx,
    Safetensors,
    Other(String),
}

pub struct PumasArtifactConsumer {
    pub consumer_name: String,
    pub task_kind: Option<String>,
    pub runtime_family: Option<String>,
}
```

Exact names can change. The important part is that Pantograph sends Pumas the
model identity, selected artifact identity, and expected artifact kind, then
Pumas returns a typed outcome.

## Proposed Response Shape

```rust
pub struct ResolveModelArtifactLoadTargetResponse {
    pub status: PumasArtifactLoadTargetStatus,
    pub target: Option<PumasArtifactLoadTarget>,
    pub diagnostics: Vec<PumasArtifactLoadTargetDiagnostic>,
}

pub enum PumasArtifactLoadTargetStatus {
    Ready,
    NotDownloaded,
    MissingArtifact,
    StaleSelection,
    ArtifactKindMismatch,
    InvalidArtifact,
    Unavailable,
}

pub struct PumasArtifactLoadTarget {
    pub model_ref: PumasModelRef,
    pub artifact_ref: PumasArtifactRef,
    pub artifact_kind: PumasArtifactKind,
    pub local_load_path: String,
    pub load_path_kind: PumasArtifactLoadPathKind,
    pub library_root_id: Option<String>,
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
    ArtifactNotDownloaded,
    ArtifactPathMissing,
    ArtifactPathNotLoadable,
    ArtifactKindMismatch,
    InvalidPackageFacts,
    StalePackageFacts,
    LibraryUnavailable,
}
```

The response should be serde-stable and append-only. Pantograph can map these
diagnostics into its scheduler/readiness/planner diagnostics without parsing
message text.

## Contract Rules

- Pumas owns resolving model refs, selected artifact refs, library roots, and
  local filesystem load paths.
- Pumas owns whether the selected artifact is currently loadable.
- Pumas owns validating that `local_load_path` is inside Pumas-managed storage.
- Pumas owns checking whether the artifact kind matches the caller's expected
  artifact kind.
- Pumas should return typed unavailable states instead of throwing opaque
  string errors for normal missing/not-downloaded/stale cases.
- Pantograph should not join root paths, scan directories, infer from file
  names, or repair selected artifact refs.
- Pantograph should pass the resolved `local_load_path` to runtime workers only
  after Pumas returns `status = Ready`.
- Runtime workers should not receive Pumas roots or call Pumas directly.

## Diffusers Image Generation Use Case

For Pantograph image generation, the request would look conceptually like:

```rust
ResolveModelArtifactLoadTargetRequest {
    model_ref: scheduler_selected_model_ref,
    artifact_ref: PumasArtifactRef {
        selected_artifact_id: package_facts.model_ref.selected_artifact_id,
        selected_artifact_path: package_facts.model_ref.selected_artifact_path,
        entry_path: Some(package_facts.artifact.entry_path),
    },
    expected_artifact_kind: Some(PumasArtifactKind::DiffusersBundle),
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
    artifact_kind: PumasArtifactKind::DiffusersBundle,
    local_load_path: "/.../Pumas-Library/shared-resources/models/image/...".to_string(),
    load_path_kind: PumasArtifactLoadPathKind::Directory,
    ...
}
```

Pantograph then includes this resolved directory in the Rust-owned worker
envelope. The Python worker loads that directory and does not resolve Pumas
state.

## Availability Semantics

`Ready` means the target is currently loadable as requested. For
`DiffusersBundle`, that means the target path is a local directory and Pumas
recognizes it as the selected Diffusers artifact.

`NotDownloaded` means the model or selected artifact is known but not locally
materialized. Pantograph can report this as a readiness failure or ask the user
to download/install the artifact through Pumas-owned flows.

`StaleSelection` means the selected artifact id/path supplied by Pantograph no
longer matches Pumas' current model record. Pantograph should fail the workflow
and refresh Pumas facts rather than silently selecting another artifact.

`ArtifactKindMismatch` means the artifact exists, but not as the requested kind.
For example, Pantograph requested a Diffusers directory but the selected
artifact is GGUF.

`InvalidArtifact` means Pumas can locate the artifact but package validation
does not consider it loadable.

## Staged Implementation

### Stage 1: Read-Only Resolver

Add the request/response DTOs and implement lookup from existing Pumas indexed
model records, selected artifact metadata, and package facts. Do not trigger
downloads or repairs from this API.

### Stage 2: Pantograph Integration

Pantograph calls the resolver at the composition boundary before worker
dispatch. Inference planning remains side-effect free; the resolver call belongs
in the host/runtime integration layer that already has Pumas access.

### Stage 3: Materialization Hooks

If Pumas later supports managed download/materialization, the same API can
return `NotDownloaded` plus actionable metadata or a separate materialization
handle. Pantograph still should not perform path repair.

## Test Expectations

Pumas should add tests for:

- valid Diffusers bundle returns a `Ready` directory load target;
- valid GGUF artifact returns a `Ready` file load target when requested as GGUF;
- requesting Diffusers for a GGUF artifact returns `ArtifactKindMismatch`;
- missing selected artifact id/path returns `MissingSelectedArtifact`;
- stale selected artifact returns `StaleSelection`;
- known but not downloaded model returns `NotDownloaded`;
- invalid package facts return `InvalidPackageFacts` or `InvalidArtifact`;
- local load path is never returned outside Pumas-managed storage;
- serde fixtures round-trip the request and response shapes.

Pantograph should add tests after the API exists for:

- `Ready` Diffusers target reaches the PyTorch worker envelope;
- non-ready target becomes terminal readiness/planning diagnostics before
  worker dispatch;
- Python worker receives no Pumas handle and performs no path resolution.

## Acceptance Criteria

- Pantograph can request a load target for a selected Pumas artifact without
  knowing Pumas library roots.
- Pumas returns typed load-target readiness diagnostics.
- Pantograph passes only Pumas-approved local load targets to workers.
- No Pantograph code joins Pumas root paths or infers load paths from model ids,
  artifact names, package hints, or filesystem scans.
- The API is reusable for image generation, text generation, audio generation,
  ONNX, GGUF, and future model families.
