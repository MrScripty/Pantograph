# M4 fixed protocol: partial evidence and preserved failure

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

The two separately admitted cancellation cases have not yet executed. Their
metadata terminal query has the same projection prerequisite. The prospective
repair uses the existing typed diagnostics refresh through an admission-checked
facade forwarder; this changes diagnostic readback only. Each refresh advances selected projections by at most one bounded page, without
a new native operation or writable alias. The producer must verify it caught up.
Source, binary, fixture and environment gates must be reviewed and frozen again
before their first invocation. Original main source/binaries/evidence stay intact.

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
not retries of a qualification case. Final lint and freeze precede native execution.

Final repair gates pass: warning-deny embedded library/tests Clippy (existing
optional inference dead-code warning disclosed), workspace format, critical lint,
accessibility and staged traceability. [Original repair-check logs](m4-metadata-repair-v1/original-validation-logs.tar.gz) preserve failed and successful commands.
