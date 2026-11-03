# Native CPU retained-owner cleanup groundwork

This additive slice starts from PR68 head
`eb091013425d1bea3ae26e2664cc92f7582010cc`. It provides opt-in native gateway
and registry methods. No scheduler selector, dispatch adapter or default policy
calls them. PriorityThenFifo, starvation boosts and the one-position warm-reuse
window remain the production scheduling behavior.

## Narrow contract

The native gateway holds its actual exclusive backend writer while it checks
an idle, ready Candle CPU allocation. A private stamp binds the actual
calibration owner Arc, loaded profile, backend instance, execution settings and
current even load epoch. Backend selection, lifecycle instance, configured model
and task lease model must match. Identity is rechecked after awaited reads and
before the synchronous registry operation. Warm selected load advances the
epoch even when it verifies and reuses the same allocation, so a fresh stamp is
required after service. No invalidation is removed.

The registry operation takes its admission lock once. It requires the entire
expected live lease to match, no pending custody for that lease, matching actual
producer source/instance/model residency, explicit non-uncertain resident RAM
accounting, and resident accounting for every resource kind charged by the task.
Another real live lease must retain the allocation. Only then is the completed
task lease removed. The successor's lease, claim and resident envelope remain.
Last-lease eviction, pin-only retention, stale identity and unknown accounting
refuse before mutation.

The core inspects at most two active reservation IDs to choose one successor.
It avoids general retention's scan for a preferred KeepAlive reason; an
Ephemeral successor proves ActiveReservations even if a later KeepAlive exists.
Map/set operations remain logarithmic in registry size, and identity work
depends on identity byte lengths. This is not a constant-time or hard latency
bound. Ordinary registry release retains its existing general retention logic.
The new API has a separate error type to preserve existing registry contracts.

The public core identity labels are facts, not an execution capability; its
caller must hold the actual producer guard. The gateway provides that guard.
The caller still asserts completion of the particular lease. This slice does
not mint an attempt, task-drain or serial cleanup proof and does not make
generic serial Ready admission eligible.

## Qualification scope

Core synthetic tests cover exact lease/source/instance/model refusal, pending
custody and rollback, unknown/partial resident accounting, successor retention,
last-lease refusal and 512 additional leases with a late KeepAlive. The latter
distinguishes this bounded successor choice from general retention scanning.

Native tests execute committed untrained BERT CPU fixtures through the real
gateway. They compare actual vectors and usage across cold service, verified
warm service and another warm service after retained cleanup. They verify the
same allocation survives, old load epochs refuse, fresh stamps release only
the task, foreign gateways cannot provide authority, actual replacement refuses
the old owner, missing resident estimates refuse, and ordinary stop removes
the physical allocation. Ordinary stop is test teardown; it does not qualify
last-lease cleanup through the new API. No Ready state, vectors, elapsed time or
native instance is injected. Test RAM estimates are explicitly synthetic
operator declarations, not allocator measurements or measured CPU capacity.

Tests use cached dependencies, `ORT_SKIP_DOWNLOAD=1`, `HF_HUB_OFFLINE=1` and
`TRANSFORMERS_OFFLINE=1`. These direct native tests do not override the Pumas
dependency pin and do not qualify the previously blocked hosted workflow path.
The earlier Pumas `70e45d8c` evidence remains preserved; the integration owner
reported its production behavior is already present in the combined cohort.
That obsolete-base patch remains unpublished. A qualified combined pinned
artifact is required before mapping the earlier hosted CPU evidence to it.

## Recorded verification

The final affected `inference`, `pantograph-runtime-registry` and
`pantograph-embedded-runtime` suites passed 1,340 primary tests including one
doctest, with six ignored (four embedded fixtures and two doctests), zero failed
and no excluded tests. A calibration subprocess repeats one included test; its
extra result line is not another unique test. All 1,198 library tests and 141
integration tests passed. The published Pumas pin and manifests are unchanged.

Independent source review closed the hidden general-retention scan, comparison
clone and legacy-error-contract findings. Independent execution on the final
compiled binaries passed all five retained registry tests, all three actual
native cleanup tests and all ten existing calibration tests. No remaining
source blocker was reported.

Formatting and whitespace checks passed. All-target Clippy for the three
affected packages passed with `--no-deps -- -D warnings -A dead_code`; the
dead-code allowance covers the existing non-PyTorch `SelectedTextLoad` warning.
This is not an unqualified warning-deny result for that feature configuration.

All three hosted workflows passed on published PR68 head `eb091013`: Quality
Gates run `37715442414` (all 13 jobs), Runtime Separation `37715442356`, and
Headless Workflow Contract `37715442358`. Those hosted results qualify that
published head only; this new slice remains local and has no hosted CI result.

## Remaining architectural gates

An async Evict/stop operation needs durable ownership and reconciliation across
cancellation and delayed producer observations. Releasing claims before an
unfenced stop could harm a replacement; holding a local mutex alone cannot fence
all direct gateway entry paths or resurrected observations. This slice refuses
Evict rather than choosing that architecture implicitly.

Native serial integration still needs held dispatch authority, actual
attempt/drain proof and a verified warm-load receipt across the epoch change.
Qualified cold/load/transfer/release timing is missing. Private CPU thread-pool
and GEMM settings do not establish full phase-level compute/link capacity.
Authoritative GPU capacity, GPU/native batching and remote-node evidence remain
absent. The full research scheduler and simulator differential acceptance remain
unqualified; supported Library archive materialization is still unavailable.
