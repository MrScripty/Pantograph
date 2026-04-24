# 02: Node Contracts And Discovery

## Purpose

Define backend-owned node and port contracts before widening runtime execution,
GUI authoring, or binding surfaces.

## Implementation Readiness Status

Ready for stage-start preflight after stage `01` is complete and its
stage-end refactor gate has been recorded.

## Implementation Notes

### 2026-04-24 Stage-Start Report

- Selected stage: Stage `02`, node contracts and discovery.
- Current branch: `main`.
- Stage base: `4ba76c98`, the Stage `01` closeout commit.
- Git status before implementation: unrelated asset changes only:
  deleted `assets/3c842e69-080c-43ad-a9f0-14136e18761f.jpg`, deleted
  `assets/grok-image-6c435c73-11b8-4dcf-a8b2-f2735cc0c5d3.png`, deleted
  `assets/grok-image-e5979483-32c2-4cf5-b32f-53be66170132.png`,
  untracked `assets/banner_3.jpg`, `assets/banner_3.png`,
  `assets/github_social.jpg`, and `assets/reject/`.
- Dirty-file overlap: none. Stage `02` implementation must not touch
  `assets/`.
- Standards reviewed through the execution-platform standards map:
  `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `CODING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
  `TESTING-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `TOOLING-STANDARDS.md`, `INTEROP-STANDARDS.md`,
  `LANGUAGE-BINDINGS-STANDARDS.md`, `SECURITY-STANDARDS.md`,
  `DEPENDENCY-STANDARDS.md`, `COMMIT-STANDARDS.md`, and
  `languages/rust/RUST-*.md`.
- Intended Wave `02` write set:
  `crates/pantograph-node-contracts/`, `crates/workflow-nodes/`,
  `crates/pantograph-workflow-service/src/graph/`, and host-owned workspace
  manifests/facades only when needed to add and expose the canonical contract
  crate.
- Adjacent inventory:
  - `node-engine/src/types.rs` currently owns `PortDataType`,
    `PortDefinition`, `NodeDefinition`, and compatibility helpers.
  - `node-engine/src/descriptor.rs` currently describes executable task
    metadata and still claims task metadata is the UI/validation source of
    truth; Stage `02` must downgrade this to execution descriptor input.
  - `node-engine/src/registry.rs` currently owns metadata lookup, category
    grouping, and port option providers for executor registrations.
  - `pantograph-workflow-service/src/graph/types.rs` duplicates node/port DTOs
    and local compatibility rules for graph-authoring projections.
  - `pantograph-workflow-service/src/graph/registry.rs` converts
    `node_engine::TaskMetadata` into workflow-service definitions and maps
    engine-only port types to GUI-facing types.
  - `pantograph-workflow-service/src/graph/effective_definition.rs` currently
    accepts dynamic definitions from `GraphNode.data["definition"]`; Stage
    `02` must replace that host-local shape reconstruction with backend-owned
    effective contracts and typed resolution diagnostics.
  - `pantograph-rustler/src/workflow_graph_contract.rs` validates graphs
    through `node_engine` directly and remains a later projection touchpoint.
- Contract freeze:
  - `NodeTypeId`, `NodeInstanceId`, and `PortId` are non-empty, trimmed,
    validated string newtypes with generated constructors only where the
    backend owns the id.
  - Canonical port values are expressed through `PortValueType`,
    `PortKind`, `PortCardinality`, `PortRequirement`, `PortVisibility`, and
    explicit `PortConstraint` values.
  - `NodeTypeContract` owns stable type id, category, label, description,
    inputs, outputs, execution semantics, capability requirements, authoring
    metadata, and optional contract version/digest.
  - `EffectiveNodeContract` and `EffectivePortContract` are backend-published
    projections for one node instance and carry
    `ContractResolutionDiagnostics`; clients must not rebuild dynamic ports
    from arbitrary node data.
  - Compatibility returns structured diagnostics with source/target node and
    port ids plus a typed rejection reason, not a bare boolean.
- Start outcome: `ready_with_recorded_assumptions`.
- Recorded assumptions:
  - Wave `02` may be executed serially by the host in this shared workspace
    when subagents are not explicitly authorized; the recorded worker write
    sets and reports still apply.
  - The first logical implementation step is the `canonical-contracts` slice:
    add `pantograph-node-contracts`, validated ids, canonical DTOs,
    compatibility diagnostics, tests, README, workspace wiring, and targeted
    verification before integrating workflow-service projections.
  - No new third-party dependency is expected for the first slice; if a
    dependency becomes necessary, stop and record dependency-standard review
    before editing manifests.
- Expected verification for the first logical step:
  `cargo test -p pantograph-node-contracts`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-node-contracts --all-targets -- -D warnings`.
