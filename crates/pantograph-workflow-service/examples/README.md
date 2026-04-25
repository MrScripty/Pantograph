# pantograph-workflow-service/examples

Executable examples for direct Rust workflow-service consumers.

## Purpose
This directory shows how a Rust host implements `WorkflowHost` and calls the
workflow service without Tauri or binding frameworks. Examples are intentionally
small and should demonstrate public API usage rather than product-specific
runtime composition.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `rust_host_workflow_run.rs` | Minimal fake-host scheduler session-run example that prints a serialized response. |

## Problem
The workflow service is host-agnostic, but consumers still need a concrete
starting point for implementing host traits and request DTOs. Without examples,
callers may copy adapter-specific code and accidentally inherit transport
concerns.

## Constraints
- Examples must compile against the public crate API.
- Examples should not depend on desktop, UniFFI, Rustler, or frontend runtime
  wiring.
- Example hosts may fake runtime facts but should keep shapes realistic.
- Examples must not become the canonical workflow engine or policy source.

## Decision
Keep runnable Rust examples here. They should demonstrate public API ergonomics
and stay smaller than integration tests or product runtime adapters.

## Alternatives Rejected
- Put examples only in README snippets: rejected because snippets can drift
  without compiler coverage.
- Point users at Tauri commands as examples: rejected because Tauri adds app
  state and transport details that are irrelevant to direct Rust consumers.

## Invariants
- Examples are consumers of `pantograph-workflow-service`, not owners of service
  behavior.
- Host fake data should preserve realistic runtime and model fields.
- Any copied pattern from an example must still satisfy production lifecycle
  and error-handling requirements.

## Revisit Triggers
- A second host style needs a dedicated example.
- Public service construction changes.
- Runtime lifecycle supervision changes the recommended host pattern.

## Dependencies
**Internal:** `pantograph-workflow-service`.

**External:** `tokio`, `async-trait`, `serde_json`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```bash
cargo run -p pantograph-workflow-service --example rust_host_workflow_run
```

## API Consumer Contract
- Inputs: public request DTOs and an implementation of `WorkflowHost`.
- Outputs: public workflow-service response DTOs.
- Lifecycle: examples create short-lived service and host values for one run.
- Errors: examples propagate public workflow-service errors through the Rust
  error boundary.
- Versioning: examples should be updated with public API changes in the same
  implementation slice.

## Structured Producer Contract
- None.
- Reason: examples do not publish generated artifacts or stable machine-read
  manifests.
- Revisit trigger: examples start producing fixture files or documented output
  snapshots consumed by tests.

## Testing
```bash
cargo run -p pantograph-workflow-service --example rust_host_workflow_run
```

## Notes
- Keep examples concise; larger behavior coverage belongs in `tests/`.
