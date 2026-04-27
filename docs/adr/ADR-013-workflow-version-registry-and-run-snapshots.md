# ADR-013: Workflow Version Registry And Run Snapshots

## Status
Accepted

## Context
Pantograph diagnostics need to compare runs without mixing execution behavior
from different workflow graphs or node versions. The GUI also needs to show a
historic workflow as it existed at run submission time without storing a unique
full graph for every run.

Earlier contracts used graph fingerprints for several unrelated meanings:
display cache invalidation, preflight cache invalidation, timing expectation
facets, and diagnostics grouping. That is not precise enough for run-centric
diagnostics because display metadata, editable model/runtime settings, inputs,
and scheduler policy are run context, not workflow-version identity.

## Decision
`pantograph-runtime-attribution` owns durable workflow-version records,
workflow-presentation revision records, and immutable workflow-run snapshots.

Workflow execution versions are keyed by stable workflow id, explicit semantic
version, and backend-computed executable fingerprint. The executable
fingerprint is derived from canonical executable topology: sorted node ids,
node types, node behavior versions, and sorted port connections. Display
metadata and editable node data are excluded.

Workflow presentation revisions are separate records keyed by workflow version
and backend-computed presentation fingerprint. They capture display metadata
needed for historic graph viewing and must not be used for diagnostics
grouping.

Workflow-run snapshots are immutable queue-admission records. They reference
the workflow version and presentation revision immediately and store run
context such as graph settings, runtime requirements, capability model
inventory, runtime capabilities, inputs, output targets, override selection,
scheduler policy, retention policy, and execution-session facts.

The backend does not infer semantic-version intent. If semantic version and
fingerprint mappings disagree, the request is rejected.

## Consequences

### Positive
- Historic diagnostics can group by workflow execution version instead of
  mutable graph display state.
- Historic graph views can recover display metadata through presentation
  revisions without changing execution identity.
- Queued and future runs become auditable before runtime execution starts.
- Model/runtime choices and graph settings remain filterable run context
  instead of silently becoming workflow-version identity.

### Negative
- Schema changes are breaking during the no-legacy cutover.
- Queue submission must resolve version and presentation records before
  scheduler admission, which adds storage work to the submission path.
- Client/bucket attribution still needs a workflow-service submission boundary
  before queued GUI/API runs can carry the same actor facts as lower-level
  attribution-started runs.

### Neutral
- Legacy `graph_fingerprint` fields may remain as cache/timing facets while
  Stage 03 typed event-ledger projections replace graph-fingerprint
  diagnostics grouping.
- Presentation revision history can grow when users make display-only edits,
  but those records are deduplicated by workflow version and presentation
  fingerprint.

## Guardrails
- Workflow versions must be resolved with backend-computed executable
  fingerprints, never caller-supplied graph hashes.
- A semantic version may not point to two executable fingerprints for the same
  workflow id.
- An executable fingerprint may not point to two semantic versions for the same
  workflow id.
- Presentation revisions must reference an existing workflow version.
- Run snapshots must reject workflow-version or presentation-revision facts
  that do not match the referenced workflow id/version.
- Queued runs must be changed by cancel-and-resubmit, not by mutating a queued
  snapshot.

## Implementation Notes
- Implementation plan:
  `docs/plans/run-centric-gui-workbench/01-workflow-identity-versioning-and-run-snapshots.md`
- Related identity decision:
  `docs/adr/ADR-012-canonical-workflow-run-identity.md`
- Related attribution owner:
  `docs/adr/ADR-005-durable-runtime-attribution.md`
