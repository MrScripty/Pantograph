# runtime-host execution fixtures

## Purpose
This directory contains JSON fixtures for the shared runtime-host execution
contract. They are checked-in examples of the scheduler-to-runtime-host
boundary and must not include legacy successful execution fields.

## Contents
| File | Description |
| ---- | ----------- |
| `runtime_host_execution_request_dispatch_selected.json` | Valid host execution request with a dispatch-selected scheduler handoff. |
| `runtime_host_execution_response_accepted.json` | Valid host response that acknowledges the request with typed diagnostics. |
| `runtime_host_execution_response_completed_outputs.json` | Valid completed host response with typed, path-free output values and terminal metadata. |

## Contract
- Fixtures must not contain local paths, executable load targets, `ModelRefV2`,
  `model_path`, frontend `modelPath`, or worker launch internals.
- Request fixtures must use scheduler-owned handoff and dispatch-decision
  payloads as nested contracts.
- Response fixtures must use typed state, diagnostic enums, bounded typed
  outputs, and path-free terminal metadata.
- Retired path-shaped fields must not be reintroduced as aliases.
