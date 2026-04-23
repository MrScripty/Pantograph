# src-tauri/src/workflow/event_adapter/tests

## Purpose
Focused event-adapter regression coverage grouped by behavior area. The parent
`tests.rs` file owns shared fixtures, while these modules isolate translation,
transport, and executor-backed adapter behavior.

## Contents
| File | Description |
| ---- | ----------- |
| `translation_projection.rs` | Backend workflow-event translation and diagnostics projection regressions. |
| `channel_transport.rs` | Direct adapter send behavior over the Tauri channel. |
| `executor_integration.rs` | Parallel workflow execution integration coverage through the adapter sink. |

## Problem
`event_adapter/tests.rs` had grown into one large regression module mixing
backend event translation semantics, Tauri channel emission, and executor-backed
integration checks. That obscured ownership boundaries and pushed the file past
the large-file threshold.

## Constraints
- Tests stay under the adapter module so they can exercise crate-private
  translation and diagnostics bridge helpers.
- Shared fixtures stay in the parent harness unless a submodule needs
  incompatible setup.
- The stable `workflow::event_adapter` production facade must not change.

## Decision
Keep shared graph/executor/channel fixtures in `tests.rs`, move backend event
translation and diagnostics projection assertions into
`translation_projection.rs`, keep direct channel-send assertions in
`channel_transport.rs`, and isolate executor-backed integration coverage in
`executor_integration.rs`.

## Alternatives Rejected
- Leave all tests in one root module.
  Rejected because the file exceeded the decomposition threshold and mixed
  unrelated behavior families.
- Duplicate fixtures in each test file.
  Rejected because adapter fixtures express one transport contract and should
  remain consistent across behavior modules.

## Invariants
- New event-adapter tests should land in the smallest matching behavior module
  before adding more root-level code.
- Translation and diagnostics projection assertions stay in
  `translation_projection.rs`.
- Direct channel transport assertions stay in `channel_transport.rs`.
- Executor-backed workflow runs stay in `executor_integration.rs`.

## Revisit Triggers
- Adapter fixture setup grows large enough to justify a dedicated shared fixture
  module.
- A new event family introduces another distinct adapter behavior area.

## Dependencies
- `src-tauri/src/workflow/event_adapter/tests.rs` owns shared fixtures and
  module registration.
- `src-tauri/src/workflow/event_adapter/translation.rs` and
  `diagnostics_bridge.rs` provide the crate-private behavior under test.
- `node-engine` provides the workflow events and executor used by the
  integration coverage.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: event-adapter tests assert a Tauri transport boundary over
  backend-owned workflow semantics.

## Usage Examples
Run the focused adapter coverage with:

```sh
cargo test --manifest-path src-tauri/Cargo.toml event_adapter
```
