# src-tauri/src/llm/commands/registry

Helper modules for the Tauri runtime-registry command boundary.

## Purpose
This directory contains DTO normalization, debug snapshot assembly, and focused
tests for `src-tauri/src/llm/commands/registry.rs`. The boundary exists so the
command module can stay a thin transport adapter while runtime-registry policy,
workflow diagnostics, and scheduler decisions remain owned by backend crates.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `debug.rs` | Runtime debug snapshot DTOs and aggregation helpers for Tauri responses. |
| `request.rs` | Runtime debug request normalization and validation. |
| `tests.rs` | Unit tests for request validation and debug snapshot helper behavior. |

## Problem
Runtime-registry commands need to expose desktop diagnostics that combine app
mode, runtime registry, health monitor, recovery manager, workflow diagnostics,
and trace facts. Keeping those helper DTOs inside the command file would make
the Tauri transport layer harder to audit and easier to confuse with the
backend-owned runtime policy.

## Constraints
- Keep registry selection and lifecycle policy in backend runtime crates.
- Keep workflow diagnostics and trace semantics in `pantograph-workflow-service`
  and workflow diagnostics modules.
- Preserve backend error categories when validating request filters.
- Do not create command-local caches or derived runtime truth.

## Decision
Use this directory for command-boundary helper types only. Helpers may normalize
Tauri request payloads and aggregate already-owned backend facts into response
DTOs, but they must not decide runtime readiness, backend selection, or
workflow relevance.
Tests that seed workflow diagnostics must use the same grouped runtime and
scheduler snapshot input structs as production workflow transport code.

## Alternatives Rejected
- Keep helper DTOs in `registry.rs`: rejected because the command file already
  owns Tauri invoke wiring and should not also contain debug-shape internals.
- Move Tauri debug payloads into runtime-registry crates: rejected because they
  include app-mode, health-monitor, and workflow-diagnostics facts that are
  desktop composition concerns.

## Invariants
- Request normalization trims filter fields without silently accepting blank
  filters.
- Debug snapshots preserve backend-owned registry and workflow trace shapes.
- Helpers remain free of Tauri macros so they can be unit tested directly.
- Runtime selection, readiness, and recovery policy stay outside this
  directory.
- Runtime debug fixtures must not reintroduce positional diagnostics snapshot
  construction that production code has already removed.

## Revisit Triggers
- Runtime debug payloads become a public non-Tauri API.
- Workflow diagnostics move behind a backend-owned projection API consumed by
  multiple adapters.
- Registry command helpers grow beyond request validation and response
  composition.

## Dependencies
**Internal:** parent `registry.rs`, shared LLM command helpers, runtime registry
state, health/recovery managers, workflow diagnostics projections, and
`pantograph-workflow-service` trace DTOs.

**External:** `serde`.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
let request = RuntimeDebugSnapshotRequest::default().normalized();
```

## API Consumer Contract
- Inputs: Tauri command request payloads and backend-owned registry,
  diagnostics, health, and recovery snapshots.
- Outputs: serialized runtime debug DTOs for desktop command callers.
- Lifecycle: helpers are stateless and operate within a single command
  invocation.
- Errors: invalid filter values map to `WorkflowServiceError::InvalidRequest`
  for consistent command error projection.
- Versioning: response fields are consumed by the desktop frontend and should
  change additively unless the frontend migrates in the same commit.

## Structured Producer Contract
- Stable fields: runtime debug snapshot fields, trace selection fields, and
  recovery debug fields are machine-consumed by the desktop frontend.
- Defaults: omitted debug filter fields are absent after normalization; omitted
  booleans keep command-level defaults.
- Enums and labels: workflow trace and registry labels retain backend-owned
  semantics.
- Ordering: registry and trace ordering follows the backend snapshots supplied
  to these helpers.
- Compatibility: field removals or semantic changes require frontend command
  consumer updates.
- Regeneration/migration: update `registry.rs`, tests, frontend command
  consumers, and this README together when response DTOs change.

## Testing
```bash
cargo test --manifest-path src-tauri/Cargo.toml llm::commands::registry
```

## Notes
- M2 should continue moving relevance and selection decisions into
  backend-owned projections so this directory remains transport-focused.
