# Pantograph Execution Platform Plans

## Purpose

This directory owns the ordered planning artifacts for Pantograph's execution
platform work: durable attribution, backend-owned node contracts, managed
runtime observability, model/license usage diagnostics, composition, and
binding projections.

## Contents

| File | Description |
| ---- | ----------- |
| `00-overview-and-boundaries.md` | Source documents, core direction, architecture boundaries, and cross-cutting standards gates. |
| `01-client-session-bucket-run-attribution.md` | Durable client, session, bucket, and workflow-run identity required before diagnostics and model usage tracking can be reliable. |
| `02-node-contracts-and-discovery.md` | Canonical node/port contracts, effective contracts, registry discovery, and graph-authoring discovery APIs. |
| `03-managed-runtime-observability.md` | Runtime-created node execution context, managed capabilities, baseline diagnostics, guarantee levels, cancellation, and progress. |
| `04-model-license-diagnostics-ledger.md` | Durable model/license usage events, Pumas license snapshots, output measurement, query projections, and retention boundaries. |
| `05-composition-factoring-and-migration.md` | Primitive/composed node strategy, trace preservation, existing-node factoring, and persisted workflow migration. |
| `06-binding-projections-and-verification.md` | Native Rust, C#, Python, and Elixir/BEAM projection expectations, support-tier alignment, and host-language verification. |
| `07-standards-compliance-review.md` | Cross-plan standards review covering planning quality and future implementation compliance gates. |
| `08-stage-start-implementation-gate.md` | Instructions for validating plan readiness, worktree hygiene, verification, and commit boundaries before each stage begins. |
| `09-stage-end-refactor-gate.md` | Instructions for deciding whether each implementation stage needs a standards refactor before the next stage begins. |
| `10-concurrent-phased-implementation.md` | Artifact layout and rules for converting a stage into safe phased parallel implementation waves when warranted. |

## Problem

The previous single plan mixed too many dependent concerns. The work needs to
be readable in execution order because later slices depend on earlier boundary
and identity decisions.

## Constraints

- The plans consume the requirement files in `../../requirements/`.
- Native Rust remains the canonical application API.
- C#, Python, and Elixir/BEAM bindings project backend-owned contracts instead
  of defining host-local node semantics.
- Runtime-managed observability must not depend on per-node diagnostics
  boilerplate.
- Model/license usage tracking depends on durable client/session/bucket/run
  attribution.

## Decision

Use `./` as the source directory for
this work. Keep plan files numbered by implementation dependency order instead
of maintaining a single oversized `final-plan.md`.

## Alternatives Rejected

- Keep `pantograph-node-runtime-and-bindings`: rejected because it undersells
  client/session/bucket attribution and diagnostics ledger work.
- Keep one `final-plan.md`: rejected because the plan became too broad to use
  as an execution guide.

## Invariants

- Backend Rust owns canonical contracts, runtime execution semantics, and
  diagnostics facts.
- Bindings remain projections over backend-owned contracts.
- Model/license diagnostics must not require explicit observability nodes.
- Durable usage records must attach to client, session, bucket, workflow run,
  graph node, model, license, and output measurement facts.

## Revisit Triggers

- A numbered slice becomes large enough to need implementation-wave subplans.
- Binding implementation sequencing diverges by host language.
- Diagnostics persistence requires a dedicated storage-engine plan.

## Dependencies

**Internal:** `../../requirements/`, `../../headless-embedding-api-v1.md`,
`../../headless-native-bindings.md`,
`../../plans/pantograph-binding-platform/final-plan.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.

## Related ADRs

- `None identified as of 2026-04-23.`
- `Reason: These files are staged plans, not frozen architecture decisions.`
- `Revisit trigger: The canonical node contract crate boundary is finalized.`

## Usage Examples

Read the files in numeric order when planning implementation:

```text
00-overview-and-boundaries.md
01-client-session-bucket-run-attribution.md
02-node-contracts-and-discovery.md
03-managed-runtime-observability.md
04-model-license-diagnostics-ledger.md
05-composition-factoring-and-migration.md
06-binding-projections-and-verification.md
07-standards-compliance-review.md
08-stage-start-implementation-gate.md
09-stage-end-refactor-gate.md
10-concurrent-phased-implementation.md
```

## API Consumer Contract

- This directory does not expose runtime APIs.
- The plans describe future API and binding contracts, but those contracts are
  not live until implemented in the owning crates and documented in their API
  surfaces.
- Compatibility expectations are defined in the numbered plan files.

## Structured Producer Contract

- Stable artifact: numbered Markdown planning documents.
- File numbers encode dependency order, not necessarily one-commit milestones.
- These files are manually maintained and are not generated from schemas.
