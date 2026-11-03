# Independent combined local C4 review

Verdict: the original bounded LOCAL M4-C4 evidence contract is satisfied by
the once-run 9ce4ca8a MAIN protocol plus the older 96691fd7 cancellation custody
observations, transferred only through the source-assisted proof below.
No new native run is required for that existing local contract. This is not
exact-9ce cancellation execution, general suite reliability, C5 acceptance or
overall M4 acceptance. Published-head embedded-runtime CI failures are relevant,
unresolved contrary test evidence and remain a material C5 blocker.

## Contract and criterion reconciliation

The reviewed contract is docs/plans/cpu-scheduler/plan.md at
9ce4ca8aca004cf51782c59e91dd9fd7af5beed5, SHA256
6eb5a3affd384c64e4c634f94fe5f507776010b25cc79eeec7450ed3664e6bb1.
Its Fixed evidence contract explicitly admits five fresh acknowledged comparison
facades, the two separately frozen cancellation cases, completion winning a
post-drain cancellation request, and controlled faults as supporting protocol
evidence. Its main-v2 continuation explicitly preserves the separate 96691fd7
supplement with disclosed limits and requires no additional cancellation
execution. C4 names actual fixed synthetic counts, outputs, phases, pressure
and cancellation custody; C5 separately requires source/producer review,
affected checks and exact published-head CI.

| Criterion | Deciding evidence and limit |
| --- | --- |
| Freeze and unchanged fixture bytes | MAIN source/tree/binary/24 input hashes and 14 fixture hashes passed prelaunch; supplement has its separately reviewed source/binary/8-input freeze. No acquisition or regeneration. |
| Fixed A,A,B,B,A work | MAIN retained owner has 3 loads, 2 retains, 5 drains, 2 switch stops and 1 final stop. Five fresh singleton comparisons have 5 loads/drains/stops with acknowledged teardown before the next owner. |
| Identical inputs and consumed declarations | hello world, one CPU thread, original untrained BERT8/12 and goldens. MAIN capacities 1104/1024 MiB, guard/resident 48/32 MiB, task per-kind maxima 536895107/536908694 bytes; pressure 80 MiB RAM. Cancellation's separately frozen guard is 600 MiB, resident 32 MiB and RAM capacity 2 GiB; these are not presented as identical MAIN envelopes. |
| Ten real outputs | Parsed stored body, public return and retained readback agree; all ten pass original tolerances, maximum absolute error 5.960464477539063e-08. Recorded BLAKE3 joins agree; independent standard-library review does not recompute BLAKE3 bodies. |
| MAIN native lineage and logical joins | Eight distinct private loaded instances; full instance/profile/settings/generation and run/node/task/attempt/request/candidate/lease joins; genuine warm +2 lineage and exact task ACK. Repeated candle-1 or numeric lease values are not global identity. |
| Reload and old owner history | Reloaded A has a fresh actual native instance. Unchanged serial_ready.rs clears prior samples/predecessor when the current owner changes; this history-clearing part is source-assisted, not a serialized history dump. |
| Charge custody | MAIN full task+guard+resident charges remain through actual drain; exact task ACK removes only the matching task lease. Switch/final native stop ACK consumes the old owner before release/incoming load. Final successful registries are empty. |
| Phase records and bounds | Actual load/publication, forward-and-drain, physical stop and separate ACK scalars; 61 ordered rows, no overflow, original 128-row/8-KiB-row/1-MiB limits. Missing phases fail; no zero substitution or speedup inference. |
| Pressure | One fresh BERT8 refusal: 620781187 required RAM exceeds 83886080 capacity; no Start/load/forward/claims. Matching raw Failed terminal, preserved error, successful session closes/shutdown and empty registry. |
| Post-load pre-Start custody | Separate 966 case: 1 actual load/publication, 0 forwards/stops, Ready without active attempt, real owned preparation shutdown signal, run/shutdown refusal and inaccessible full guard/resident fence. Final observation precedes owner/TempDir destruction. |
| Public Running post-drain custody | Separate 966 case: 1 actual load/drain, public request targets the actual Running attempt while all charges remain. Completed wins, exact task-only ACK returns idle custody, final 1 stop clears every charge/owner. |
| Controlled faults | Existing drop/pin/retainer/stale-proof/failed-stop/ACK branches remain source/test or fault-injection evidence. No simulated stop failure is called actual Candle failure. |
| Once-only execution and honest failures | v1 and v2 stay failed; v3 stays rejected before models; repair and v4 are separately admitted fresh work. No failed owner is resumed or recovered. Original bytes and rejection markers remain intact. |

