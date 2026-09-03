# Architecture Decision Records

[ARCHITECTURE.md](../../ARCHITECTURE.md) is the readable system overview. These
records own decisions that need independent history and consequences.

| ADR | Decision |
| --- | --- |
| [001](ADR-001-headless-embedding-service-boundary.md) | Headless workflow service and adapter boundary |
| [002](ADR-002-runtime-registry-ownership-and-lifecycle.md) | Runtime-registry ownership and lifecycle |
| [003](ADR-003-runtime-redistributables-manager-boundary.md) | Managed runtime redistributables boundary |
| [004](ADR-004-verification-baseline-restoration.md) | Historical verification-baseline restoration policy |
| [005](ADR-005-durable-runtime-attribution.md) | Durable runtime attribution |
| [006](ADR-006-canonical-node-contract-ownership.md) | Canonical node-contract ownership |
| [007](ADR-007-managed-runtime-observability-ownership.md) | Managed-runtime observability ownership |
| [008](ADR-008-durable-model-license-diagnostics-ledger.md) | Model/license diagnostics ledger |
| [009](ADR-009-composed-node-contracts-and-migration.md) | Composed node contracts and migration |
| [010](ADR-010-binding-projection-ownership-and-support-tiers.md) | Binding projections and support tiers |
| [011](ADR-011-scheduler-only-workflow-execution.md) | Scheduler-only workflow execution |
| [012](ADR-012-canonical-workflow-run-identity.md) | Canonical workflow-run identity |
| [013](ADR-013-workflow-version-registry-and-run-snapshots.md) | Workflow versions and immutable run snapshots |
| [014](ADR-014-run-centric-workbench-projection-boundary.md) | Run-centric workbench projection boundary |
| [015](ADR-015-authoritative-runtime-node-type-injection.md) | Runtime node-type injection authority |
| [016](ADR-016-workflow-error-diagnostics-spine.md) | Workflow error diagnostics spine |
| [017](ADR-017-rust-workspace-policy.md) | Rust workspace metadata, lint, and dependency policy |

ADR status is authoritative inside each record. Superseded implementation
plans referenced by older ADRs are historical provenance, not current
instructions. New decisions receive the next unused number; accepted records
are not silently rewritten into implementation plans.
