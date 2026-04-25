# Execution Platform Implementation Waves

## Purpose

This directory contains the concurrent implementation plan for the numbered
Pantograph execution-platform stages.

## Contents

| Folder | Description |
| ------ | ----------- |
| `01-client-session-bucket-run-attribution/` | Durable attribution worker waves and coordination ledger. |
| `02-node-contracts-and-discovery/` | Canonical node contract and discovery worker waves. |
| `03-managed-runtime-observability/` | Runtime-owned context, lifecycle, and diagnostics worker waves. |
| `04-model-license-diagnostics-ledger/` | Durable usage ledger, retention, and query worker waves. |
| `05-composition-factoring-and-migration/` | Composition, node factoring, and clean workflow upgrade waves. |
| `06-binding-projections-and-verification/` | Native Rust base API projection and host binding verification waves. |

Stage `07` standards compliance review did not receive an implementation-wave
folder because its stage-start gate selected single-worker documentation review
instead of concurrent implementation.

## Problem

The execution-platform plan spans multiple crates and support surfaces. The
work can use parallel workers only after each stage records frozen boundaries,
non-overlapping write sets, report files, and one-at-a-time integration rules.

## Constraints

- `../08-stage-start-implementation-gate.md` decides whether a stage actually
  uses concurrent workers.
- `../10-concurrent-phased-implementation.md` is the authoritative execution
  rule set for worker prompts, isolated worktrees, reports, and integration.
- Shared contracts, workspace manifests, generated artifacts, and public
  facades are host-owned unless a wave assigns one explicit worker owner.
- Old workflow-session and graph compatibility surfaces are removed or upgraded
  cleanly; workers must not preserve residual backward-compatible APIs.

## Decision

Keep one concurrent plan folder per numbered implementation stage. Each stage
folder contains a stage README, host-owned coordination ledger, wave specs, and
report placeholders so implementation can begin from explicit worker contracts.

## Alternatives Rejected

- One global worker pool for all stages: rejected because later stages depend on
  committed outputs and refactor gates from earlier stages.
- Worker prompts without committed wave specs: rejected because write-set drift
  would be hard to audit.

## Invariants

- Execute one implementation stage at a time.
- Execute one wave at a time inside the selected stage.
- Integrate worker outputs one at a time.
- Read worker reports before integration.
- Update the stage coordination ledger after each integration.
- Stages whose start gate selects single-worker execution do not require a
  stage-specific implementation-wave folder.

## Revisit Triggers

- A stage-start gate finds dirty files overlapping the planned write sets.
- A worker needs files outside its assigned write set.
- A shared manifest, generated artifact, or public facade needs concurrent
  edits by more than one worker.

## Dependencies

**Internal:** `../00-overview-and-boundaries.md`,
`../08-stage-start-implementation-gate.md`,
`../09-stage-end-refactor-gate.md`, and
`../10-concurrent-phased-implementation.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `None identified as of 2026-04-24.`
- `Reason: These are implementation coordination artifacts, not frozen architecture decisions.`
- `Revisit trigger: A stage completion ADR supersedes a coordination decision.`

## Usage Examples

For Stage `02`, read:

```text
02-node-contracts-and-discovery/README.md
02-node-contracts-and-discovery/waves/wave-01.md
02-node-contracts-and-discovery/coordination-ledger.md
```

## API Consumer Contract

- These files are not runtime APIs.
- Worker prompts may quote these files as the implementation contract for a
  stage wave.

## Structured Producer Contract

- Each concurrent stage folder must contain `README.md`,
  `coordination-ledger.md`, `waves/`, and `reports/`.
- Wave files use `wave-XX.md` names and define objective, write sets,
  forbidden files, verification, report paths, and integration order.
