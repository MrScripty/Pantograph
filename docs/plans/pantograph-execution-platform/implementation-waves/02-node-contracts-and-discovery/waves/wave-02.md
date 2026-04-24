# Wave 02: Canonical Contracts And Projections

## Objective

Implement canonical node contracts and projection integration in bounded,
parallel slices.

## Workers

| Worker | Primary Write Set | Report |
| ------ | ----------------- | ------ |
| canonical-contracts | `crates/pantograph-node-contracts/` | `reports/wave-02-worker-canonical-contracts.md` |
| workflow-service-projections | `crates/pantograph-workflow-service/src/graph/` | `reports/wave-02-worker-workflow-service-projections.md` |
| workflow-nodes-registration | `crates/workflow-nodes/` | `reports/wave-02-worker-workflow-nodes-registration.md` |

## Worker Boundaries

- `canonical-contracts` owns validated IDs, canonical DTOs, compatibility
  checks, effective-contract resolution contracts, errors, tests, and README.
- `workflow-service-projections` owns workflow-service graph DTO projection,
  connection candidate routing, and removal of duplicated compatibility policy.
- `workflow-nodes-registration` owns conversion of concrete node descriptors
  into canonical registrations without making `node-engine` the semantic owner.

## Shared Files

Workspace manifests, root exports, `node-engine` public docs, and ADRs are
host-owned unless the host assigns one explicit worker owner.

## Forbidden Files

- Host binding generation.
- GUI-local node catalogs.
- Durable ledger implementation.

## Verification

Workers run their targeted tests where possible:

```bash
cargo test -p pantograph-node-contracts
cargo test -p workflow-nodes
cargo test -p pantograph-workflow-service
```

## Integration Order

1. Integrate `canonical-contracts`.
2. Integrate `workflow-nodes-registration`.
3. Integrate `workflow-service-projections`.
4. Host applies shared manifest/facade/doc updates.

## Escalation Rules

- Stop if workflow-service needs to redefine compatibility semantics locally.
- Stop if generated binding or GUI catalog changes become necessary.