- Expected Stage `02` verification remains the command set listed in
  `Verification Commands`.

### 2026-04-24 Wave 02 Canonical Contracts Progress

- Added `crates/pantograph-node-contracts` as the backend-owned canonical node
  contract crate.
- Implemented validated `NodeTypeId`, `NodeInstanceId`, and `PortId` newtypes
  with backend-owned generated constructors and boundary parsing.
- Implemented canonical node and port DTOs:
  `NodeTypeContract`, `PortContract`, `PortKind`, `PortCardinality`,
  `PortRequirement`, `PortVisibility`, `PortValueType`, `PortConstraint`,
  `EditorHint`, `NodeExecutionSemantics`, `NodeCapabilityRequirement`, and
  `NodeAuthoringMetadata`.
- Implemented effective-contract DTOs:
  `NodeInstanceContext`, `EffectiveNodeContract`, `EffectivePortContract`,
  `ContractExpansionReason`, and `ContractResolutionDiagnostics`.
- Implemented structured compatibility checks that return a
  `CompatibilityResult` with a typed `ConnectionRejectionDiagnostic` instead
  of a bare boolean.
- Added README coverage for the new crate boundary and tests for id parsing,
  generated ids, compatibility rules, structured rejections, port direction
  validation, effective static contracts, and JSON shape.
- Verification: `cargo fmt --all -- --check`,
  `cargo test -p pantograph-node-contracts`,
  `cargo clippy -p pantograph-node-contracts --all-targets -- -D warnings`,
  and `cargo check --workspace --all-features` passed.
- Remaining Wave `02` work: convert concrete workflow-node registrations into
  canonical contracts and route workflow-service graph projections through the
  new crate.

### 2026-04-24 Wave 02 Workflow-Nodes Registration Progress

- Added a direct `workflow-nodes` dependency on `pantograph-node-contracts`.
- Added `workflow_nodes::builtin_node_contracts` and
  `workflow_nodes::task_metadata_to_contract` to project concrete built-in
  `node_engine::TaskMetadata` descriptors into canonical
  `NodeTypeContract` records.
- Preserved concrete descriptor facts while moving semantic projection into the
  canonical contract model: port directions, requirements, cardinality, value
  types, execution semantics, category tags, and selected capability
  requirements.
- Preserved engine-only value types such as model handles and tensors as
  canonical `PortValueType` variants rather than downcasting them to generic
  GUI strings.
- Added workflow-nodes tests proving all built-in descriptors have valid
  canonical contracts, common port directions/value types are preserved,
  extended engine value types survive projection, and invalid descriptor ids
  are rejected by canonical id validation.
- Verification: `cargo fmt --all -- --check`,
  `cargo test -p workflow-nodes`,
  `cargo clippy -p workflow-nodes --all-targets -- -D warnings`, and
  `cargo check --workspace --all-features` passed.
- Remaining Wave `02` work: route workflow-service graph definitions,
  effective contracts, connection candidates, and compatibility rejections
  through `pantograph-node-contracts`.

