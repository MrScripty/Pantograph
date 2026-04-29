# Pantograph Execution Platform Plans

## Purpose

This directory owns the ordered planning artifacts for Pantograph's execution
platform work: durable attribution, backend-owned node contracts, managed
runtime observability, model/license usage diagnostics, composition, and
binding projections. It also owns the active extensions for ArtifactStore,
media format settings, managed media dependencies, and the consolidated
run-centric workbench.

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
| `11-artifact-format-settings-and-managed-media-dependencies.md` | ArtifactStore, binary-safe media payloads, canonical workbench Settings ownership, format defaults, managed OCIO/ffmpeg/OIIO dependencies, and related verification. |
| `12-run-centric-workbench-consolidation.md` | Canonical consolidation of the run-centric GUI workbench plans into execution-platform ownership. |
| `13-managed-media-conversion-leases.md` | Real managed media conversion/transcoding and per-conversion active-version lease attribution. |
| `implementation-waves/` | Stage-specific concurrent implementation wave specs, coordination ledgers, and worker report paths. |
| `reviews/run-centric-workbench/` | Historical review evidence moved from the retired run-centric workbench plan directory. |

## Current Stage Status

| Stage | Status | Notes |
| ----- | ------ | ----- |
| `01` | Complete | Runtime attribution implementation and tests are present. |
| `02` | Complete | Canonical node contracts, discovery projections, compatibility checks, and tests are present. |
| `03` | Complete | Runtime-owned observability, managed capabilities, diagnostics adapters, and tests are present. |
| `04` | Complete | Model/license diagnostics ledger, retention, typed events, incremental projections, and tests are present. |
| `05` | Complete | Composed-node contracts, migration records, workflow canonicalization migration, and tests are present. |
| `06` | Reopened / not complete | Binding projection code exists, but current UniFFI/C# verification fails because execution-session run requests omit `workflow_semantic_version`; one UniFFI graph-save test uses an invalid workflow identity. |
| `07` | Stale / not complete | Standards review docs exist, but they depend on the old Stage `06` completion claim and need refresh after Stage `06` is corrected. |
| `08` | Gate-only | Reusable stage-start implementation gate, not an implementation stage. |
| `09` | Gate-only | Reusable stage-end refactor gate, not an implementation stage. |
| `10` | Coordination-only | Reusable concurrent implementation scaffold, not an implementation stage. |
| `11` | Complete with Stage `13` follow-up | ArtifactStore, format settings, canonical Settings ownership, managed media dependency scaffolding, active-version descriptor metadata, binding/API/frontend projections, and media stream artifactization are implemented. Real conversion process invocation and per-conversion lease-token attribution are owned by Stage `13`. |
| `12` | Complete with follow-ups | Imports the run-centric GUI workbench plans into execution-platform ownership and implements Scheduler-default workbench pages, active-run navigation, current API/frontend projections, review findings, and rollout gates. GUI screenshot coverage remains a follow-up until a harness exists. |
| `13` | In progress | Neutral conversion contracts, command planning, workflow-service conversion hook, descriptor/diagnostics attribution metadata, managed executable resolution, and Tauri host conversion adapter injection are implemented. Remaining work: private temp-file fallback, cancellation/drop cleanup coverage, real-binary fixture tests, GUI/API conversion failure display, and stage-end review. |

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
- Media/artifact payload handling, persistent settings, or managed dependency
  scope changes beyond the completed execution-platform stages.

## Dependencies

