# 06: Binding Projections And Verification

## Purpose

Project the backend-owned execution-platform surface into native Rust, C#,
Python, and Elixir/BEAM without creating host-local node semantics.

## Binding Projection Types

- `FfiNodeDefinition`
- `FfiPortDefinition`
- `FfiEffectiveNodeContract`
- `FfiPortOptionQuery`
- `FfiDiagnosticsQuery`
- `FfiModelLicenseUsageRecord`
- `FfiUsageSummary`
- `FfiUsageTimeSeriesPoint`
- `WorkflowErrorEnvelope`

These must project canonical backend types and preserve stable ids, validation
outcomes, and diagnostics correlation fields.

## Required Binding Surface

Supported graph-authoring bindings must expose:

- node definition discovery
- node definition lookup by `node_type`
- category/grouped node discovery
- queryable port discovery
- port option queries
- connection candidate lookup
- atomic connect and insert-and-connect operations
- effective contract lookup where dynamic contracts exist
- diagnostics and model/license usage queries for supported execution lanes

C#, Python, and Elixir/BEAM may differ by support tier, but they must not differ
semantically for the same supported operation.

## Host-Lane Direction

- Native Rust is the canonical application API.
- C# projects the native headless contract through the product-native library.
- Python host bindings are distinct from Python-backed workflow nodes and the
  Python sidecar.
- Elixir is the product-facing host-language framing for the BEAM lane; Rustler
  remains the implementation mechanism.
- Generated host bindings are artifacts and must not be hand-edited.

## Affected Structured Contracts And Persisted Artifacts

- FFI DTOs, generated host bindings, product-native shared libraries, host
  package metadata, version manifests, smoke-test fixtures, and host-facing
  documentation for support tiers.

## Standards Compliance Notes

- Language-binding compliance requires the three-layer architecture: Rust core
  library, thin FFI wrapper crate, and generated host bindings. Core crates must
  compile and test without binding frameworks.
- Rust interop and unsafe compliance require raw FFI handling to stay in thin
  wrapper modules, unsafe blocks to carry `SAFETY:` invariants, callbacks to
  document thread and lifetime contracts, and foreign buffers to be copied
  before storage or sharing.
- Cross-platform and release compliance require explicit supported Rust target
  triples, product-native library naming, checksums, SBOM expectations,
  version-matched host packages, and artifact names that use Pantograph product
  identity rather than binding-framework names.
- Dependency compliance requires UniFFI, Rustler, Python, and C# tooling to
  live in the narrowest owner crates or packages, with optional features where
  consumers should not always pay their cost.
- Testing compliance requires native Rust contract tests plus host-language
  smoke or acceptance tests for every supported binding surface.

## Risks And Mitigations

- Risk: bindings become alternate implementations of node semantics.
  Mitigation: expose only backend-owned discovery and diagnostics projections.
- Risk: generated host bindings drift from the native artifact. Mitigation:
  version-match generated bindings and product-native libraries from the same
  build or release.
- Risk: FFI safety rules are scattered across business logic. Mitigation:
  isolate unsafe wrapper modules and keep domain crates FFI-unaware.

## Tasks

- Project discovery and diagnostics DTOs through the native headless facade.
- Add C# host smoke for registry discovery and diagnostics queries.
- Define Python host-binding package and smoke expectations.
- Define Elixir/BEAM Rustler surface and smoke expectations.
- Reconcile support tiers with the binding-platform plan.
- Document unsupported gaps as support-tier limitations.

## Verification

- Supported host lanes use backend discovery, not host-local node catalogs.
- Host-language tests load the real native or NIF artifact where applicable.
- Generated host bindings are version-matched to the native artifact.
- Supported bindings have native-language and host-language verification.
- Diagnostics query projections are preserved across at least one host-language
  boundary.
- FFI DTO serialization shapes are round-tripped and checked against host
  expectations for supported lanes.

## Completion Criteria

- GUI, native Rust, C#, Python, and Elixir/BEAM surfaces consume backend-owned
  discovery and diagnostics projections according to documented support tiers.
- Verification covers native contract logic, runtime-managed observability, and
  at least one host-language binding path.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- A host binding requires semantics not present in the native Rust facade.
- Native artifact naming or packaging conflicts with release standards.
- Host smoke tests cannot load the real generated artifact for a supported
  surface.
