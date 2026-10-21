# Completed embedding result ownership attribution

Operation `continue` under `docs/plans/cpu-scheduler/plan.md`. The user authorized
PR88 integration and the next proportionate allocation/lifetime foundation.
PR88 merged normally as `dc9267831ee846229d365fa17e5bf9250b01868e`, parents
`4561e838f48ec82464322c05c50e82d79ca3acae` and
`8306e2f131c32e6db44a5a5108ab9abdb1f6304e`, with the exact independently approved
tree `88aaf720384c5ff1537810a58dfd6fc055bc38b3`. Its 22 check runs and four PR
workflows passed. CodeRabbit skipped draft source review and supplies no source
approval. All20 post-merge check runs and all four workflows subsequently passed
at exact main dc926783; the separate integration receipt owns their identities.

## Reviewed contract

Independent pre-code review admitted an actual backend ownership hook for one
narrow domain: the immutable original completed embedding-result payload held
by `EmbeddingJobs`. This is post-materialization ownership attribution, not a
complete allocation observer. The hook is inside the blocking worker immediately
after the forward returns, so an unpolled completed join also retains accounting.
The result field precedes its lease in `Completed<T>`; the original payload drops
before the lease updates accounting. Shared completion readers share this one
owner. Caller copies allocate distinct payloads and remain outside the domain.

Checked payload capacity is:

`outer Vec capacity * size_of::<EmbeddingResult>() + sum(initialized inner Vec capacities * size_of::<f32>())`.

The immutable safe-Rust vectors uniquely own these payload allocations. Capacity
includes spare outer slots and inner elements, but excludes allocator metadata,
rounding and construction transients before the completed owner exists. This
does not claim the beginning of the original allocator lifetime was observed.

One lazy ledger belongs to the existing job-owner instance. Its opaque UUID and
four fixed counters report registered/released composite owners, live payload
capacity bytes and peak observed live payload capacity bytes. Zero-capacity
results still register an owner. Shared/Arc aliases mint no new lease. Overlapping
completed results retained across slot replacement remain counted until their
actual last owner drops. Drain observes the real worker join and retires only
the matching slot; another completion reader can keep payload live afterward.
The scope can span model replacements and is not a model/workload/attempt key.
Live capacity means registered ownership whose accounting has not retired yet.
Registration follows materialization and release follows payload drop; concurrent
snapshots can include these bookkeeping windows. The observed counter peak is
not an instantaneous physical allocator peak.

Observation is enabled solely by the existing explicit calibration owner.
Generic loads, failed results and ordinary no-owner execution acquire no result
lease. The disabled path initializes no UUID or ledger allocation. Overflow,
invalid release and poisoned accounting produce fixed unavailable diagnostics,
without replacing the inference outcome. Missing observations never mean zero
attempt-wide memory. No public API, allocator, dependency, IPC or admission
mechanism changes.

The existing host logger consumes bounded snapshots at worker completion, slot
update and drain. Snapshots copy fixed scalars under the ledger lock, then build
JSON and invoke the logger outside all guards. Lease Drop updates counters only.
Snapshots may race or arrive out of order; there is no promised final-release
event when a late reader drops. Later snapshots observe that release. This is
neither an allocation event stream nor an allocation-duration measurement.

## Coverage and remaining software work

Complete tracking is limited to the registered immutable original result-owner
payload after materialization. Inputs/tokenizer allocations, model residency,
Candle intermediate/shared storage, GEMM scratch, worker stacks/Rayon work,
allocator overhead, earlier construction peaks and caller output copies remain
untracked. No RSS or reservation arithmetic fills those gaps. The observed peak
cannot be used as complete incremental memory or an admission estimate.

Pinned Candle `88ed7911de9e88196b1f55b199145d22647f415e` routes CPU allocation
through concrete Vec storage without an injectable complete allocation/drop
observer. Tensor reshape can share private Arc storage, and BERT/GEMM/Rayon
temporaries escape returned-tensor boundaries. Full attribution therefore needs
separately reviewed dependency-level storage/drop hooks, alias identity and
scratch/worker context propagation. That is remaining software implementation,
not missing external measurements. No external input blocks this foundation.
Representative frozen model/workload/settings and target-hardware measurements
qualify usefulness and performance later; this slice makes no such claim.

