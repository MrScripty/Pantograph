# Inference Execution Boundary Contracts

## Purpose

This directory contains the staged plan for reshaping Pantograph inference
around Pumas-resolved model sources, Transformers-aligned task/package
semantics, explicit backend compatibility facts, and durable diagnostics
integration.

## Contents

| File | Description |
| ---- | ----------- |
| `plan.md` | Main Pantograph inference boundary plan covering inference contracts, backend mapping, workflow migration, managed dependencies, diagnostics-ledger integration, consumer guardrails, and implementation blast-radius controls. |
| `pumas-library-plan.md` | Pantograph-side mirror of the Pumas Library producer plan covering model identity, artifact/package facts, task evidence, generation defaults, backend hints, custom-code/security facts, legacy reference resolution, fast selector snapshots, and explicit owner/client/read-only access roles. |

## Problem

The inference redesign crosses Pantograph and Pumas responsibilities. Keeping
all implementation detail in one file made the inference plan carry Pumas
library implementation concerns that should be owned by the Pumas repository.

## Constraints

- `plan.md` remains the Pantograph inference source of truth.
- `pumas-library-plan.md` records the Pumas-side producer requirements and the
  settled implementation baseline Pantograph consumes; active Pumas
  implementation details live in the Pumas repository.
- Cross-repo DTOs and fixtures must stay synchronized before implementation.
- Implementation slices must declare blast radius, write sets, serial ownership
  points, and verification gates before code changes.
- Pumas must not inherit inference execution, scheduler, or diagnostics-ledger
  write policy.

## Decision

Keep the main inference plan and the Pumas Library plan in the same plan family,
but split their implementation responsibilities into separate Markdown files.
The main plan links to the Pumas plan where package-fact or selector details
are required. The settled Pumas fast selector implementation lives at:

```text
/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library/docs/plans/pumas-fast-model-snapshot-and-client-api/
```

## Alternatives Rejected

- Keep Pumas details inside `plan.md`.
  Rejected because it blurs ownership between Pantograph inference and Pumas
  model-library work.
- Move every Pumas detail out of this plan family.
  Rejected because Pantograph still needs a local consumer-facing mirror of the
  Pumas producer baseline, but the authoritative implementation details now
  live in the Pumas repository.

## Invariants

- Inference consumes Pumas facts; it does not own Pumas indexing/import/storage
  policy.
- Pumas supplies model/package facts; it does not choose Pantograph runtime
  scheduling policy.
- Diagnostics-ledger writes remain in workflow-service/node-execution
  boundaries, not inference or Pumas.

## Revisit Triggers

- Pumas producer contracts change after the fast selector baseline.
- Cross-repo fixtures drift between Pantograph and Pumas.
- A new shared contract crate or schema location becomes necessary.

## Dependencies

**Internal:** `plan.md`, `pumas-library-plan.md`, source-directory READMEs, and
future implementation fixtures.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
and `/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library`.
The active Pumas fast selector implementation baseline is under
`docs/plans/pumas-fast-model-snapshot-and-client-api/` in the Pumas repository.

## Related ADRs

- `None identified as of 2026-05-02.`
- `Reason: This is a staged implementation plan split, not a finalized runtime
  architecture decision by itself.`
- `Revisit trigger: accepting the Pumas/Pantograph package-facts contract as a
  stable cross-repo architecture boundary.`

## Usage Examples

Start with the main inference plan:

```text
docs/plans/inference-execution-boundary-contracts/plan.md
```

Use the Pumas split plan when implementing model-library package facts:

```text
docs/plans/inference-execution-boundary-contracts/pumas-library-plan.md
```

## API Consumer Contract

- This directory exposes human-readable implementation plans, not runtime APIs.
- Implementers should treat `plan.md` as the Pantograph-side entry point and
  `pumas-library-plan.md` as the Pumas-side detail plan.
- Cross-repo DTO or fixture changes must update both relevant plans or record a
  re-plan trigger.

## Structured Producer Contract

- Stable artifact category: Markdown implementation plans.
- File names are lowercase, hyphen-separated slugs.
- These files are manually maintained and are not generated from schemas.
- The Pumas split plan must stay linked from the main inference plan whenever
  Pumas package-fact requirements affect Pantograph inference contracts.
