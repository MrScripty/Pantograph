# Production observation qualification checkpoint

Base: local qualified `fd224c366645c19f868c36ed331264a82b79db68` (tree
`1be7b921b6ad6a17a187d4427c1ade081c4cd5f0`). Publication is stopped. This
slice is local, observation-only and preserves the default priority/FIFO policy.

## Actual gap

The selected PyTorch text gateway already measures custody wait, selected model
load, text execution and worker drain with an opt-in recorder. Native selected
text execution reaches this guard. Defaults perform no timing work. It is not
limited to authored estimates, but production startup does not attach a collector
and production backends do not implement `runtime_service_timing_owner_facts`.
Only controlled test backends supply those facts. Consequently even an attached
recorder cannot currently produce exact production calibration.

The existing profile binds immutable model content, resolved package, requested
workload/options, selected runtime/device, owner epoch, runtime generation and
backend-owned implementation/effective configuration/physical-device facts. It
does not establish freshness, distinguish injected owners, or distinguish the
actual load outcome. Selected PyTorch text currently reloads and allocates a new
runtime generation per request. A failed or abandoned attempt is already
ineligible for successful phase reuse.

## Exact safe slice

Reuse the gateway's opt-in phase guard and monotonic clock. Add an optional capture
envelope with its clock epoch, capture timestamp, owner provenance and selected
load disposition reported by the backend's successful `BackendStartOutcome`.
`Reloaded` means a new model load rather than reuse; it is not a claim about cold
filesystem caches, allocator state or total preparation. Missing legacy capture
metadata remains unusable for production qualification.

The guard records its timestamp at drop, immediately after the gateway finishes
the attempt; there is no intervening await. It is a capture timestamp, not a
separate backend completion acknowledgement. All capture and query metadata is
trusted in-process evidence; replayed JSON cannot authenticate owner provenance.
Identity preflight borrows the exact payload and caps raw strings/bytes at 64 KiB,
nodes at 2048 and compound depth at 32. Exceeding these bounds leaves identity
unknown without truncating it or rejecting execution. Caller request IDs above
64 KiB omit optional correlation hashing and continue to execute unchanged.

Conservatively mark every `with_backend` owner injected, even if its label says
PyTorch or it subsequently switches to a built-in backend. `InferenceGateway::new`
uses the private built-in registry and may mark observations built-in; controlled
test clocks always force injected provenance. Owner labels cannot upgrade it.

Provide a read-only snapshot of the same instrumentation clock and a pure
qualification accessor. It requires built-in provenance, exact profile equality,
the same clock epoch, a positive freshness limit and checked nonnegative age,
known reload/reuse state and four unique completed observed phases. A clock
regression anywhere in an attempt prevents qualification. Return phase costs
separately; do not convert them to predictions, average generations, infer
transfer zero or enable completion ranking. Failure, cancellation, caller drop,
configured values, duplicate phases, injected owners and missing facts refuse.

Tests use controlled clocks and fake backends only. They cover clock domains,
freshness boundaries/regressions, identity mismatch, separate load/execution
costs, unknown reuse, failed/canceled/abandoned attempts and disabled defaults.
An independent reviewer checks this diff and focused tests before the local
implementation commit. No model execution or runtime/model download is needed.

## Remaining architectural decisions

1. The PyTorch worker must return authoritative implementation/version, resolved
   effective load and generation settings, and physical-device facts from its
   actual successful load. Rust currently owns only model path/type/device labels;
   hashing labels or requested defaults would invent authority. Other backend and
   non-text paths need their own owners. No changes to audio/rerank/inspector owners
   belong in this slice.
2. Specify bounded retained observation storage, cohort equivalence and sample
   sufficiency across reload generations. Exact generation equality currently
   prevents pooling a previous reload's samples. Do not weaken that fence simply
   to meet a sample-count threshold.
3. Scheduler preparation also includes host/admission delay and runtime replacement;
   gateway load alone does not. Worker drain is not reservation release/reconcile.
   Transfer, successful owner release, conditional residency/capacity and future
   predecessor-output classes need authoritative observations before constructing
   an `EmbeddedCompletionTimingRecord` or dependency forecast.
4. No production calibration numbers are available. Before enabling a measured
   predictor or changing defaults, qualify the full native dispatch seam and its
   overhead with representative real deployments separately from synthetic policy
   fixtures. The unavailable local C3 archive still prevents simulator differential
   verification; no POC code is copied and no research-completion claim is made.

Stop at these decisions. This checkpoint adds evidence qualification, not the
researched scheduler or a production-calibrated timing source.
