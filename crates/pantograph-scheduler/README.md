# Pantograph scheduler contracts

This crate owns scheduler queue, policy, admission, batching, dispatch and
lifecycle contracts. Workflow readiness/publication, native execution and physical
resource custody remain with their existing owners. The curated crate exports
in `src/lib.rs` are the entry point; no runtime or application startup is needed
to evaluate a pure policy.

## Bounded grouping consumer contract

`evaluate_scheduler_bounded_grouping` evaluates one borrowed, ordered window of
`SchedulerBatchCandidate` values. It immediately proposes the unchanged input
prefix: all members when compatible, otherwise the anchor singleton. It adds no
fill wait, queue scan, task reordering or default-policy activation.

The operation accepts one through eight candidates. Before any existing candidate
validator or compatibility equality, it preflights every member's raw input:

- At most eight elements in selected devices, intent trait settings, dependency
  overrides, estimate hints, both model references' migration diagnostics and
  each override's extra-index URLs.
- Raw opaque ID inputs at most 128 bytes; model ID/revision/artifact ID at most
  512 bytes; model artifact path at most 1024 bytes; shape at most 128 bytes.
- Migration diagnostic code at most 128 bytes, message/input at most 1024 bytes;
  trait strings and every override string at most 1024 bytes.
- At most 64 KiB of aggregate admitted raw-string bytes, using checked accounting.
  This is an input-size bound, not the total byte visits across later passes.

These are policy eligibility bounds. The existing generic serialized batching
contract remains version 1 with its existing validation semantics. Passing that
generic contract alone does not establish these additional policy bounds.

Refusal order is window size, all-member raw preflight, existing candidate
validation, then duplicate `(workflow_run_id, task_id)` identities. These refuse
the complete window with fixed typed context before incompatibility is examined.
Candidate strings, model paths, override URLs and payloads are not retained in
refusals. Candidate validation retains its current scope; it does not establish
complete upstream dependency correctness.

Compatibility uses the existing exact task-family, full selected model reference
(including migration metadata), runtime, ordered device-set and input-shape
rules. An otherwise valid incompatible window selects one member and reports
that member's exact incremental memory. Unselected-member aggregate overflow
cannot reject this singleton. A compatible group sums all selected increments
with checked arithmetic and refuses overflow. Zero increments fail the existing
candidate contract before grouping. Inputs are borrowed and remain unchanged.

The immutable proposal exposes selected count, disposition and incremental memory
sum. Selected count means the caller's original input prefix `[0..count)`; the
value does not retain or bind identities. It carries no Ready proof, live owner
or CPU capability, physical-capacity claim, reservation or dispatch authority.
A consumer must obtain fresh authoritative facts before each ordinary Start.

## Fairness and execution boundary

This kernel does not change the M3 queue or mint a service opportunity. Each
actual member continues to require its existing M3 grant and matched cleanup
before that turn is released. A proposal cannot authorize holding one old grant
over several members or skipping a protected competitor.

The existing runtime-host batch envelope executes supported members sequentially.
It does not supply fairness re-entry between members, and the already-Running
assignment broker is not a Ready-frontier integration boundary. M5 C2 must review
that integration and prove actual grouped-versus-singleton CPU behavior before
execution or overhead/reuse improvement is accepted. The pure policy establishes
neither native/continuous batching nor a performance result.

The [CPU plan](../../docs/plans/cpu-scheduler/plan.md#m5-c1-bounded-grouping-kernel-admission)
owns staged admission and acceptance. M6 application choice/diagnostics and actual
real-model qualification remain separate work.

## Focused verification

Use the repository's pinned Rust toolchain:

```sh
cargo test --locked -p pantograph-scheduler
cargo clippy --locked -p pantograph-scheduler --all-targets -- -D warnings
cargo doc --locked -p pantograph-scheduler --no-deps
```

External-facing grouping tests exercise the public API; existing batching tests
continue to cover the general serialized contract. These checks prove the library
kernel's contract, without exercising a workflow/native execution boundary.
