# 02: Node Contracts And Discovery

## Purpose

Define backend-owned node and port contracts before widening runtime execution,
GUI authoring, or binding surfaces.

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

- Decide which crate owns canonical node contracts.
- Define the initial Rust type names and module layout.
- Expose node definitions from the canonical registry.
- Expose grouped/category node definitions.
- Expose queryable port metadata.
- Expose port option queries.
- Expose effective node contract lookup.
- Expose contract-resolution diagnostics for dynamic shape changes.
- Document which DTOs are projections, not canonical definitions.

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