### 2026-04-24 Wave 02 Workflow-Service Projection Progress

- Added a direct `pantograph-workflow-service` dependency on
  `pantograph-node-contracts`.
- Routed built-in workflow graph definitions through
  `workflow_nodes::builtin_node_contracts` so workflow-service consumes
  canonical `NodeTypeContract` records instead of converting
  `node_engine::TaskMetadata` directly.
- Preserved canonical value type facts in workflow-service projections by
  adding explicit GUI-facing variants for model handles, embedding handles,
  database handles, vectors, tensors, and audio samples.
- Replaced workflow-service local port compatibility rules with canonical
  `PortValueType` compatibility checks from `pantograph-node-contracts`.
- Verification: `cargo test -p pantograph-workflow-service`,
  `cargo check --workspace --all-features`, `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
  passed.
- Remaining Wave `02` work: replace dynamic
  `GraphNode.data["definition"]` reconstruction and connection rejection
  surfaces with backend-owned effective contracts and typed diagnostics.

### 2026-04-24 Wave 02 Effective Contract Resolution Progress

- Added canonical effective-contract merge behavior to
  `pantograph-node-contracts` so dynamic ports are resolved into
  `EffectiveNodeContract` values without dropping unrelated static ports.
- Extended workflow-service `NodeRegistry` to retain canonical
  `NodeTypeContract` records alongside existing DTO projections.
- Replaced direct `GraphNode.data["definition"]` reconstruction in
  workflow-service with an `effective_node_contract` resolver that validates
  node ids, converts dynamic port overlays into canonical `PortContract`
  records, records typed resolution diagnostics, and then projects the
  effective contract to existing graph DTOs for current callers.
- Preserved existing graph-edit DTO compatibility while moving dynamic shape
  semantics behind the canonical effective-contract API.
- Verification: `cargo test -p pantograph-node-contracts`,
  `cargo test -p pantograph-workflow-service`,
  `cargo check --workspace --all-features`, `cargo fmt --all -- --check`,
  `cargo clippy -p pantograph-node-contracts --all-targets -- -D warnings`,
  and `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
  passed.
- Remaining Wave `02` work: project structured compatibility diagnostics into
  connection candidate and rejection response surfaces.

### 2026-04-24 Wave 02 Compatibility Diagnostic Projection Progress

- Added a workflow-service `PortDefinition` to `PortContract` projection helper
  so graph validation can build canonical compatibility checks without
  reimplementing port semantics.
- Added `check_connection_ports` to route direct connection validation through
  canonical `CompatibilityCheck` and `check_compatibility` diagnostics.
- Extended `ConnectionRejection` with an optional `contract_diagnostic` field
  carrying the canonical source/target node ids, port ids, value types,
  rejection reason, and diagnostic message for incompatible direct connections.
- Added regression coverage proving an image output rejected against a text
  input includes the canonical typed rejection diagnostic.
- Verification: `cargo test -p pantograph-workflow-service`,
  `cargo check --workspace --all-features`, `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-workflow-service --all-targets -- -D warnings`
  passed.
- Remaining Stage `02` follow-up: decide whether aggregate "no compatible
  insert path" and candidate filtering responses should expose a collection of
  suppressed diagnostics or remain coarse graph-authoring summaries.

### 2026-04-24 Wave 02 Binding Validation Projection Progress

- Added workflow-service `validate_workflow_graph_contract` to validate
  workflow graph JSON against backend-owned node contracts, effective
  definitions, canonical compatibility checks, target capacity, duplicate ids,
  missing nodes/ports, and cycles.
- Added `convert_graph_from_node_engine` so existing binding JSON surfaces can
  reuse workflow-service graph contract validation without retaining
  node-engine as the binding validation policy owner.
- Routed Rustler and UniFFI workflow JSON validation entry points through
  workflow-service contract validation and `NodeRegistry` instead of direct
  `node_engine::validation::validate_workflow` calls.
