# 09: Stage-End Refactor Gate

## Purpose

Define the instruction set used at the end of each implementation stage to
decide whether a standards refactor is warranted before the next stage begins.

This is not a standing refactor plan. It is a repeatable gate for keeping the
codebase at a standards-compliant starting point between execution-platform
stages.

## Scope

In scope:

- files touched by the implementation stage being closed
- tests, docs, configs, generated artifacts, and build metadata touched by that
  same stage
- standards issues introduced or exposed by those touched files
- small in-scope refactors needed to make touched files compliant before the
  next stage begins

Out of scope:

- full-codebase standards refactors
- opportunistic cleanup in untouched files
- refactors that require changing ownership boundaries beyond the stage's
  touched files
- launching a broad refactor without first creating a dedicated
  `../../refactors/<refactor-slug>/` plan

## Source Prompt

This gate is adapted from:

`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/prompts/full-codebase-standards-refactor.md`

The full prompt remains the pattern for broad planning-only standards refactors.
This gate deliberately narrows the process to stage-touched files.

## When To Run

Run this gate after each numbered implementation stage:

- after the stage's intended implementation and required tests are complete
- before starting the next numbered execution-platform stage
- before declaring the stage complete in PR notes or implementation reports

## Inputs

- the stage plan file being closed
- the standards directory:
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
- the touched-file list for the stage, preferably from the stage branch or
  implementation range
- the verification commands and results for the stage
- any implementation notes, known tradeoffs, or skipped checks

## Touched-File Boundary

The refactor candidate set is limited to files touched during the stage.

Acceptable ways to determine the set include:

```bash
git diff --name-only <stage-base>...HEAD
git diff --name-only --cached
git status --short
```

If a standards issue clearly requires changing untouched files, do not silently
expand the refactor. Record the issue as out of scope and create or update a
dedicated refactor plan under `../../refactors/<refactor-slug>/`.

## Decision Flow

1. Gather touched files and group them by code area.
2. Map applicable standards to the touched files.
3. Check whether stage changes introduced or exposed standards drift in those
   files.
4. Decide one outcome:
   - `not_warranted`: touched files satisfy applicable standards, or remaining
     issues are unrelated and outside the touched-file boundary
   - `in_scope_refactor`: touched files need a contained standards cleanup that
     can be completed before the next stage
   - `separate_refactor_plan_required`: standards issues exceed the touched-file
     boundary, require broader architectural sequencing, or need concurrent
     worker coordination
5. Record the outcome in the stage implementation notes, PR notes, or a
   stage-specific refactor report.

## Standards Passes

Use the standards categories as a combined constraint set rather than isolated
checklists.

Required pass groups:

- Planning and documentation: plan updates, README coverage, ADR needs, public
  contract docs, changelog or migration notes.
- Architecture and coding: layer ownership, backend-owned data, file-size
  decomposition, service boundaries, validation at boundaries.
- Rust API and async: validated domain types, structured errors, feature
  contracts, task ownership, cancellation, shutdown, panic policy.
- Testing and tooling: unit/integration/host tests, fixture validation,
  formatter/lint/typecheck commands, feature checks, audit expectations.
- Security and dependencies: credential/path/payload validation, bounded
  queues, listener exposure, dependency owner and transitive cost.
- Interop and bindings: FFI safety, unsafe isolation, serialization shape,
  generated binding drift, native/host artifact version matching.
- Frontend and accessibility when UI files are touched: semantic controls,
  keyboard access, accessible names, backend-owned state rendering.
- Release and cross-platform when public APIs or artifacts are touched:
  target coverage, artifact naming, checksums, SBOM expectations, migration
  notes.

## In-Scope Refactor Rules

When the outcome is `in_scope_refactor`:

- refactor only files in the touched-file set
- preserve the behavior delivered by the stage unless the stage plan explicitly
  calls for a behavior change
- solve overlapping standards issues together instead of making isolated
  cosmetic edits
- update or add tests that validate the standards-sensitive behavior
- keep public facades stable unless the stage plan includes a migration path
- avoid broad dependency additions; if required, record dependency owner,
  feature gating, audit impact, and release impact
- run the stage's verification commands again after the refactor

## Separate Refactor Plan Rules

When the outcome is `separate_refactor_plan_required`, create Markdown planning
artifacts under:

`../../refactors/<refactor-slug>/`

Use the broad refactor prompt's artifact layout when warranted:

- `pass-instructions/`
- `findings/`
- `implementation-waves/`
- `reports/`
- `coordination-ledger.md`
- `final-plan.md`

The separate plan must state why the issue cannot be safely handled within the
stage-touched-file boundary.

## Required Report Shape

Record at least:

- stage name and plan file
- touched-file source command or method
- touched files reviewed
- applicable standards groups
- outcome: `not_warranted`, `in_scope_refactor`, or
  `separate_refactor_plan_required`
- findings and decisions
- files changed by any in-scope refactor
- verification commands run after the decision or refactor
- residual risks or separate refactor links

## Verification

- The touched-file set is recorded.
- Applicable standards groups were checked against the touched files.
- Any refactor stayed inside the touched-file boundary.
- Verification was re-run after an in-scope refactor.
- Out-of-scope standards issues were not hidden; they were recorded as separate
  refactor-plan candidates.

## Risks And Mitigations

- Risk: the gate becomes an excuse for broad opportunistic cleanup. Mitigation:
  enforce the touched-file boundary and require a separate refactor plan for
  broader work.
- Risk: implementers skip the gate when tests pass. Mitigation: make the gate a
  completion criterion in every numbered implementation stage.
- Risk: known standards drift remains in touched files because it predates the
  stage. Mitigation: if the file is touched, bring the touched file to a
  compliant baseline or explicitly record why a separate plan is required.
- Risk: refactors change behavior at the end of a stage. Mitigation: preserve
  behavior unless the stage plan includes the behavior change and rerun
  verification after cleanup.

## Re-Plan Triggers

- The touched-file set reveals standards issues that require changes in
  untouched files.
- Refactor work needs concurrent implementation slices or cross-boundary
  ownership changes.
- Verification after an in-scope refactor fails in ways unrelated to the stage.
- A standards file changes and alters the gate for future stages.

## Completion Criteria

- Every implementation stage closes with a recorded gate outcome.
- In-scope refactors are completed and verified before the next stage begins.
- Broader refactor pressure is moved into a dedicated `../../refactors/` plan
  instead of being mixed into the next feature stage.
