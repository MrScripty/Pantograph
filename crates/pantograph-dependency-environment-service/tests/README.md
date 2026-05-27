# Tests

## Purpose
This directory contains public API tests for the dependency-environment service
crate.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `service_contract.rs` | Contract tests for validated service output and provider errors. |

## Problem
The service boundary must prove it can consume validated requests and reject
invalid provider output before workflow-service wiring depends on it.

## Constraints
- Tests must not call Pumas, inspect model files, install packages, or use
  legacy dependency resolvers.
- Tests may reuse shared dependency-planning JSON fixtures as contract inputs.
- Durable state is not used in this slice.

## Decision
Use focused integration tests through the public crate API. This catches public
export and dependency-shape regressions without coupling tests to private
helpers.

## Alternatives Rejected
- Unit-test only inside `src/lib.rs`: rejected because public API integration
  tests better match how workflow-service will consume the crate.

## Invariants
- Not-implemented output is validated and diagnostic.
- Invalid provider output is rejected by the service facade.
- The service API does not expose model paths or local load paths.

## Revisit Triggers
- Production Pumas provider tests are added.
- Workflow-service vertical slice starts using the crate.

## Dependencies
**Internal:** `pantograph-dependency-environment-service`,
`pantograph-dependency-planning`.

**External:** `serde_json` for fixture decoding.

## Related ADRs
- None identified as of 2026-05-26.
- Reason: The active plan records this boundary before production provider
  wiring.
- Revisit trigger: Provider lifecycle ownership changes.

## Usage Examples
```bash
cargo test -p pantograph-dependency-environment-service
```

## API Consumer Contract
- Tests exercise the public service facade exactly as a backend composition
  caller would.
- Provider output must be validated before crossing back to callers.
- Errors remain typed and inspectable.

## Structured Producer Contract
- Test fixtures model shared dependency-planning request/result contracts.
- Fixture shape changes must be coordinated with dependency-planning DTOs.