- Verification: `cargo test -p pantograph-workflow-service graph::contract_validation`,
  `cargo test -p pantograph-uniffi test_validate_empty_workflow`,
  `cargo check -p pantograph_rustler -p pantograph-uniffi`,
  `cargo check --workspace --all-features`, `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-workflow-service -p pantograph-uniffi -p pantograph_rustler --all-targets -- -D warnings`
  passed.
- Verification limitation: `cargo test -p pantograph_rustler
  test_validation_empty_graph` still fails during test binary linking on
  missing Erlang NIF symbols such as `enif_release_resource`, which appears to
  be the existing Rustler test-link environment constraint rather than a Rust
  type-checking failure.

### 2026-04-24 Wave 03 Documentation And ADR Progress

- Updated `node-engine` documentation to frame task metadata and descriptors as
  execution inputs rather than canonical GUI or binding node contracts.
- Added `docs/adr/ADR-006-canonical-node-contract-ownership.md` to record
  canonical node contract ownership in `pantograph-node-contracts` and
  projection responsibilities for workflow-service, node-engine, bindings, and
  GUI adapters.
- Updated the ADR index with the Stage `02` node-contract ownership decision.

### 2026-04-24 Stage-End Verification And Gate

- Final Stage `02` verification passed:
  `cargo test -p pantograph-node-contracts`,
  `cargo test -p workflow-nodes`,
  `cargo test -p pantograph-workflow-service`,
  `cargo check --workspace --all-features`,
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
  `cargo test --workspace --doc`.
- Stage-end refactor gate outcome: `not_warranted`.
- Touched-file source command: `git diff --name-only 4ba76c98...HEAD`.
- Gate decision: touched files already received in-scope ownership alignment,
  DTO projection cleanup, binding validation routing, README updates, ADR-006,
  and targeted regression tests during implementation; remaining aggregate
  candidate diagnostic collection and Rustler test-link behavior are recorded
  follow-ups rather than standards drift requiring an in-stage refactor.
- Dirty-file overlap at closeout: none. Existing unrelated `assets/` deletes
  and untracked assets remain outside the Stage `02` write set.

## Type Families To Define

### Identity Types

- `NodeTypeId`
- `NodeInstanceId`
- `PortId`

These should be validated domain types where bug cost justifies it, not raw
strings flowing through core logic.

### Contract Types

- `NodeTypeContract`
- `PortContract`
- `PortKind`
- `PortCardinality`
- `PortRequirement`
- `PortVisibility`
- `PortValueType`
- `PortConstraint`
- `EditorHint`
- `NodeExecutionSemantics`
- `NodeCapabilityRequirement`
- `NodeAuthoringMetadata`

### Effective Contract Types

- `NodeInstanceContext`
- `EffectiveNodeContract`
- `EffectivePortContract`
- `ContractExpansionReason`
- `ContractResolutionDiagnostics`

## Required Behavior

- Backend-owned node and port contracts are the source of truth.
- Frontend and bindings render/edit projected contracts instead of inventing
  shape or semantics.
- Dynamic node shape is acceptable only when resolved and published by the
  backend as an effective contract.
- Port ids must be stable and semantic, not positional-only.
- Compatibility decisions must use explicit type semantics and documented
  coercion rules.

## Implementation Decisions

### Canonical Ownership

- `crates/pantograph-node-contracts` owns canonical node, port, effective
  contract, compatibility, and discovery semantics.
- `crates/pantograph-runtime-registry` currently owns runtime lifecycle,
  observation, reservation, admission, reclaim, warmup, and technical-fit
  policy. Stage `02` must not broaden that crate into node contract ownership
  because its existing README and API contract are runtime-policy-specific.
- `crates/workflow-nodes` may define concrete node registrations, but those
  registrations must be converted into canonical registry contracts before they
  are visible to GUI, workflow service, runtime execution, or bindings.
