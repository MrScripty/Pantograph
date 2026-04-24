# Wave 02: Composition And Factoring Implementation

## Objective

Implement composition semantics, concrete node factoring, and runtime lineage in
bounded parallel slices.

## Workers

| Worker | Primary Write Set | Report |
| ------ | ----------------- | ------ |
| composition-contracts | `crates/pantograph-node-contracts/` | `reports/wave-02-worker-composition-contracts.md` |
| workflow-nodes-factoring | `crates/workflow-nodes/` | `reports/wave-02-worker-workflow-nodes-factoring.md` |
| runtime-lineage | `crates/pantograph-embedded-runtime/` lineage modules | `reports/wave-02-worker-runtime-lineage.md` |

## Worker Boundaries

- `composition-contracts` owns composed-node external contracts, internal graph
  mapping contracts, upgrade metadata, and typed migration errors.
- `workflow-nodes-factoring` owns concrete primitive/composed node
  registrations and node README updates.
- `runtime-lineage` owns composed-parent lineage projection during execution.

## Shared Files

Saved workflow fixtures, migration fixtures, public facades, ADRs, and release
notes are host-owned unless the host assigns one explicit worker owner.

## Forbidden Files

- Host binding implementation.
- GUI redesign.
- Credential/session attribution logic.

## Verification

```bash
cargo test -p pantograph-node-contracts
cargo test -p workflow-nodes
cargo test -p pantograph-embedded-runtime
```

## Integration Order

1. Integrate `composition-contracts`.
2. Integrate `workflow-nodes-factoring`.
3. Integrate `runtime-lineage`.
4. Host integrates saved-workflow upgrade fixtures and release notes.

## Escalation Rules

- Stop if a compatibility projection must remain public after migration.
- Stop if primitive model/license attribution is hidden by composition.
