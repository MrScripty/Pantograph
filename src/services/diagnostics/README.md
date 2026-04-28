# src/services/diagnostics

## Purpose
This directory contains frontend TypeScript mirrors for diagnostics and
workbench projection DTOs that cross the Tauri boundary. It exists so pages and
workflow services share typed contracts without owning diagnostics accumulation
or durable audit state in the browser.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `types.ts` | Diagnostics, scheduler, I/O artifact, Library usage, retention, local Network, and projection-state DTOs mirrored from backend command responses. |

## Problem
Workbench pages need dense run lists, selected-run diagnostics, I/O artifact
metadata, Library usage rows, and scheduler facts, but the frontend must not
rebuild those views from raw diagnostic events. A single DTO module keeps those
wire shapes explicit while the backend-owned ledger and materialized
projections remain the source of truth.

## Constraints
- DTOs must mirror Rust serialization field names and enum labels.
- Frontend code may filter presentation rows, but must not infer durable
  scheduler, diagnostics, retention, or Library truth from raw payload JSON.
- Projection freshness and cursor state must stay visible to callers.
- Legacy snapshot DTOs may remain while native commands still expose them, but
  new workbench pages should prefer projection-specific DTOs.

## Decision
Keep this directory as a type-only diagnostics contract boundary. The retired
frontend diagnostics store and panel no longer accumulate traces in the browser.
Workbench pages call `WorkflowProjectionService` and `WorkflowCommandService`
methods that return the DTOs defined here. Backend projections provide run
history, scheduler timelines, selected-run estimates, retention summaries, I/O
artifact metadata, Library usage, and local Network status.

## Alternatives Rejected
- Rebuild workbench views from raw ledger rows in TypeScript.
  Rejected because it would bypass validated backend projections and duplicate
  projection cursor behavior.
- Keep a browser diagnostics service as a second source of run history.
  Rejected because it would drift from the typed event ledger and materialized
  projection model.

## Invariants
- DTO names, field casing, and enum labels must match backend command
  responses.
- Projection response types must expose `projection_state` when the backend can
  report freshness or cursor progress.
- I/O artifact DTOs carry typed `retention_state` values. Consumers must not
  infer deleted, expired, external, truncated, or too-large states from
  `payload_ref` presence.
- I/O artifact DTOs carry producer and consumer node/port fields. Consumers
  should use those endpoint fields for browsing and filtering instead of
  parsing payload JSON or overloading the event `node_id`.
- Run-list responses carry backend-owned `facets` derived from materialized
  projections. Consumers should use those counts for mixed-version and policy
  summaries instead of rebuilding them from raw ledger events or sampled pages.
- Scheduler estimate query DTOs expose a narrow estimate-shaped projection for
  selected runs so callers do not mine full run-detail payloads for scheduler
  estimate facts.
- Library usage query DTOs include optional `workflow_run_id` filters so
  frontend consumers can request selected-run asset usage without
  reconstructing active-run Library state from raw ledger events.
- Retention and Pumas command DTOs mirror backend command/result shapes so GUI
  controls can display outcomes without optimistic local audit mutation.

## Revisit Triggers
- Rust-to-TypeScript DTO generation replaces manual interface mirrors.
- Native legacy snapshot commands are removed and their TypeScript interfaces
  can be deleted.
- Projection APIs are versioned independently from the desktop frontend.

## Dependencies
**Internal:** `src/services/workflow`.

**External:** TypeScript type system.

## Related ADRs
- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples
```ts
import type { WorkflowRunListQueryResponse } from '../diagnostics/types.ts';

function getRunCount(response: WorkflowRunListQueryResponse): number {
  return response.runs.length;
}
```

## API Consumer Contract
- Inputs: request DTOs in `types.ts` use backend wire names and optional
  filters exactly as accepted by Tauri commands.
- Outputs: response DTOs preserve backend-authored rows, facets, summaries, and
  projection freshness fields without client-side replacement.
- Lifecycle: consumers may refresh projections on page mount or user action;
  they must not assume active-run selection is persisted.
- Errors: command failures are normalized by workflow service boundaries rather
  than by this type module.
- Compatibility: additive fields are allowed when backend commands add
  projection facts; field removals require a plan update and matching frontend
  migration.

## Structured Producer Contract
- `types.ts` is the structured producer for frontend diagnostics and workbench
  projection DTOs.
- Stable fields include IDs, timestamps, status labels, projection-state
  cursors, retention state labels, and audit outcome labels.
- Omitted optional fields mean the backend lacks that fact for the current row;
  callers must render absence explicitly instead of inventing defaults.
- Row ordering is backend-owned unless a page applies a documented presentation
  sort over an already fetched page.
- DTO changes that affect persisted consumers, saved fixtures, or contract
  tests require coordinated Rust and TypeScript updates.