- `crates/pantograph-workflow-service` consumes discovery and compatibility
  decisions from `pantograph-node-contracts`. It must not duplicate port
  compatibility rules.
- `crates/pantograph-frontend-http-adapter`, GUI clients, and binding crates
  expose projections only.

### Contract Shape Decision

- `NodeTypeId`, `NodeInstanceId`, and `PortId` are validated newtypes in the
  `pantograph-node-contracts` API.
- Port identity is stable and semantic. Positional indexes may appear only as
  display/order metadata and cannot be used as the durable identity.
- Contract DTOs use explicit enums for kind, cardinality, requirement,
  visibility, execution semantics, and rejection reason.
- Dynamic shape is represented only through `EffectiveNodeContract` plus
  `ContractResolutionDiagnostics`; clients never reconstruct dynamic ports from
  host-local rules.

### Discovery API Decision

- `pantograph-node-contracts` exposes synchronous domain operations for
  contract lookup, category grouping, compatibility checks, connection
  candidates, effective contract resolution, queryable port metadata, and
  option lookup.
- Async behavior belongs in outer adapters if discovery is served over HTTP,
  IPC, or bindings.
- Discovery responses include stable ids and enough diagnostic identifiers for
  GUI and host-language consumers to explain rejection without reimplementing
  compatibility logic.

### Persistence And Compatibility Decision

- Saved workflow graphs persist node type ids, node instance ids, port ids, and
  contract version or digest where available.
- The first registry implementation must reject unknown ids and incompatible
  connections with typed diagnostics instead of silently coercing them.
- Saved-workflow upgrade or regeneration rules are deferred to stage `05`, but
  stage `02` must avoid introducing contract shapes that make clean upgrade or
  typed rejection impossible.

## Discovery Surface

Supported graph-authoring clients must be able to discover:

- all node definitions
- node definitions by category
- one node definition by stable `node_type`
- queryable ports
- options for queryable ports
- effective contract for a node instance
- connection candidates
- structured connection rejection reasons

## Affected Structured Contracts And Persisted Artifacts

- Canonical node contracts, port contracts, effective contracts, compatibility
  decisions, and discovery projections.
- Saved workflow graphs and any serialized graph-edit operations that store
  node type ids, port ids, or contract versions.
- Binding and GUI DTOs generated from canonical backend contracts.

## Standards Compliance Notes

- Rust API compliance requires validated ids, explicit enums for kind,
  cardinality, visibility, requirement, execution semantics, and rejection
  reasons, plus typed contract-resolution errors.
- Architecture compliance requires contract rules to live below GUI and binding
  adapters. Frontend and host-language code may cache projections but cannot
  define compatibility rules or dynamic shape semantics.
- Documentation compliance requires public contract docs for discovery DTOs,
  effective contract semantics, compatibility rules, and rejected connection
  reasons.
- Testing compliance requires native contract tests, schema or serialization
  round trips, property tests for graph/port compatibility, and host projection
  tests proving no host-local semantics.
- Frontend/accessibility compliance requires GUI palettes, inspectors, and
  port editors to render backend facts with semantic controls and keyboard
  reachable graph-authoring actions.

## Risks And Mitigations

- Risk: dynamic node shape leaks into host-local reconstruction. Mitigation:
  require backend-published effective contracts and contract-resolution
  diagnostics.
- Risk: compatibility rules become stringly typed. Mitigation: model port kinds,
  constraints, and rejection reasons as typed contracts before projection.
- Risk: discovery APIs become too broad. Mitigation: classify internal-only,
  experimental, and supported surfaces before binding exposure.

## Tasks

- Add `crates/pantograph-node-contracts` with README coverage required for a
  new source crate.
- Implement canonical node contracts in `pantograph-node-contracts`.
- Define the initial Rust type names and module layout under the
  `pantograph-node-contracts` public API.