The MAIN rows are supported by the prior independent raw review, whose JSON
SHA256 is 21c39599e4a858838a378c3fb5489b53e92cca189fb4a44f0911b06e4c638ce0.
It independently checked all seven owner directories, 125 SQLite/export events,
61 collector rows, 16 session closes and native stop/ACK ordering. The actual
outer terminal result and child both exit 0, supervision is reaped/complete,
and no wrapper errors, forced signals, retained owner or failure artifact exists.
OS containment is never native cleanup authority.

## Cancellation source bridge

The supplement actually executed once at
96691fd764ddf6af838e66c3b064cd34f419b1c5, tree
00103d81a0926035d5a2127239a86db8d7d959a6, test binary SHA256
6d318a6465b568a8d7d013c24cc1c35c7114caac95deeabe3d4ae2625508ad09.
Both cases passed at exit 0. This review reverified its 16 preserved files, actual
binary, 8 pre-run inputs and all 966 frozen source hashes against original Git
blobs. Raw case JSON confirms the recorded task/attempt/lease, charges, public
request, terminal, exact cleanup and conservative fence invariants.

Only six Rust paths change from that source to MAIN 9ce: producer plus synthetic
AABBA tests, two production workflow worker/finalizer files, and two workflow
test files. Git tree comparison establishes 433 identical blobs across inference,
registry, runtime-host contracts, embedded native source/tests, serial_ready,
runtime_branch_run_finalization and dependency manifests. Thus the real load
completion/publication, cfg(test)-only pause placement, owned shutdown signal,
native pre-Start recheck, cap/receipt custody, registry task release, active-task
cancellation API, facade shutdown supervision and qualification case code have
not changed.

The load gate is after genuine load/publication while the owner cell is fenced
and before the production shutdown recheck can publish idle authority or Start.
A shutdown signal refuses that recheck while keeping charges. The drain gate
holds the genuine receipt before cleanup during public Running; cancellation
intent cannot supply its cleanup capability or erase charges. The same production
custody paths remain in 9ce.

The changed caller settlement accepts failed/cancelled outcomes, settles its
event first and requires the matching registered current task to own unassigned
work without batch/event custody. Under one store lock, the new finalizer checks
coherent active graph/run identity, no attempts, no Running records and no
pending cleanup before metadata finish. It calls no native stop or reservation
release and cannot rebuild a cap or open the serial fence. Therefore the older
pre-Start cancellation refusal may now gain a workflow metadata terminal while
the native fence remains; that new metadata outcome is source-inferred, not an
actual 9ce cancellation observation. The real Running/drained case is excluded
by custody guards and its successful outcome bypasses the failure settlement.
The ordinary finish/record sequence is factored unchanged.

The independently reviewed exact-M4-path deterministic regression requires one
preparation failure, zero forward/cleanup calls, no Start, one Failed terminal,
metadata close and a still-closed serial fence. Started/unacknowledged cleanup
tests protect the complementary boundary. Existing logs record the exact
checkpoint's eight preparation tests, invalid-dispatch regression, 1020 workflow
library plus 65 integration tests, 12 producer readback tests and 4 synthetic
AABBA tests passing; no tests were rerun here. These tests support composition
and are not substituted for actual native observations.

This bridge preserves the supplement's limits. Its private native profile/
settings/generation and full phase/cleanup receipts were not serialized.
Ephemeral per-case SQLite/raw native logs were not retained. Genuine barriers
and cap return are source-assisted, not newly reconstructed event replay.
The local 600/32 MiB guard/resident snapshots, logical session/run/task/attempt/
lease joins, actual errors/counts and returned vector remain the observed data.
No full MAIN-style private native lineage is inferred for these two cases;
no interrupted compute, cancellation-terminal winner, per-attempt training,
post-process resident lifetime or recovery is qualified.

