# pantograph-workflow-service/src/workflow/tests/fixtures

Shared workflow facade test fixtures split by responsibility.

## Purpose
This directory holds private fixture families re-exported through
`workflow/tests/fixtures.rs` for behavior-focused workflow service tests.

## Contents
| File | Description |
| ---- | ----------- |
| `core_hosts.rs` | Baseline workflow host, inspection host, and ready runtime capability helpers. |
| `execution_hosts.rs` | Timeout, blocking-run, admission-gated, and retention-hint recording hosts. |
| `preflight_hosts.rs` | Capability, preflight, and technical-fit decision hosts. |
| `runtime_hosts.rs` | Runtime unload-selection and affinity-preserving capacity hosts. |
| `scheduler_diagnostics.rs` | Mock scheduler diagnostics provider. |

## Problem
The shared fixture boundary extracted from `workflow/tests.rs` remained too
large and mixed unrelated host responsibilities in one file.

## Constraints
- Preserve the `workflow::tests` helper surface used by behavior modules.
- Keep fixtures private to the workflow service test module.
- Avoid changing production workflow service APIs.

## Decision
Split fixtures by host responsibility and re-export them from
`workflow/tests/fixtures.rs`, so behavior modules can continue using `super::*`
without duplicating setup.

## Alternatives Rejected
- Keep all fixtures in one file: rejected because the fixture boundary stayed
  large enough to obscure unrelated host behavior.
- Move fixtures to a crate-wide public test-support module: rejected because the
  helpers are still specific to workflow facade tests.

## Invariants
- Fixture modules remain private implementation details of `workflow::tests`.
- Behavior tests import fixtures only through the parent test module.
- Runtime capability helpers remain shared so backend alias expectations stay
  consistent across capacity, preflight, and execution tests.

## Revisit Triggers
- A second crate needs the same workflow facade fixtures.
- A fixture family grows large enough to need builder-level decomposition.
- Production host traits change in a way that makes shared fixture builders more
  stable than direct host structs.

## Dependencies
**Internal:** workflow host traits, workflow service DTOs, scheduler diagnostics
provider contracts, runtime capability DTOs, and technical-fit contracts.

**External:** none beyond the crate's existing test dependencies.

Reason: these fixtures support workflow facade tests only and should inherit the
parent test module dependency surface.

Revisit trigger: promote a fixture only when it becomes reusable outside
`workflow::tests`.

## Related ADRs
- `docs/adr/ADR-003-rust-workspace-policy.md`

## Usage Examples
Run the full workflow facade test module that consumes these fixtures:

```bash
cargo test -p pantograph-workflow-service workflow::tests
```

## Testing
```bash
cargo test -p pantograph-workflow-service workflow::tests
```