- Extract or replace the current GUI-facing contract DTOs and compatibility
  logic in `pantograph-workflow-service/src/graph/types.rs`,
  `pantograph-workflow-service/src/graph/registry.rs`, and
  `pantograph-workflow-service/src/graph/effective_definition.rs`.
- Convert `node-engine::TaskMetadata` and `workflow-nodes` descriptors into
  canonical contracts without making `node-engine` the source of GUI/binding
  semantics.
- Expose node definitions from the canonical registry.
- Expose grouped/category node definitions.
- Expose queryable port metadata.
- Expose port option queries.
- Expose effective node contract lookup.
- Expose contract-resolution diagnostics for dynamic shape changes.
- Document which DTOs are projections, not canonical definitions.

## Intended Write Set

- Primary:
  - `crates/pantograph-node-contracts/`
  - `crates/workflow-nodes/`
- Adjacent only if required by existing call sites:
  - `crates/pantograph-workflow-service/`
  - `crates/pantograph-frontend-http-adapter/`
  - `crates/node-engine/`
- Forbidden for this stage unless the plan is updated first:
  - durable model/license ledger implementation
  - host binding generation
  - GUI-local node catalogs

## Existing Code Impact

- `crates/node-engine/src/types.rs`, `crates/node-engine/src/descriptor.rs`,
  and `crates/node-engine/src/registry.rs` currently define task metadata,
  executor registration, port data types, and compatibility helpers. Stage `02`
  must treat these as execution/descriptor inputs and migrate canonical
  contract semantics into `pantograph-node-contracts`.
- `crates/pantograph-workflow-service/src/graph/types.rs` currently defines
  GUI-facing node, port, connection, rejection, and graph DTOs with raw string
  ids. Stage `02` must either replace these with projections from
  `pantograph-node-contracts` or make them thin DTO wrappers over canonical
  types.
- `crates/pantograph-workflow-service/src/graph/registry.rs` currently converts
  `node_engine::TaskMetadata` into workflow-service node definitions and owns
  compatibility conversion. Stage `02` must move that policy into
  `pantograph-node-contracts`.
- `crates/pantograph-workflow-service/src/graph/effective_definition.rs`
  currently reads dynamic node definitions from `GraphNode.data["definition"]`.
  Stage `02` must replace this with backend-published effective contracts and
  typed contract-resolution diagnostics.
- `crates/pantograph-workflow-service/src/graph/session_connection_api.rs` and
  related graph mutation modules currently consume local compatibility
  decisions. Stage `02` must route connection candidates and rejections through
  canonical contract APIs.
- `crates/pantograph-rustler/src/workflow_graph_contract.rs` currently
  validates graphs through `node_engine` directly. Binding projection work must
  eventually consume backend-owned contract projections instead of direct
  node-engine semantics.

## Verification Commands

Expected stage verification:

```bash
cargo test -p pantograph-node-contracts
cargo test -p workflow-nodes
cargo check --workspace --all-features
```

If workflow-service integration is touched, also run:

```bash
cargo test -p pantograph-workflow-service
```

Stage completion also requires the Rust baseline verification from
`RUST-TOOLING-STANDARDS.md` unless the stage-start report records an existing
repo-owned equivalent:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Verification

- Native Rust tests validate contract parsing and compatibility without GUI or
  host-language bindings.
- GUI and headless consumers can build palettes and inspectors without local
  node catalogs.
- Compatibility rejection tests report node, port, and rule identifiers.
- Binding DTOs have no independent semantics not traceable to canonical types.
- Saved graph fixtures round-trip through canonical contract parsing and reject
  invalid ids or incompatible ports with typed diagnostics.

## Completion Criteria

- Canonical node, port, effective contract, and discovery type families are
  defined.
- Graph authoring can be backend-driven without host-maintained node catalogs.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- Existing saved workflows cannot be represented with stable node and port ids.
- Effective contract resolution requires GUI-only information.
- Binding support tiers need different semantics for the same supported
  operation.
