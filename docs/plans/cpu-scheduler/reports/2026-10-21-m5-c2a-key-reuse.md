# M5 C2a: bounded runtime workload-key reuse

## Authority and scope

Operation `continue` under [the active CPU plan](../plan.md). The current user
delegation admits the smallest safe C2 runtime integration on a separate branch,
focused deterministic CPU tests, bounded synthetic comparison, independent
review and a dependent draft. It does not authorize merge, default activation,
security changes, model downloads or real-model inference.

Base: PR85 `f3f438a23b507a0f82a0c2f316a83f1148798643`, itself based on PR83
`bd66da4e0ba20d17f30a70140a448b55d57fce1e`. Existing branches, PR84 and M3
history remain protected. Canonical standards baseline remains `188beda1`;
Implementation, Verification/oracles, Contracts, Concurrency, Performance,
Diagnostics, Library and Rust API/async routes apply. No local AGENTS/skill
instructions apply. Independent pre-code review approved this narrowed C2a
contract with the bounds, equality, lock and measurement conditions below.

## Missing facts and selected behavior

`WorkflowRuntimeDispatchCandidateFact.loaded_runtime_memory_estimate_bytes`
describes model residency; the current embedded producer derives it from package
logical size times three. It is not a measured or bounded per-member incremental
execution footprint. No authoritative incremental/peak per-member grouping memory
or grouping-latency estimate is supplied by this boundary. The serial ring records
successful execution through drain and matched cleanup; it cannot supply memory,
cold preparation or transfer forecasts. Graph `context_shape_key` is declared
metadata, not observed tokenizer/tensor shape.

Do not fill these fields with zero, arbitrary positive values, residency memory,
synthetic timings or a purported tensor shape. C2a instead owns one useful pure
operation already repeated by production selection and capture: JSON encoding
and hashing the exact workload-key preimage. It changes no resource admission.
The compatibility-only scheduler projection therefore has no cost fields; C1's
existing full candidate validation and checked memory sum remain unchanged.

An explicit `with_bounded_grouping()` serial configuration requires M3 fair
service opportunities and remains off by default. The store produces an
authoritative first-at-most-eight Ready runtime window without skipping an
unsupported selected member. Complete retained task/record/proof/input snapshots
are bounded before cloning; selected actual text has an aggregate 64 KiB cap.
Unsupported, unknown, oversized or stale member lookups preserve the original
key calculation. Valid mixed windows retain singleton execution and may memoize
only their anchor; no member is combined into a native execution.

Only exact derived workload keys are cached, with at most eight bounded snapshots.
Each hit checks the complete current snapshot for exact equality after raw bounds.
An absent/changed anchor refills from the fresh Ready store; a changed individual
member misses. The fill-time compatibility proposal is discarded, so no retained
plan can select a task or delay a protected competitor. Completed/abandoned scope
members are removed. No cache lock spans store acquisition, owner queries,
callbacks or awaits.

Every actual member still enters its own M3 opportunity and ordinary preparation,
reservation, owner/history/currentness, Start/input/proof, execution, cancellation,
drain and matched cleanup path. Final native-request hashing is independent of
the cache. Runtime candidate sets and rollback custody are never cached. Owner
generation changes cannot become a reused execution capability.

## Declared verification and prospective comparison

Write set: scheduler compatibility projection/shared comparator and external API
tests; workflow serial configuration/key cache, authoritative store capture and
focused tests, one worker refresh point; consumer documentation and this plan's
ledger/issues/evidence. No dependency, manifest, lockfile, UI, IPC, persisted
schema, native port, CI mechanism or default changes.

Required checks: scheduler package and affected workflow-service tests; focused
store/worker negative cases for bounds, changed snapshots, fallback, M3
competitors, cancellation and cleanup; affected formatting, warning-deny Clippy,
Rustdoc and decision traceability; independent exact-diff review before a normal
dependent draft and hosted CI observation.

One bounded comparison is prospectively fixed: two controlled worker cohorts,
baseline and C2a, each with eight Ready text members of 1,024 bytes using the
existing deterministic CPU test port. Compare actual results, order, attempts,
grants and matched cleanup; count every workload-key serialization, including
eager fill, misses, lookahead and independent final binding. The separate two-member case is a focused regression, not another comparison cohort.
Retain all outcomes. New snapshot-bound serialization, cloning, raw checks and
equality add work not included in the workload-key encoding counter. Fewer key
encodings alone establish no reduction in total serialization or CPU overhead.
Report elapsed time descriptively with no speed threshold or winner retry.
This is synthetic scheduler-mechanism evidence, not native throughput,
pretrained/real-model quality, physical memory measurement or end-to-end speedup.

## Status and limits

Implementation is locally verified and independently source/test reviewed; a
normal dependent draft and hosted CI remain. [Check receipt](2026-10-21-m5-c2a-checks.json)
records exact source hashes and commands. Scheduler: 232 passed, one existing cost
probe ignored. Workflow: 1,037 ordinary unit tests passed; the separately held
comparison passed on its first execution (1,038 total tested cases). Formatting,
warning-deny own-package Clippy and Rustdoc passed. An existing inference dependency
unused-field warning remains; it was not suppressed or repaired here.

The declared eight-member synthetic comparison observed 30 workload-key encodings
for baseline versus 16 for C2a, with 22 hits; exact outputs/order/member identities,
cleanup and keys matched. Observed elapsed times were 29.727789 ms and 29.418059 ms.
These single unoptimized synthetic observations establish no speedup or reduction
in total serialization/CPU work. Full-snapshot bounds, clones and equality add
work outside the key counter. Initial compiler and invalid-fixture failures are
preserved; neither executed the held comparison. The invalid fixture attempted
32,000-byte text despite the ordinary 2,048-byte result cap and was replaced by a
legal informational proof diagnostic to exercise conservative fallback. A compatibility result and cache
hit do not establish overall M5 acceptance. Actual grouped-versus-singleton CPU
execution qualification, M6 application opt-in/diagnostics and real-model
qualification remain separate work.

PR85's hosted sandboxed-Chrome step passed with unchanged browser/workflow source.
The historical PR83 failure remains undiagnosed; its redirected log read returned
Forbidden and was not retried. Local Chromium remains blocked by the existing
sandbox-helper ownership problem; no security settings were changed.


## Parent-coordinated integration readiness

Independent read-only REST/refs/ancestry review found main `4aec0d5` is an
ancestor of M3 `b63aa3e`, PR83 `bd66da4e`, and PR85 `f3f438a2`. PR83 adds
19 commits over M3; PR85 adds exactly `529cd902` and `f3f438a2`. PR81 was
non-draft, clean/mergeable with 22 passing checks. PR83 remained draft/unstable
with its historical frontend and quality-summary failures. PR85 remained
draft/clean with all 16 checks passing. All three GitHub review arrays were
empty; independent agent reviews are distinct evidence. Repository merge-commit
support and absent main protection do not confer parent authorization or M4
acceptance. These observations must be refreshed before any integration action.

Safe later sequence, only under parent coordination: merge-commit PR81 and verify
main contains exact M3; retarget PR83 to main and recheck remaining diff, checks
and M4 readiness before a guarded merge-commit at `bd66da4e`; verify that head
is in main, retarget PR85 to main and recheck its two-commit delta before a later
guarded merge-commit at `f3f438a2`. After PR85 integration, retarget the dependent
C2a draft to main and recheck its remaining delta. Keep branches; use no squash,
rebase, deletion or duplicate PR84 integration. PR84 `9e068bbc` is divergent,
while its repair is already represented in PR83's recorded lineage. No retarget
or merge has been performed.
