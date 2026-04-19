# Plan: Managed Binary Cross-Platform

## Source Of Truth Status

This plan is now a historical precursor, not the active source of truth for
runtime redistributable work.

Use
`IMPLEMENTATION-PLAN-pantograph-runtime-redistributables-manager.md`
for new implementation. That newer plan supersedes this document because the
problem has expanded beyond cross-platform binary fetch/install into:

- backend-owned version catalogs and selected-version policy
- persistent install/download job state with restart reconciliation
- workflow and scheduler readiness enforcement
- explicit standards-compliance refactors for the immediate touched files
- GUI-facing runtime-manager projection contracts with Tauri kept adapter-only

This document remains useful only as a record of the narrower earlier framing:
shared binary orchestration plus `llama.cpp` and `Ollama` migration.

## Objective

Create a reusable managed-binary subsystem for cross-platform fetch, install,
validate, and launch workflows, then migrate `llama.cpp` and `Ollama` onto it
without breaking existing app-facing commands.

## Scope

### In Scope

- Generic managed-binary contracts and orchestration
- Thin per-platform adapters with one platform per file
- `llama.cpp` migration to the shared subsystem
- `Ollama` migration to the shared subsystem
- Process ownership, PID lifecycle, and overlap protection for managed launches
- Linux x86_64 and Windows x86_64 verification

### Out of Scope

- New model/runtime features unrelated to binary management
- UI redesign
- Non-binary runtime refactors outside fetch/install/launch boundaries

## Inputs

### Problem

The current cross-platform binary work is partial: `llama.cpp` has a thin
platform abstraction, but the overall binary-management flow is still
binary-specific and `Ollama` remains Linux-only and inconsistently integrated.

### Constraints

- Follow `CROSS-PLATFORM-STANDARDS.md`
- Follow `PLAN-STANDARDS.md`
- Follow `COMMIT-STANDARDS.md`
- Preserve public facades unless an explicit break is approved
- Required platforms remain Linux x86_64 and Windows x86_64

### Assumptions

- Managed binaries continue to be sourced from vendor GitHub releases
- Tauri command signatures should remain stable
- Managed installs remain under the app/runtime binary directory

### Dependencies

- Tauri runtime and command layer
- Existing `ProcessSpawner` abstraction
- CI support for Linux and Windows build verification

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Vendor archive layouts differ by binary/platform | High | Keep install rules in binary/platform adapters only |
| Launch/runtime env rules differ by platform | High | Keep env setup in thin platform launch adapters |
| Install/start overlap corrupts runtime state | High | Add per-binary transition locking and explicit ownership |
| Shared system drifts into `llama.cpp`-specific design | High | Require `Ollama` migration before considering architecture complete |

## Definition of Done

- `llama.cpp` and `Ollama` both use the same managed-binary subsystem
- Shared orchestration is binary-agnostic
- Platform logic is isolated to thin adapter files
- Linux x86_64 and Windows x86_64 verification exists
- Managed process lifecycle ownership is explicit and race-safe
- Public command facades remain stable

## Milestones

### Milestone 1: Shared Contracts

**Goal:** Define the reusable subsystem boundary and capture the implementation sequence.

**Tasks:**
- [ ] Add this implementation plan to the repo
- [ ] Introduce generic managed-binary contracts and registry boundaries
- [ ] Define lifecycle ownership for install/start/stop transitions

**Verification:**
- Architecture review against `CROSS-PLATFORM-STANDARDS.md`
- Commit message and scope follow `COMMIT-STANDARDS.md`

**Status:** In progress

### Milestone 2: Llama.cpp Migration

**Goal:** Move `llama.cpp` fetch/install/launch onto the shared subsystem.

**Tasks:**
- [ ] Extract shared download/extract/install helpers
- [ ] Re-home `llama.cpp` platform adapters under the managed-binary subsystem
- [ ] Keep `llama.cpp` commands/process spawning facade-stable

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Targeted tests for managed-binary helper logic

**Status:** Not started

### Milestone 3: Ollama Migration

**Goal:** Use the same subsystem for `Ollama` availability and install logic.

**Tasks:**
- [ ] Add `Ollama` binary definitions and per-platform adapters
- [ ] Replace binary-specific `Ollama` download/check code with shared orchestration
- [ ] Unify backend availability reporting with managed-binary state

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Targeted tests for binary resolution and validation

**Status:** Not started

### Milestone 4: Managed Launch Ownership

**Goal:** Close the loop on managed process startup and lifecycle ownership.

**Tasks:**
- [ ] Add managed launch resolution for `Ollama`
- [ ] Ensure `Ollama` startup/stop ownership is explicit and race-safe
- [ ] Document any residual best-effort macOS limitations

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Affected tests for launch resolution and process ownership

**Status:** Not started

## Execution Notes

Update during implementation:
- 2026-03-09: Plan created after reviewing current `llama.cpp` and `Ollama` binary handling.
- 2026-04-19: Marked historical-only after the broader
  `IMPLEMENTATION-PLAN-pantograph-runtime-redistributables-manager.md` plan
  became the active source of truth. Do not start new work from this file.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Keep commits focused on one milestone task or tightly related task group.
- Follow `COMMIT-STANDARDS.md`, including staged diff review and affected verification before commit.

## Re-Plan Triggers

- `Ollama` cannot fit without special-casing the shared core
- Windows verification cannot be added or maintained
- Public facade changes become necessary
- New launch/install race conditions change sequencing

## Recommendations

- Use `Ollama` as the second migration target because it is different enough
  from `llama.cpp` to prove the shared system is genuinely reusable.

## Completion Summary

### Completed

- N/A

### Deviations

- The newer runtime redistributables manager plan superseded this document's
  narrower scope before implementation proceeded from it.

### Follow-Ups

- Use `IMPLEMENTATION-PLAN-pantograph-runtime-redistributables-manager.md` for
  all future runtime redistributable implementation sequencing.

### Verification Summary

- N/A

### Traceability Links

- Module README updated: N/A
- ADR added/updated: N/A
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`

## Brevity Note

This plan stays in the repo as a concise historical precursor, but it is no
longer the active implementation plan.