## Write set and verification

The source write set is `crates/inference/src/backend/candle_embedding.rs`, new
private `candle_embedding/result_payload.rs` and focused job tests. Documentation
is this report, canonical plan/ledger/issues and the affected host-options guide.
The ninth path is .github/workflows/quality-gates.yml for the reviewed explicit
hosted test selection described below.
Standards baseline `188beda1` remains adopted; Implementation, Verification,
Contracts, Concurrency, Diagnostics, Performance, Library and Rust API/async
routes apply. The explicit CI addition also routes Tooling/Build and Rust Tooling.
Dependency discovery changes no dependency identity or resolution.

Model-free cases cover capacity arithmetic and spare storage, overlapping leases,
overflow/poison, unpolled join completion, caller loss, last-reader retirement,
payload-before-lease drop order, retained output across drain/replacement,
cancellation, join panic and unavailable measurement preserving the result.
An ordinary existing untrained BERT-8 fixture verifies the actual execute gate
enabled/disabled. Existing native numerical and cleanup regressions remain
required; these are software regressions, not a new held performance cohort.

## Local evidence and review

All 12 new cases passed (five ledger cases and seven job cases), and the full
inference library with backend-candle passed 582 cases, none ignored/filtered.
The actual execute-gate case uses the existing untrained BERT-8 fixture; it
qualifies the production wiring, not representative model memory or performance.
All-target/all-feature own-package Clippy passed with warnings denied. The
narrow Candle-feature warning-deny attempt failed on the existing unsuppressed
selected_text_execution dead-code warning; focused Clippy with its own lints
denied passed while retaining that compiler warning. No lint allowance changed.

Ordinary all-feature Rustdoc passed with the existing unclosed HTML Vec<u8> tag
warning in kv_cache/types.rs:179. The optional warning-deny Rustdoc attempt failed
on that exact baseline defect. The entire unchanged file matches base dc926783
(SHA256 05872528ebec88abe6505337c9f81ca0d9bee2222a9c7c1309abfbd44a071864).
No unrelated KV-cache edit or warning suppression was made. Formatting, staged
traceability (eight initial paths, zero mapped impacts) and whitespace passed. Declared
pre-push tests passed 261 node-engine cases with one existing ignored case and
168 workflow-nodes cases. Ordinary locked preparation fetched missing test/PyO3
source crates; manifests, lockfiles and dependency identities are unchanged.

Independent exact-source review approved the bounded component, including the
field-drop, disabled-gate, unpolled-join, alias and failure behavior. The linked
draft PR owns exact published head/tree and hosted CI; no hosted candidate pass
is claimed by local evidence. The branch/worktree remains retained-protected
for parent review; all prior branches, PR84 lineage and unrelated work remain
protected. Overall M5/M6 and complete incremental-memory acceptance remain open.

## Hosted coverage closure

Initial draft head0b2ca052 adds the reviewed backend source, but the existing
default inference CI command does not enable backend-candle. Its ordinary
Candle tests are locally proven and compiled through other consumers; they
were not selected for hosted execution. Independent pre-code review admitted
one focused Rust job step plus retained discovery/results/revision artifacts.
All twelve fully qualified cases must discover exactly once and pass, with
a final12passed/0failed/0ignored summary. Existing workflow jobs, steps, browser
assertions, deadlines, sandbox and other controls remain unchanged. This is
continuing ordinary regression coverage, not a new performance cohort.

The exact extracted Bash step replay passed all12 cases locally, including
its discovery and per-case/result parsing. YAML parsing and structural comparison
prove that removing the two added steps reproduces the original workflow.
A normal fast-forward follow-up preserves the initial implementation commit and
its own CI evidence; only the updated exact candidate may claim hosted coverage.
