# crates/pantograph-rustler/src

## Purpose
Rustler NIF adapter surface for Pantograph workflow APIs.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | NIF implementations, resources, and adapter logic delegating to shared service contracts. |

## Headless Workflow NIFs

- `workflow_run/3`
- `workflow_get_capabilities/3`

These NIFs delegate business rules to `pantograph-workflow-service`.

## Dependencies
- Internal: `pantograph-workflow-service`, `node-engine`.
- Host/runtime: `reqwest`, optional `pumas-library`.

## Notes

- Adapter validates workflow existence + logical graph validity.
- Embedding payload parsing is strict (no silent vector truncation).
- Model signature uses deterministic model hash selection (`sha256` then `blake3`) when Pumas metadata is available.
