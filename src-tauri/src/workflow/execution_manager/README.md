# src-tauri/src/workflow/execution_manager

## Purpose
This directory contains focused helper modules behind the public
`workflow::execution_manager` facade. It exists to keep execution-state
lifecycle, undo/redo state, and later checkpoint-transport scaffolding out of
the monolithic `execution_manager.rs` entrypoint while preserving the current
Tauri-owned execution-manager API.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `state.rs` | Focused execution-state lifecycle owner for one Tauri execution handle, including undo/redo state projection and graph snapshot restore helpers. |

## Problem
`execution_manager.rs` currently owns both the manager facade and the per-
execution state implementation. Phase 6 will need a Tauri-side checkpoint
transport boundary, and letting that work accumulate in one file would push the
module back toward the same oversized insertion-point problem already being
reduced elsewhere in the workflow stack.

## Constraints
- Tauri remains a transport/composition boundary and must not become the owner
  of backend workflow-session semantics.
- `workflow::execution_manager` remains the stable public entrypoint for callers.
- Execution-state lifecycle, undo/redo state, and later checkpoint transport
  helpers must remain thin wrappers around backend-owned `node-engine`
  behavior rather than independent policy owners.

## Decision
Move the per-execution state implementation into focused helper modules under
`execution_manager/` while leaving `execution_manager.rs` as the public manager
facade. This keeps the current surface stable and creates a standards-compliant
insertion point for later Phase 6 checkpoint and restore transport work.

## Alternatives Rejected
- Continue adding checkpoint-oriented helper logic directly to
  `execution_manager.rs`.
  Rejected because the file already mixes manager and state concerns.
- Move execution management into backend crates.
  Rejected because Tauri still owns host-local execution handles and app-state
  composition.

## Invariants
- `ExecutionManager` remains the public Tauri-facing execution-manager facade.
- `ExecutionState` remains a thin owner of one `WorkflowExecutor`, undo/redo
  stack, and lifecycle timestamps.
- Undo/redo restore behavior remains delegated to backend-owned
  `WorkflowExecutor::restore_graph_snapshot`.

## Revisit Triggers
- Phase 6 checkpoint transport needs more than one focused helper module under
  this directory.
- The Tauri execution manager needs a reusable host implementation outside the
  desktop app boundary.
