# Native retained warm attempt and receipt decision

This next bounded slice starts from reviewed PR68 head `9fae65ab`.
Keep async eviction out of this capability: another live lease must retain the
allocation, and last-lease cleanup refuses unchanged. The accepted registry
fence can therefore serve success-path cleanup without async stop/reconciliation.

## Chosen native boundary

Use a dedicated opt-in native warm attempt that shares the existing selected
embedding pipeline. Bind eight bounded caller-custody attempt identities to the
actual request and exact selected registry lease wherever the native DTO has
matching fields. Run/node/attempt/candidate labels are caller custody until an
actual workflow adapter exists; they are not independent admission proofs.
The complete request/target/decision undergo one borrowed preflight capped at
64 KiB of raw bytes, 2,048 nodes and depth 32 before validation, hashing or
cloning. This bounds in-memory preparation, not filesystem latency or physical
worker duration.
Artifact rechecks inherit the existing loader's filesystem reads; this slice
does not establish a byte/memory ceiling or elapsed-time bound for that I/O.

The caller must exclusively own the selected lease's admission/session lifecycle
through receipt consumption. Read-only registry preflight does not pin ordinary
same-lease mutations. A controlled external-release race must refuse the receipt
after actual drain and cannot claim to preserve an externally ended peak claim.
A general native serial port additionally needs private attempt custody checked
by every same-lease mutation, or actual exclusive lifecycle custody from the
existing worker. Do not hide that extra admission architecture in a receipt API.

Acquire the actual owned backend writer before starting a supervised native
attempt. The worker retains that guard through load, execution and physical
worker drain even when the result collector is dropped. Collector drop requests
cooperative cancellation; it does not detach the backend guard from live work.
No receipt or claim release is possible on abandonment or cancellation.

The strict load route requires the existing model to match verified load inputs
and target. Mismatch refuses before constructing/replacing tensors. Default
selected loading retains its current cold fallback. Only the actual successful
load-publication CAS may mint a private, non-Clone reuse receipt. It must prove
the same authority, model/profile, instance and effective settings, with the
exact checked old-even to odd to new-even transition (old plus two). A boolean
reuse label, changed snapshot or tolerated generation gap cannot substitute.

After real embedding forward, actual drain, response/usage validation and final
cancellation/settings checks, return a private non-Clone drained attempt receipt
bound to that selected lease and request. Consuming it uses the accepted native
retained-owner fence with its new owner. This is the first native success-path
attempt/drain to retained cleanup integration; it does not alter serial admission.

## Generic contract remains blocked

The scheduler executing ticket currently checks exact pre-dispatch owner
equality at drain. The workflow serial adapter also checks exact equality of
post-drain collection evidence and passes the original owner to cleanup.
Changing only one equality or returning an ordinary new snapshot is unsafe.
The future host/workflow contract must carry a linear attempt-bound new-owner
cleanup capability and explicitly verify the receipt transition. Preserve the
old ranked proof and all existing equality checks until that handoff exists.
Do not implement `SerialRuntimeHostBatchExecutionPort` using the ordinary native
port or declare native serial Ready eligible from this slice.

## Acceptance before integration

- Real committed CPU fixtures: cold bootstrap through the existing path, strict
  warm attempt with actual vectors/usage, exact epoch transition, physical drain,
  and receipt consumption releasing only the completed lease.
- Foreign/stale owner, mismatched request/lease identity and oversized tags refuse
  before forward; claims remain charged and no receipt is issued.
- Modified load inputs/target refuse before cold replacement; original native
  instance and claims survive, while calibration may safely invalidate.
- Cancellation and collector drop issue no receipt, retain claims and keep the
  actual backend writer held until controlled real worker drain completes.
- Replayed/foreign cleanup and intervening direct gateway load/replacement cannot
  release claims through the receipt; last-lease cleanup remains unsupported.
- Existing gateway, calibration, reservation and default workflow regressions
  remain passing; independently review the new source and real CPU tests.

Cold/load/transfer/release timing and full phase-level CPU/link capacities remain
unknown. No GPU/native batching or remote-node evidence is added. The combined
qualified Pumas artifact is still required for hosted workflow qualification;
do not duplicate its obsolete-base patch or reuse the Python POC implementation.

## Recorded qualification

The final locked/offline affected inference, registry and embedded suites passed
1,349 primary tests, zero failed and six ignored, including one passing doctest
and two ignored doctests. A calibration subprocess repeats an included case and
is not counted twice. All 1,207 library and 141 integration tests passed.
No test was excluded; published Pumas pin and manifests remain unchanged.

Formatting and diff checks passed. Scoped all-target Clippy passed with
`-D warnings -A dead_code`; the allowance covers the existing non-PyTorch
selected-load fields. The locked/offline default-feature inference build also
passed, with that existing dead-code warning. The final loader-helper relocation
only changes item order to satisfy Clippy; its reviewed body and calls are unchanged.

Independent source review approved the explicitly caller-custodied retained-only
receipt after closing default-cancellation, metadata-bound and final-revalidation
findings. Independent execution on final binaries passed nine native attempt
cases, five retained registry cases and ten calibration cases. Native tests
include actual cold/warm vectors and usage, exact load epoch transition, strict
artifact refusal before replacement, foreign/pending/changed lease refusal,
oversized/deep/wide DTO refusal, intervening-load/drop refusal, cancellation and
collector abandonment at real load/forward/drain boundaries, and external lease
release during actual work refusing any receipt. Control hooks run only in tests;
they inject neither results nor timing evidence.

Published PR68 parent `9fae65abe41ad91434b984f95f95dee94438dda2` passed all three
exact-head workflows: Quality Gates `37718493576` (all 13 jobs), Runtime
Separation `37718493558` and Headless Workflow Contract `37718493569`.
Those results qualify that published parent only. This new receipt slice stays
local and has no hosted CI result. No default scheduling or main merge occurred.
