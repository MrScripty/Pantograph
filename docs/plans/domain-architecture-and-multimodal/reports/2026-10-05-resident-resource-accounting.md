# Resident model envelopes in shared resource admission

Separate branch `feat/scheduler-resident-resource-accounting` starts from frozen
`eabbcc83e6dce54c852eb3806b5ec0a2907ccd6d`, tree
`494546ce5fcb65a96fb664c7fd8b30eaa7326b3e`. Parent owns review and publication.
PR54 and all qualification branches remain unchanged. No native dependency
install/retry, dependency pin change, model load or hardware measurement.

## Finding and bounded acceptance

The research-plan follow-up asks whether retained weights and transient inference
claims occupy the new shared domains. Source shows a concrete missing link:

- Registry model residency records hold model identity, usage, pinning and load
  time, without byte claims (`state.rs`). Shared domains sum task reservations
  (`resource_domain.rs`), and task release removes those entries (`lib.rs`).
- The embedded producer's static package-size estimator distinguishes loaded
  memory from runtime overhead, but emits full peak RAM/VRAM task hints
  (`inference_resource_estimator.rs`). Dispatch projects these full hints into
  reservation requirements (`runtime_dispatch_candidate_provider.rs`).
- Candidate loaded-memory estimates also enter dispatch/batch facts. They are
  not retained resident allocation claims. Producer mode observations carry
  model/runtime identity and lifecycle, without per-kind memory declarations
  (`runtime_registry_observations.rs`). A package size or candidate identity
  alone cannot establish actual device split or an allocation alias.

After the last task lease is released, a still-loaded producer therefore stops
consuming declared capacity. The smallest owner capability is a separately
declared resident envelope, integrated into the same registry and admission lock.
Acceptance was reported before implementation: charges survive task cleanup,
idle/failed/stopping states; confirmed stop releases them; identities and
declarations remain fresh; local/shared/unified accounting and provisional
rollback include them; unknown shared-pool residency is unavailable rather than
free; real portable tests verify contention, lifecycle and arithmetic.

## Implemented owner capability

`RuntimeModelResourceResidency` exposes the observed model/instance plus optional
resident requirements in runtime snapshots. Existing observation and explicit
transition paths initialize or invalidate this entry. A producer calls
`declare_model_residency_resources` with the exact current observed model and
nonblank runtime instance, supplying explicit positive RAM/VRAM estimates.
Identity validation precedes publication. No model metadata is inferred from
paths, device names, package content or a previous producer instance.

The registry aggregates and validates growth under its existing lock, including
live leases and provisional rollback custody. Rejection restores the previous
declaration. For the same resident identity, a smaller or partial update holds
the componentwise maximum previous/new envelope; it cannot free a previously
declared allocation merely by lowering an estimate. Runtime-local and shared
evaluations expose resident bytes separately within total reserved bytes, and
selected commits recheck current totals. Unified pools sum each bound kind.

Task release never clears resident accounting. Ordinary same-identity observations
preserve it; missing model/instance metadata, health failures, omitted shared-pool
members and stop requests do not prove deallocation. Authoritative observed stop,
explicit stopped transition, or reclaim after producer inactivity clears it.
Changed model or instance invalidates the estimate, including instance changes
through explicit transitions or partial observations. Stale producer publication
is rejected; new-instance declarations require fresh model observation.

A loaded shared-pool member without a declaration for a bound kind returns
`ModelResidencyResourcesUnavailable`, including when another runtime requests
that pool. Progressive publication of known per-kind declarations remains
possible while other members are unknown; admission stays blocked until all
relevant declarations exist. Declared estimates cannot overrun known capacity.
The embedded host maps unavailable/stale residency to `RuntimeNotReady`, invalid
declarations to `InvalidRequest`, and treats missing estimates as an unavailable
capacity probe. Existing shared-domain and custody contracts remain in place.

## Executed evidence and limits

```bash
source /workspace/pantograph-tools/activate.sh
cargo test --locked --offline -p pantograph-runtime-registry -p pantograph-scheduler -p pantograph-app-config
cargo clippy --locked --offline -p pantograph-runtime-registry -p pantograph-scheduler -p pantograph-app-config --all-targets -- -D warnings
ORT_SKIP_DOWNLOAD=1 cargo check --locked --offline -p pantograph-embedded-runtime --lib --tests --features backend-pytorch
cargo fmt --all -- --check
node scripts/check-critical-antipatterns.mjs
bash scripts/check-scheduler-only-workflow-execution.sh
git diff --check
```

The full portable suites pass 129 registry, 133 scheduler and 10 AppConfig tests:
272 total, zero failed or ignored. Fifteen new public tests use actual registry
state, leases, producer observations, transitions, reclaim, custody and contending
threads. They cover retained charges after task release, unknown-member blocking,
health/omission safety, model/instance invalidation and stale publication, growth
and budget-shrink rejection, monotonic partial declarations, unified RAM/VRAM,
runtime-local accounting, distinct equal-content allocations, selected-commit
rechecks, rollback/transfer, overflow, and legacy snapshot decoding.

All-target portable Clippy passes with `-D warnings`. The normal default-backend
embedded library/test compiler check with PyTorch also passes. An earlier
`--no-default-features --features backend-pytorch` compiler attempt failed with
50 pre-existing test call sites referring to default-backend
`InferenceGateway::new`; the normal supported feature configuration is the
recorded successful check. Neither check executes native tests or proves native
link/runtime availability. Final traceability/source gates run before handoff.
Logs: `/tmp/resident-resources-final-tests.log`,
`/tmp/resident-resources-clippy.log`,
`/tmp/resident-resources-embedded-defaults-check.log`; the earlier feature-selection
failure is `/tmp/resident-resources-embedded-check.log`.

These are explicit envelopes on controlled toy capacities, not measurements.
The existing static estimator and all dependency pins remain unchanged. Current
mode snapshots do not automatically publish per-kind resident envelopes; that
producer bridge is the next integration gap, and configured shared domains now
fail closed for those missing declarations. Legacy local-only admission retains
its prior missing-estimate behavior; declared resident envelopes count there.

Existing task claims are full peak envelopes that may include weights. This
slice keeps them fully charged alongside residency rather than subtracting an
unproven resident component. A proven per-device resident/transient split,
automatic physical capacities/bindings, allocation alias sharing, individual
model unload without stopping its producer, and allocator-level safety remain
open. No native ONNX/GTK or real-model success is claimed; their frozen blockers
remain with the parent.
