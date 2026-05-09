# Workflow Run Node IO, Streaming, And Runtime Settings

## Purpose

This directory contains the focused repair plan for workflow run inspection,
text streaming visibility, and inference runtime device settings. It exists
because these issues cut across node-engine execution, embedded runtime
adapters, diagnostics ledger projections, ArtifactStore retention, and the
run-centric workbench.

## Contents

| File | Description |
| ---- | ----------- |
| `plan.md` | Standards-compliant implementation plan for restoring saved per-node IO, fixing text streaming visibility, and exposing inference runtime device settings through existing Pantograph systems. |

## Problem

Pantograph already has retention policy, ArtifactStore descriptors, diagnostics
ledger projections, run snapshots, and a graph page for executed workflow runs,
but current user testing shows executed runs can complete with no inspectable
per-node inputs or outputs. Text generation streaming also does not appear in
the connected text output, and inference device/runtime settings are not
available to route work to GPU-capable llama.cpp execution.

Initial code search found that most required systems already exist. The
diagnostics ledger has `node_input` and `node_output` artifact roles,
workflow-service records workflow-level IO artifacts, node-engine has resolved
inputs and completed output maps, llama.cpp emits stream chunks and final text
on `response`, and the frontend has `NodeStream`/`NodeCompleted` handlers. The
plan therefore repairs missing joins and ignored settings rather than adding a
parallel persistence, streaming, or runtime configuration system.

The backend service/domain crates are the source of truth for retained run
facts, runtime setting semantics, node IO evidence, and streaming contracts.
Tauri may expose commands and translate events, but it must not own final
backend contract semantics.

Artifact identity is scoped to a single immutable run. A retained node IO
artifact represents the final fact for one run/role/node/port. Comparing the
same workflow across executions belongs in workflow history read models, not in
per-run artifact identity.

## Constraints

- Existing retention policy, ArtifactStore, diagnostics ledger, run-detail
  projection, and run-centric workbench ownership must be repaired and reused.
- Do not introduce a second node-IO persistence system.
- Backend service/domain crates remain the source of truth for persisted run
  facts and runtime settings that affect execution.
- Tauri is an IPC/app-shell adapter for these contracts, not the authoritative
  owner of execution facts, retained node IO, stream semantics, or runtime
  setting defaults.
- Frontend state may present selected run/node views but must not repair missing
  backend data locally.
- Pumas remains the canonical model source; Pantograph owns runtime selection,
  execution settings, and scheduler/runtime policy.

## Blast Radius

Source changes from the plan should stay within the active vertical slice and
inside the existing execution/run-inspection ownership boundaries:

- Node-engine may expose normalized execution evidence but does not own
  persistence.
- Workflow-service owns retained IO materialization, ArtifactStore usage, and
  run-inspection read-model assembly.
- Diagnostics-ledger changes are limited to events, projections, or queries
  that existing `IoArtifactObserved` records cannot already satisfy.
- Inference owns backend runtime settings and llama.cpp config application, not
  scheduler policy.
- Tauri remains transport/adaptation only.
- Frontend services and components own presentation and UI-only state only.

Pumas implementation, scheduler admission policy, broad saved-workflow
compatibility shims, unrelated backend support, and new persistence systems are
outside the plan unless a re-plan trigger is explicitly recorded.

## Decision

Use a staged audit-first implementation plan. The first milestone must prove
where existing node IO disappears before source implementation begins. Later
milestones restore the existing projection path with
`IoArtifactObserved(node_input/node_output)`, repair text streaming as a
canonical `response` event/display path, and expose backend-owned runtime
device settings through Pumas defaults plus Pantograph workflow/run overrides.
Before implementation spreads, freeze one backend-owned node IO evidence
adapter, one retained-value materialization rule, and one runtime settings
contract. Add a backend run-inspection read model when the executed graph page
needs graph, status, and artifact projections together. That read model is a
factual inspection contract, not a UI contract: backend owns stable facts and
freshness; frontend owns layout, grouping, labels, panels, selected-node state,
empty states, and visual treatment.

