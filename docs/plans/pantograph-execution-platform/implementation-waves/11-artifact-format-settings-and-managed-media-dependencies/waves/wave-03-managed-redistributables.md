# Wave 03: Managed Redistributables

## Objective

Generalize or split the existing managed-runtime boundary so runtime sidecars,
tool binaries, and native library/artifact dependencies are distinct product
categories with shared validated install/activation infrastructure.

## Dependencies

Wave `01` managed redistributable DTOs must be frozen and committed.

## Workers

Parallel workers may be assigned only after the host records exact non-
overlapping files in the coordination ledger.

## Candidate Write Sets

- Generalized redistributable domain/contracts and tests.
- OCIO managed dependency catalog/status/activation and safe native boundary
  scaffold.
- ffmpeg, `ocioconvert`, and `oiiotool` managed tool binary catalog/status and
  readiness tests.

## Forbidden Files

- ArtifactStore physical payload storage files owned by Wave `02` unless the
  host explicitly changes integration order.
- Frontend and binding files owned by Wave `05`.
- `.pantograph/**`, `assets/**`, generated output, and unrelated manifests.

## Standards

Dependency, security, cross-platform, Rust unsafe, interop, async/concurrency,
and testing standards.

## Verification

Defined by implementation owner before launch; must include managed catalog,
expected-file, checksum/signature, readiness, and activation tests.

## Report Path

`reports/wave-03-worker-<name>.md`

## Escalation Rules

Escalate if code depends on unmanaged host PATH/system-library discovery,
requires unsafe outside an adapter boundary, or needs new third-party crates
without dependency review.

## Integration Order

Redistributable category/domain contracts before OCIO/tool-specific catalogs.

