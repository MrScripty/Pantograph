# Runtime Operations

Pantograph keeps workflow policy in backend services, concrete runtime
execution in the runtime host, and desktop commands as transport/composition
adapters. See [ARCHITECTURE.md](../ARCHITECTURE.md) for ownership boundaries.

## Python Workers

Python-backed nodes execute in child processes; Pantograph does not embed one
global Python interpreter. The worker entry points are:

- `crates/inference/torch/worker.py`
- `crates/inference/audio/worker.py`

Interpreter selection uses, in order:

1. `PANTOGRAPH_PYTHON_ENV_MAP_JSON`, an `env_id` to executable JSON object;
2. `PANTOGRAPH_PYTHON_ENV_MAP_FILE`, containing the same mapping;
3. `PANTOGRAPH_PYTHON_EXECUTABLE`;
4. `PYO3_PYTHON`;
5. `python3` or `python` on `PATH`; and
6. the project `.venv` executable when available.

Example:

```bash
export PANTOGRAPH_PYTHON_ENV_MAP_JSON='{"venv:pytorch":"/opt/pantograph/pytorch/bin/python"}'
```

The selected interpreter and dependency environment are runtime inputs. A
missing mapping, dependency, model fact, or worker capability must remain an
explicit unavailable/unsupported outcome; do not fall back to a different
model or execution path.

The current process adapter does not yet provide complete task/reader/monitor
ownership and typed shutdown evidence. Operational automation must account for
that known limitation until the
[architecture remediation](plans/current-standards-remediation/architecture-lifecycle-and-bindings/plan.md)
is accepted.

## Runtime Registry Inspection

The desktop host exposes three diagnostic commands:

- `get_runtime_registry_snapshot` returns backend-owned runtime-registry state;
- `get_runtime_debug_snapshot` aggregates registry, health, recovery,
  scheduler, and workflow diagnostics; and
- `reclaim_runtime_registry_runtime` requests targeted backend-owned reclaim
  and returns the resulting state.

These commands are projections. Runtime identity, state transitions, reclaim
eligibility, and reconciliation remain owned by the runtime registry and
embedded runtime, not by Tauri or the frontend.

## Runtime-Host Observations

Host-request observations are diagnostic samples, not execution or cancellation
authority. Their SQLite writes run on Tokio's blocking pool, with a shared limit
of 64 queued or running writes. Saturation, an unavailable Tokio runtime, and
recording failures are logged; they do not replace the host's execution result.
Normal completion awaits an admitted write, so ledger contention can still delay
response delivery without blocking a Tokio worker. The measured host elapsed time
excludes that recording delay.

Dropping an execution future submits an abandonment sample without waiting.
These samples are best-effort and may not be visible immediately or survive
runtime shutdown. There is no explicit observation flush guarantee; existing
backend cancellation and reservation cleanup owners remain unchanged. Successful
queries continue to require the exact request fingerprint, host epoch and
freshness bounds. Missing samples do not authorize a timing prediction.

## Prepared Reservation Ownership

Resource-backed dispatch preparation carries registry rollback custody until the
workflow task binds its in-memory cleanup intent. A failed start, rejected
selection, cancelled preparation or failed bind drops that custody. Fresh claims
are removed; a replacement restores the entire previous lease. Once binding and
custody transfer succeed, normal task completion/cancellation/failure cleanup
releases the selected lease. See [ADR-002](adr/ADR-002-runtime-registry-ownership-and-lifecycle.md).

During a pending replacement, snapshots account for the componentwise maximum
old/new claim and retain the predecessor's pin/keep-alive protection. This held
capacity protects rollback; it is not evidence that both requests are executing.
Same-owner updates and retention mutation remain unavailable until transfer or
rollback. An explicit registry release ends the lease, so a later custody drop
cannot revive it. Do not manually release a prepared replacement to imitate
cancellation: cancellation belongs to its custody owner.

Publication rejects changed observed runtime status/instance and rechecks current
configured capacity and owner conflicts. This does not prove physical host-wide
capacity, unchanged model/catalog epochs, or recovery after process abort. Missing
capacity remains unknown. Verification evidence and native execution limits are
recorded in the [reservation custody report](plans/domain-architecture-and-multimodal/reports/2026-10-05-reservation-custody.md).

## Shared Resource Admission

The registry can explicitly bind logical RAM/VRAM claims from registered runtimes
to a shared backing pool with `configure_resource_domain`. For example, bind
PyTorch and Candle RAM to one host domain; bind both RAM and VRAM to one domain
when those allocations use a unified-memory pool. Capacity and membership come
from the composition owner; the registry does not discover hardware or infer
shared allocation aliases. Equal-content copies are charged separately.

Shared admission applies in addition to runtime-local budgets. Evaluations expose
domain requested/reserved/available bytes without allocating leases. Selected
commits recheck all domains under the registry lock. Provisional replacements
protect the componentwise maximum old/new claims until transfer or rollback,
including when RAM and VRAM share one backing pool. Releases restore capacity.

Every runtime/resource-kind pair can belong to at most one domain. Membership is
fixed for the registry lifetime; capacity and margin may change only if they
still cover all live claims. The desktop restores explicit declarations from
`runtime_resource_domains` in its app-data `config.json` before gateway and
workflow startup. Omission or an empty list composes the existing empty registry
with no shared bindings. Unconfigured resources retain runtime-local
accounting. Claims remain declared envelopes: missing claims, allocations outside
this registry, undeclared allocations, per-device
subdivision within one runtime, automatic host binding, and measured allocator
safety are not established by this API.

To activate a pool, close Pantograph, add the section below to the existing app
configuration, and restart. The byte values are an illustrative declaration;
replace them with the capacity and margin you intend to admit. Device selection
and offload settings do not imply any backing-pool membership.

