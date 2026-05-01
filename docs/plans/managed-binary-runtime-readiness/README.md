# Managed Binary Runtime Readiness Plan

## Purpose

This directory contains the implementation plan for making Pantograph managed
binary state the single source of truth for all product-managed binaries,
including inference runtime sidecars, media tool binaries, and native
redistributable artifacts used by workflow execution and conversion paths.

## Contents

| File | Description |
| ---- | ----------- |
| `plan.md` | Ordered implementation plan for managed binary source-of-truth cleanup across runtime sidecars, media tools, native artifacts, and real llama.cpp model-load verification. |

## Problem

Workflow submission can now resolve saved workflow IDs, but llama.cpp-backed
runs still fail after a short scheduler model lifecycle event that reports
model load completion in roughly 17 ms. That timing indicates the scheduler is
accepting a readiness projection before the requested llama.cpp process and
model are proven loaded.

## Constraints

- Backend Rust owns managed binary truth across runtime sidecars, media tools,
  and native redistributable artifacts.
- Workflow, scheduler, diagnostics, and Tauri process launch must consume the
  same managed binary facts.
- Existing managed runtime installs and state files must be migrated or
  resolved without losing user-installed binaries.
- Runtime startup events must distinguish process spawn, HTTP readiness, and
  requested model readiness.

## Decision

Use `plan.md` as the active execution plan. The plan introduces a
category-aware managed binary facade above the existing
`crates/inference::managed_runtime` and
`crates/inference::managed_redistributables` boundaries, so inference runtimes,
media tools such as `ffmpeg`, and native artifacts such as OpenColorIO/OCIO
remain distinct underneath one discoverable backend-owned source of truth.

## Alternatives Rejected

- Patch scheduler diagnostics only.
  Rejected because it would hide the false-positive model-load event without
  fixing runtime launch or binary resolution.
- Add a second llama.cpp-specific readiness check in Tauri.
  Rejected because ADR-003 requires backend-owned managed-runtime truth and
  Tauri must remain an adapter.

## Invariants

- A scheduler `model load completed` event means the requested runtime process
  is alive, the requested model path is active, and the backend is ready to
  serve that model.
- Managed runtime sidecars are resolved through backend managed-runtime
  contracts, not direct host PATH assumptions except where a definition
  explicitly declares system-command precedence.
- Media redistributables such as `ffmpeg`, `ocioconvert`, `oiiotool`, and
  OpenColorIO/OCIO artifacts stay separate from inference runtime sidecars
  while sharing discoverable managed-binary views where useful.

## Revisit Triggers

- A third managed runtime family is added.
- llama.cpp upstream changes readiness semantics or server endpoints.
- Runtime install paths move again or require state migration beyond path
  fallback resolution.

## Dependencies

**Internal:** `crates/inference::managed_runtime`,
`crates/inference::managed_redistributables`,
`crates/pantograph-embedded-runtime`, `src-tauri/src/llm/process_tauri.rs`,
workflow scheduler diagnostics, and Settings managed runtime UI.

**External:** GitHub release assets for llama.cpp/Ollama and local filesystem
state under Pantograph app data.

## Related ADRs

- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`
- `docs/adr/ADR-007-managed-runtime-observability-ownership.md`

## Usage Examples

Start implementation from:

```text
docs/plans/managed-binary-runtime-readiness/plan.md
```

## API Consumer Contract

- Human implementers consume this plan as staged instructions.
- Before editing, verify current dirty source files are committed, stashed, or
  explicitly assigned to the work.
- Verification commands and deviations must be recorded in the plan during
  execution.

## Structured Producer Contract

- Stable artifact category: Markdown implementation plan.
- `plan.md` must preserve objective, scope, inputs, milestones, verification,
  risks, re-plan triggers, and completion criteria.
- This plan is manually maintained and is not generated from schemas.
