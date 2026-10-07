# Owned dependency lookahead integration

Base is local `0d42c693`; remote main reverified as `a8483e51`. Branch:
`scheduler/owned-dependency-lookahead`. Default scheduling stays unchanged.

The active-run session store owns a bounded advisory pair: a Ready predecessor
and its unique AwaitingInputs runtime successor, exactly two unfinished runtime
tasks, with every other successor dependency completed. At most 128 graph tasks
and 16 bindings per task are inspected before cloning the two snapshots. Preserve
typed intent, descriptor/template, record/version, exact known scalar inputs and
symbolic predecessor source-task/port obligations. No future value or Ready proof
is created. Require the same immutable model, task kind and saved static dependency
requirements/bindings/override/environment class as the current actual Ready proof;
otherwise decline this small subset without speculative dependency admission.

Add a separate scheduler advisory successor snapshot with a validated typed intent
and complete offer universe, not a dispatch request. Share hard runtime/device
eligibility with actual dispatch. The bounded two-completion evaluator still
compares every legal first/successor path under the fixed terminal-prefix objective.
No future reservations, environment installs or speculative admission occur.

Native timing source capability defaults false; conditional forecast defaults
None. A source must echo the exact first observation, successor symbolic workload,
descriptor/configuration and offer, and supply qualified conditional capacity plus
complete load/transfer/execution/cleanup costs. The forecast explicitly covers
Ephemeral release, owner reconciliation and retained-versus-unloaded residency;
equal model identity never implies reuse. Missing, unknown, stale or uncalibrated
observations preserve the existing one-decision result. Authored test estimates
remain explicitly synthetic and require the existing native configured opt-in.

Recheck the complete pair and referenced input snapshot under the first start
lock. Install an attempt-bound successor cleanup gate atomically with first start.
Block successor AwaitingInputs advancement and final start while the gate is
unacknowledged. Completed graph state and assignment state are not cleanup proof.
Clear only after a validated actual lifecycle application matches first task,
attempt and bound lease/event; no-release-intent Ok is insufficient. Old retry
acknowledgements cannot clear a newer gate. Failure/unknown cleanup leaves it
closed; ordinary failure/cancellation input guards remain required after cleanup.
Both single and existing batch-envelope finalization paths use this gate; no new
batch semantics are introduced. In-memory recovery must preserve pending gates.

Independent review confirmed the advisory-versus-Ready distinction and identified
the publication-before-cleanup race, the lease-less Ok pitfall and retry fencing.
Implement the explicit gate and typed acknowledgement before enabling the caller.

Public-path acceptance: supported scalar dependency chain, first-choice reversal,
real first output used as actual successor input, normal successor admission and
replanning, paused/failed cleanup blocks successor, exact matching cleanup resumes,
old attempt acknowledgement is rejected, changed cohort/input rejects first start
with provisional rollback, missing/stale/unknown forecast and default opt-out.
Measure bounded adapter/dispatch cost separately from modeled completion. No model
downloads, public writes, merge into main or unrelated worker-file edits.

Implementation and qualification are recorded in
[owned successor results](scheduler-v2-owned-successor-results.md), including
local current-main preservation and remaining offline/calibration limitations.
