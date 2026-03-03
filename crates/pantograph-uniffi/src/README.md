# crates/pantograph-uniffi/src

## Purpose
UniFFI adapter surface for Pantograph workflow APIs.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | UniFFI exports and adapter implementation delegating to shared service contracts. |
| `bin/` | Binding generation helper utilities. |

## Headless Workflow Exports

- `workflow_run(base_url, request_json, pumas_api?) -> response_json`
- `workflow_get_capabilities(base_url, request_json, pumas_api?) -> response_json`

These exports delegate business rules to `pantograph-workflow-service`.

## Dependencies
- Internal: `pantograph-workflow-service`, `node-engine`.
- Host/runtime: `reqwest`, optional `pumas-library`.

## Notes

- Adapter validates workflow existence + logical graph validity.
- Embedding payload parsing is strict (no silent vector truncation).
- Model signature uses deterministic model hash selection (`sha256` then `blake3`) when Pumas metadata is available.
