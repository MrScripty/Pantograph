# M4 fixed protocol: partial evidence and preserved failure

Later continuation: parent separately admitted one main-v2 protocol. It supports
retained/comparison native counts but fails pressure run/session finalization;
see the [main-v2 report](2026-10-20-m4-main-v2.md). This original failed protocol
and its outcome remain unchanged.

Status: **M4-C4 Partial/Blocked; M4 not accepted**. The prospectively fixed main
protocol ran once at source `7139b2ad547e5b195482146f17b6e27e633c92ed`, tree
`75852b54dd591d5e01a00f5345256cbff260711f`. Its frozen release binary SHA256 was
`c10590a7b8b9bd0b4462ed3e6b2bc2c5d727d33f393c878ca9c3aed6e1a4b17f`.
It exited1 at2026-10-20T03:22:52Z after the first retained A task, reporting
`M4 retained vector artifact missing`. It has not been rerun or continued.

The immutable [original archive](m4-main-failure-v1/original-main-evidence.tar.gz)
contains the original source/input/environment/accounting/binary manifests,
build logs, execution status, raw events, artifact bodies, databases and failing
result. [File hashes](m4-main-failure-v1/original-file-sha256.json) and
[archive hash](m4-main-failure-v1/archive-sha256.json) preserve all71 original
nonbinary files. Both frozen binaries remain under
`/workspace/m4-qualification-v1/binaries`; their hashes are archived, not their
large executable contents. No model regeneration or download occurred.

The independent [read-only reanalysis script](m4-main-failure-v1/prefix-review/review_prefix.py)
and [JSON report](m4-main-failure-v1/prefix-review/final-report.json) verify frozen
Git blobs, inputs, binaries, archive bytes and all prefix joins. The54 original
main files retain their initial fingerprint. Two diagnostic SQLite sidecars,
already present when the71-file archive was captured, are disclosed separately;
their presence does not change the original database or event/body bytes.

Independent read-only reviews established that the first embedding succeeded.
The160-byte `vectors:vector` artifact exactly equals the public response and is
within `5.96e-08` of the independent BERT8 golden. All four diagnostic projection
tables and projection state were empty despite12 raw ledger events. Inspection
reads materialized projections; its `projection_batch_size` validates a parameter
but does not refresh them. The facade/producer omitted explicit projection
refresh. This is a producer/readback defect, not evidence of missing native output.

The prefix contains one actual load, forward/drain, exact task registry ACK and
final shutdown stop ACK; zero retains. Actual task lease3 acquired536895107 bytes
of each RAM and VRAM logical capacity. Guard1 RAM50331648 and resident RAM33554432
remained through task ACK and were cleared only after physical stop ACK. Native
owner generation advanced4→6 with the same private instance/profile/settings.
All four session closes succeeded. Recorded intervals are load1020950ns,
publication32764ns, forward-and-drain2228239ns, physical stop22891ns and stop
ACK5048ns. These single-call observations establish no performance comparison.

The remaining retained calls, both retains, B switch/reload, five comparison
owners, pressure refusal and3versus5 objective were not executed. None may be
inferred from unit tests, earlier facade integration or cancellation supplements.
The original main result remains failed. Any additional main protocol requires
prospective parent coordination after reviewed source fixes.

A second source-review defect was found before later main assertions ran:
Started events identify run/node/task/attempt but need not carry a reservation ID;
Completed carries the exact lease. The producer must join their shared identity
and match the Completed lease to genuine native/registry records. It must not
invent a Started lease or weaken the Completed join.

Before either separately admitted cancellation case executed, its
metadata terminal query was found to have the same projection prerequisite. The prospective
repair uses the existing typed diagnostics refresh through an admission-checked
facade forwarder; this changes diagnostic readback only. Each refresh advances selected projections by at most one bounded page, without
a new native operation or writable alias. The producer must verify it caught up.
Source, binary, fixture and environment gates were independently reviewed and
frozen again before their first invocation, as recorded below. Original main
source/binaries/evidence stay intact.

Publication remains blocked by the unanswered automatic-review rejection of the
exact12-file07138639 source/test upload to `MrScripty/Pantograph`. No retry,
alternative route, merge or branch deletion is authorized. Local source repair
does not satisfy M4-C5 published-head CI.

The metadata repair received independent source review from two separate reviewers.
Its11pure producer tests and one no-model facade projection regression pass.
The original unqualified exact-name filter ran zero tests; the qualified run then
failed during Pumas initialization because its default registry directory was
read-only. Both logs are retained. The explicit disposable registry configuration
passes and performs zero native loads/forwards/stops. A misspelled npm script
failed; the actual critical lint gate passes. These are supporting-check outcomes,
not retries of a qualification case. Those checks preceded the independently frozen native supplement.

Final repair gates pass: warning-deny embedded library/tests Clippy (existing
optional inference dead-code warning disclosed), workspace format, critical lint,
accessibility and staged traceability. [Original repair-check logs](m4-metadata-repair-v1/original-validation-logs.tar.gz) preserve failed and successful commands.


## Two-case cancellation supplement

Both separately admitted cases ran once at clean source
`96691fd764ddf6af838e66c3b064cd34f419b1c5`, tree
`00103d81a0926035d5a2127239a86db8d7d959a6`, with frozen test binary SHA256
`6d318a6465b568a8d7d013c24cc1c35c7114caac95deeabe3d4ae2625508ad09`.
Independent preflight verified966 source hashes,14 fixture hashes and all manifests,
the exact two-case list, fresh isolated registry/evidence paths and full additive
budgets. Execution03:36:06.900–03:36:07.697UTC exited0:2passed,0failed,0ignored.
No case was retried. [Original supplement archive](m4-cancellation-v1/original-evidence.tar.gz)
retains every manifest, build/list log, execution status, case JSON and registry.
The executable remains externally retained at `/workspace/m4-cancellation-v1/binaries`.

| Case | Actual outcome and deciding evidence |
| --- | --- |
| Post-load, before Start | One real load/publication, zero forwards/stops. Task remained Ready without active attempt. The owned facade shutdown signal refused run and shutdown, fenced the private owner, and retained the same600MiB guard plus32MiB resident charges. Both actual errors are retained. This is a native preparation signal, not public active-task cancellation. |
| Public Running, after drain | One load and one forward/drain. Public cancellation targeted the observed Running attempt; request preserved both leases and all charges. Completion won and the terminal run is Completed. Exact task lease3 removal returned the idle capability; guard1/resident stayed charged. One final stop/ACK cleared every lease, claim and resident owner. Projection cursors12/8 and metadata charge preservation are recorded. |

These JSON records contain actual session/run/task/attempt/lease identifiers,
registry instance and charges, counters, public results and projection responses.
The genuine cap/receipt barriers are supported by the frozen source; raw native
profile/generation/full phase receipts and ephemeral SQLite event streams were
not captured by these cases. The first case's full fence is observed before
owner destruction, not a post-process lifetime claim. No compute interruption,
per-attempt timing/training or full native identity qualification is inferred.
The second returned vector matches the existing golden within declared tolerance.

The supplement establishes its two bounded cancellation custody observations.
It does not replace the missing retained/comparison/pressure main steps. M4-C4
remains Partial/Blocked and M4-C5 publication/exact-head CI remains blocked.


[Independent raw supplement review](m4-cancellation-v1/independent-review.json)
passes the bounded custody claims and re-verifies every source/input/manifest/
binary hash without modifying originals. Review SHA256:
`bf59970f50683b3952f2491042ecb61599321c344593255b4f75d27076046797`.
It explicitly preserves the main failure and observability limits above.