**Internal:** `../../requirements/`, `../../headless-embedding-api-v1.md`,
`../../headless-native-bindings.md`,
`../../plans/pantograph-binding-platform/final-plan.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.

## Related ADRs

- `../../adr/ADR-005-durable-runtime-attribution.md`
- `Reason: Stage 01 accepted durable runtime attribution ownership, SQLite
  persistence, credential storage, bucket semantics, and execution-session
  terminology decisions.`
- `../../adr/ADR-006-canonical-node-contract-ownership.md`
- `Reason: Stage 02 accepted canonical node contract, effective contract, and
  backend-owned discovery ownership.`
- `../../adr/ADR-007-managed-runtime-observability-ownership.md`
- `Reason: Stage 03 accepted runtime-owned observability, managed capability,
  cancellation, progress, and guarantee classification ownership.`
- `../../adr/ADR-008-durable-model-license-diagnostics-ledger.md`
- `Reason: Stage 04 accepted durable model/license diagnostics ledger
  ownership, SQLite persistence, retention, and query projection boundaries.`
- `../../adr/ADR-009-composed-node-contracts-and-migration.md`
- `Reason: Stage 05 accepted composed-node contract ownership, primitive trace
  preservation, runtime lineage, and saved-workflow migration behavior.`
- `../../adr/ADR-010-binding-projection-ownership-and-support-tiers.md`
- `Reason: Stage 06 accepted binding projection ownership, generated artifact
  policy, and evidence-based host support tiers. Completion is reopened by the
  2026-04-29 audit until UniFFI/C# execution-session request drift is fixed and
  verified.`
- `Revisit trigger: Future execution-platform work changes one of these
  ownership boundaries or supersedes an accepted stage decision.`
- `11-artifact-format-settings-and-managed-media-dependencies.md`
- `Reason: Stage 11 extends the execution platform with ArtifactStore,
  canonical workbench Settings ownership, binary-safe media payloads, and
  managed media dependencies after the original Stage 01-07 closeout.`
- `12-run-centric-workbench-consolidation.md`
- `Reason: Stage 12 imports the run-centric GUI workbench plan set so
  execution-platform remains the canonical planning home.`
- `13-managed-media-conversion-leases.md`
- `Reason: Stage 13 owns real media conversion process execution and
  per-conversion lease-token attribution that Stage 11 intentionally deferred
  after completing descriptor attribution and managed dependency scaffolding.`

## Implementation Entry Point

Start implementation from this file.

Execution order:

1. Read this `README.md`.
2. Read `00-overview-and-boundaries.md`.
3. Read `08-stage-start-implementation-gate.md`.
4. Read `10-concurrent-phased-implementation.md` and
   `implementation-waves/README.md` before deciding how the stage will be
   implemented.
5. Select the first incomplete numbered implementation stage, beginning with
   `01-client-session-bucket-run-attribution.md`.
6. Read the selected stage plan and the matching
   `implementation-waves/<stage-slug>/README.md`.
7. Apply `08-stage-start-implementation-gate.md` before editing source code,
   tests, configs, manifests, generated files, or build metadata.
8. If the start gate selects concurrent implementation, use the matching folder
   under `implementation-waves/<stage-slug>/` before launching workers.
9. Implement one logical step or one approved worker wave at a time.
10. Record progress, findings, verification, deviations, and discovered bugs in
   the selected stage plan, the stage coordination ledger, or the assigned
   worker report before continuing.
11. Commit each completed logical step atomically after its required
   verification passes, following `COMMIT-STANDARDS.md`.
12. Do not begin the next logical step with dirty source, test, config, manifest,
   lockfile, generated, or build files left behind from the previous step.
13. Apply `09-stage-end-refactor-gate.md` after the stage implementation and
    verification complete, before starting the next numbered stage.

Before continuing artifact/settings/media-dependency implementation, resolve the
reopened Stage `06` binding verification work or explicitly record why Stage
`11` can proceed without touching affected binding surfaces. For artifact work,
select `11-artifact-format-settings-and-managed-media-dependencies.md` as the
active implementation stage after reading the completed Stage `01` through
Stage `05`, reopened Stage `06`, stale Stage `07`, Stage `11`, and Stage `12`
context. For run-centric GUI work, select
`12-run-centric-workbench-consolidation.md` after Stage `06` and Stage `11`
dependencies are either complete or explicitly excluded from the implementation
slice. For real media transcoding, select
`13-managed-media-conversion-leases.md` only after Stage `11` descriptor,
settings, managed dependency, and ArtifactStore surfaces are complete.

Use the execution prompt at
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/prompts/implement-plan.md`
for every numbered implementation stage. That prompt is authoritative for the
implementation loop: read plan and standards, inspect git status, apply
worktree hygiene, implement one logical step at a time, verify, update plan
status, commit atomically, handle unexpected issues through re-planning, and
close with a verification summary.

Dirty worktree rule:

- Dirty implementation files that overlap the selected stage write set block
  implementation unless explicitly allowed.
- Unrelated dirty files must not be reverted, reformatted, or overwritten.
- Completed logical steps must not leave unresolved dirty implementation files
  before the next step starts.

Progress record locations:

- Stage-level progress and re-plan decisions: the selected numbered stage plan
  or its implementation notes.
- Concurrent wave status and integration decisions:
  `implementation-waves/<stage-slug>/coordination-ledger.md`.
- Worker findings, skipped checks, unexpected bugs, and verification notes:
  the assigned `implementation-waves/<stage-slug>/reports/wave-XX-worker-*.md`
  file.

## Usage Examples

For implementation, read the gate and concurrency rules before executing any
numbered stage:

```text
README.md
00-overview-and-boundaries.md
08-stage-start-implementation-gate.md
10-concurrent-phased-implementation.md
implementation-waves/README.md
01-client-session-bucket-run-attribution.md
implementation-waves/01-client-session-bucket-run-attribution/README.md
```

The numbered plan files remain dependency-ordered by architecture stage, not by
the complete implementation reading sequence:

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
11-artifact-format-settings-and-managed-media-dependencies.md
12-run-centric-workbench-consolidation.md
implementation-waves/
```

For each implementation stage, apply `08-stage-start-implementation-gate.md`
before source edits. If concurrent implementation is selected, use the matching
stage folder under `implementation-waves/`. After implementation, apply
`09-stage-end-refactor-gate.md` before starting the next numbered stage.

## API Consumer Contract

- This directory does not expose runtime APIs.
- Completed stages describe implemented API and binding contracts where the
  owning crates, ADRs, and module documentation now expose them. Stage `06`
  binding projection ownership exists, but completion is reopened until
  UniFFI/C# verification passes the current workflow-run contract. Historical
  future-tense notes remain audit context and are superseded by the latest
  implementation progress sections and ADRs when they conflict.
- Compatibility expectations are defined in the numbered plan files.

## Structured Producer Contract

- Stable artifact: numbered Markdown planning documents.
- File numbers encode dependency order, not necessarily one-commit milestones.
- These files are manually maintained and are not generated from schemas.
