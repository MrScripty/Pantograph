# 06: Binding Projections And Verification

## Purpose

Project the backend-owned execution-platform surface into native Rust, C#,
Python, and Elixir/BEAM without creating host-local node semantics.

## Implementation Readiness Status

Ready for stage-start preflight after stages `01` through `05` are complete,
their stage-end refactor gates have been recorded, and the native Rust base API
for the projected surface is stable enough to bind. The stage-start report must
reconcile exact host-lane smoke commands with the binding-platform plan before
source edits begin.

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
- C# projects the native headless contract through the product-native library
  after the base Rust API for that surface is resolved.
- Python host bindings are distinct from Python-backed workflow nodes and the
  Python sidecar. Python projects the same base Rust API as C# and BEAM, not a
  Python-specific semantic layer.
- Elixir is the product-facing host-language framing for the BEAM lane; Rustler
  remains the implementation mechanism.
- Generated host bindings are artifacts and must not be hand-edited.
- Binding architecture must make the process for adding a future language
  explicit: define the Rust API first, add a thin projection owner, generate or
  package host bindings, and add language-native tests that load the real
  native artifact.

## Implementation Decisions

### Binding Ownership

- Native Rust application APIs remain in the backend crates that own the
  semantics: runtime attribution, node contracts, embedded runtime,
  diagnostics ledger, and workflow service.
- `crates/pantograph-uniffi` owns non-BEAM FFI projection DTOs and generated
  host-binding artifacts for supported non-BEAM lanes.
- `crates/pantograph-rustler` owns the Elixir/BEAM projection through Rustler.
- Binding crates convert between host-safe DTOs and backend-owned contracts.
  They do not define node catalogs, compatibility rules, diagnostics meaning,
  credential policy, or ledger semantics.

### Support-Tier Decision

- Native Rust is required for every implemented surface.
- Elixir/BEAM is supported through Rustler for product-facing BEAM use cases.
- C# and Python support tiers must match
  `../../plans/pantograph-binding-platform/final-plan.md` at stage start.
- C# and Python must be brought to the same quality bar as the BEAM lane before
  they are marked supported: real generated/native artifact loading,
  language-native tests, native-side projection tests, and at least one
  cross-layer acceptance path for each supported surface.
- If C# or Python lack a real generated-artifact smoke path and
  language-native tests at implementation time, the surface remains documented
  as unsupported or experimental rather than being represented as complete.

### DTO Projection Decision

- FFI DTOs are append-only where possible and preserve stable ids, schema
  version or contract digest fields, typed error envelopes, and diagnostics
  correlation ids.
- Host payloads are validated at the FFI boundary before conversion into
  backend domain types.
- Generated files are build artifacts. Manual edits belong in Rust projection
  definitions, host package templates, or generation configuration, not in
  generated output.

### Verification Decision

- Every supported binding surface needs native wrapper tests and at least one
  host-language smoke or acceptance path that loads the real generated/native
  artifact.
- Wrapper-only tests are insufficient for a supported host lane.
- C# and Python supported lanes require language-native tests, not only Rust
  tests that exercise C#/Python-shaped DTOs.
- Binding test scaffolds should be reusable enough that adding a later language
  follows the same tier definitions, artifact-loading checks, and acceptance
  path pattern.
- Experimental lanes may have narrower smoke coverage, but unsupported gaps
  must be documented rather than inferred.

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

- Reconcile support tiers and exact smoke commands with the binding-platform
  plan in the stage-start report.
- Freeze the native Rust base API for each surface before implementing the
  corresponding C#, Python, or BEAM projection.
- Project discovery and diagnostics DTOs through the native headless facade.
- Add C# language-native host tests for registry discovery and diagnostics
  queries using the real generated/native artifact.
- Define Python host-binding package, language-native tests, and smoke
  expectations using the real generated/native artifact.
- Define Elixir/BEAM Rustler surface and smoke expectations.
- Define reusable binding-tier criteria for future language additions.
- Document unsupported gaps as support-tier limitations.

## Intended Write Set

- Primary:
  - `crates/pantograph-uniffi/`
  - `crates/pantograph-rustler/`
- Adjacent only if required by generated projection integration:
  - backend crates that expose projection DTOs
  - host smoke fixtures or package metadata
  - workspace manifests for binding-specific tooling already approved by the
    binding-platform plan
- Forbidden for this stage unless the plan is updated first:
  - canonical node, runtime, attribution, or ledger semantics
  - hand-edited generated binding artifacts
  - GUI implementation

## Existing Code Impact

- `crates/pantograph-rustler/src/workflow_graph_contract.rs` currently exposes
  graph operations and validation through `node_engine` JSON contracts. Stage
  `06` must project backend-owned node contracts and diagnostics rather than
  continuing host-local graph semantics for supported surfaces.
- `crates/pantograph-rustler/src/frontend_http_nifs.rs` currently exposes
  workflow-session APIs backed by `pantograph-workflow-service`. Stage `06`
  must distinguish internal workflow-session surfaces from durable
  client-session/bucket/run attribution surfaces.
- `crates/pantograph-uniffi/src/frontend_http.rs` and related UniFFI surfaces
  must preserve typed error envelopes and stable ids from the backend-owned
  attribution, contract, runtime, and ledger crates.
- Existing binding tests are mostly Rust-side smoke/contract tests. Supported
  host lanes still need real host-language smoke commands and language-native
  tests recorded at stage start before they can be marked complete.

## Verification Commands

Expected native verification:

```bash
cargo test -p pantograph-uniffi
cargo test -p pantograph-rustler
cargo check --workspace --all-features
```

Expected host verification must be recorded in the stage-start report for each
supported lane before editing that lane. A lane without a real host smoke
command and language-native tests cannot be marked supported by this stage.

Stage completion also requires the Rust baseline verification from
`RUST-TOOLING-STANDARDS.md` unless the stage-start report records an existing
repo-owned equivalent:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Verification

- Supported host lanes use backend discovery, not host-local node catalogs.
- Host-language tests load the real native or NIF artifact where applicable.
- C# and Python host tests exercise the generated/package artifact from their
  native language runtime.
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
  language-native host tests for every supported C#, Python, and BEAM binding
  surface.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- A host binding requires semantics not present in the native Rust facade.
- Native artifact naming or packaging conflicts with release standards.
- Host smoke tests cannot load the real generated artifact for a supported
  surface.
- A C# or Python lane cannot provide language-native tests for a surface that
  would otherwise be marked supported.
