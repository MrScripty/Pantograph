# crates/pantograph-timing-contracts

## Purpose
This crate owns Pantograph timing attempt contracts shared by runtime,
workflow, scheduler, and diagnostics producers. It exists so lower-level
runtime crates can emit the same timing attempt ids and checked duration
diagnostics as workflow-service without depending on workflow orchestration.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `src/lib.rs` | Canonical timing attempt ids, attribution, checked duration semantics, and timing diagnostic DTOs. |

## Invariants
- Timing attempt ids use the canonical `timing_attempt_` prefix.
- Duration arithmetic is checked. Impossible timestamp order returns typed
  diagnostics instead of saturating or normalizing the value.
- This crate must stay free of workflow-service, inference-backend, scheduler,
  Tauri, and diagnostics-ledger dependencies.
- Producers may attach attribution fields that are available at their layer,
  but missing attribution must not be filled with guessed fallback values.

## Revisit Triggers
- Timing baseline/deviation enforcement moves from plan follow-up into code.
- Timing diagnostics need durable ledger-specific payloads that cannot remain
  host-agnostic.
