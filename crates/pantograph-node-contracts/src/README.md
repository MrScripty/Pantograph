# pantograph-node-contracts

## Purpose

`pantograph-node-contracts` owns Pantograph's canonical node and port
contracts. It exists so workflow-service, runtime, GUI, and binding surfaces
consume backend-owned discovery facts instead of duplicating node shape or
compatibility rules.

## Contents

| File | Description |
| ---- | ----------- |
| `lib.rs` | Public canonical contract, effective contract, compatibility, and error API. |

## Problem

Node metadata and graph-authoring compatibility rules previously lived across
`node-engine`, workflow-service graph DTOs, and adapter validation. That makes
it easy for GUI or binding surfaces to reconstruct dynamic shape locally and
produce compatibility answers that drift from backend execution semantics.

## Constraints

- `node-engine` can continue to describe executable task metadata, but it is
  not the canonical GUI or binding contract owner.
- Workflow-service may project contracts for graph authoring, but it must not
  duplicate compatibility policy.
- Dynamic node shape must be published by backend effective contracts and
  diagnostics, not rebuilt from arbitrary host-local node data.
- This crate must stay transport-neutral and must not depend on GUI, Tauri,
  UniFFI, Rustler, or workflow-service.

## Decision

Keep canonical node-contract identity, DTOs, effective-contract projections,
and compatibility diagnostics in this crate. Other crates convert their local
execution descriptors into these contracts before exposing node definitions,
connection candidates, or graph-authoring diagnostics.

## Alternatives Rejected

- Keep workflow-service graph DTOs as the semantic source of truth. Rejected
  because workflow-service is an orchestration boundary, not the canonical
  node-contract owner.
- Keep `node-engine::TaskMetadata` as the GUI/binding contract. Rejected
  because task descriptors describe executable nodes and should not own all
  graph-authoring projection semantics.

## Invariants

- `NodeTypeId`, `NodeInstanceId`, and `PortId` are validated before entering
  canonical contracts.
- Compatibility results carry structured source/target ids and typed rejection
  reasons.
- Effective contracts include resolution diagnostics so callers can explain
  why a node shape differs from its static type contract.
- Host adapters project contracts; they do not define compatibility rules.

## Revisit Triggers

- Saved workflow migration in Stage `05` requires contract digests to become
  persisted compatibility gates.
- Binding projection in Stage `06` requires additional serialization metadata
  or support-tier annotations.
- Runtime-managed observability in Stage `03` requires new capability
  requirement fields on node contracts.

## Dependencies

**Internal:** None.

**External:** `serde`, `serde_json`, `thiserror`, and `uuid` from the workspace.

## Related ADRs

- `../../../docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `Reason: This crate sits in the backend service/domain layer consumed by
  adapters.`
- `Revisit trigger: A later node-contract ADR supersedes this crate boundary.`

## API Consumer Contract

- Consumers parse caller-supplied ids through the validated id newtypes.
- Consumers use `CompatibilityResult` instead of reimplementing type
  compatibility.
- Consumers render `EffectiveNodeContract` and `ContractResolutionDiagnostics`
  as backend facts.

## Structured Producer Contract

- Concrete node registries provide `NodeTypeContract` values with stable port
  ids.
- Effective-contract producers preserve the canonical static contract unless a
  backend-owned resolution reason explains the change.
- Compatibility diagnostics should be safe to return to GUI, binding, and
  headless clients.
