# Frozen-cohort completion policy kernel

`evaluate_scheduler_cohort_completion` is a new pure Rust advisory API in
`pantograph-scheduler`. It follows the research thesis sections 10.1–10.3:
freeze the comparison population, optimize two completion events, and complete
all remaining represented obligations using one common continuation. It does
not replace existing selectors or activate native lookahead.

## Declared scope and objective

The owner freezes one to four tasks, one or two capability-valid serialized,
single-device placements per task, a dependency snapshot, observation and
objective identities, and evidence. Exactly one task borrows the already-admitted
first request; every other task borrows an advisory forecast without a Ready
proof. Every task must share the first task's workflow ID and workflow-run ID;
cross-workflow and cross-run cohorts are outside this API. Placements exactly cover their request's candidate universe. The kernel
checks associations, constraints and supported shape; the owner remains
responsible for capability discovery and completeness of the represented cohort.

Internal prerequisite edges must name represented tasks and form a DAG. Every
external prerequisite must already be owner-satisfied at freeze time. Unknown
edges, cycles and omitted placements fail explicitly. Dependency unlocking is
hypothetical; it never creates real output or readiness.

For every feasible placement of the fixed first task, explore every legal second
task/placement after the first projected completion. After event two, repeatedly
choose the feasible `(earliest predicted completion, task ID, placement ID)`
minimum. Stop only when every represented task completes or the path reaches a
known dead end. The same rule is used for every path. A one-task cohort terminates
after its first event; a two-task cohort has no continuation tail.

Compare complete paths by `(terminal completion, sum of task completions,
ordered task/placement IDs)`. Completion here ends all serialized setup,
transfer, execution, cleanup, retention and reload stages, including the last
task's cleanup. All six durations must be explicitly present, with zero supplied
for an absent stage, and all arithmetic is checked. Stages must be disjoint to
avoid double charging. This is represented-task completion, not a guarantee
about arbitrary unseen workflow descendants.

## Evidence and failure semantics

Each row borrows the exact frozen cohort, task and placement, with its complete
ordered predecessor task/placement history. Its timing context identifies the
host, runtime instance, artifact, workload, resource condition, residency and
aggregation convention. Forecast workload fingerprints must match their frozen
snapshot identities. Contexts must bind to the previous row's post-completion
condition and residency; the empty history binds to the initial observation.
Thus release, reconciliation, retention and reload cannot be inferred from
matching model IDs or by subtracting an old lease.

Every legal placement at every visited search/continuation state requires a row.
`Fits` requires a complete fresh sample and an explicit resulting transition.
`WaitingForResources` and `ImpossibleFit` require neither; a nonterminal state
with no fitting placement is a known dead end. Unknown capacity, stale/foreign/
missing/duplicate evidence, missing stages, overflow or budget exhaustion makes
the entire comparison incomplete. No task is dropped and no partial search wins.
All supplied rows, including unused alternatives, are validated. Rows describing
unvisited descendants are optional; when supplied, their predecessor evidence is
required too. Labels and source enums do not authenticate historical observations
or establish calibration. Synthetic evidence requires explicit opt-in.

A complete result contains the first advisory ID, cohort/objective/dependency
identities, the entire stage/completion path, source and work counts. Incomplete
results contain a typed reason and work counts, with no action. A consumer must
retain its existing safe selector. Any eventual execution still needs current
resource, instance, Ready/dependency, cancellation and custody revalidation.

## Fixed work bounds

- At most 4 tasks and 2 placements per task; no truncation
- At most 158 evidence rows, enough for the entire unconstrained fixed-first tree
- At most 64 optimized branch expansions and 512 completion-event advances
- At most 131,072 bounded work units, independently reducible for tests

The initial shape guards are fixed bounded work. Metered validation and search
count task/row operations, evidence comparisons, candidate evaluations, objective
comparisons and all continuation work. One unit operates on bounded records
(maximum four actions and 128-byte identity/context labels); sort work is charged
by the square of its at-most-eight-choice population. Evidence lookup scans the
whole table so permutations preserve counts. A fully feasible four-task fixture
explores 14 optimized branches and 24 continuation events, for 38 event advances
and 12 complete candidate plans. These are deterministic operation counts, not
CPU instructions, measured runtime or a dispatch deadline guarantee.

## Verification and boundaries

The focused suite covers third-task reversal (pair A2/B4 becomes terminal A102/B5),
fourth-task reversal, dependency unlocking, cold/warm and reload/cleanup/transfer/
retention reversals, direct compatibility with the existing two-task selector,
stable permutations, invalid evidence/dependencies, dead ends, checked overflow
and exact budget exhaustion. An independent numeric reference enumerates full
tiny schedules, then filters them by the specified continuation rule; it does
not reuse production search helpers. Its 192 generated cohorts have fitting
placements and either chain or independent-task dependencies. Other shapes,
including fan-out/join dependencies and blocked paths, are covered by directed
tests. Repeated identical-input evaluation checks determinism; it is not an
independent stochastic nonanticipation experiment. A separate fixture demonstrates
that the fixed continuation can be worse than globally reordering the tail.

### Private-stack qualification (historical)

Local qualification of `cc64e28` on private predecessor `f98e4ae`, on
2026-10-08: `cargo test --locked --offline -p pantograph-scheduler` passed 197 tests, including all 15 cohort tests; the existing
controlled two-completion timing probe remained ignored. The new numeric oracle
checks the 192 fitting chain/independent cohorts described above; direct two-task
compatibility checks 24 matrices. The reporting-only successor preserves kernel
bytes and renames the identical-input test without expanding this evidence claim.
`cargo clippy --locked --offline -p pantograph-scheduler --all-targets -- -D warnings`,
`cargo fmt --all -- --check` and `git diff --check` passed. Builds used one job,
no debug information and no incremental compilation in a separate scheduler-only
target. No embedded/native consumer was rebuilt and no performance claim follows
from these correctness checks.

### Public-only qualification

Separately, on 2026-10-08, local head
`2389a96d9b1c4eda921c04c65e8971e64356b096` and published head
`7b278df2976d3f9ae0696bf3ea2cd50e0d94f0d5` share the verified tree
`671378482e2a13c9136122432c5a6cf2ccd1ee4f`, based directly on public PR68 head
`1479748faa9ca383619384633acff5a88f4471be`. On this exact tree,
`cargo test --locked --offline -p pantograph-scheduler` passed **196 tests**, with
all 15 cohort tests passing and the same existing timing probe ignored. The
scheduler all-targets Clippy, formatting and diff checks above also passed.

The historical 197-pass result includes the private initial-drain test
`initial_receipt_requires_unranked_binding_valid_owner_and_all_eight_identities`,
which is absent from the public-only base. The reporting successor neither added
nor removed a scheduler test; the one-test difference comes from the distinct
source bases. The private native predecessors remain outside this cohort PR.

On published head `7b278df`, [Quality Gates](https://github.com/MrScripty/Pantograph/actions/runs/37746312733),
[Headless Workflow Contract](https://github.com/MrScripty/Pantograph/actions/runs/37746312802)
and [Runtime Separation](https://github.com/MrScripty/Pantograph/actions/runs/37746312742)
passed. These Actions results do not clear separate security findings on parent
PR68 or imply native/default cohort activation.

This is an exact implementation of this bounded algorithm, not a global-optimum
claim or calibrated performance result. It does not add overlap, batching,
preemption, preload/eviction plans, remote modes, mutable priority services,
production profile collection, owner cohort certification or native wiring.
Existing selectors, workers, lifecycle/dependency/wire contracts and defaults
remain unchanged. No POC implementation was copied.
