# Standards Compliance Analysis

## Purpose
This directory records the iterative standards-compliance audit for Pantograph.
It separates raw pass findings from the final layered refactor plan so later
work can resolve issues without losing the evidence behind each recommendation.

## Contents
| File | Description |
| ---- | ----------- |
| `pass-01-inventory-and-documentation.md` | File-size, directory README, generated artifact, and documentation-shape findings. |
| `pass-02-architecture-and-boundaries.md` | Layering, ownership, frontend/backend contract, and adapter-surface findings. |
| `pass-03-runtime-security-and-concurrency.md` | Boundary validation, listener, process lifecycle, spawn, and runtime safety findings. |
| `pass-04-tooling-tests-release-and-dependencies.md` | Quality gate, CI, test, dependency ownership, launcher, and release findings. |
| `pass-05-updated-rust-and-standards-delta.md` | April 21 standards delta, Rust-specific workspace, binding, async, release, and tooling findings. |
| `refactor-plan.md` | Consolidated multi-layer refactor plan derived from all passes. |

## Problem
Pantograph spans Rust crates, a Tauri desktop app, Svelte packages, Python
workers, and host-language bindings. A single standards pass hides overlapping
issues, so this directory keeps each audit pass independently reviewable.

## Constraints
- Findings must leave unrelated dirty work untouched.
- Generated, vendored, and build output directories are evidence only when they
  affect source-root policy or tooling behavior.
- The plan must prefer existing backend-owned workflow-service boundaries over
  broad rewrites.

## Decision
Use one markdown file per pass and one consolidated plan. When the external
standards change, add a delta pass instead of rewriting earlier evidence. This
keeps raw findings stable while allowing the plan to be revised as overlapping
issues are resolved.

## Alternatives Rejected
- One large audit document: rejected because cross-standard overlaps would be
  difficult to trace back to the pass that found them.
- Inline TODOs in source files: rejected because the user asked for planning
  artifacts, not code changes.

## Invariants
- Every finding should identify the affected path or subsystem.
- Remediation should be sequenced by dependency order, not by file order.
- Additional non-standards issues stay recorded even when they are outside the
  compliance refactor.

## Revisit Triggers
- Any standards document in the external Coding-Standards directory changes.
- A compliance milestone removes or substantially reshapes a hotspot.
- CI or local verification output changes the risk ranking.

## Dependencies
**Internal:** `docs/`, `src/`, `src-tauri/`, `crates/`, `packages/`, `bindings/`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.

## Related ADRs
- `None identified as of 2026-04-21.`
- `Reason: This directory records audit evidence, not a new runtime architecture decision.`
- `Revisit trigger: A compliance milestone chooses a new cross-package architecture boundary.`

## Usage Examples
Start with `refactor-plan.md` for execution order. Use the pass files when a
milestone needs exact source evidence or when a finding must be reclassified.

## API Consumer Contract
- None.
- Reason: These files are human-readable planning artifacts and expose no runtime API.
- Revisit trigger: A tool starts parsing these files as machine-readable issue input.

## Structured Producer Contract
- None.
- Reason: This directory does not generate or publish structured machine-consumed artifacts.
- Revisit trigger: The refactor tracker is converted into JSON, YAML, or issue-sync metadata.
