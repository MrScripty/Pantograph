# runtime_host_execution_tests fixtures

## Purpose
This directory contains JSON fixtures for the runtime-host execution contract.
They are checked-in examples of the Milestone 5b replacement boundary and must
not include legacy successful execution fields.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `runtime_host_execution_request_dispatch_selected.json` | Valid host execution request with a dispatch-selected scheduler handoff. |
| `runtime_host_execution_response_accepted.json` | Valid host response that acknowledges the request with typed diagnostics. |

## Problem
Runtime-host execution payloads will be transported between scheduler-driven
dispatch and runtime execution code. Fixtures keep the serialized contract
visible before Pumas load-target resolution and runtime migration are wired.

## Constraints
- Fixtures must not contain local paths, executable load targets, `ModelRefV2`,
  `model_path`, frontend `modelPath`, or worker launch internals.
- Request fixtures must use scheduler-owned handoff and dispatch-decision
  payloads as nested contracts.
- Response fixtures must use typed state and diagnostic enums.

## Decision
Keep one valid request fixture and one valid response fixture for the first
runtime-host contract slice. Negative tests mutate these fixtures to keep
rejection cases focused.

## Alternatives Rejected
- Store only Rust constructors: rejected because this boundary is a serialized
  host-facing contract.
- Preserve legacy path-shaped request fixtures: rejected because those paths
  are deletion targets.

## Invariants
- Fixture contract versions remain explicit.
- Scheduler handoff facts remain nested under `handoff`.
- Host responses carry correlation and diagnostics only.

## Revisit Triggers
- Runtime-host responses begin carrying output artifact references.
- Host-only Pumas load-target facts get their own serialized contract.

## Dependencies
**Internal:** `runtime_host_execution.rs` and `pantograph-scheduler`.

**External:** None.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
let _ = include_str!("runtime_host_execution_request_dispatch_selected.json");
```

## API Consumer Contract
- Inputs: JSON fixture files decoded by runtime-host execution tests.
- Outputs: canonical serialized request and response payloads.
- Lifecycle: fixtures are immutable during test runs.
- Errors: fixture validation failures indicate contract drift.
- Compatibility: fixture changes must be reviewed as host-facing contract
  changes.

## Structured Producer Contract
- Stable fields are the contract version, `execution_request_id`, nested
  `handoff`, response correlation ids, response state, and typed diagnostics.
- Optional fields may be omitted only when omission is documented by the DTO.
- Array order is not a contract.
- Retired path-shaped fields must not be reintroduced as aliases.
