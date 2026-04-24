# Wave 02: Ledger Implementation

## Objective

Implement ledger storage, runtime submission, and workflow-service query
projections in bounded parallel slices.

## Workers

| Worker | Primary Write Set | Report |
| ------ | ----------------- | ------ |
| ledger-storage-retention | `crates/pantograph-diagnostics-ledger/` | `reports/wave-02-worker-ledger-storage-retention.md` |
| runtime-ledger-submission | `crates/pantograph-embedded-runtime/` ledger submission boundaries | `reports/wave-02-worker-runtime-ledger-submission.md` |
| workflow-service-query-projections | `crates/pantograph-workflow-service/` diagnostics query use cases | `reports/wave-02-worker-workflow-service-query-projections.md` |

## Worker Boundaries

- `ledger-storage-retention` owns canonical event types, SQLite persistence,
  migrations, retention/pruning, query DTOs, and ledger README.
- `runtime-ledger-submission` owns managed model capability interception and
  submission of validated usage facts through the ledger trait.
- `workflow-service-query-projections` owns application-level query use cases
  that delegate to the ledger and project client/session/bucket/run history.

## Shared Files

Workspace manifests, lockfiles, public facade exports, and ADRs are host-owned.

## Forbidden Files

- GUI diagnostics views.
- Host binding projections.
- Node factoring or migration logic.

## Verification

```bash
cargo test -p pantograph-diagnostics-ledger
cargo test -p pantograph-embedded-runtime
cargo test -p pantograph-workflow-service
```

## Integration Order

1. Integrate `ledger-storage-retention`.
2. Integrate `runtime-ledger-submission`.
3. Integrate `workflow-service-query-projections`.
4. Host resolves shared manifest/facade changes.

## Escalation Rules

- Stop if retention policy remains unbounded.
- Stop if ordinary nodes must manually create compliance ledger records.
