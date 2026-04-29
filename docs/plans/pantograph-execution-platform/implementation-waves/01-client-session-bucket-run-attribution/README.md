# 01 Client Session Bucket Run Attribution Waves

## Purpose

Define safe concurrent waves for Stage `01`, durable client/session/bucket/run
attribution.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `coordination-ledger.md` | Host-owned Stage `01` status, decisions, verification, and handoff record. |
| `waves/` | Ordered wave specifications for contract freeze, implementation, and integration. |
| `reports/` | Worker reports required before host integration. |

## Problem

Stage `01` changes identity and execution attribution across storage and
workflow-service boundaries. The wave folder keeps concurrent work bounded
while preserving durable attribution as the first dependency for later stages.

## Constraints

- The stage must not touch GUI, model/license ledger, or node contract
  implementation.
- Shared manifests, generated artifacts, facade exports, and ADRs are
  host-owned.
- Workflow-session public compatibility wrappers are not preserved.

## Decision

Use three waves: contract freeze, bounded implementation, and host integration.
This keeps attribution storage and workflow-service cutover separable while
preserving one integration gate.

## Alternatives Rejected

- Implement attribution and workflow-service cutover in one unbounded pass:
  rejected because storage and public API replacement need separate write-set
  control.

## Invariants

- Execution requires validated attribution before workflow-run start.
- Worker output is not integrated before its report exists.
- Host-owned files are changed only during integration or by explicit owner.

## Stage Objective

Add `pantograph-runtime-attribution`, replace affected workflow-session public
entry points with durable client-session APIs, and require workflow-run
attribution before execution starts.

## Non-Goals

- No GUI implementation.
- No model/license diagnostics ledger.
- No node contract registry implementation.
- No backward-compatible workflow-session public wrappers.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned contract freeze, dependency review, and API cutover inventory. |
| `waves/wave-02.md` | Parallel implementation of attribution domain/storage and workflow-service integration. |
| `waves/wave-03.md` | Host-owned integration, public API cutover verification, docs, ADR, and stage-end gate. |

## Global Host-Owned Files

- `Cargo.toml`
- `Cargo.lock`
- generated binding artifacts
- public facade exports that cross worker boundaries
- ADR files

## Stage Verification

```bash
cargo test -p pantograph-runtime-attribution
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Revisit Triggers

- SQLite or credential digest dependency review fails.
- Workflow-session public API cutover cannot be completed without host binding
  edits in the same wave.
- Existing dirty files overlap attribution or workflow-service write sets.

## Dependencies

**Internal:** `../../01-client-session-bucket-run-attribution.md`,
`../../10-concurrent-phased-implementation.md`, and
`coordination-ledger.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../adr/ADR-005-durable-runtime-attribution.md`

## Usage Examples

Start Stage `01` by reading `waves/wave-01.md`, then record the contract freeze
result in `coordination-ledger.md`.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume the wave specs as scope, write-set, verification, and
  reporting contracts.

## Structured Producer Contract

- Wave files use `waves/wave-XX.md` names.
- Worker reports live under `reports/`.
- The coordination ledger records wave status, integration decisions,
  verification, and deviations.
