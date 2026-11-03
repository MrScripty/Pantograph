# Exact runtime service timing observations

This independent scheduler successor starts at RAM review repair
`43f0a7776baf879bc800f10a0c18f013f3d80d44`. It adds opt-in service observations
through the actual selected-text gateway and host port. It does not change ranking,
resource accounting, model loading policy, PR54/55 or the Pumas feature pin.

## Identity and evidence

Portable timing contracts distinguish observed nanoseconds, configured estimates,
and unavailable values. Measured zero and configured zero remain distinct from
unknown. Successful observations can be queried only with an identical validated
profile and a unique observed successful phase in a completed attempt. Failed,
abandoned, estimated, duplicate-phase or mismatched records cannot become a
successful service sample.

An exact profile binds the gateway owner epoch and loaded runtime generation to a
BLAKE3 digest of authoritative model content identity, model/artifact/package facts,
selected runtime/device facts, request workload/options, and fresh loaded-owner
implementation, effective configuration and physical-device fingerprints. Resolved
defaults belong in the owner configuration facts. Paths and revision labels add
conservative invalidation but cannot substitute for content identity. The recorder
receives the digest, not prompt text, paths or raw configuration. A new owner epoch
or runtime generation invalidates comparison even when the digest is unchanged;
this slice does not generalize samples across freshly loaded generations.

Missing content identity, runtime generation or verified owner facts produces an
explicit unknown identity. Native backends currently return no timing owner facts
by default; no implementation, physical device or immutable model identity is
invented from scheduler labels. Thus the positive exact-profile tests use controlled
owners, and this checkpoint does not qualify comparable native-model timings.
Unqualified raw phase observations remain visibly unknown in identity.

## Lifecycle and collection

The four phases are gateway custody wait (replacement drain and write-lock wait),
selected-model load (the actual backend load call, potentially including reuse),
text execution (full stream collection), and request-worker cleanup (the backend
producer drain). These are service spans, not pure compute time, guaranteed cold
load, model eviction or acknowledged resident shutdown.

Each opt-in attempt holds a fixed four-phase record. Failed work retains its failed
outcome; dropping the calling future emits an abandoned attempt without inventing
worker cleanup or a confirmed stop. Clock reversal produces unavailable evidence,
not a zero-duration sample. The recorder contract requires bounded, nonblocking,
nonpanicking consumption without gateway reentry. Saturation drops a sample and
cannot change the execution result. No store, background sampler, benchmark,
configuration/UI toggle or ranking consumer is added.

Both gateway constructors disable instrumentation. The disabled path performs no
new phase clock reads, identity hashes, timing UUID creation, owner-fact reads or
recording work; existing host journal behavior remains as before. A deterministic
test verifies zero injected-clock and owner-fact calls. This is not a claim of zero
machine-instruction overhead or a performance benchmark.

Full peak task claims and resident lifecycle protections are unchanged. The native
controlled host regression reaches owner candidate selection, scheduler admission
and the real selected-text host port. It captures all four phases while verifying
that the full peak claim remains held until custody is released. Controlled owner
fingerprints and the test backend do not qualify GPU execution or real model
latency.

## Executed qualification

- Six portable timing-contract tests pass, including estimate/unknown/known-zero
  wire provenance, validated profiles and exclusion of mismatched/partial evidence.
- All 723 inference library tests pass serially. Eight deterministic timing tests
  cover exact phase spans, disabled collection, missing or mismatched owner facts,
  load/cleanup failure, actual future drop, saturated consumption, clock reversal,
  and fresh invalidation after content/revision/artifact/options/implementation/
  configuration/device changes. No sleeps or latency thresholds are used.
- All 66 focused host tests pass, including the actual port timing regression.
- The full embedded suite executes 502 tests: 500 pass; the descriptor-count and
  warmup-timeout assertions remain the same two independently reproduced baseline
  failures. No assertion or timeout is weakened.
- Mixed-backend warning-deny all-target Clippy passes for inference, embedded
  runtime and timing contracts. Formatting, critical/accessibility/traceability
  gates, 28 traceability tests and nine ONNX no-build-download graphs are checked
  with the source checkpoint.

Builds use Rust 1.92, locked/offline official dependencies, native Python 3.12 and
`backend-llamacpp,backend-pytorch` without default ONNX features. BLAKE3 was already
an existing workspace dependency; only the inference dependency edge is added.
Initial wrong-module-path (zero timing tests) and compile-failure logs are retained
and excluded from qualification; the final full library run executes all eight.
Broader inference wire-fixture and parallel Python-fixture failures reported with
the frozen RAM ancestry remain outside this slice. GTK/WebKit desktop execution,
real model quality/timings, physical GPU inventory and cross-platform native builds
remain unqualified. The actual host RAM source still fails closed for this
container's unsupported `/..` cgroup mount layout.

Completion-oriented ranking, calibrated distributions and frozen-cohort evaluation
remain subsequent work requiring comparable owner evidence.
