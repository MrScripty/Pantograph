# Stage 05 Composition Contracts Module Split Refactor Plan

## Objective

Split large Stage 05-touched composition and migration source files into
smaller modules without changing public behavior or serialization contracts.

## Scope

In scope:

- `crates/pantograph-node-contracts/src/lib.rs`
- `crates/pantograph-node-contracts/src/` helper modules created from that file
- `crates/pantograph-workflow-service/src/graph/canonicalization.rs`
- `crates/pantograph-workflow-service/src/graph/` helper modules created from
  canonicalization logic
- focused tests and README updates required by the split

Out of scope:

- new composition behavior
- GUI or binding projection work
- workflow-node descriptor changes
- migration semantics changes

## Findings

- Stage 05 added composed-node and migration DTOs to
  `pantograph-node-contracts/src/lib.rs`, bringing the file to 1411 lines.
- Stage 05 added migration-record production to
  `pantograph-workflow-service/src/graph/canonicalization.rs`, which now has
  824 lines.
- Both files remain functionally verified, but the module boundaries should be
  tightened before additional composition or migration behavior accumulates.

## Proposed Slices

1. Move composed-node DTOs and validation into
   `crates/pantograph-node-contracts/src/composition.rs` and re-export from
   `lib.rs`.
2. Move contract-upgrade DTOs and validation into
   `crates/pantograph-node-contracts/src/migration.rs` and re-export from
   `lib.rs`.
3. Move node-contract tests into a focused test module or integration tests
   without weakening crate-private coverage.
4. Split workflow graph canonicalization into legacy migration, inference
   setting expansion, and tests modules under
   `crates/pantograph-workflow-service/src/graph/`.

## Verification

```bash
cargo fmt -p pantograph-node-contracts -p pantograph-workflow-service -- --check
cargo test -p pantograph-node-contracts
cargo test -p pantograph-workflow-service canonicalize_workflow_graph
cargo check -p pantograph-workflow-service
cargo clippy -p pantograph-node-contracts -p pantograph-workflow-service --all-targets -- -D warnings
cargo check --workspace --all-features
```

## Re-Plan Triggers

- Re-exporting modules would change public serialization or import paths.
- Tests require behavior changes rather than mechanical module relocation.
- Canonicalization split exposes a migration behavior gap not covered by Stage
  05.