```json
{
  "runtime_resource_domains": [
    {
      "domain_id": "host.ram",
      "total_bytes": 1073741824,
      "safety_margin_bytes": 134217728,
      "bindings": [
        { "runtime_id": "pytorch", "resource_kind": "ram_bytes" },
        { "runtime_id": "candle", "resource_kind": "ram_bytes" }
      ]
    }
  ]
}
```

For unified memory, use **one domain with one capacity**, listing both RAM and
VRAM bindings against it. Each logical claim is charged once; duplicate bindings
and overlapping domain membership are rejected. Do not describe one backing pool
as two independent capacities. Distinct allocations remain fully charged;
verified allocation aliases are not inferred from hardware names.

Startup canonicalizes known runtime aliases and seeds declared identities without
capabilities or readiness. Producer reconciliation still supplies actual runtime
facts. Unknown runtimes, malformed declarations, duplicate domain IDs and unsafe
budgets fail startup. A present unreadable or invalid app configuration also fails
startup, rather than falling back to unconstrained defaults. Missing config files
still use defaults when absence is confirmed. Broken configuration or app-data
symlinks and filesystem permission/metadata failures are errors. The production
`pantograph-app-config` loader composes the startup registry; Tauri then manages
and passes that same registry to its existing gateway/workflow consumers.
Existing config commands preserve the section but reject live
changes to it: shared backing membership requires editing the file and restarting.

### Resident Model Envelopes

The active PyTorch producer in `InferenceGateway` publishes ordered allocation
observations during host sync, warmup, restore, stop and reclaim reconciliation.
The registry applies its explicitly configured estimates under the existing
admission lock. Configure exact observed model targets in `config.json`:

```json
"runtime_model_resident_estimates": [
  {
    "runtime_id": "pytorch",
    "model_id": "/models/exact-target",
    "requirements": {
      "claims": [
        { "kind": "ram_bytes", "bytes": 4294967296 },
        { "kind": "vram_bytes", "bytes": 0 }
      ]
    }
  }
]
```

The numbers are operator estimates, never measurements or a guessed device split.
`model_id` must equal the gateway's observed model target, usually its load path;
there is no path normalization or substitution with a catalogue identifier.
These settings require restart, like resource domains. Missing, legacy and empty
configuration preserves unconfigured lifecycle/admission behavior. Configuration
alone creates no loaded model, readiness or resident allocation. A declared zero
for a kind is known zero; an omitted kind is unknown and blocks a bound shared pool.
Estimates for other producers can be persisted, but automatic publication in this
successor is limited to the active PyTorch owner.

`model_resource_residency` in runtime snapshots holds the exact model/instance
and its optional envelope. Resident bytes and complete task peak envelopes count
against local/shared budgets; unified pools sum bound kinds. Equal content in
different producers remains separate allocations. Task cleanup retains weights;
acknowledged gateway stop releases resident bytes while live task custody stays
charged. Confirmed replacement requires fresh estimates for the new identity.
Rejected publication keeps the actual new identity unknown and blocks shared
admission; the next owner snapshot retries after task capacity becomes available.
Smaller/partial declarations for the same identity retain the componentwise maximum.

A gateway source token and monotonically sequenced snapshots reject old loads,
stops and reload reports even when model/instance labels are reused. Once bound,
unsequenced observations, stopped transitions, manual declarations and negative
inactivity projections cannot release this producer's envelope. Matching negative
health assessments can still make dispatch less permissive. A different gateway
source requires fresh startup composition rather than an implicit owner handoff.

Readiness loss, failed effectful load and unacknowledged stop do not prove release.
They retain the previous envelope and expose `resident_resources_uncertain`;
shared admission returns `ModelResidencyResourcesUnavailable`, including for a
request from another runtime. PyTorch retains uncertainty after losing load
metadata, so a subsequent stop must obtain worker shutdown acknowledgement.
Successful gateway stop/switch supplies logical absence evidence. There is no new
process watcher or controller, and an unseen process loss requires the existing
owner to reconcile or stop before capacity becomes available.

Terminal host cleanup publishes current allocation evidence before releasing its
task lease. If full peak claims temporarily prevent a resident estimate from
fitting, unknown accounting blocks competing admission until publication retries
after release. This retry also runs when other leases retain the producer. A
failed uncertain allocation is eligible for normal reclaim once reservations and
pins permit it; reclaim calls the matching lifecycle owner despite lost readiness
and keeps shared admission blocked until ordered release evidence arrives.

All task peak claims remain fully charged, including any weights already inside
them. This deliberately conservative accounting does not establish a measured
resident/transient split or physical GPU allocator safety. The bridge covers the
gateway's current text-model lifecycle; independent diffusion caches, standalone
backend unloads, and allocations outside that owner are not measured or projected.
Native GUI/runtime qualification remains separate.

## Recovery And Reclaim

Recovery follows this ownership flow:

```text
desktop health/manual trigger
  -> recovery coordinator
  -> embedded-runtime restart plan
  -> producer stop/restore
  -> runtime-registry reconciliation
  -> projected diagnostics
```

Before forcing recovery:

1. capture the runtime and debug snapshots;
2. record the stable runtime identity and reported lifecycle state;
3. prefer targeted reclaim over broad restart;
4. verify the post-operation registry snapshot; and
5. preserve incomplete or failed shutdown/reclaim outcomes in the incident
   record.

Do not edit registry state directly, infer ownership from a process ID, or
treat log output as authoritative runtime state.

## Trust Warning

The current Diffusers worker has a critical remote-code trust bypass, and the
generated-component path can fail open. Do not use untrusted model packages or
generated UI source until the
[security remediation](plans/current-standards-remediation/security-and-dynamic-code/plan.md)
closes those paths.
