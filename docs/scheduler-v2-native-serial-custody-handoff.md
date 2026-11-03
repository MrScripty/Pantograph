# Retained native CPU serial dispatch and owner handoff

This slice builds on published receipt commit `9f803851`. It connects the actual
opt-in `WorkflowService::new_serial_ready_cpu` worker to a dedicated
`EmbeddedRetainedCpuSerialPort`, the strict native warm attempt, its qualified
drain receipt, and successor-safe registry cleanup. Default hosts and the
PriorityThenFifo policy remain unchanged. This is a restricted resident CPU
integration, not the full researched scheduling algorithm.

## Exclusive selected lease

The registry issues an opaque, non-Clone execution custody guard for the exact
already-admitted lease. Acquisition rejects provisional lineage, changed lease
fields and an existing execution guard. Under the registry lock, same-lease
ordinary/observed/provisional replacement, release, retention updates and
unqualified retained cleanup refuse while this guard owns the lease. Read-only
queries are advisory and cannot mutate the claim.

The native supervisor takes this guard before acquiring its actual backend
writer. Prepared guard Drop removes only its own nonce and leaves the admitted
claim intact. Immediately before invoking strict native load under that writer,
the guard becomes irreversible. Cancellation, collector loss, receipt loss,
invalid cleanup or owner change after this point leaves the selected claim
charged and fenced. Only consumed successful retained cleanup removes it.
There is deliberately no automatic recovery API, retry or ordinary release
fallback for an abandoned execution guard. The generic serial admission also
stays poisoned without matched drain and cleanup acknowledgment.

The guard moves into the existing actual native attempt binding, survives real
load/forward/physical drain, and moves into that SAME linear receipt. No receipt
map, independent lifetime manager or metadata-only cleanup authority is added.
Another live lease must retain the allocation; last-lease and missing resident
accounting cases refuse. If the successor disappears before cleanup, the selected
claim remains charged and fenced.

## Verified linear transition

The scheduler has an additive trusted-host receipt path. It validates all eight
attempt identities, exact predecessor for a ranked dispatch, unchanged loaded
instance/profile/effective settings/thread budget, and checked even-epoch `+2`.
The actual native receipt attests the private successful load publication and
physical success/drain. Existing held-owner acquire/drain equality paths stay
strict for other ports.

The host returns response, scheduler drain proof, actual current owner and a
non-Clone cleanup capability carrying the native receipt. The workflow validates
its predecessor/current owner and generation authority before storing the SAME
capability. Cleanup consumes it; it accepts no caller replacement gateway,
registry, lease or owner. A copied snapshot cannot invoke native serial cleanup.
The admitted workflow task ID is bound separately from the native embedding
workload type.

The terminal native response uses `DeferredToScheduler` because the receipt
still owns cleanup. The existing orchestrator maps this instruction to
`RetryDeferred`. Only an actual native success/drain receipt accepts that matched
event (or `RuntimeHostCompleted`); failed/deferred DTOs do not mint a receipt.
Sample recording for this deferred release additionally requires the same
receipt, a successful singleton terminal response and matched cleanup. Legacy
event handling stays unchanged.

## Comparable local service observations

Warm loads advance the epoch, so exact-generation samples otherwise never
accumulate across attempts. After consumed receipt cleanup and a fresh actual
private-owner query, the workflow migrates only samples belonging to the exact
attested predecessor and the same generation authority. Profile, settings,
instance, thread budget, input/readiness key and sample timestamp remain intact.
The existing ring stays capped at 64 keys and eight samples per key; migration
touches at most 512 samples. Arbitrary reload, owner change, failed cleanup and
stale samples cannot authorize continuity.

The public atomic generation handle is an advisory mirror. Native execution and
cleanup use a separate private epoch and actual owner checks; mirror writes
cannot advance the private epoch or restore a stale cleanup stamp. Advisory
mismatch can withhold ranking, and receipt cleanup alone does not fabricate
missing phase timing. Observed cost remains actual dispatch through drain and
cleanup, not a worst-case latency or a full completion forecast.

## Bounds and acceptance

The singleton host request and resolved metadata use the inherited borrowed
64 KiB/2,048-node/depth-32 preflight before projection/cloning. Native execution
uses its configured 1–4 CPU pool, one bounded text input and finite bounded vector
validation. The committed BERT-8 fixtures use one worker and actual numerical
golden vectors and usage. Test timeouts detect hangs; they are not physical work
deadlines. Existing artifact filesystem reads and native worker elapsed time
still have no hard I/O or latency ceiling in this slice.

Acceptance covers actual host dispatch and four public workflow runs through the
real worker, exact epoch transition, vectors/usage, retained session successor,
consumed cleanup and current-generation observation counts. Controlled actual
load/forward/drain boundaries exercise competing release/replacement/retention,
cancellation and collector loss. Protocol fixtures exercise all identity/owner
dimensions, strict epoch arithmetic, first-two Ready ranking after reuse-qualified
samples, and failed cleanup withholding continuity and reopening. Unknown actual
ownership refuses native execution with no cold/unranked fallback.

## Remaining gates

Async last-lease eviction, stop/reconciliation and quiescence-qualified recovery
remain separate architectural work. Full phase timing and CPU/compute/link
capacity evidence are still missing. No GPU/native batching, remote node,
pretrained semantic quality or complete research algorithm is claimed. Supported
simulator archive materialization and differential acceptance remain outstanding.

The combined Pumas cohort `0aa462e1e976c6720cfa7ad357b50457cdec96bd` now has
independent CPU qualification against Pantograph `9fae65ab`: two hosted tests,
four Candle BERT forwards, golden vectors at `1e-4`, cold cleanup, warm reuse and
lease release. Its Library receipt is `libfile_fe4c858b73d88191bc10e4ce62d8a509`;
this executor read that receipt. The archive reference is
`libfile_49d9cba076e48191a41eb273c1929374` with reported expected SHA256
`b7d3312cadc6a7fdf2b2b855d571d7cf2ff560e0f9e949431f500858dcf11da2`.
These are parent/cohort results, not qualification of this new serial port or
local verification of archive bytes. Production pins/manifests stay unchanged
pending coordinated adoption. The separate physical-owner successor has 28
integration failures and is excluded. No obsolete duplicate Pumas patch is used.

## Recorded qualification

The final locked/offline six-package test run passed 2,617 primary cases with
zero failures: 2,244 library cases, 372 integration cases and one doctest. One
calibration child case also appears in raw output and is not counted twice.
Seven cases remain ignored: two controlled dispatch cost probes, two hosted
Pumas profile tests, one two-completion cost probe and two inference doctests.
All configured runnable cases passed without exclusions.

Independent review passed 59 focused cases using completed binaries and approved
only this retained-resident, explicit opt-in scope. Four actual public CPU warm
runs validate physical execution and cleanup continuity; the first-two Ready
ranking assertion remains a separate synthetic protocol fixture. Full simulator
differential acceptance and realistic cost qualification remain outstanding.

Six-package all-target Clippy passed with `-D warnings -A dead_code`; the latter
is the existing non-PyTorch selected-load allowance. Default-feature inference
checking, formatting and whitespace checks passed. Builds used
`ORT_SKIP_DOWNLOAD=1`, offline model settings and locked/offline Cargo; no model,
runtime or ONNX downloads occurred.

Published base `9f803851a038daeb394518413eca250a9a55ec9c` passed Quality Gates
(37721837778), Runtime Separation (37721837800) and Headless Workflow Contract
(37721837859). These hosted results qualify that base, not the new local
integration. The integration remains unpublished, with no default scheduler
activation or main merge.
