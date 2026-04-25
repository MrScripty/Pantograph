# docs/adr

## Purpose
This directory records Pantograph architecture decisions that are stable enough
to guide multi-commit implementation work. It exists so runtime, workflow, and
host-boundary changes can point to a durable decision record instead of forcing
developers to reconstruct intent from commit history.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `ADR-001-headless-embedding-service-boundary.md` | Freezes the host-agnostic workflow service boundary and the separation between service logic and adapters. |
| `ADR-002-runtime-registry-ownership-and-lifecycle.md` | Freezes ownership, lifecycle, facade, and observability boundaries for the `RuntimeRegistry` layer. |
| `ADR-003-runtime-redistributables-manager-boundary.md` | Freezes the backend-owned runtime redistributables manager boundary across inference, Pantograph-facing runtime views, workflow readiness, and Tauri/GUI adapters. |
| `ADR-005-durable-runtime-attribution.md` | Freezes durable client/session/bucket/workflow-run attribution ownership, SQLite persistence, digest-only credential storage, bucket namespace semantics, and execution-session terminology. |
| `ADR-006-canonical-node-contract-ownership.md` | Freezes canonical node/port/effective-contract ownership in `pantograph-node-contracts` and projection responsibilities for workflow-service, node-engine, bindings, and GUI adapters. |
| `ADR-007-managed-runtime-observability-ownership.md` | Freezes embedded-runtime ownership of runtime-created node execution context, managed capabilities, transient diagnostics, cancellation/progress lifecycle, and guarantee classification. |
| `ADR-008-durable-model-license-diagnostics-ledger.md` | Freezes durable model/license diagnostics ledger ownership, SQLite persistence, retention/pruning semantics, runtime submission, and workflow query projection boundaries. |
| `ADR-009-composed-node-contracts-and-migration.md` | Freezes composed-node contract ownership, primitive trace preservation, runtime lineage projection, and saved-workflow migration strategy. |

## Problem
Pantograph is actively deepening its backend/runtime architecture. Without an
ADR index, boundary decisions are easy to lose across roadmap updates, plan
revisions, and implementation commits, which increases the risk of policy logic
leaking into the wrong layer.

## Constraints
- ADRs must stay specific to Pantograph’s real architectural seams.
- The directory must remain easy to scan during implementation reviews.
- ADR references must stay stable enough for README and plan traceability.

## Decision
Keep an indexed `docs/adr/` directory for accepted Pantograph boundary
decisions. Each ADR records one significant architecture choice with explicit
context, decision, and consequences, and module READMEs should reference the
relevant ADRs directly.

## Alternatives Rejected
- Keep architecture decisions only in plans and commit messages.
  Rejected because plans are milestone-scoped and commit history is not an
  adequate substitute for stable boundary records.
- Create one monolithic architecture document for all decisions.
  Rejected because targeted ADRs are easier to update, review, and reference.

## Invariants
- Each ADR file in this directory describes one stable architectural decision.
- ADRs use the standard `Context`, `Decision`, and `Consequences` structure.
- This README indexes the ADR files that are intended for active traceability.

## Revisit Triggers
- A major boundary decision lands without an ADR entry.
- Existing ADRs are deprecated or superseded.
- The directory starts holding drafts or notes that are not actual ADRs.

## Dependencies
**Internal:** Pantograph plans, module READMEs, and architecture reviews that
link to these decisions.

**External:** None.

## Related ADRs
- `None identified as of 2026-04-13.`
- `Reason: This README is the ADR index rather than a module boundary decision.`
- `Revisit trigger: The ADR process or repository-level architecture review policy changes.`

## Usage Examples
```markdown
## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`
- `docs/adr/ADR-005-durable-runtime-attribution.md`
- `docs/adr/ADR-006-canonical-node-contract-ownership.md`
- `docs/adr/ADR-007-managed-runtime-observability-ownership.md`
- `docs/adr/ADR-008-durable-model-license-diagnostics-ledger.md`
- `docs/adr/ADR-009-composed-node-contracts-and-migration.md`
```

## API Consumer Contract
- Human readers use this directory as the canonical index for accepted
  architecture decisions.
- File names are stable references for plans, READMEs, and reviews.
- New ADRs should be added here when a significant architectural boundary is
  accepted.

## Structured Producer Contract
- ADR filenames remain stable once referenced by plans or READMEs unless an ADR
  is explicitly superseded.
- This index lists accepted ADRs and their intent in review order.
- When an ADR is added, deprecated, or superseded, this README must be updated
  in the same change set.
