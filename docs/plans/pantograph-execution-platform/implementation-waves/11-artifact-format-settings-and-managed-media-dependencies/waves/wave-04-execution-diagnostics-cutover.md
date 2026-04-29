# Wave 04: Execution Diagnostics Cutover

## Objective

Move workflow execution outputs, streaming media, run snapshots, and diagnostic
metadata to ArtifactStore descriptors and typed format metadata.

## Dependencies

Waves `01`, `02`, and required Wave `03` capability contracts must be committed.

## Workers

Parallel workers may be assigned by producer family only after the host records
exact non-overlapping files in the coordination ledger.

## Candidate Write Sets

- Workflow/node execution output conversion before value-size validation.
- Python bridge and worker adapters for media descriptor conversion.
- Diagnostics ledger/event/projection metadata for artifact lifecycle and
  actual format/converter versions.
- Tests for diffusion previews, audio/video stream metadata, oversized JSON
  rejection, and retained metadata after body deletion.

## Forbidden Files

- Frontend Settings/output selectors owned by Wave `05`.
- Managed redistributable catalog internals owned by Wave `03`.
- `.pantograph/**`, `assets/**`, generated output, and unrelated manifests.

## Standards

Architecture, security, Rust API, Rust async, interop, testing, and
documentation standards.

## Verification

Defined by implementation owner before launch; must include media descriptor
cutover tests and no-inline-JSON assertions.

## Report Path

`reports/wave-04-worker-<name>.md`

## Escalation Rules

Escalate if any migration path requires raising JSON payload caps or logging
chunk bodies into diagnostic events.

## Integration Order

Execution conversion before diagnostics projection expansion, then producer
family migrations.

