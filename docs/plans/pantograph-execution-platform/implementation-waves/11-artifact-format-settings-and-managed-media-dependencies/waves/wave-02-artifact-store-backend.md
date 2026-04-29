# Wave 02: ArtifactStore Backend

## Objective

Implement backend ArtifactStore ownership for physical payload bodies,
descriptors, memory/disk policy, lifecycle, consume acknowledgement, cleanup,
and binary-safe read/stream handles.

## Dependencies

Wave `01` contracts must be frozen and committed.

## Workers

Parallel workers may be assigned only after the host records exact non-
overlapping files in the coordination ledger.

## Candidate Write Sets

- ArtifactStore storage and policy owner under a backend crate/module selected
  by Wave `01`.
- Workflow-service API/adapters for descriptor lookup, policy query/update,
  read handle creation, stream metadata, and consume acknowledgement.
- Focused tests for restart/recovery, retention, storage-tier opacity, and
  binary-safe contracts.

## Forbidden Files

- Managed redistributable implementation files owned by Wave `03`.
- Frontend and binding files owned by Wave `05`.
- `.pantograph/**`, `assets/**`, generated output, and unrelated manifests.

## Standards

Rust API, security, async/concurrency, testing, and documentation standards.

## Verification

Defined by implementation owner before launch; must include targeted
ArtifactStore tests and `cargo fmt --all -- --check`.

## Report Path

`reports/wave-02-worker-<name>.md`

## Escalation Rules

Escalate if implementation needs inline JSON bodies, client-visible raw paths,
unbounded caches, or broader storage-engine migration.

## Integration Order

ArtifactStore core before workflow-service API projection tests.

