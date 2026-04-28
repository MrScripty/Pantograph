# pantograph-workflow-service/tests/fixtures

Shared public contract fixtures used by Rust integration tests and frontend
service-boundary tests.

## Purpose
This directory holds deterministic JSON payloads for API shapes that must stay
aligned across Rust workflow-service contracts and TypeScript workflow service
DTOs.

## Contents
| File | Description |
| ---- | ----------- |
| `run_projection_contract.json` | Shared run-list and run-detail projection responses consumed by Rust contract tests and TypeScript projection service tests. |

## Problem
Run projection DTOs cross the Rust workflow-service boundary, Tauri command
boundary, and TypeScript service boundary. Keeping separate inline fixtures in
each layer makes it easy for one side to drift while local tests still pass.

## Constraints
- Fixtures must remain deterministic JSON with no generated timestamps.
- Fixtures must represent public serialized DTOs, not private database rows.
- Rust contract tests must validate fixture compatibility through public DTO
  deserialization.
- Frontend tests must consume the fixture through service methods rather than
  bypassing the service boundary.

## Decision
Keep cross-layer projection fixtures in the Rust contract-test fixture
directory and let frontend tests read them by path. The workflow-service crate
owns the canonical public DTO shape, while TypeScript tests prove the same
shape is accepted by GUI service consumers.

## Alternatives Rejected
- Keep duplicated inline fixtures in Rust and TypeScript tests.
  Rejected because duplicated JSON can diverge without either side detecting
  the contract gap.
- Generate TypeScript types in this slice.
  Rejected because generated binding parity is broader than the run
  projection acceptance gap being closed here.

## Invariants
- Fixtures must contain serialized public API shapes, not private ledger rows.
- Run projection fixtures must keep scheduler-selected runtime, device, and
  network-node fields alongside scope fields so Rust and TypeScript tests catch
  placement-facet contract drift.
- Rust tests must deserialize fixtures into public DTOs before asserting
  frontend compatibility.
- TypeScript tests must use fixtures through the workflow service boundary,
  not by importing backend-only types.

## Revisit Triggers
- Generated bindings or schema checks replace fixture-based DTO parity.
- A projection response grows large enough to need fixture generation tooling.
- Another frontend package needs to consume the same fixture path.

## Dependencies
**Internal:** `crates/pantograph-workflow-service/tests/contract.rs` and
`src/services/workflow/WorkflowService.projections.test.ts`.

**External:** `serde_json` for Rust fixture validation and Node `fs` for
frontend test fixture loading.

## Related ADRs
- `docs/adr/ADR-014-workbench-projection-boundary.md`

## Usage Examples
Validate the shared run projection contract fixture:

```bash
cargo test -p pantograph-workflow-service --test contract run_projection_cross_layer_fixture_deserializes
node --experimental-strip-types --test src/services/workflow/WorkflowService.projections.test.ts
```

## Testing
```bash
cargo test -p pantograph-workflow-service --test contract run_projection_cross_layer_fixture_deserializes
node --experimental-strip-types --test src/services/workflow/WorkflowService.projections.test.ts
```
