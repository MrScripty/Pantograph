# Scheduler and grouped-workflow development

This development candidate adds opt-in bounded completion policies, local CPU
runtime ownership protocols, executable CPU groups and authoritative backend
group validation. It does not activate a new default scheduler or complete the
researched v2 algorithm.

## Scheduling and evidence

The default remains `PriorityThenFifoSchedulerPolicy`, including starvation
boosts and the existing one-position warm-reuse window. Ordinary admission,
selected reservation commit, cancellation and runtime/session ownership remain
authoritative. A forecast or ranking result does not grant a runtime lease.

`pantograph-scheduler` contains pure completion-cost, two-completion and frozen
cohort evaluators. They require explicit comparable timing, workload, artifact,
owner and resource identities; missing components do not become zero. Synthetic
fixtures are identified separately from measured observations. The evaluators
have finite population and work budgets and return fallback/refusal when evidence
is missing, stale, incomparable, invalid or over budget. The cohort evaluator is
limited to four tasks, two placements per task, 64 branch expansions, 512
completion events and 131,072 work units. These limits describe computation,
not a demonstrated production dispatch-latency guarantee.

Embedded completion timing is a trusted in-process opt-in through
`EmbeddedCompletionTimingOptIn` and hosted startup's `with_completion_timing`.
It does not accept timing authority from workflow/session JSON. The separate
`WorkflowService::new_serial_ready_cpu` constructor owns its serial execution and
cleanup ports. Its supported CPU path uses owner-bound observations and checks
the authoritative Ready population again before starting work. Insufficient
timing retains serial baseline ordering on that port; uncertainty in ownership
or cleanup can refuse further work.

The retained Candle CPU path adds actual owner/worker drain, task release,
retaining-lease custody, declared resource envelopes and acknowledged eviction
protocols. Declared ledger checks and owner-local calibration do not establish
physical setup/peak/growth memory limits, generic CPU-slot capacity or hard
execution/cancellation bounds. There is no new GPU, native batching, distributed
worker or remote-node capability in this candidate.

## Groups and current validation

Executable groups support top-level built-in `json-filter` and `merge` CPU
children. Group authoring, saved/reopened graphs, exposed-port mappings and
duplicate output fanout are retained. Lowering preserves the authored graph;
it does not silently ungroup unsupported content. Native inference, unknown
children and nested groups are refused by this execution projection.

`WorkflowGraphCurrentValidationSummaryResponse` adds optional `group_preflight`
facts. Failures carry explicit diagnostic codes, rejection kind, affected
group/child/field, message, repair hint and blocking status. Facts are bound to
the graph session, full authored semantic revision and validation session ID.
Malformed, invalid and unsupported groups produce blocking diagnostics and a
false submission gate. Valid group facts do not override other inference or
dependency failures.

Starting a new generation synchronously revokes the previous summary. Stale,
canceled, superseded and replayed validation generations cannot authorize
submission or snapshot publication. A live graph session retains at most 16,384
distinct generation IDs; exhaustion refuses a new generation without replacing
current authority. This is a per-session bound, not a global session limit.
Actual authored group facts are rechecked before publication and runtime
acquisition, including after asynchronous capacity work. Resource-free ephemeral
session creation retains its metadata-only contract. These checks do not freeze
saved files across arbitrary host callbacks.

Only the first deterministic group failure is reported. The new failure facts
have bounded text and identity fields; graph parsing and lowering remain
graph-sized. Broad native/group execution and browser/desktop GUI behavior are
not established by the backend tests. A later diagnostics UI is a separate
integration and is not included here.

## Verification and remaining rollout gates

The unchanged production/test sources have qualified native and non-native
Rust suites, feature-light compilation, Clippy and independent executable
review. Directed public API cases cover valid saved fanout, typed refusals,
semantic edits, generation races/replay/exhaustion, forged publication and
acquisition boundaries. CPU native fixtures verify ownership and cleanup
protocols; they are not physical capacity or kernel-performance certification.

Use the repository's normal contributor checks and hooks. Native dependencies
must be available; a missing dependency is a failed gate. Set
`ORT_SKIP_DOWNLOAD=1` for validation without ONNX build downloads. Existing cached
inputs permit locked/offline Cargo checks; no model download is required by the
synthetic fixtures. See [development](development.md) and
[runtime operations](runtime-operations.md) for repository procedures.

Default rollout still requires qualified real timing coverage, resource and
adapter capabilities, measured dispatch overhead and completion quality, and
research-simulator differential acceptance. Native inference inside groups,
physical capacity enforcement, preload/staging planning, GPU/native batching
and remote execution require separate implementation and qualification.
