# Opt-in serial Ready selection wiring

This slice connects a bounded first-two Ready comparison to the existing WorkflowService TaskWorker start/dispatch/cleanup path. `WorkflowService::new_serial_ready_cpu` requires an explicit `SerialRuntimeHostBatchExecutionPort` that owns execution, actual worker drain and reservation cleanup. An ordinary batch port cannot enable this mode. Default service construction and PriorityThenFifoSchedulerPolicy remain unchanged.

The constructor installs shared serial admission before any repository escapes, uses a fresh singleton assignment repository and prevents execution/cleanup port replacement. The worker acquires one shared permit before preparation or claims, promotes ownership before the first event mutation, and reacquires for each continuation. A competing command waits without a claim. Abandonment or missing drain/cleanup proof poisons admission and wakes waiters with refusal. The legacy all-Ready runner refuses the opted-in service.

Only the authoritative first two Ready runtime tasks are compared; an unsupported first task is never skipped. The graph and record populations are bounded to 128 before cloning. Both admitted intent/record/proof/input snapshots, environment identities and the complete population order/state/version are revalidated under the existing store lock immediately before Start. Provider callbacks remain outside that lock; the final loaded-owner generation check is atomic. Actual host handoff IDs, intent, proof and exact inputs must still match before forwarding.

Selection requires the same actual CPU owner, profile, effective settings, generation authority and thread budget (one to four), admitted Candle/CPU embedding constraints, and at least three fresh successful samples per exact text. Observations retain the actual owner's generation Arc as well as its snapshot, preventing reuse under a different authority with copied snapshot bytes. Each sample covers measured host dispatch through actual drain and matched cleanup; it does not measure preparation, cold load or queue wait. The maximum observed cost determines the lower-cost position; ties preserve the first position.

History is private, with at most 64 exact keys and eight samples per key. Selection scans at most 512 entries twice and hashes at most two bounded inputs. Work has explicit finite bounds; these empirical costs are not worst-case guarantees. Missing, stale or incomparable timing retains FIFO on the same constructor-owned serial port. Baseline execution obtains optional collection evidence from the actually held/drained owner; it does not bind an advisory pre-query as a ranked decision.

After a qualified comparison, owner mismatch refuses before any backend forward. No ranked-to-unranked downgrade is allowed. Cleanup retains the original actually drained owner stamp even when it becomes stale, so a replacement race must refuse rather than weaken the fence. Exact lifecycle event/lease acknowledgement releases admission. Missing drain proof cannot use an ordinary terminal cleanup callback. Only successful Completed release contributes history; cancellation is checked at completion and again after the final owner callback before recording.

## Validation scope

Controlled fixtures use two admitted Ready CPU-labelled tasks with actual persisted completed text sources, the normal candidate/selected-reservation/assignment pipeline, an actual held Tokio owner guard, actual async execution/drain, and matched lifecycle applications. They run synthetic work, including a 25 ms delay; they execute no native embedding model and qualify no authoritative CPU/GPU capacity.

Worker-boundary cases cover FIFO bootstrap, learned ordering, stale/missing evidence, stable ties, changed generation authority, owner change before forward and after drain, cleanup refusal, abort poisoning, competing commands without claims, exact population/order/input fences and cancellation exclusion. A configured Worker handle/command case also traverses its shared responder registry and JoinSet and proves dropping a terminal receiver does not abandon owned work. Existing default paths remain in the full-suite regression run.

The bounded selector's observed debug-build CPU is reported separately from simulated execution. An initial populated six-sample probe measured 1,000 selections in 39.288 ms (~39.3 microseconds each) in this executor. This is a small synthetic caller-seam measurement, not release p95/max, maximum-history qualification, or realistic Candle dispatch cost. The rollout gates in `scheduler-v2-production-migration.md` remain mandatory before changing default scheduling.

## Remaining production integration

The native gateway does not yet implement the new serial owner capability. Its selected warm load can advance the calibration generation while retaining the same loaded instance. Native activation needs an owner-held verified reuse transition and real generation-fenced release/reconciliation across direct gateway entry paths. The generic service constructor cannot be used with an ordinary native port to bypass those obligations. Ephemeral cleanup cannot invent retained residency or bootstrap warm timing.

The exact completed CPU dependency-planning candidate is integrated separately after public-source verification. Remaining native deferred-readiness diagnosis and full-suite fixture alignment are reported with that follow-up. No full research two-event scheduler, simulator differential acceptance, GPU/native batching, remote execution or complete phase timing is claimed.

No public write, main merge, runtime/model download, credentials or network-setting change is part of this slice. Tests use cached dependencies with `ORT_SKIP_DOWNLOAD=1` and locked/offline Cargo.
