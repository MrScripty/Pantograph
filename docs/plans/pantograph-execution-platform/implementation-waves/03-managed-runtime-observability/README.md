# 03 Managed Runtime Observability Waves

## Purpose

Define concurrent waves for Stage `03`, runtime-created node execution context,
managed capabilities, baseline diagnostics, cancellation, progress, and
guarantee classification.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `coordination-ledger.md` | Host-owned Stage `03` status, decisions, verification, and handoff record. |
| `waves/` | Ordered wave specifications for runtime context freeze, implementation, and integration. |
| `reports/` | Worker reports required before host integration. |

## Problem

Stage `03` touches runtime context, managed capabilities, diagnostics
adaptation, cancellation, progress, and guarantee state. The wave folder keeps
runtime-owned observability separate from durable ledger work.

## Constraints

- Durable ledger storage remains out of scope.
- Node authors must not have to manually emit baseline diagnostics.
- Shared manifests, public facades, ADRs, and ledger implementation files are
  host-owned.

## Decision

Freeze runtime context and event contracts first, then implement context,
diagnostics adaptation, and lifecycle work in bounded slices before host
integration.

## Alternatives Rejected

- Add diagnostics only inside individual nodes: rejected because observability
  would depend on node-author boilerplate.

## Invariants

- Runtime wrappers own baseline execution observations.
- Spawned runtime work has tracked lifecycle and cancellation.
- Worker output is not integrated before its report exists.

## Stage Objective

Move ordinary node execution onto a runtime-created context without adding
node-authored diagnostics boilerplate or implementing durable ledger storage.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned runtime context and event contract freeze. |
| `waves/wave-02.md` | Parallel runtime context/capabilities and event adaptation work. |
| `waves/wave-03.md` | Host-owned integration, cancellation/guarantee verification, ADR, and gate. |

## Global Host-Owned Files

- workspace manifests and lockfiles
- public runtime facade exports
- ADR files
- durable ledger implementation files

## Stage Verification

```bash
cargo test -p pantograph-embedded-runtime
cargo test -p node-engine
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Revisit Triggers

- Managed capability routing requires durable ledger storage in Stage `03`.
- Node-engine must become the owner of durable attribution or compliance
  semantics.
- Cancellation or spawned task ownership cannot be isolated to one lifecycle
  owner.

## Dependencies

**Internal:** `../../03-managed-runtime-observability.md`,
`../../10-concurrent-phased-implementation.md`, and
`coordination-ledger.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../adr/ADR-007-managed-runtime-observability-ownership.md`

## Usage Examples

Start Stage `03` by reading `waves/wave-01.md`, then record runtime context
freeze in `coordination-ledger.md` before assigning implementation slices.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume the wave specs as scope, write-set, verification, and
  reporting contracts.

## Structured Producer Contract

- Wave files use `waves/wave-XX.md` names.
- Worker reports live under `reports/`.
- The coordination ledger records wave status, integration decisions,
  verification, and deviations.
