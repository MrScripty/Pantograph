# Pantograph Inference Interface Contracts

This crate owns the path-free DTO contract that describes how a connected
Pumas model reference becomes a generic inference-node interface. The graph
editor, workflow validation, scheduler admission, and runtime-host
materialization all consume these contracts rather than carrying Pumas package
facts, executable paths, scheduler choices, or frontend-only metadata.

## Dependencies

- Depends on `pantograph-dependency-planning` only for stable path-free model
  references, scheduler intent identifiers, and the typed dependency-environment
  action enum.
- Does not import workflow-service, scheduler, runtime-host, Pumas lookup or
  lifecycle code, inference execution, worker dispatch, or frontend rendering
  policy.

## Invariants

- All public serialized DTOs use `serde(deny_unknown_fields)`.
- Requests and persisted snapshots are versioned.
- Model references are Pumas-owned references, not local paths.
- Inference interface ports, defaults, option sets, availability, drift, and
  validation summaries are typed and bounded.
- Live validation event/session envelopes are workflow-service owned. This
  crate intentionally does not export graph validation event or stream DTOs.
- Authored snapshots preserve graph shape and drift context only. They must not
  contain Pumas package facts, runtime load targets, executable paths,
  scheduler decisions, full runtime API schemas, or media payloads.
- Scheduler/runtime/device fields express constraints or diagnostics only.
  Scheduler-selected execution remains outside this crate.
- Dependency-environment action intents carry graph-session identity, graph
  revision, optional validation-session identity, target node id, and
  resolve/check/install action only. They must not carry model paths, Pumas
  facts, platform context, identity keys, dependency-planning requests, or
  dependency-environment requests.
- Inference connection surfaces carry graph revision, optional validation-session
  identity, target node id, descriptor fingerprint, descriptor-backed port
  descriptors, validation summary, optional drift report, and bounded
  diagnostics only. They must not carry Pumas package facts, runtime-host
  payloads, scheduler placement decisions, local paths, frontend layout state, or
  live validation transport events.

## API Consumer Contract

Consumers may validate incoming DTOs with the provided `Validated*` wrappers or
`validate` methods before storing, displaying, or enqueueing inference work.
Frontend code should use the coarse availability and validation summary fields
for UI state and should display diagnostics as supporting detail. It must not
infer enqueue permission from raw diagnostic counts.

Dependency-environment UI callers send `DependencyEnvironmentActionIntent`.
Workflow-service validates the intent against the current graph revision and
validation session, then derives the canonical dependency-environment request
from backend-owned descriptor and dependency-planning state.
If that derivation cannot happen, workflow-service returns
`DependencyEnvironmentActionIntentResult` with typed diagnostics instead of
constructing a partial dependency-environment request. Sidecar target and
association failures are represented by the typed `DependencySidecar*`
diagnostic codes, not by message parsing, frontend path inference, or partial
request construction.

Graph connection callers consume `InferenceConnectionSurface` for generic
`llm-inference` dynamic task ports. Workflow-service validates that the surface
matches the current graph revision and validation session before using it for
connection candidates, insert-on-edge preview, connection commits, or queue
submission gating. Missing, pending, stale, unavailable, blocked, or
drift-blocked surfaces return typed diagnostics and must not fall back to static
task ports or frontend-invented ports.

## Structured Producer Contract

Backend producers build descriptors from Pumas model references, selected
artifact facts, inference capability facts, runtime availability facts, and
optional graph-authored constraints. Producers must emit unavailable or
not-implemented diagnostics when facts are missing. They must not guess from
model names, paths, runtime-specific blobs, or package-fact metadata bags.

Backend producers build connection surfaces from current descriptor projection
records, authored snapshots, drift reports, and draft validation summaries. A
surface is a presentation/admission contract over backend-owned validation state;
it is not a scheduler dispatch request and not a runtime-host execution payload.
Saved graph artifacts may keep authored snapshots for historical shape and drift
explanation, but executable queue admission must use current backend validation
authority.

## Revisit Triggers

- A new value category is needed beyond scalar, artifact, reference, or
  constraint.
- Artifact-reference defaults become necessary.
- Runtime/device capability reporting needs richer typed advisory alternatives.
- Graph patch operation ownership moves; those operations should stay with
  graph mutation code, not this DTO crate.
- A consumer needs shared live validation event transport; that should first be
  evaluated against the workflow-service ownership boundary instead of adding
  unscoped event DTOs here.