## Alternatives Rejected

- Add a new per-node IO database table outside the diagnostics ledger and
  ArtifactStore path.
  Rejected because Pantograph already has retention and run-detail projection
  systems intended for this purpose.
- Treat the text generation `stream` graph port as the primary final output.
  Rejected because canonical LLM final text is `response`; live chunks are an
  event stream and must not replace final output retention.
- Store runtime device choices as frontend-only settings.
  Rejected because device, GPU offload, and CPU thread settings affect
  execution behavior and must be backend-owned, snapshotted, and reproducible.

## Invariants

- Every completed workflow run with retention enabled must expose node status
  and retained per-node IO according to policy.
- Terminal workflow outputs and per-node inspection records are related but
  distinct facts.
- `IoArtifactProjectionRecord` remains the canonical persisted node IO
  projection unless implementation proves a composite read DTO is needed.
- A composite run-inspection DTO may reduce frontend race-prone composition,
  but it must remain presentation-neutral.
- Run inspection is descriptor-first: graph/run pages should load artifact
  metadata, handles, retention state, and bounded previews by default, while
  full artifact bodies are fetched lazily through existing artifact APIs.
- A run executes once. A repeated execution of the same workflow is a new run,
  even when cached node results are reused.
- Text generation final output is `response`.
- Text streaming events are associated with the canonical text response path and
  must not be the only retained evidence of generated text.
- `stream` is a live delivery mode for a logical output. Connecting a stream
  path must not change final retained run artifacts.
- Stream chunks must not make durable storage or UI rendering scale with
  unbounded token-rate work; final retained text remains the completed
  `response`.
- Runtime settings that affect execution are captured in run snapshots and must
  be applied by the backend instead of being frontend-only or hardcoded.
- Runtime settings converge into one backend-normalized effective-settings
  snapshot rather than parallel Tauri/frontend/inference semantics.
- File size standards are decomposition review triggers. Split touched files
  when a stable responsibility boundary exists; document a reason when a file
  remains oversized for clarity.

## Revisit Triggers

- The audit proves the intended existing retention path was never implemented
  for node IO.
- Cached node execution cannot produce inspectable IO without changing
  node-engine event lifecycle ownership.
- Scheduler-run streaming uses a separate transport that cannot be reconciled
  with the existing `NodeStream` frontend path.
- Backend run-inspection cannot provide graph/status/artifact freshness without
  duplicating persisted projection facts.
- ArtifactStore retention cannot safely represent node IO under current value
  size, redaction, or binary payload constraints.
- Pumas changes its model summary/detail contracts in a way that affects
  runtime default settings.
- A second local inference backend reaches production readiness and needs
  backend-specific device settings beyond the first llama.cpp slice.

## Dependencies

**Internal:** `crates/node-engine`, `crates/pantograph-embedded-runtime`,
`crates/pantograph-workflow-service`, `crates/inference`, `src-tauri/src/workflow`,
`src/services/workflow`, `src/components/workbench`, and the execution-platform
Stage `11` and Stage `12` plans.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
and Pumas Library model package facts/summary APIs.

## Related ADRs

- `None identified as of 2026-05-08.`
- `Reason: This plan repairs implementation and projection gaps inside existing
  accepted execution-platform architecture rather than choosing a new
  architecture.`
- `Revisit trigger: implementation requires changing retention ownership,
  ArtifactStore semantics, or scheduler/runtime policy boundaries.`

## Usage Examples

Implementation should start from:

```text
docs/plans/workflow-run-node-io-streaming-runtime-settings/plan.md
```

## API Consumer Contract

- This directory does not expose runtime APIs.
- Human implementers consume `plan.md` as staged implementation guidance.
- Implementation must update source module READMEs or ADRs when public API,
  persisted projection, ArtifactStore, or runtime-setting contracts change.

## Structured Producer Contract

- Stable artifact category: Markdown implementation plan.
- The plan records milestones, status, risks, re-plan triggers, and verification
  criteria.
- Status and execution notes must be updated during implementation.
