# Warm-service provenance and grouping observations

Operation `continue` under `docs/plans/cpu-scheduler/plan.md`, from verified main
`4561e838f48ec82464322c05c50e82d79ca3acae`, tree
`0d8d5fd0b09eda03df01a441dc294d07121c93bf`. The user explicitly authorized this
next software milestone after integration. Standards baseline remains `188beda1`;
Implementation, Verification, Contracts, Concurrency, Diagnostics, Performance,
Library and Rust API/async routes apply. All prior branches/worktrees and PR84
remain retained-protected. This branch owns only this diagnostic producer.

## Contract admitted for independent review

Produce historical warm-service forecasts from the existing serial owner's
bounded ring, after the existing successful drain and matched-cleanup recording
gate. A forecast is an observed historical maximum, not a future bound. Include
the exact derived workload key, current owner generation, source/units, eligible
sample count and oldest age. Missing/currentness/clock/freshness/lock failures
return fixed unavailable reasons and no estimate. Preserve existing sample/key
limits, default settings, ordering, resource claims, fair opportunities and Start
validation. Do not install generic completion timing into serial composition.

The production consumer is the existing host diagnostic logger. Emit bounded
forecasts for the retained compatible prefix of the already bounded Ready window
(at most eight members), using keys already calculated by grouping fill. Emission
before cache publication is labeled `captured_compatible_prefix`, not a claim
that a poisoned cache accepted the entries. Read
resident-owner evidence outside history/cache/store locks, check exact owner
identity and freshness, then recheck currentness after releasing history. Retain
no owner capability in the diagnostic. No new forecast key encoding, cache hit,
history sample, execution or scheduling decision is introduced. Logging and owner
queries occur outside guards. Unknown/incompatible members remain ordinary
singleton fallback; no forecast result controls execution.

Measure actual monotonic elapsed spans at the existing grouping owner: refresh
probe (bounds/equality), authoritative window capture (store wait/bounds/cloning),
fill (compatibility and keys), lookup (bounds/equality/fallback), and key
calculation (bounds/encoding/hash). Fixed per-phase service-lifetime aggregates
hold calls and elapsed nanoseconds. Nested spans overlap and must not be summed.
Checked overflow/invalid time makes a phase unavailable, never a saturated valid
duration. Zero elapsed time is valid clock resolution, not a fabricated positive
sample. Snapshots/logging are diagnostic only; no duplicate profile database or
permanent unbounded history is added. Observer/logging overhead is not calibrated;
these spans do not prove total CPU overhead, speedup or end-to-end latency.
Each phase records its own measurement after its timed operation; inclusive
parent spans may include nested recorder cost. There is no guessed subtraction.
One opaque diagnostic UUID identifies the shared mode lifetime across snapshots;
it is not a native owner, reservation or execution capability.
Phase calls include refused/failed planning attempts; an available aggregate
means its arithmetic is valid, not that work was admitted or completed. These
planning spans never enter the successful-service forecast history.

## Memory boundary

Pinned Candle `88ed7911de9e88196b1f55b199145d22647f415e` exposes known live Vec
length/capacity, not complete attempt-scoped allocation peaks. Tensor IDs can
alias storage; internal BERT temporaries, GEMM scratch and Rayon attribution are
not visible at the returned-tensor boundary. Complete allocation/drop attribution
requires a separately reviewed dependency-level mechanism. This slice changes no
native dependency, allocator, telemetry authority or resource accounting and
publishes no fabricated incremental-memory value. Reservation deltas and RSS are
not substitutes. No external model or hardware input blocks this software work.

## Write set and acceptance

Workflow `serial_ready.rs`, new private `serial_ready/observations.rs` and focused
tests, `serial_ready/grouping.rs`, the existing worker capture point; this report,
canonical plan/ledger/issues and directly affected consumer documentation. No
manifest/lockfile, provider/native owner, public/serialized IPC, browser harness,
CI configuration, persisted state, default activation or admission policy change.

Independent pre-code review admitted this contract with the captured-prefix,
opaque-scope, lock and inclusive-observer-cost conditions above. Focused tests use controlled
owners/history and hand-set nanosecond samples: eligible maximum/count/age,
stale/insufficient/owner/clock/lock refusals, ring immutability and cache work-count
preservation; phase arithmetic/overflow and actual capture/fill/lookup production
wiring. Existing failure/cancellation/cleanup regressions retain the recording
oracle. Run affected workflow-service tests, formatting, own-package Clippy,
Rustdoc and traceability, then independent exact-diff review before a normal draft
PR and exact-head CI. No held performance cohort or native/model workload is
introduced. Representative model/workload/backend/device measurements qualify
accuracy and usefulness later; no new hardware is required for these contract tests.

## Local verification and publication checkpoint

Five new ordinary tests cover checked aggregate arithmetic/scope and the two
controlled-worker oracles. Both worker cases passed; the second executes two
ordinary members and requires observed capture/probe/fill/lookup/key-calculation
calls after two requests and matched cleanups. It does not manually record a
capture sample, so deleting the production capture call breaks that regression.
The full workflow-service library passed **1,047 cases, zero failures/ignored**.
Its existing comparison case is an ordinary regression replay here, not a new
held performance qualification; no elapsed result is promoted to a speed claim.

Own-package all-targets warning-deny Clippy, warning-deny Rustdoc, affected Rust
formatting (including the included worker test file), staged traceability and
whitespace passed. Declared pre-push checks passed261 node-engine cases with one
existing ignored case, and168 workflow-nodes cases. The existing optional-consumer
inference dead-code warning is retained without suppression. An initial short
filter plus `--exact` discovered zero tests and was not accepted as verification;
explicit discovery and the corrected execution supplied the named results above.

Independent source review approved the bounded producer for draft publication
after these checks, with no remaining blockers. The exact published head/tree and
normal hosted CI belong to the linked draft PR; local success does not establish
hosted CI, representative forecast accuracy, complete memory coverage or overall
M5/M6 acceptance. No new model/native cohort, hardware qualification, dependency,
default or admission policy is introduced.
