# 04 Model License Diagnostics Ledger Waves

## Purpose

Define concurrent waves for Stage `04`, durable model/license usage ledger,
retention, pruning, and query projections.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `coordination-ledger.md` | Host-owned Stage `04` status, decisions, verification, and handoff record. |
| `waves/` | Ordered wave specifications for schema freeze, implementation, and integration. |
| `reports/` | Worker reports required before host integration. |

## Problem

Stage `04` spans durable ledger storage, runtime submission, retention, and
workflow-service query projections. The wave folder keeps the ledger write
model typed and central while allowing bounded implementation slices.

## Constraints

- GUI and host binding projection files remain out of scope.
- Shared manifests, public facades, and ADRs are host-owned.
- Runtime integration must not require ordinary nodes to hand-author compliance
  records.

## Decision

Freeze ledger schema, retention, and dependencies first, then integrate storage,
runtime submission, and query projections through one ledger boundary.

## Alternatives Rejected

- Add one independent table per diagnostics concern: rejected because it would
  fragment audit truth and projection ownership.

## Invariants

- Model/license usage records remain typed and attributable.
- Retention cleanup preserves audit metadata.
- Worker output is not integrated before its report exists.

## Stage Objective

Add `pantograph-diagnostics-ledger`, persist time-of-use license snapshots and
typed output measurements, expose bounded query projections, and keep durable
ledger storage separate from transient trace storage.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned ledger schema, retention, and dependency freeze. |
| `waves/wave-02.md` | Parallel ledger storage, runtime submission, and query projection work. |
| `waves/wave-03.md` | Host-owned integration, retention verification, ADR, and gate. |

## Global Host-Owned Files

- workspace manifests and lockfiles
- public facade exports
- ADR files
- GUI implementation files
- host binding projection files

## Stage Verification

```bash
cargo test -p pantograph-diagnostics-ledger
cargo test -p pantograph-embedded-runtime
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Revisit Triggers

- SQLite cannot satisfy migration, retention, or indexed query requirements.
- Pumas cannot supply stable time-of-use license facts.
- Runtime integration requires ordinary nodes to hand-author compliance records.

## Dependencies

**Internal:** `../../04-model-license-diagnostics-ledger.md`,
`../../10-concurrent-phased-implementation.md`, and
`coordination-ledger.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../adr/ADR-008-durable-model-license-diagnostics-ledger.md`

## Usage Examples

Start Stage `04` by reading `waves/wave-01.md`, then record schema and
retention freeze in `coordination-ledger.md`.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume the wave specs as scope, write-set, verification, and
  reporting contracts.

## Structured Producer Contract

- Wave files use `waves/wave-XX.md` names.
- Worker reports live under `reports/`.
- The coordination ledger records wave status, integration decisions,
  verification, and deviations.
