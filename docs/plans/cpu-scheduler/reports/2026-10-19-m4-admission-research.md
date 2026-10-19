# M4 serial residency admission research

Status: contract/fixture work only; no production implementation or model run.
M3 PR81 head `b63aa3ebe5fab1531aef952a35ce74932bf76b30` remains preserved and its
acceptance awaits parent-monitored CI. The canonical [M4 contract](../plan.md#m4-serial-residency-contract-2026-10-19-independently-admitted-m3-gate-pending)
owns admitted behavior and the exact conditional write set. This report records
source findings, not a second implementation plan.

## Observed source boundaries

- `serial_cpu_port.rs::NativeCleanup::apply` consumes the actual drained receipt
  via `release_retained()`. Afterwards no receipt remains for future eviction.
  Retention requires another live lease. A next-same peek cannot settle future
  service order when a newly protected/high-priority unit arrives.
- `gateway_cpu_warm_attempt.rs` requires actual qualified resident ownership and
  retaining successor. Its successful proof is a genuine warm owner transition;
  a cold result cannot be relabelled as one. `execute_selected_embedding_owned`
  already separates a prepared backend internally, but ordinary selected execution
  includes a forward and generic replacement loads the candidate before retiring
  the old owner. Neither is a load-only, stop-before-load M4 path.
- `gateway_cpu_last_lease_eviction.rs` already supervises physical stop under the
  native writer and acknowledges a new producer sequence. Its registry ticket
  requires the exact sole started Ephemeral lease, no pins and known resident
  charges. A fresh generic incoming task reservation would make this refuse.
- `RuntimeReservationExecutionCustody::begin_execution` marks the boundary before
  actual physical work. Acquiring guard custody before a real supervised cold
  load, starting it immediately before that load, and retaining it idle preserves
  an honest physical ownership obligation. A new sealed native idle capability
  can supply load/drain proof and later genuine warm lineage while reusing the
  existing last-lease ticket internally. No fake drained-task proof or invented
  execution on an already idle lease is needed.
- The exact guard custody also prevents ordinary lease release/replacement from
  erasing the guard charge. All task/guard/resident resource claims remain additive.
  Existing task claim authority is `SchedulableTaskIntent.estimate_hints` consumed
  by `reservation_requirements_from_estimate_hints`; existing model estimates and
  separate cold-load peak configuration must be known. Numeric producer capacity
  remains a prospective freeze gate rather than a post-outcome adjustment.
- `task_execution_worker.rs` already owns a fair scope after actual M3 grant and
  rechecks the original admission at final Start. Preparation belongs there,
  outside store/queue/context locks and before candidate preselection allocates
  the incoming provisional lease. If proof/input changes after a successful load,
  the task refuses Start but the actual loaded owner remains charged and owned.
- Existing M1/M2 service timing starts at native dispatch, so preparation can stay
  separate. Changed loaded instance/profile invalidates warm history and the M2
  predecessor; absent load/stop forecasts remain unavailable.

## Exclusion and lifecycle

A dedicated gateway is required to prevent ordinary same-gateway mutation from
replacing a resident behind idle authority. Public `EmbeddedRuntime::gateway()`
returns a writable Arc; returning the existing runtime or a generic host/service/
extensions/registry alias defeats that boundary. The bounded facade constructs
and privately owns infrastructure, exposes named authorized session operations
and read-only diagnostic values, and has no inner-runtime/Deref escape.

Ordinary session host helpers do not cold-load Candle; they can create retention
leases or refuse a stopped owner. M4 must therefore explicitly reject KeepAlive
bootstrap before side effects and support metadata-only ephemeral sessions. This
also avoids removing an application's retainer merely to make stop eligible.

Shutdown must close facade admission/auto-resume, drain the existing workflow
runtime and task worker, consume actual idle authority through stop+ACK, then
perform remaining embedded teardown. Calling generic embedded shutdown alone does
not establish that sequence. Generic Candle lifecycle reconciliation cannot stand
in for native ACK: `resident_lifecycle_snapshots` currently enumerates PyTorch and
llama_cpp, not Candle (the disclosed CPU-14 boundary).

Pre-Start loading has no Running task attempt. Its cancellation evidence must
use a named qualification-only native signal; public active-task cancellation
cannot cover that phase. Public post-drain cancellation is a separate observation
and may still complete successfully. Caller wait drop never means a worker abort.

## Fixed fixtures and acceptance observations

[Fixture admission](m4-contract-v1/fixture-admission.json) checks all14 existing
reference-manifest files, original manifest SHA256, golden vector widths/finite
normalization and exact input hashes. It performs no native or Transformers
forward, acquisition or regeneration. The existing references use untrained BERT
widths8/12 and qualify numerical execution, not semantic retrieval quality.

The prospective hand oracle is `A,A,B,B,A`: retained mode3 loads,2 same-owner
retains,2 switch stops and1 final idle stop; explicit stop-between-tasks comparison
5 loads and5 stops. Both retain five outputs. Actual protocols, phase durations,
pressure admission and cancellation/ownership traces remain unexecuted and cannot
be inferred from this fixture package. Controlled failure branches are labelled
separately from actual Candle behavior; no performance threshold is admitted.

## Independent review and next gate

Existing independent GPT-6.1 Sol High reviewers `m3_design_review` and
`m3_source_review` validated the ownership gap, rejected stale next-same snapshots
and fake warm receipts, and found the load-started guard custody simplification
sound. Both reviewers independently admitted the final contract after the explicit
shutdown/facade and pre-Start cancellation clarifications. M3 acceptance and actual M4 implementation,
native evidence, focused PR CI and final acceptance remain separate gates.

Independent `m3_cpu_producer` fixture review passed: all14 original hashes and
exported golden/text/tolerance data match; script root resolution is independent
of cwd and has only stdlib data/hash operations. Reviewed script SHA256:
`d35d1ee9a406640cc65186775e17ba098891788d0c8f774589b432fe7e880a97`.
Producer admission will freeze exact phase intervals, full identities, resource
claims/capacity and separate fixed owners/counts for pressure/cancellation/fault
cases. Reload is included within three loads; none of those prospective scenarios
have executed.

The [accounting witness](m4-contract-v1/accounting-witness.json) is a static
feasibility check, not frozen native capacity or measured RAM/VRAM. It preserves
the existing conservative estimator's per-kind task claims, including its VRAM
claim on CPU, rather than silently rewriting that claim to zero. Actual consumed
claims and complete future-publication headroom must be checked before physical
work; missing or larger-than-witness claims refuse. Numeric native protocol
budgets/pressure cases remain a prospective producer admission gate.
