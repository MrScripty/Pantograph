# 02: Node Contracts And Discovery

## Purpose

Define backend-owned node and port contracts before widening runtime execution,
GUI authoring, or binding surfaces.

## Implementation Readiness Status

Ready for stage-start preflight after stage `01` is complete and its
stage-end refactor gate has been recorded.

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
