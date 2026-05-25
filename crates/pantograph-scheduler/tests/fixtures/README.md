# pantograph-scheduler test fixtures

## Purpose
This directory contains canonical JSON examples for scheduler contracts that
can be persisted, transported, or mirrored by other Pantograph layers. The
fixtures make scheduler DTO shape changes visible in review and keep contract
tests grounded in machine-consumed payloads instead of Rust-only constructors.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `schedulable_task_intent_valid.json` | Path-free workflow task intent with canonical Pumas model identity. |
| `capability_hint_snapshot_valid.json` | Backend-owned graph/editor hint payload for runtime, device, and trait options. |
| `readiness_admission_decision_ready.json` | Ready dependency admission result with scheduler-owned readiness proof. |
| `runtime_handoff_readiness_admitted.json` | Runtime-host handoff before dispatch-selected execution facts are attached. |
| `dispatch_decision_valid.json` | Scheduler-selected runtime/device/model/reservation/batch decision payload. |
| `dispatch_selection_request_valid.json` | Scheduler-owned dispatch-selection request with one typed candidate and real reservation/resource facts. |
| `resource_residency_snapshot_valid.json` | Platform-neutral resource observation and fit-assessment snapshot. |
| `batch_policy_decision_valid.json` | Compatible batch-group decision across scheduler task candidates. |

## Problem
Scheduler contracts cross process, persistence, graph/editor, runtime host, and
diagnostics boundaries. Fixtures provide stable examples for those consumers
and prevent accidental shape drift, especially around no-legacy requirements
such as rejecting `ModelRefV2`, local paths, and worker launch facts.

## Constraints
- Fixtures must remain deterministic and must not reference local user paths,
  installed runtime binaries, GPU ids that require local hardware, or Pumas
  library internals outside typed references.
- Fixtures must use public serde field names and contract versions from
  `pantograph-scheduler`.
- Fixture changes must land with matching source, tests, README, and plan
  notes because they represent structured contract changes.
- Fixtures should model valid canonical shapes; malformed payloads should be
  constructed in focused tests unless a persisted invalid-shape fixture is
  explicitly useful.

## Decision
Keep one compact valid fixture per scheduler contract family. Negative tests
mutate those fixtures or construct focused invalid DTOs so each rejection case
stays easy to read.

## Alternatives Rejected
- Store expected JSON inline in every test: rejected because it hides shared
  contract changes across test files.
- Generate fixtures during tests: rejected because generated output would make
  persisted field changes less visible in review.
- Add compatibility fixtures for retired path-shaped success data: rejected
  because Milestone 5a and Milestone 5b require replacement and deletion, not
  legacy acceptance.

## Invariants
- No fixture may carry executable Pumas load targets, local filesystem paths,
  `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
  `modelPath`, or worker launch details.
- Every fixture must decode through a public scheduler DTO and validate through
  its matching `Validated*` wrapper where one exists.
- Fixture ids must be stable, human-readable contract examples rather than
  random values.
- Fixture updates must preserve explicit contract versions.

## Revisit Triggers
- A scheduler DTO gains migration support and requires old-version fixtures.
- Generated language bindings begin consuming these fixtures as golden files.
- Runtime host acceptance tests need separate end-to-end fixtures that include
  host-local, Pumas-approved load-target resolution after scheduler dispatch.

## Dependencies
**Internal:** `pantograph-scheduler` public serde DTOs and validation wrappers.

**External:** None beyond the test crate's existing `serde_json` use.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
use pantograph_scheduler::{
    SchedulerDispatchDecision, ValidatedSchedulerDispatchDecision,
};

let decision: SchedulerDispatchDecision = serde_json::from_str(include_str!(
    "dispatch_decision_valid.json"
))?;
let _validated = ValidatedSchedulerDispatchDecision::try_from(decision)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## API Consumer Contract
- Inputs: checked-in JSON fixture files loaded by scheduler integration tests
  or future binding/golden-file checks.
- Outputs: representative canonical payloads for scheduler DTO consumers.
- Lifecycle: fixtures are immutable during a test run and may be read in any
  order.
- Errors: invalid fixture decoding or validation is a contract failure.
- Compatibility: changing a fixture field name, enum value, or required field
  is a public contract change and must be paired with migration planning when
  persisted consumers exist.

## Structured Producer Contract
- Stable fields are the serialized field names, contract versions, enum labels,
  and typed id formats expected by public scheduler DTOs.
- Optional fields may be omitted only when omission is an intended public
  default.
- Array ordering is stable only where the corresponding DTO documents ordered
  semantics.
- New fixtures must be minimal canonical payloads that avoid local environment
  assumptions.
- Retired fields must be removed rather than preserved as compatibility aliases.
