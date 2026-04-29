# 2026-04-29 Worker Report: UniFFI Session Request Repair

## Scope

Owned write set: `crates/pantograph-uniffi/`.

Repair UniFFI runtime tests after the workflow-service session-run contract
started requiring `workflow_semantic_version`.

## Changes

- Added `workflow_semantic_version` to execution-session run JSON fixtures.
- Replaced the invalid graph-save workflow identity containing spaces with a
  validated Pantograph workflow identity.
- Updated assertions to match the corrected persisted workflow identity.

## Verification

Passed:

- `cargo test -p pantograph-uniffi`

