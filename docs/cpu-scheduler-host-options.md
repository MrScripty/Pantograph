# CPU scheduler constructor configuration

M6a adds read-only host observability. It exposes the execution path and serial
policy selected at construction; it enables no mode and changes no default.
M4 residency acceptance, overall M5 execution/performance acceptance and real-model
qualification remain separate gates owned by the [CPU scheduler plan](plans/cpu-scheduler/plan.md).

## Host contract

`WorkflowService::scheduler_configuration()` returns an immutable, nonserialized
`WorkflowSchedulerConfiguration`. The types are available from
`pantograph_workflow_service` and its `workflow` module. Reading the value takes no
store/execution lock and queries no provider, native port or resident owner.

| Getter | Meaning |
| --- | --- |
| `execution_owner()` | `Ordinary` construction or the dedicated `SerialReadyCpu` execution path. |
| `serial_completion_advisory()` | Constructor requested predecessor-conditioned serial completion advice. |
| `serial_fair_opportunities()` | Constructor requested authenticated per-member service opportunities. |
| `serial_residency()` | Constructor requested serial native preparation/residency policy. |
| `serial_key_reuse()` | Constructor requested bounded immutable workload-key reuse. |
| `serial_minimum_samples()` | Configured serial minimum, 3 through 8, or `None` for ordinary construction. |
| `serial_maximum_sample_age()` | Configured serial sample age in `(0, 1 hour]`, with nanosecond precision, or `None` for ordinary construction. |

Every flag reports configuration only. A true flag proves neither active native
support nor a loaded owner, entitlement, readiness, cache hit, resource capacity,
successful execution or qualification. The sample age is not an execution
deadline. Ordinary construction returns false for every serial flag and `None`
for both serial bounds; separately installed generic completion-timing providers
are outside this snapshot's scope.

The hosted composition owner emits a fixed scalar diagnostic at
`workflow::configuration` after loaded-session capacity configuration succeeds
and before sharing the service. It includes `configuration_only=true`, the
execution path and these serial fields. It contains no model, task, account,
input, path, token or owner capability. Existing host logging configuration owns
log delivery and retention; absence of a delivered message proves nothing about
runtime support. If loaded-session capacity validation fails, this diagnostic is not emitted.
The message describes service construction only; later hosted startup, readiness
producer initialization or execution may still fail.

## Existing explicit host choice

Ordinary construction remains `WorkflowService::new()`. A host that already owns
a qualified serial CPU port can use the existing dedicated constructor:

```rust
use std::sync::Arc;
use pantograph_runtime_host_contracts::SerialRuntimeHostBatchExecutionPort;
use pantograph_workflow_service::{WorkflowSchedulerExecutionOwner, WorkflowService};
use pantograph_workflow_service::workflow::WorkflowSerialReadyConfig;

fn serial_service(port: Arc<dyn SerialRuntimeHostBatchExecutionPort>) -> WorkflowService {
    let service = WorkflowService::new_serial_ready_cpu(
        port,
        WorkflowSerialReadyConfig::default().with_fair_service_opportunities(),
    );
    assert_eq!(
        service.scheduler_configuration().execution_owner(),
        WorkflowSchedulerExecutionOwner::SerialReadyCpu,
    );
    service
}
```

The caller supplies the actual execution/drain/cleanup owner; this example creates
no owner, model, reservation or entitlement. Fair service requires existing
backend-validated client/bucket attribution. The concrete embedded retained CPU
port is available under `pantograph-embedded-runtime`'s `backend-candle` feature;
the host must still satisfy its ordinary readiness, resource and native-owner
contracts. The constructor configuration API itself requires no Cargo feature.
Hosted composition preserves constructor-owned serial execution and cleanup
ports. Generic hosted completion timing cannot be combined with serial CPU mode.

The existing optional serial completion-advisory builder changes advisory output,
while immediate serial policy retains dispatch authority. Residency and key-reuse
builders remain explicit and require fair service opportunities; observing their
flags does not promote the unresolved M4/C2 acceptance gates. M6a adds no
application selector, UI, IPC field, persisted preference or provider capability.

## Lifecycle and rollback

The snapshot is stable across service clones and loaded-session capacity changes.
It has no setter or live-mode toggle. With fair service opportunities enabled, each member retains its own M3
opportunity. Existing per-member preparation/resource/owner checks, final Start
validation, cancellation and matched cleanup remain in force. Serial workflow deadlines remain unsupported: run
and resume refuse a supplied `timeout_ms`.

Choose ordinary construction at the next clean startup to stop opting in. To
replace an existing service, first use its owning host's existing shutdown/drain
protocol and establish that execution has actually drained and cleanup completed.
Dropping a service or caller wait does not stop native execution or release
custody. M6a adds no automatic rollback or recovery from uncertain cleanup.

Focused tests cover every admitted serial option combination, exact sample-bound
precision, ordinary construction, clone/capacity stability, and preservation through
hosted composition. Rejecting ports/providers ensure these reads perform no
native execution or runtime queries. These are constructor/consumer contract
tests, not model or performance qualification.

## Bounded runtime observations

With the existing explicit bounded-grouping option, host logs can report
`workflow::serial_observations` warm-service forecasts for the captured compatible
prefix (at most eight Ready members). Exact precomputed workload digests and the
current private owner select the existing cleanup-qualified history; output
contains only a diagnostic scope, digest, scalar generation, fixed reason,
eligible count, oldest age and historical maximum in nanoseconds. No input text,
model/task identifier or owner capability is disclosed. A historical maximum is
not a future bound, queue delay, cold preparation estimate or group completion
forecast. Insufficient, stale, changed-owner, invalid-clock and unavailable
history states carry no estimate. Logging before cache publication does not prove
that a poisoned cache retained the captured prefix.

`workflow::serial_ready` grouping-observation records use the same opaque
mode-lifetime scope for five fixed service-lifetime phase totals: probe, capture,
fill, lookup and key calculation. Each reports checked call counts and inclusive
monotonic elapsed nanoseconds, or a fixed unavailable state after overflow/invalid
time. Nested spans overlap and cannot be summed. Each phase excludes its own
recorder, while parent spans can include nested instrumentation; overhead is
uncalibrated. These records do not measure total CPU work or establish speedup.
Disabled grouping collects no such clocks, scopes or records. Existing host log
configuration owns delivery/retention; no new history database is created.

All observations are read-only diagnostics. They change no admission, fair turn,
resource claim, Start check, cleanup requirement, policy default or execution
mode. No complete incremental allocation measurement is supplied: known live
buffer capacities, reservation estimates and sampled RSS cannot substitute for
allocation/drop, alias, scratch and worker attribution. That backend foundation
requires a separate reviewed design. See the [producer contract and evidence](plans/cpu-scheduler/reports/2026-10-21-provenance-producer.md).
