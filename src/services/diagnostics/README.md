# Diagnostics projection types

## Purpose

Holds the TypeScript projection of the typed backend read models the
workbench pages consume: run lists, run detail, scheduler timeline, node
status, I/O artifact inspection, Library usage, local Network status and
Pumas model facts. The workbench never reads diagnostic ledger rows; it reads
these projections (ADR-014).

## Contents

- `types.ts`: the projection record and response shapes, one interface per
  backend projection, plus the Pumas library and Hugging Face search shapes.
- `projectionState.ts`: helpers for `ProjectionStateRecord` (freshness
  cursors) used by pages to show "Current at seq N" style lines.

## Problem

Pages need stable, typed shapes for backend responses without inventing
fields or copying one sample response. The shapes must be traceable to their
producer so a renamed backend field is a visible contract change, not a
silent `undefined`.

## Constraints

- A TypeScript type is not runtime proof. Presenters still decode values
  (`nonEmptyString`, numeric guards) before use.
- Shapes that pass through an external producer unchanged mirror that
  producer's serialisation. `PumasHfModelSearchResult` mirrors Pumas
  `HuggingFaceModel` (pumas-core `models/model.rs`,
  `rename_all = "camelCase"`).
- Optional fields stay optional; a page omits the row rather than defaulting.

## Decision

Keep one projection module for all workbench read models, owned here, and
document the producer of any shape that is not generated from the host's
Rust types. The heading set of this file is required by
`scripts/check-decision-traceability.sh` (plan issue WR-001).

## Alternatives Rejected

- Per-page type files: the same projection is consumed by several pages
  (run list by the picker, Scheduler and Sessions), so one owner avoids drift.
- Index-signature "anything goes" shapes: they hide producer changes, as the
  former `{ id?: string; [key: string]: unknown }` search result did.

## Invariants

- No type here describes raw `diagnostic_events` rows or `payload_json`.
- Field names match the producer's serialised names exactly.
- Adding a field here does not make it available: the backend projection
  must emit it, and the consuming presenter must decode it.

## Revisit Triggers

- The host starts generating TypeScript from its Rust projection types
  (then this module becomes generated output).
- A projection gains fields the redesign assumed (device memory, retention
  durations, node progress, client sessions; see the plan's issues).

## Dependencies

- Consumed by `src/components/workbench/**` presenters and pages, and by
  `src/components/workbench/shell/` for the run list.
- Produced by the Tauri workflow commands under `src-tauri/src/workflow/`.

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`
- `docs/adr/ADR-016-workflow-error-diagnostics-spine.md`

## Usage Examples

```ts
import type { RunListProjectionRecord } from '../../services/diagnostics/types';
import { mockProjectionState } from '../../services/diagnostics/projectionState'; // tests only
```
