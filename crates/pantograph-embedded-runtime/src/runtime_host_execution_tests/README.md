# runtime_host_execution_tests

## Purpose
This directory contains structured fixtures for the runtime-host execution
contract tests. The fixtures prove Milestone 5b starts with a host-facing
request/response boundary that consumes scheduler handoff facts without
reintroducing legacy model paths.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `fixtures/runtime_host_execution_request_dispatch_selected.json` | Canonical request carrying a dispatch-selected scheduler handoff. |
| `fixtures/runtime_host_execution_response_accepted.json` | Canonical accepted response with typed host diagnostics. |

## Problem
Runtime-host execution is the replacement path for old `model_path` and
`ModelRefV2` successful execution. Fixtures make the new boundary reviewable
before any Pumas load-target service or runtime migration is wired.

## Constraints
- Fixtures must not include executable Pumas load targets, local paths,
  `ModelRefV2`, `model_path`, frontend `modelPath`, or worker launch details.
- Request fixtures must include scheduler dispatch decisions rather than
  readiness-only handoff.
- Response fixtures must use typed diagnostics instead of free-form status
  strings.

## Decision
Keep runtime-host fixtures next to the module tests so they can evolve with
the embedded-runtime host boundary while the scheduler crate remains the owner
of scheduler DTO fixtures.

## Alternatives Rejected
- Reuse scheduler fixtures directly as runtime-host fixtures: rejected because
  runtime-host request/response contracts need their own envelope.
- Add path-shaped compatibility fixtures: rejected because successful legacy
  path behavior is a deletion target, not an accepted contract.

## Invariants
- Runtime-host request fixtures consume `SchedulerRuntimeHandoff`; they do not
  define scheduler decisions themselves.
- Runtime-host response fixtures carry correlation and diagnostics only.
- Any fixture shape change must update module tests, README notes, and the
  Milestone 5b plan status in the same slice.

## Revisit Triggers
- Runtime-host execution begins returning durable output artifact references.
- Pumas load-target resolution introduces a separate host-only executable-fact
  contract.

## Dependencies
**Internal:** `runtime_host_execution.rs` and `pantograph-scheduler` DTOs.

**External:** `serde_json` through test code only.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```bash
cargo test -p pantograph-embedded-runtime runtime_host_execution
```

## API Consumer Contract
- Inputs: JSON fixtures decoded through public runtime-host DTOs.
- Outputs: validation success or typed `RuntimeHostExecutionContractError`.
- Lifecycle: fixtures are immutable during tests and may be read in any order.
- Errors: expected failures assert typed contract errors where validation is
  performed after serde decoding.
- Compatibility: fixture changes are host-facing contract changes.

## Structured Producer Contract
- Stable fields are the serialized field names, contract version, enum labels,
  and scheduler handoff nesting used by runtime-host DTOs.
- Optional fields may be omitted only when the DTO documents the omission as a
  public default.
- Fixture order is not a contract.
- Retired path-shaped fields must be removed rather than preserved as aliases.
