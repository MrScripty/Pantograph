# Bounded completion first slice: local results

2026-10-07; branch `scheduler/bounded-completion-v2`, worktree
`/workspace/Pantograph-completion`. Remote main was checked again after
validation and remained `a8483e511dcec4f36e269e6e4debf181a318222f`.
The plan checkpoint is commit `170f9aae` in
[the migration plan](scheduler-v2-production-migration.md).

## Implementation

`pantograph-scheduler::select_scheduler_candidate_with_completion` is an opt-in
pure local selector at the reservation-selection boundary. It minimizes checked
integer preparation/load/owner-delay + required transfer + execution cost for
one Ready task on a serialized single-device, nonbatch path. It does not attach
a warmth reward or prune cold alternatives. Equal totals use candidate ID.

All eligible alternatives need complete comparable samples. Evidence is borrowed
from the exact immutable request and offer; cloned or foreign snapshots are
rejected. Current and sample host, runtime instance, artifact, workload,
resource/residency condition and timing convention must match. The cohort must
share host/workload/resource condition/convention and provenance class. Measured
and explicitly enabled synthetic sources stay distinct. Sample counts, age,
future timestamps, missing stages and overflow have explicit typed diagnostics.
Pointer binding associates a snapshot; it does not authenticate historical
timings or fingerprints. A trusted production profile producer is still missing.

Fixed work limits are 64 offers/rows, 8 devices per offer, 32 request diagnostics,
and 128 bytes per context field. Over-limit input refuses before ranking or
legacy fallback; invalid/incomplete evidence within the bounds retains existing
sole-eligible/otherwise-ambiguous selection. Existing eligibility is reused for
hard runtime/device constraints and Fits assessments. The result is an advisory
candidate ID, never a resource lease or executable dispatch proof.

Default dispatch callers, priority/FIFO session admission, starvation boosts,
one-position warm-reuse, public wire contracts, runtime ownership, output and
cancellation custody are unchanged. Only the scheduler crate and these two
documents were edited. Inspector and rerank/audio adapter workers' surfaces were
not edited. The Rust design was written afresh using production contracts; no
POC implementation was copied, translated, ported or made a dependency.

## Verification

All 286 scheduler and runtime-registry tests passed, including 11 new completion
tests. There are no ignored tests in these two crates. Commands used the pinned
Rust 1.92.0 toolchain with `ORT_SKIP_DOWNLOAD=1`:

```sh
cargo test --locked --offline -p pantograph-scheduler -p pantograph-runtime-registry
cargo clippy --locked --offline -p pantograph-scheduler --all-targets -- -D warnings
```

Clippy, changed-file rustfmt and `git diff --check` passed. Cargo.lock and
manifests were unchanged. The initial offline workspace resolution required the
new pinned Pumas source checkout; a normal locked scheduler build resolved that
source, then the full checks ran offline. No models, ONNX artifacts or native
runtimes were downloaded.

New tests cover the thesis's hand calculation: warm CPU 12 seconds versus cold
GPU preparation 2 + execution 3 chooses cold; adding 8 seconds of required
transfer reverses the choice to warm. These are synthetic values, not hardware
measurements. Fixture SHA256:
`47b68d85f535d7f00ff176ca232fbde0f8b12286bcb4a5a2040bcbe4f94cf76c`.
Further tests cover all identity fields, foreign/duplicate/missing evidence,
missing stages, overflow, stale/future samples, insufficient counts, mixed source
classes, synthetic opt-in, hard constraints, missing Fits evidence, duplicate
offers, unsupported batching/multiple devices, stable ordering and work limits.
Ranking leaves the request unchanged and ordinary dispatch refuses unreserved
offers. Existing registry suites verify changed instance/capacity rejection,
concurrent commit exclusion, selected-only publication and reservation custody.
There is no new production integration to qualify; those seams must be exercised
around the new selector when a real producer is connected.

An independent reviewer inspected design and implementation and independently
ran all 11 new tests. Verdict: no blocking findings. A benchmark-coverage finding
was addressed by adding maximum-length fields/diagnostics and a late-invalid
fallback fixture; final independent review closed it.

## Policy cost, not a production dispatch SLA

The release example `completion_policy_cost` warms 1,000 calls and measures
10,000 calls per fixture using a monotonic clock and `black_box`. It includes
selection/result allocation and fallback but excludes request validation,
profile lookup, resource evaluation, lease commit, runtime work and artifact
access. It is a synthetic policy-only cost experiment, not hardware inference
or the POC's whole-run planning benchmark.

Host: Linux 6.18.44 x86_64, Xeon Platinum 8573C virtual CPU, 5 visible CPUs,
17 GiB RAM, no swap. Regular speed; no environment speed/settings changes.

| Fixture | Candidates | p50 us | p95 us | Max us |
| --- | ---: | ---: | ---: | ---: |
| Typical | 1 | 0.392 | 0.430 | 4134.398 |
| Typical | 8 | 2.756 | 2.827 | 9194.568 |
| Typical | 64 | 29.060 | 59.279 | 26481.866 |
| Maximum fields | 64 | 102.727 | 259.145 | 27350.453 |
| Last row invalid, then legacy fallback | 64 | 110.508 | 355.642 | 6539.601 |

Maximum-field fixtures use 128-byte IDs and all seven context fields, with 32
diagnostics each carrying a 1024-byte message and hint. The final invalid row
forces full ranking validation before the bounded legacy fallback. The measured
process used 4.76069 s user CPU + 0.01463 s system CPU, 4.78140 s wall and 9344 KiB
peak RSS across startup, all loops, warmups and sorting (compiler excluded).
An earlier run measured maximum-field/fallback p95 223.899/255.830 us with
maxima 2654.470/3359.783 us. Keep both observations: host scheduling and shared
environment noise produce outliers. The deterministic operation bound does not
promise an elapsed-time ceiling. No default latency gate is declared passed.

Reproduce with downloads disabled:

```sh
ORT_SKIP_DOWNLOAD=1 cargo run --release --locked --offline \
  -p pantograph-scheduler --example completion_policy_cost
```

## Remaining blockers and next production milestone

Library reads supplied the thesis and evaluation, but documented local transfers
failed and `/workspace/research-inputs` contains no bytes. Neither report byte
hash nor the ZIP/C3 source hash was verified by this executor. Actual archive
inspection and differential checks against the simulator's earliest baseline
remain blocked; no alternative storage route or another executor's paths were
used. The supplied SHA and two-event source mapping are reported only as
delegated evidence in the migration plan.

Resolve the owner and contract for trustworthy comparable completion profiles:
runtime/artifact/workload identity, aggregation and uncertainty, phase scope,
clock domain, residency/resource-condition invalidation and diagnostic transport.
Then connect this opt-in selector to existing evaluate/select/selected-commit
and shadow-score real dispatches under a frozen latency budget. Full native
workflow/session/output/cancellation acceptance has not been run; it requires a
qualified build without downloading ONNX. These are release gates, not implied
by scheduler/registry contract tests.

Continue toward the actual researched scheduler as a separate fresh Rust
milestone: disclosed bounded cohort, mode-diverse action coverage, successive
event completions, unfinished obligations in common continuation, frozen
priority/protection semantics, bounded deterministic work and first-action
authoritative revalidation. Profile creation, supported stage/transfer modes,
native batching and remote offers need explicit capabilities before they enter
its action space. This earliest baseline is not the completed research planner.

No push, public writes or main merge were performed. No credentials, network
settings, model/runtime downloads or unrelated workers' working paths changed.