## Failed attempts remain failed

MAIN-v1 at 7139b2ad exits 1 after its first successful embedding because diagnostic
inspection did not refresh materialized projections. Its original 54 MAIN files
still match the preserved prefix map. Later explicit bounded refresh/readback
and realistic Started/Completed joins repair observation without changing that
failure or crediting missing work.

MAIN-v2 at c25fd49d exits 1 despite correct retained/comparison counts and vectors:
pressure has no Failed terminal and execution-session close returns scheduler_busy.
The later metadata repair addresses that precise gap; it does not make v2 pass.

MAIN-v3 at 9ce was NOT_ADMITTED with zero native invocations because the wrapper
could lose a live child's completion owner after post-spawn observer failure.
The failure marker, rejected wrapper and all 40 stopped-root files remain intact.
The separate wrapper repair has 11 model-free process fault tests and independent
review; v4 uses its exact pair identity and outer-exit/failure veto. Its reviewed
success neither reuses the stopped claim nor weakens native criteria.

All 360 MAIN-v4 post-execution hashes and all 637 historical preservation hashes
match during this review, including v2, stopped v3 and repair history. Prior
MAIN-only and supplement reviews retain their original limited verdicts.
This combined source-assisted decision is a later scoped disposition.

## Contrary CI evidence and exact C5 handoff

Read-only preserved remote logs identify these exact failing published heads:

| Head | Quality Gates / focused job | Observed embedded-runtime result |
| --- | --- | --- |
| PR83 5b4fc6dda17342e6b27f90ebb852ffd2b7ea47eb | 39368594419 / 118451011171 | 600 passed, 3 failed, 2 ignored; exit 101 |
| PR84 9e068bbc2a040f20de88dfc70d914339cd5a8910 | 39371059809 / 118458798990 | 600 passed, 3 failed, 2 ignored; exit 101 |

The command is cargo test -p pantograph-embedded-runtime --lib. In both,
actual_facade_session_uses_private_load_owner_and_acknowledged_shutdown reports
infer already TerminalFailed at serial_cpu_residency_tests.rs:532; the two
native cancellation tests end with Elapsed at final result unwrap lines 328/456.
The logs do not establish which awaited step timed out or why. Environment,
fixture, inventory, concurrency or production causes remain hypotheses.
These are relevant failures of the very native tests discussed here; they are
not dismissed as unrelated, waived, relabeled, or overridden by local successes.

They do not retroactively change the captured once-run local observations or
the identity of unchanged custody mechanisms. The original local C4 contract
accepts those named local observations with source assistance, so that bounded
evidence criterion is supported. General reproducibility/current broad-suite
reliability is not established; these failures block C5 and overall M4 acceptance.
The three other successful workflows at each precursor head do not supply
missing Quality Gates success. No exact composed/pending published head CI
exists. The recorded broad Candle dependency Clippy failure is also retained;
scoped producer/supported-backend lint only proves its declared configuration.

The candidate handoff report was reviewed for criterion coverage, source transfer,
failure preservation, CI block and minimal integration order. Its proposal keeps
PR83's accepted M3 base/history, adds existing local e8d5d20d's exact three-file
PR84 repair and 9ce's M4 regression, then bounded documentation/evidence. Parent
must coordinate the overlapping PR84 patch rather than integrate it twice.
Any future publication needs separate narrow authorization and remote-ref
readback; exact published-head affected checks/required CI must pass before C5.
Resolving these actual embedded test failures requires a bounded reviewed
diagnosis/repair proposal; this handoff performs no repair or rerun. Parent owns
base movement, merge order and overall acceptance. No branch closing/deletion,
PR metadata update or remote mutation is authorized here.

Signed off by /root/m4_v4_admission_review for bounded LOCAL C4 only. This review
writes only this new report and independent-review.json outside original roots.
No models, Cargo, reference generation, download, environment change, publication
or merge was performed. The companion JSON records exact evidence/source hashes
and binds this report; both output digests are returned to the parent.
