# Pantograph Inference Interface Contracts

This crate owns the path-free DTO contract that describes how a connected
Pumas model reference becomes a generic inference-node interface. The graph
editor, workflow validation, scheduler admission, and runtime-host
materialization all consume these contracts rather than carrying Pumas package
facts, executable paths, scheduler choices, or frontend-only metadata.

## Dependencies

- Depends on `pantograph-dependency-planning` only for stable path-free model
  references and scheduler intent identifiers.
- Does not import workflow-service, scheduler, runtime-host, Pumas lookup or
  lifecycle code, inference execution, worker dispatch, or frontend rendering
  policy.

## Invariants

- All public serialized DTOs use `serde(deny_unknown_fields)`.
- Requests and persisted snapshots are versioned.
- Model references are Pumas-owned references, not local paths.
- Inference interface ports, defaults, option sets, availability, drift, and
  validation summaries are typed and bounded.
- Authored snapshots preserve graph shape and drift context only. They must not
  contain Pumas package facts, runtime load targets, executable paths,
  scheduler decisions, full runtime API schemas, or media payloads.
- Scheduler/runtime/device fields express constraints or diagnostics only.
  Scheduler-selected execution remains outside this crate.

## API Consumer Contract

Consumers may validate incoming DTOs with the provided `Validated*` wrappers or
`validate` methods before storing, displaying, or enqueueing inference work.
Frontend code should use the coarse availability and validation summary fields
for UI state and should display diagnostics as supporting detail. It must not
infer enqueue permission from raw diagnostic counts.

## Structured Producer Contract

Backend producers build descriptors from Pumas model references, selected
artifact facts, inference capability facts, runtime availability facts, and
optional graph-authored constraints. Producers must emit unavailable or
not-implemented diagnostics when facts are missing. They must not guess from
model names, paths, runtime-specific blobs, or package-fact metadata bags.

## Revisit Triggers

- A new value category is needed beyond scalar, artifact, reference, or
  constraint.
- Artifact-reference defaults become necessary.
- Runtime/device capability reporting needs richer typed advisory alternatives.
- Graph patch operation ownership moves; those operations should stay with
  graph mutation code, not this DTO crate.
