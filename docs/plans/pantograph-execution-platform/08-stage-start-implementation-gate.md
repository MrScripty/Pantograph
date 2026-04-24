# 08: Stage-Start Implementation Gate

## Purpose

Define the instruction set used before beginning any numbered implementation
stage. The gate makes the selected stage plan, standards, worktree state,
scope, verification, and commit expectations explicit before source files are
edited.

This gate is not an implementation plan by itself. It is the readiness check
that must pass before executing a stage plan.

## Scope

In scope:

- the numbered stage plan selected for implementation
- standards referenced by that plan and by this execution-platform plan set
- current git status and worktree hygiene
- implementation sequence, verification criteria, commit boundaries, and
  expected report updates
- existing dirty files that could overlap with the stage write set

Out of scope:

- broad refactor discovery
- source-code edits before preflight is complete
- expanding the stage beyond its plan without updating the plan first
- overwriting unrelated user or local changes

## Source Prompt

This gate is adapted from:

`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/prompts/implement-plan.md`

The full prompt remains the implementation execution pattern. This gate narrows
it into the start-of-stage checklist for Pantograph execution-platform work.

## When To Run

Run this gate before each numbered implementation stage:

- before editing source code, tests, configs, generated artifacts, or build
  metadata
- after selecting the stage plan to implement
- after any prior stage has completed `09-stage-end-refactor-gate.md`

## Inputs

- the selected stage plan file
- `07-standards-compliance-review.md`
- `09-stage-end-refactor-gate.md`
- `10-concurrent-phased-implementation.md`
- the standards directory:
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
- current git status and relevant diffs
- expected stage branch, base ref, or implementation range
- known dirty files, generated files, or unrelated local changes

## Preflight Checklist

Complete these checks before implementation starts:

1. Read the full selected stage plan.
2. Read `07-standards-compliance-review.md`.
3. Read the standards referenced by the selected stage.
4. Inspect `git status --short`.
5. Identify the intended write set for the stage.
6. Compare the intended write set with existing dirty files.
7. Confirm the plan has objective, scope, ordered tasks, verification, risks,
   completion criteria, and re-plan triggers.
8. Confirm the stage can complete without changing unrelated files.
9. Confirm expected verification commands or define the missing ones in the
   plan before editing.
10. Confirm whether implementation will be single-worker or needs explicit
    concurrent-worker planning.

## Worktree Hygiene Rules

- Do not begin implementation when dirty implementation files overlap the
  intended write set unless the user explicitly allows it.
- Do not revert, overwrite, reformat, or clean unrelated existing changes.
- If unrelated dirty files exist outside the intended write set, record them
  and leave them untouched.
- If generated files are dirty before implementation, identify their generator
  and whether they belong to the stage before editing.
- If the implementation depends on existing dirty changes, pause and clarify
  ownership before proceeding.

## Start Outcomes

The gate must produce one outcome:

- `ready`: the stage plan is complete enough, the worktree is safe, and
  implementation may begin
- `ready_with_recorded_assumptions`: gaps can be inferred safely, assumptions
  are recorded in the plan, and implementation may begin
- `blocked_needs_plan_update`: the stage plan lacks required details that can
  be resolved by updating the plan before editing
- `blocked_needs_user_clarification`: scope, dirty files, ownership, or risk
  cannot be resolved safely without user input

## Execution Rules After Passing

Once the outcome is `ready` or `ready_with_recorded_assumptions`:

- implement the selected stage in the plan's order
- complete one logical step at a time
- run the verification required for each logical step where feasible
- update the plan or implementation notes when implementation reveals material
  facts
- keep unrelated fixes separate from stage work
- commit code, tests, and documentation together when they belong to the same
  logical step
- do not begin the next logical step with unresolved dirty files from the
  previous step unless the plan explicitly allows that sequence

## Commit And History Expectations

When commits are part of the implementation workflow:

- use `COMMIT-STANDARDS.md`
- keep commits atomic by logical step
- use conventional commit format
- include agent metadata when applicable
- do not include verification command output or logs in commit messages
- inspect staged changes before committing
- avoid mixing unrelated cleanup with feature implementation

## Concurrent Worker Gate

If a stage requires parallel implementation:

- follow `10-concurrent-phased-implementation.md`
- define stage-specific implementation waves before launching workers
- define non-overlapping write sets before launching workers
- provide each worker one complete prompt with scope, write boundaries,
  validation expectations, report path, and escalation rules
- run one worker wave at a time
- integrate worker outputs one at a time
- update the stage plan after reading worker reports
- do not use concurrent workers to bypass unresolved architecture or ownership
  questions

If the stage needs concurrent work but lacks wave specs, the start outcome is
`blocked_needs_plan_update`.

## Unexpected Issue Handling

When implementation reveals facts that change objective, scope, sequencing,
compatibility, persistence, security, risk profile, or verification:

- stop the current logical step
- record the issue in the selected stage plan or implementation notes
- decide whether it is an in-scope adjustment, a plan update, a re-plan
  trigger, or a user-clarification blocker
- continue only when the safe path is explicit

## Required Start Report Shape

Record at least:

- selected stage name and plan file
- current git status summary
- dirty files that exist before implementation
- intended write set or code areas
- applicable standards reviewed
- start outcome
- assumptions recorded before editing
- verification commands expected for the stage
- known blockers or re-plan triggers

## Verification

- The selected stage plan was read before editing.
- Applicable standards were identified before editing.
- Git status and overlapping dirty files were reviewed.
- Missing plan details were recorded or clarified before implementation.
- The intended write set is known enough to avoid unrelated changes.
- The stage-end refactor gate is queued for completion before the next stage.
- If concurrent implementation is selected, wave specs, worker reports, and a
  coordination ledger exist before workers are launched.

## Risks And Mitigations

- Risk: implementation starts against an incomplete plan. Mitigation: require
  objective, ordered tasks, verification, risks, completion criteria, and
  re-plan triggers before editing.
- Risk: stage work overwrites unrelated local changes. Mitigation: compare the
  intended write set against dirty files before editing.
- Risk: commits become too broad to audit. Mitigation: commit by logical step
  and keep unrelated fixes separate.
- Risk: implementation discoveries remain tribal knowledge. Mitigation: update
  the plan or implementation notes when material facts change the safest path.

## Re-Plan Triggers

- The plan lacks enough detail to infer a safe implementation sequence.
- Existing dirty files overlap the intended write set.
- Verification commands cannot be identified for the stage.
- The stage requires broader refactoring before feature work can start.
- Parallel work is needed but write boundaries cannot be made non-overlapping.
- Parallel work is needed but `10-concurrent-phased-implementation.md` has not
  been applied to the selected stage.

## Completion Criteria

- A start outcome is recorded before implementation begins.
- Any safe assumptions are written into the plan or implementation notes.
- Any blockers are resolved before source edits begin.
- The stage has an explicit path to final verification and
  `09-stage-end-refactor-gate.md`.
