# src-tauri/src/workflow/headless_workflow_commands_tests

## Purpose
This directory keeps headless workflow command tests split by behavior while the
root `headless_workflow_commands_tests.rs` file owns shared fixtures and module
registration.

## Contents
| File | Description |
| ---- | ----------- |
| `diagnostics_helpers.rs` | Covers scheduler/runtime helper recording and trace identity joining. |
| `diagnostics_projection.rs` | Covers diagnostics projection, stored runtime metadata, model target lookup, observed runtime ids, and clear-history behavior. |
| `transport_responses.rs` | Covers diagnostics request DTOs, scheduler/trace response adapters, validation errors, and workflow error-envelope serialization. |

## Problem
The original headless workflow command test module mixed transport response
checks, diagnostics helper recording, diagnostics projection, stored runtime
metadata, and error-envelope assertions in one large file. That made it harder
to find the behavior under test and pushed the root test module over the
large-file threshold.

## Constraints
- Keep the parent `headless_workflow_commands.rs` production facade unchanged.
- Preserve existing test names and behavior so filtered test runs remain useful.
- Keep shared fixtures in one place to avoid drifting scheduler/runtime test
  setup across behavior modules.
- Do not move backend-owned diagnostics projection policy into the Tauri command
  tests.

## Decision
Keep shared fixture builders in the root test module so behavior modules can use
`super::*` without duplicating scheduler sessions, capability responses, or the
diagnostics projection macro.

## Alternatives Rejected
- Keep all tests in the root module.
  Rejected because the root test module exceeded the large-file threshold and
  mixed unrelated transport and diagnostics behaviors.
- Duplicate fixtures in each behavior module.
  Rejected because shared scheduler and capability fixtures express one backend
  contract and should not diverge between tests.

## Invariants
- New headless workflow command tests should join the module matching the
  behavior under test before adding another root-level test file.
- Behavior modules should not construct alternate fixtures when the root helper
  already expresses the backend-owned session or capability contract.

## Revisit Triggers
- Add a new module when a behavior area grows beyond a focused diagnostics,
  transport, or projection responsibility.
- Revisit fixture placement if tests need incompatible scheduler/session
  contracts that cannot be represented by the existing root builders.

## Dependencies
- `src-tauri/src/workflow/headless_workflow_commands.rs` declares the root test
  module.
- `src-tauri/src/workflow/headless_diagnostics.rs` owns the production
  diagnostics helper and projection functions under test.
- `pantograph-workflow-service` owns the workflow scheduler/session DTOs used by
  the fixtures.

## Related ADRs
- `docs/standards-compliance-analysis/refactor-plan.md`
- `src-tauri/src/workflow/README.md`

## Usage Examples
Run the focused moved coverage with:

```sh
cargo test --manifest-path src-tauri/Cargo.toml headless_workflow_commands
```
