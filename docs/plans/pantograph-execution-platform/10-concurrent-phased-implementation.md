# 10: Concurrent Phased Implementation

## Purpose

Define the artifact layout and execution rules for implementing a numbered
execution-platform stage with safe parallel worker waves when parallelism is
warranted.

This file satisfies the concurrent phased implementation expectations from
item 7 of:

`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/prompts/full-codebase-standards-refactor.md`

## Scope

In scope:

- stage-specific implementation wave planning
- non-overlapping worker write sets
- worker prompt requirements
- worker report paths
- host-owned coordination ledger
- one-wave-at-a-time execution and one-branch-at-a-time integration

Out of scope:

- requiring every stage to use parallel workers
- broad refactor planning unrelated to the selected implementation stage
- delegating unresolved architecture or ownership decisions to workers
- allowing workers to write outside assigned ownership boundaries

## When To Use

Use this file only when `08-stage-start-implementation-gate.md` determines that
a stage is too large or naturally separable enough to justify concurrent
workers.

Do not use parallel workers when:

- the next step is blocked on a single architecture decision
- write sets cannot be made non-overlapping
- verification requires every change to land before any slice can be tested
- the stage is small enough that coordination cost exceeds benefit

## Required Artifact Layout

For a concurrent stage, create artifacts under:

```text
docs/plans/pantograph-execution-platform/implementation-waves/<stage-slug>/
├── README.md
├── coordination-ledger.md
├── waves/
│   ├── wave-01.md
│   └── wave-02.md
└── reports/
    ├── wave-01-worker-<name>.md
    └── wave-02-worker-<name>.md
```

Use the numbered stage id in `<stage-slug>`, for example:

```text
01-client-session-bucket-run-attribution
03-managed-runtime-observability
06-binding-projections-and-verification
```

## Stage Wave README

Each concurrent stage must include a stage wave `README.md` with:

- selected stage plan file
- reason concurrent implementation is warranted
- source standards reviewed
- stage objective and non-goals
- expected waves in execution order
- integration branch or base ref
- global files that no worker may edit without host approval
- stage-level verification commands
- stage-level re-plan triggers

## Coordination Ledger

The host owns `coordination-ledger.md`.

The ledger must track:

- stage name and plan file
- branch or worktree strategy
- wave status
- worker names or identifiers
- assigned write sets
- report paths
- integration order
- verification results after each integration
- conflicts, escalations, and decisions
- final stage-end refactor gate outcome link or summary

Workers may append reports to their assigned report files, but the host updates
the coordination ledger.

## Wave Spec Requirements

Each `waves/wave-XX.md` file must define:

- wave objective
- dependency on prior waves
- workers in the wave
- each worker's exact write set
- files and directories explicitly forbidden to that worker
- required standards for the worker to keep in context
- verification commands each worker must run or explain if not runnable
- report file each worker must write
- expected output shape
- escalation rules
- integration order after the wave completes

## Worker Prompt Requirements

Each worker prompt must be complete without follow-up messages.

The prompt must include:

- selected stage plan file
- wave spec file
- worker write boundary
- forbidden files or directories
- required standards files
- expected implementation outcome
- verification commands
- report file path
- instruction not to revert or overwrite other workers' changes
- instruction to adjust to existing changes rather than resetting them
- escalation criteria for blocked or unsafe work

## Write-Set Rules

- Worker write sets must not overlap.
- Shared generated files require host-owned integration, not worker ownership,
  unless the wave spec assigns a single owner.
- Shared manifests, lockfiles, launcher scripts, root configs, and public
  facades require explicit ownership in the wave spec.
- If overlap becomes necessary during implementation, stop the wave and update
  the wave spec before continuing.

## Execution Rules

- Execute one wave at a time.
- Launch workers in the same wave only after validating non-overlapping write
  sets.
- Do not launch dependent waves until the prior wave is integrated and
  verified.
- Integrate worker outputs one at a time.
- Read each worker report before integration.
- Update the coordination ledger after each integration.
- Run the wave or stage verification required by the wave spec after
  integration.
- Clean up temporary worktrees, branches, or clones when no longer needed.

## Verification

- Stage wave artifacts exist before workers are launched.
- Every worker has an assigned report file.
- Every worker has a non-overlapping write set.
- The coordination ledger records integration order and verification.
- Dependent waves do not start until prerequisites are integrated.
- The selected stage still completes `09-stage-end-refactor-gate.md`.

## Risks And Mitigations

- Risk: workers duplicate or conflict with each other. Mitigation: require
  exact write sets and one-wave-at-a-time execution.
- Risk: parallel work hides architecture decisions. Mitigation: block parallel
  work until ownership and boundaries are explicit.
- Risk: generated files or manifests become integration hot spots. Mitigation:
  assign one owner or reserve them for host integration.
- Risk: worker reports are skipped. Mitigation: integration is blocked until
  the report exists and has been read.

## Re-Plan Triggers

- A worker needs to edit outside its assigned write set.
- Two workers need the same file.
- Verification cannot run until multiple waves are integrated.
- A worker finds that the stage plan's scope or architecture is incorrect.
- Integration requires changing files reserved for host ownership.

## Completion Criteria

- Concurrent stages have a `README.md`, `coordination-ledger.md`, wave specs,
  and report paths before worker implementation starts.
- All worker outputs are integrated one at a time with verification recorded.
- The stage plan and coordination ledger reflect any deviations.
- The stage completes `09-stage-end-refactor-gate.md` before the next stage
  begins.
