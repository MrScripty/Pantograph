# Explicit native last-lease eviction and reconciliation

This slice adds one consuming operation to a custody-bearing, successful and
physically drained Candle CPU attempt: `CandleCpuDrainedAttempt::evict_last`.
It can retire the actual loaded allocation when that exact task is the only
remaining ephemeral, unpinned lease. It does not activate default scheduling or
change `EmbeddedRetainedCpuSerialPort`'s retained-only cleanup contract.

The current warm attempt still requires a retaining successor at admission and
drain. This explicit operation covers the subsequent boundary when the successor
has ended. Integrating an automatic retained-versus-evict choice into the generic
workflow, and admitting an initial sole lease, remain separate composition and
admission decisions. There is no last-lease fallback in existing cleanup APIs.

## Ownership handoff

The native receipt captures its original gateway, registry, exact lease, private
loaded-owner stamp and execution custody. Eviction accepts no replacement owner,
gateway, registry, copied snapshot or caller-supplied release frame. The gateway
first holds its actual backend writer, validates the private loaded owner and
current config/lifecycle, and checks host cancellation.

Under one registry admission lock, the same execution guard becomes a non-Clone
last-lease stop ticket. The ticket binds its nonce, complete lease, producer
source, runtime instance, model target, last delivered producer sequence and
unchanged resident resource envelope. It requires exactly one active lease and
one matching unpinned model, an ephemeral unpinned task and known resident RAM
accounting (and every other kind charged by that task). Successors, pending
custody, unknown accounting and changed identity refuse before physical stop.

The handoff marks the runtime Stopping and installs a runtime-wide fence. Both
task and resident charges remain intact. Ordinary release/replacement/retention,
all admission paths, producer observations, unsequenced health/omission
projections, runtime transitions and reclaim cannot clear or replace this
obligation. Generic projections with no error return simply leave it unchanged.
The existing producer binding protects omission handling; the active lease also
remains present. Registry metadata registration and safe capacity configuration
do not grant ownership or clear these charges.

There is no await between installing the ticket and moving that exact ticket
and actual backend writer into supervision. The worker checks cancellation once
more immediately before native stop. After that irreversible boundary it runs
the actual backend stop/drain and acknowledgement regardless of host cancellation
or collector loss. It invokes the held backend directly and avoids reentrant
gateway stop helpers. The supervisor retains the writer until reconciliation
finishes, preventing an intervening load or backend replacement.

## Actual acknowledgement and delayed observations

Candle stop invalidates the private owner, drains its native load and execution
workers and removes the loaded model. The supervised gateway verifies readiness
and actual CPU residency are absent, resets its modes/config/lifecycle, then
allocates a checked sequence from the actual gateway producer counter while
still holding the same backend writer.

Allocating after physical stop makes the sequence newer than every previously
sampled frame, including frames not yet delivered to the registry. Comparing
only with the last delivered sequence would be insufficient. The registry
ticket synchronously rechecks its exact nonce, lease, source, instance and
unchanged envelope before atomically clearing the selected claim, resident
envelope and fence. It preserves the source/sequence tombstone, so delayed
pre-stop Resident frames cannot resurrect the allocation. A genuinely later
trusted producer load remains possible. Public registry ticket acknowledgement
is a trusted producer boundary, like existing producer observations; its labels
alone do not prove physical release. The public native receipt API obtains that
proof from its captured actual backend.

Host cancellation detected before ticket preparation leaves the selected
execution custody charged and fenced; the runtime can remain Ready. Cancellation
after handoff but before irreversible stop leaves the runtime-wide ticket fenced.
Failed stop, supervision panic, lost ticket, sequence overflow
or failed acknowledgement also keep accounting and exclusion. No Drop path
reclaims them. Ordinary teardown afterward does not recover a lost ticket.
The obligation survives collector loss within this process; this is not a
persistent journal or cross-process recovery protocol.

## Bounded work and acceptance scope

Ticket preparation and acknowledgement use exact map/set lookups and singleton
length checks, with no global reservation or model scan. Comparison/clone cost
still depends on the bounded native receipt metadata and configured resource
claims; map operations depend logarithmically on registry size. The inherited
native stop/drain and filesystem paths have no hard wall-time ceiling. No stop
latency, load/transfer forecast or physical capacity is fabricated.

Synthetic registry fixtures test charged state throughout pending stop, ordinary
and observed/provisional admission fencing, release/retention/transition/reclaim
refusal, all observation forms, tombstones, invalid acknowledgement and ticket
loss, exact unchanged envelope, successors, pins and prepared-custody refusal.
The synthetic shared RAM fixture explicitly checks task plus resident charges
remain 110 declared bytes during stop and become zero only after exact ACK.
These are fixture declarations, not measured physical capacity.
Actual committed CPU numerical fixtures run cold bootstrap and successful warm
forward/drain before consuming eviction. They test vectors/usage, native absence,
delayed actually sampled producer frames, retaining successor and stale-owner
refusal, pre-cancellation, and cancellation/collector loss on both sides of actual
stop. A private-owner change during paused preparation refuses physical stop.
Test hooks pause around real stop and do not fabricate release. Controlled
supervision fault tests failure before physical stop; checked counter overflow
tests failed reconciliation after actual successful stop. Neither is an injected
native backend stop-error qualification.

## Remaining gates

Abandoned execution recovery requires separately evidenced quiescence and remains
unimplemented. Generic eviction composition, initial sole-lease execution,
durable restart recovery and stop deadlines remain unresolved. The complete
research scheduler, full phase-level CPU/compute/link capacities, realistic
dispatch-cost acceptance and simulator differential testing remain outstanding.
Supported simulator archive materialization/hash verification is still missing.
No GPU, native batching, remote-node or pretrained semantic evidence is inferred.
The independently qualified combined Pumas cohort still awaits coordinated pin
adoption; manifests and production pins are unchanged. Separately owned
inspector/audio/rerank adapters and the unqualified physical-owner successor are
outside this slice.

## Recorded acceptance

The final locked/offline six-package test run passed 2,627 primary cases, zero
failed and seven ignored: 2,254 library, 372 integration and one doctest. Raw
output repeats one included calibration child, which is not counted twice.
The unchanged seven ignores are two controlled dispatch cost probes, two hosted
Pumas profile tests, one two-completion cost probe and two inference doctests.
All configured runnable cases passed without exclusions.

The completed two-package library graph separately passed 562 inference and 87
registry cases. Independent direct execution of those final binaries passed 18
native warm/custody/eviction cases and 12 registry retained/custody/eviction cases,
30 total with zero failures, including the six new native and four new registry
cases. This is focused independent acceptance, not a second full aggregate run.

Published parent `5c82c0223552bf7c2ddb3c0418c8aeda239b8561` passed exact-head
Quality Gates (37724963370), Runtime Separation (37724963373) and Headless
Workflow Contract (37724963359). PR68 remains draft/open on that serial head.
Those hosted results do not qualify this unpublished local eviction slice.
There is no main merge or default activation.

Six-package all-target Clippy passed with `--no-deps -- -D warnings -A dead_code`;
the allowance is the existing non-PyTorch selected-load baseline. Default-feature
inference checking, formatting and whitespace checks passed. Cargo was locked
and offline with `ORT_SKIP_DOWNLOAD=1`, `HF_HUB_OFFLINE=1` and
`TRANSFORMERS_OFFLINE=1`; no model, runtime or ONNX downloads occurred.
