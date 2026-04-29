# 06 Binding Projections And Verification Waves

## Purpose

Define concurrent waves for Stage `06`, native Rust base API projection,
UniFFI/Rustler binding DTOs, and language-native host verification.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `coordination-ledger.md` | Host-owned Stage `06` status, decisions, verification, and handoff record. |
| `waves/` | Ordered wave specifications for API freeze, projection work, host verification, and integration. |
| `reports/` | Worker reports required before host integration. |

## Problem

Stage `06` projects backend-owned contracts into host-language surfaces. The
wave folder keeps native API freeze, binding projection implementation, and
host verification separated so support tiers are evidence-based.

## Constraints

- Native Rust remains the canonical API.
- Generated artifacts are not hand-edited.
- Host lanes are supported only when real native artifacts load in tests or
  smoke checks.

## Decision

Freeze native API/support tiers, implement UniFFI and Rustler projections, run
host-language verification, then reconcile status in one integration gate.

## Alternatives Rejected

- Treat generated binding files as hand-owned source: rejected because it would
  break artifact reproducibility and version matching.

## Invariants

- Host bindings are projections over backend-owned contracts.
- Unsupported or blocked lanes are labeled explicitly.
- Worker output is not integrated before its report exists.

## Stage Objective

Project backend-owned execution-platform contracts into C#, Python, and
Elixir/BEAM after the native Rust base API is resolved, with host-language
tests strong enough to make future binding additions repeatable.

## Waves

| Wave | Purpose |
| ---- | ------- |
| `waves/wave-01.md` | Host-owned Rust API/support-tier freeze and smoke command selection. |
| `waves/wave-02.md` | Parallel UniFFI and Rustler projection work. |
| `waves/wave-03.md` | Parallel language-native host smoke/acceptance tests. |
| `waves/wave-04.md` | Host-owned artifact/version integration, docs, ADR, and gate. |

## Global Host-Owned Files

- workspace manifests and lockfiles
- generated host binding artifacts
- package metadata shared by multiple lanes
- public native Rust facade files
- ADR and release note files

## Stage Verification

```bash
cargo test -p pantograph-uniffi
cargo test -p pantograph-rustler
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

Host-language verification commands must be recorded in wave `01` before a lane
is marked supported.

## Revisit Triggers

- Native Rust base API is not stable enough to bind.
- C# or Python cannot provide language-native tests that load the real native
  artifact.
- A host lane needs semantics not present in the backend-owned Rust API.

## Dependencies

**Internal:** `../../06-binding-projections-and-verification.md`,
`../../10-concurrent-phased-implementation.md`, and
`coordination-ledger.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../adr/ADR-010-binding-projection-ownership-and-support-tiers.md`

## Usage Examples

Start Stage `06` by reading `waves/wave-01.md`, then record native API and
support-tier decisions in `coordination-ledger.md` before projection work.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume the wave specs as scope, write-set, verification, and
  reporting contracts.

## Structured Producer Contract

- Wave files use `waves/wave-XX.md` names.
- Worker reports live under `reports/`.
- The coordination ledger records wave status, integration decisions,
  verification, and deviations.
