# crates/pantograph-embedded-runtime/src/runtime_registry_tests

## Purpose
This directory contains behavior-focused runtime-registry test modules split
out of the embedded-runtime registry test index. The boundary keeps registry
observation, lifecycle transition, health, and warmup coverage reviewable while
preserving one shared mock host controller surface.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `observations.rs` | Runtime mode-info reconciliation, snapshot override, active-runtime registration, reservation shaping, retention-hint sync, and scheduler diagnostics tests. |
| `lifecycle.rs` | Runtime registry sync, stop-all, restore, and transition reconciliation tests. |
| `health_warmup.rs` | Health-assessment synchronization, unhealthy projection, and active-runtime warmup coordination tests. |

## Problem
Runtime-registry tests span several independent behavior families and had grown
large enough to obscure which backend-owned registry contract a failure
protects. Splitting by behavior keeps review and future additions bounded.

## Constraints
- Tests must continue to exercise backend-owned runtime-registry translation
  helpers, not host adapter local policy.
- Shared mock host controllers stay in the parent test module.
- Production registry helpers remain separate from test-only fixtures.

## Decision
Keep shared fixtures and producer matching/reclaim smoke tests in the parent
`runtime_registry_tests.rs` module. Move observation/diagnostics tests into
`observations.rs`, lifecycle transition tests into `lifecycle.rs`, and health
plus warmup coordination tests into `health_warmup.rs`.

## Alternatives Rejected
- Keep one monolithic runtime-registry test file.
  Rejected because observation, lifecycle, health, and warmup behavior evolved
  independently and should remain independently reviewable.

## Invariants
- Each test module should cover one runtime-registry behavior family.
- Test modules may use parent imports and mock controllers through `super::*`.
- Registry policy assertions stay backend-owned and must not be duplicated in
  Tauri adapter tests as an alternate source of truth.

## Revisit Triggers
- A behavior module grows past the review threshold.
- Runtime-registry ownership moves to a separate crate or public host-facing
  harness.

## Dependencies
**Internal:** parent runtime-registry tests module, embedded-runtime registry
helpers, runtime health helpers, and `pantograph-runtime-registry` DTOs.

**External:** Rust unit test harness only.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: these tests protect backend-owned runtime registry translation and
  lifecycle semantics.
- Revisit trigger: runtime registry policy ownership changes.

## Usage Examples
```bash
cargo test -p pantograph-embedded-runtime runtime_registry
```

## API Consumer Contract
- These files are crate-local tests and expose no public APIs.
- Test names should describe the registry behavior contract they protect.

## Structured Producer Contract
- Test fixtures must produce host runtime snapshots, health assessments, and
  registry transitions through the same DTOs used by production helpers.
