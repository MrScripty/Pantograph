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

## Implementation Progress

### 2026-04-25 Wave 01 Stage-Start Preflight

Stage-start outcome: `ready_with_recorded_assumptions`.

- Stage `01` through Stage `05` implementation outputs are present. The Stage
  `05` stage-end refactor gate required the composition contract module split;
  that split had been implemented and verified locally at stage start, but the
  commit was temporarily blocked because `.git` was mounted read-only while the
  working tree root was writable. The user explicitly directed continuation, so
  Wave `01` proceeded as documentation-only stage-start work while code edits
  remained scoped away from the dirty Stage `05` refactor files until git became
  writable again.
- Pre-existing unrelated dirty asset files remain outside this stage and must
  not be staged, reformatted, or reverted.
- Existing dirty Stage `05` refactor files do not overlap the Wave `01` write
  set. They do block a normal clean stage transition and must be committed
  before any final Stage `06` completion claim.
- Implementation remains sequential in this session. The wave plan defines
  parallel UniFFI/Rustler lanes, but no subagents were authorized. The read-only
  `.git` mount at stage start prevented atomic worker integration commits at
  that point.

Frozen native Rust base API for Stage `06` projection:

- Existing product-native UniFFI surface remains `version()`,
  `validate_workflow_json`, `validate_orchestration_json`, and
  `FfiPantographRuntime` workflow, attribution, persistence, graph edit-session,
  connection candidate, connect, insert-and-connect, and insertion-preview JSON
  methods.
- New binding projection work must remain additive over backend-owned
  contracts and must not define node semantics in wrapper crates. The base
  owners for upcoming projection work are workflow-service `NodeRegistry`,
  effective node-contract resolution, graph connection diagnostics, and
  diagnostics-ledger query projections.
- Required additive projection surface for supported graph authoring is
  registry-backed node definition discovery, lookup by `node_type`,
  category/grouped discovery, queryable port discovery, port-option queries,
  effective contract lookup, and diagnostics/model-license usage query
  projection.
- Generated C# and future Python artifacts are owned by `pantograph-uniffi`,
  packaging scripts, and `bindings/` harness/package directories. Generated
  artifacts must remain build artifacts under `target/` or package output and
  must not be hand-edited.
- The Elixir/BEAM lane is owned by `pantograph-rustler` plus
  `bindings/beam/pantograph_native_smoke`; Rustler code may project backend
  facts but must not become a separate node catalog or diagnostics owner.

Support-tier reconciliation for this stage start:

- Native Rust: required and supported for every implemented surface.
- C# over `pantograph_headless`/UniFFI: candidate `supported` for the current
  direct runtime workflow/session/persistence/graph-edit JSON surface because
  the repo has generated-artifact packaging, dotnet-backed smoke coverage, and
  packaged quickstart checks. Full graph-authoring support remains incomplete
  until registry discovery, queryable-port, and port-option projections are
  exposed through the binding surface.
- Python host bindings: `unsupported` for this stage start. `python3` exists on
  this machine, but there is no `bindings/python/` package, generated artifact,
  or language-native import/load smoke command yet. Python work must first add
  a documented host-binding package and real generated/native artifact smoke
  path before any supported or experimental completion claim.
- Elixir/BEAM over Rustler: `experimental` for this stage start. A real BEAM
  smoke runner exists at `./scripts/check-rustler-beam-smoke.sh`, but the
  current machine does not have `mix` on `PATH`, and the current Rustler README
  `Supported` row is broader than the host-language coverage. Stage `06` must
  either downgrade/narrow that row or expand the BEAM harness before any BEAM
  surface remains supported.

Exact verification commands recorded for host lanes:

```bash
cargo test -p pantograph-uniffi
cargo test -p pantograph-rustler
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
./scripts/check-rustler-beam-smoke.sh
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

Host command availability observed during preflight:

- `.NET`: available at `/usr/bin/dotnet`, version `10.0.103`.
- `mix`/Elixir: not available on `PATH`; BEAM smoke is a required command only
  where the BEAM toolchain is installed.
- `python3`: available at `/usr/bin/python3`, version `3.12.3`; no Python
  binding package or generated-artifact smoke command exists yet.

### 2026-04-25 Wave 02 UniFFI Discovery Projection Progress

- Added additive `FfiPantographRuntime` JSON methods for backend-owned graph
  authoring discovery:
  - `workflow_graph_list_node_definitions`
  - `workflow_graph_get_node_definition`
  - `workflow_graph_get_node_definitions_by_category`
  - `workflow_graph_get_queryable_ports`
  - `workflow_graph_query_port_options`
- The implementation projects `pantograph-workflow-service` `NodeRegistry`
  definitions and `node-engine` queryable port providers. It does not define
  wrapper-local node metadata, compatibility policy, diagnostics meaning, or
  generated artifacts.
- The direct runtime now retains the same `ExecutorExtensions` handle used by
  embedded execution so option-provider queries can receive backend extension
  facts such as an optional Pumas API.
- Restored the direct workflow execution-session methods expected by the
  generated C# smoke and metadata gate:
  - `workflow_create_session`
  - `workflow_run_session`
  - `workflow_close_session`
  - `workflow_get_session_status`
  - `workflow_list_session_queue`
  - `workflow_cancel_session_queue_item`
  - `workflow_reprioritize_session_queue_item`
  - `workflow_set_session_keep_alive`
- Added UniFFI runtime coverage for node definition discovery, grouped
  discovery, queryable port exposure, execution-session create/run/status/
  queue/keep-alive/close, unknown-node rejection envelopes, and
  non-queryable-port rejection envelopes.
- Extended the C# native smoke to assert generated binding access to
  backend-owned node definitions, grouped definitions, and queryable ports.
- Updated the UniFFI metadata gate to require the new graph-authoring discovery
  methods.

Verification passed:

```bash
cargo test -p pantograph-uniffi direct_runtime_exposes_backend_owned_graph_authoring_discovery
cargo test -p pantograph-uniffi direct_runtime_runs_workflow_from_json
cargo test -p pantograph-uniffi
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
```

### 2026-04-25 Wave 02 Rustler Discovery Projection Progress

- Added additive Rustler NIFs for backend-owned graph authoring discovery:
  - `node_registry_list_definitions`
  - `node_registry_get_definition`
  - `node_registry_definitions_by_category`
  - `node_registry_queryable_ports`
- `node_registry_list_definitions`,
  `node_registry_get_definition`, and
  `node_registry_definitions_by_category` project
  `pantograph-workflow-service` `NodeRegistry` definitions, not Rustler-local
  task metadata.
- `node_registry_queryable_ports` projects backend `node-engine` queryable port
  providers after built-in registration.
- Extended the BEAM smoke harness stubs and tests for node definitions, grouped
  definitions, and queryable ports.
- Updated BEAM support-tier documentation so the broad Rustler surface is
  `Experimental` until host smoke coverage justifies supported status.

Verification passed:

```bash
cargo check -p pantograph_rustler
```

Verification not completed in this environment:

```bash
cargo test -p pantograph_rustler
./scripts/check-rustler-beam-smoke.sh
```

`cargo test -p pantograph_rustler` still fails at link time because Erlang
`enif_*` symbols are supplied by the BEAM host runtime. The BEAM smoke runner
fails immediately because `mix` is not installed on this machine.

### 2026-04-25 Wave 03 Host-Language Verification Progress

- C# host verification is complete and committed. The native smoke loads the
  real generated/native artifact and now asserts generated access to backend-owned
  graph-authoring discovery. The package and packaged quickstart scripts pass
  without hand-editing generated artifacts.
- Python remains `unsupported`. The host has `python3`, but the repository has
  no `bindings/python/` package, generated Python artifact, or import/load
  smoke command.
- BEAM remains `experimental`. The smoke fixture source now covers the new
  Rustler graph-authoring discovery NIFs, but the host smoke cannot run here
  because `mix` is not installed.
- Workspace verification passed after Wave `02` and Wave `03` changes:

```bash
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

The workspace check emitted existing `pumas-library` dead-code warnings outside
this repo's current change set. Doctests emitted Cargo's expected note that doc
tests are not supported for the Rustler `cdylib` crate type.

### 2026-04-25 Wave 04 Binding Integration And Gate

- Added `docs/adr/ADR-010-binding-projection-ownership-and-support-tiers.md`
  to freeze binding projection ownership, generated artifact policy, and
  evidence-based support tiers.
- Updated `docs/headless-native-bindings.md` so graph-authoring discovery is no
  longer documented as a current gap after the Stage `06` projections.
- Confirmed support tiers match real host evidence:
  - Native Rust: supported for implemented execution-platform surfaces.
  - C#: supported for generated/native surfaces covered by smoke and packaged
    quickstart verification.
  - Python: unsupported until a real generated/native host package and
    import/load smoke command exist.
  - Elixir/BEAM: experimental on this host because `mix` is unavailable.
- Recorded the stage-end refactor gate at
  `implementation-waves/06-binding-projections-and-verification/reports/stage-end-refactor-gate.md`
  with outcome `not_warranted`.
- Reran artifact verification after updating packaged headless binding docs:

```bash
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
cargo fmt --all -- --check
```

The packaging script emitted the existing CSharpier availability warning while
still producing the C# binding, native library, and checksum artifacts.

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
