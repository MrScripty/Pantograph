# Pantograph Implementation Plan: Python Runtime Separation

## Status
Complete

## Current Source-of-Truth Summary

This document records the completed source of truth for Pantograph's Python runtime separation work. Pantograph no longer embeds Python
in-process by default, Python-backed nodes execute through a host-managed
process adapter, and dependency preflight guards now enforce readiness before
Python-backed execution proceeds.

## Objective
Decouple Pantograph's compiled/runtime dependency graph from Python so model-specific Python environments remain optional, arbitrary, and consumer-managed per model binding.

## Non-Goals
- No in-process Python embedding in Pantograph runtime.
- No assumption of a single global Python version/env.
- No fixed-arity metadata parsing assumptions.

## Standards Alignment
- Dependency feature-gating and lean library/application boundaries:
  - `DEPENDENCY-STANDARDS.md` (`Feature-Gate Heavy Optional Functionality`, `Keep Library Packages Lean`)
- Optional infrastructure must degrade gracefully:
  - `ARCHITECTURE-PATTERNS.md` (`Infrastructure Failure Recovery`, `Startup Resilience`)
- Foreign runtime separation from core/default test paths:
  - `LANGUAGE-BINDINGS-STANDARDS.md` (core compiles without foreign runtimes)
- Verification order and runtime validation:
  - `TESTING-STANDARDS.md` (`Verification Layers`)
- Commit formatting and atomic cadence:
  - `COMMIT-STANDARDS.md` (Conventional Commits, one logical change per commit)

## Execution Tasks

### 1. Remove default compile-time Python linkage from Pantograph app
- Update Tauri crate default features to exclude Python-backed backend features.
- Ensure default build path does not pull `pyo3`/`libpython`.
- Commit after completion:
  - `fix(build): remove python backends from default tauri features`

### 2. Route Python-backed workflow nodes through host boundary
- Ensure `pytorch-inference` and `audio-generation` are handled at host boundary (not core embedded path).
- While adapter wiring is in progress, return explicit actionable runtime errors instead of generic fallthrough.
- Commit after completion:
  - `refactor(workflow): move python-backed node handling to host boundary`

### 3. Introduce external Python runtime adapter (process-based)
- Add a host-side runtime adapter that executes Python workers out-of-process using interpreter paths resolved from dependency bindings/env registration.
- No shell-string interpolation; use structured command args.
- Preserve dynamic metadata behavior (unknown/additive fields tolerated).
- Commit after completion:
  - `feat(runtime): add external python process adapter`

### 4. Preflight and guardrails integration
- Enforce dependency-plan readiness and pinning (`unpinned_dependency`) before any runtime execution attempt.
- Ensure no install/execute action occurs for manual intervention states.
- Commit after completion:
  - `fix(deps): enforce pinning and readiness preflight in python runtime path`

### 5. Cross-platform path/process hardening
- Normalize and validate interpreter/script paths via path APIs.
- Ensure spaces-in-path and quoting-safe process invocation behavior.
- Commit after completion:
  - `fix(runtime): harden cross-platform python process path handling`

### 6. Verification and regression coverage
- Validate default binary has no `libpython` dependency.
- Add tests for host boundary behavior and explicit failure mapping.
- Add tests for dynamic metadata field tolerance for dependency payloads.
- Commit after completion:
  - `test(runtime): add python separation regression coverage`

### 7. Documentation and migration notes
- Document runtime separation model and operational requirements for model env provisioning.
- Include migration notes from in-process execution assumptions.
- Commit after completion:
  - `docs(runtime): document python environment separation and migration`

## Acceptance Criteria
1. Default Pantograph build/run works without Python runtime libraries installed.
2. Default Pantograph binary does not link `libpython`.
3. Python-backed model execution is host-managed via external envs (no in-process embedding).
4. Unpinned/unready dependency bindings deterministically block execution with clear codes/messages.
5. Metadata consumption remains dynamic and tolerant of additive/unknown fields.


## Completion Summary

### Completed

- Default Pantograph builds no longer require in-process Python linkage.
- Python-backed nodes execute through `ProcessPythonRuntimeAdapter`.
- Dependency readiness and pinning guards now gate Python-backed execution.
- Runtime separation and migration guidance is documented in
  `docs/python-runtime-separation.md`.

### Follow-Ups

- Later environment-management enhancements belong in the dependency execution
  plans rather than reopening this completed runtime-separation baseline.
