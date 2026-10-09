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

`estimate_scheduler_empirical_service_duration` adds a pure, opt-in timing
primitive over at most 128 distinct individual successful attempts. It sums
paired setup, transfer, execution, cleanup, retention and reload durations
within each attempt before taking an exact nearest-rank empirical quantile.
Running elapsed time conditions on strictly longer total service observations;
the quantile is taken over their remaining durations. Both original and surviving
populations require the declared minimum sample support. Missing stages, failed
or censored attempts, duplicate IDs, stale/future records and changed context or
source refuse the entire estimate. The convention identifies disjoint serialized
service through acknowledged drain, with elapsed from that same origin. It
does not describe overlapping phases or resident-cache lifetime.

The helper requires exact host, runtime-instance, artifact, workload, resource,
residency and convention identities from a trusted producer. Owner generation
and clock-domain qualification remain the producer's responsibility; matching
labels do not authenticate measurements. Synthetic observations require explicit opt-in
and remain labelled. This is a success-conditioned empirical description, not
an unconditional survival predictor, calibrated confidence bound or failure-risk
model. Producers must retain failures/censoring separately. Original and tail
support, quantile rank and deterministic work counters are returned. All identity
comparisons, order comparisons and adjacent swaps are metered within 32,768 work
units; the hard population guard precedes row inspection. An incomplete estimate
requires existing safe fallback. No default selector, timing source, store,
native phase observer, lease or runtime/session behavior uses this helper yet.

The existing opt-in selected-text recorder now retains additive lifecycle
metadata: completed/failed, cancellation/shutdown requested, or abandoned. A
monotonic interval starts at gateway custody entry after validation and ends at
the successful worker-cleanup acknowledgement while the backend is owned. It
includes inter-phase gaps; classification/publication/Drop do not move its drain
endpoint. Failed or dropped attempts retain partial intervals and phase evidence
without asserting physical release or successful service. Legacy phase records
remain readable but cannot supply a whole interval.

`estimate_selected_text_empirical_service_duration` is a pure opt-in bridge to
`estimate_scheduler_empirical_service_total_duration`, with a distinct whole
custody-through-worker-drain convention. It rounds each measured total to us once
and validates both capture and actual drain freshness in the original clock
domain. Its core sample timestamp is the actual drain event, so later publication
cannot refresh old service evidence. The raw interval getter retains capture-age
semantics for diagnostics. All supplied attempts must
qualify; failed/canceled/abandoned rows refuse the estimate rather than being
filtered out. Returned termination counts describe only the supplied window.
The existing best-effort recorder can drop any outcome under saturation; neither
these counts nor success quantiles establish unbiased coverage or failure rates.
Its bounded prequalification is separate from the quantile kernel work counters.

No selector or new telemetry store consumes this bridge. Its legacy exact-instance
key still changes after each selected load; this bridge remains unqualified for
cross-reload history. A separate opt-in native history identity is now available
for a deliberately narrow Linux CPU profile: standard GPT2LMHeadModel with at
most 8 MiB of installed contiguous tensor data and a simple WordLevel tokenizer.
It requires an explicitly serial tokenizer environment (`TOKENIZERS_PARALLELISM=false`);
other or absent settings refuse because effective native Rayon configuration is
not inspectable. Collection does not change environment settings.
It hashes actual weights/buffers, effective model/tokenizer/generation settings,
embedded worker/build versions, thread settings and the process CPU domain.
Paths, optional package labels, generated load IDs and correlation IDs cannot
substitute for that evidence. Unsupported/custom/quantized/GPU/oversized owners
refuse timing identity while preserving ordinary inference behavior.

History keys are separate from raw runtime-instance IDs and native load fences.
The actual caller's load ACK is paired with weak model/tokenizer objects and a
monotonic generation; ordinary shared-worker load/unload/shutdown, failed loads,
foreign replacements and A-to-B-to-A transitions invalidate it. No new reuse path
is added: selected text still reloads. Active native collectors hold bounded shared-worker custody; every ordinary
load/unload/shutdown drains their actual native borrows before physical effects,
including a blocking collector whose Rust caller was aborted. Weak saved stamps
do not retain old models. Fresh inspection after acknowledged worker
drain must reproduce every native fact and the same live fence. A new strict
history interval getter also requires built-in provenance, known load disposition,
fresh capture and actual drain, completed lifecycle and all four complete phases.
Old profile equality/getters and the empirical bridge are unchanged. This slice
adds no automatic collection/pooling policy, live prediction binding, calibration,
telemetry store or selector.

`estimate_selected_text_empirical_history_duration` is a separate pure consumer
of individually qualified history rows. It has a comparable-history context
(owner, exact clock, joint identity and reload/reuse stratum) with no live runtime
instance or running-elapsed field. Genuine reloads can contribute to one window;
every raw row retains and must validate its own actual load/drain fence. Mixed
reload/reuse conditions, foreign identities/clocks, stale capture or drain,
failed/canceled/abandoned/unknown/injected attempts and duplicate IDs refuse the
whole estimate. No row is filtered to manufacture a successful population.

The history API has its own whole-interval convention and shares the existing
metered nearest-rank kernel, retaining the 128-row/32,768-work-unit caps. Bridge
prequalification is separately bounded, as in the old exact-instance bridge;
returned counters cover the kernel. Scheduler context text retains its 128-byte
cap; longer observer IDs remain raw evidence and refuse estimation. Whole ns
intervals round upward once to us.
The result is an opt-in, not-started, success-conditioned empirical description;
recorder loss prevents claims of complete coverage or failure rates. This adds no
store, collection campaign, calibrated prediction, live elapsed origin,
reservation/admission authority, default selection or performance claim. The
legacy exact-instance API and its running-residual semantics are preserved.

Opt-in load timing includes the initial native hash, and the whole interval also
includes pre-execution revalidation. The post-drain hash runs after the intrinsic
drain endpoint. These inspection costs are not subtracted from observations;
comparing such observations with uninstrumented dispatch requires measurement.

Inspection caps accepted tensor bytes, traversal, vocabulary, metadata and active
collectors/transitions; overflow refuses evidence. Opaque native tokenizer getters
can copy strings before Python checks their size, so these limits do not prove a
universal refusal-work or allocation bound. A bounded native export API or proof
of bounded loader-consumed input would be required for that stronger contract.
This opt-in observer is advisory and supplies no deterministic dispatch guarantee.

`RuntimeServiceTimingInspectionLedger` adds a separate pure pre-call contract in
`pantograph-timing-contracts`. It reserves caller-selected aggregate call, declared
work and temporary-copy budgets atomically before invoking an adapter. Unknown
bounds or overflow/exceeded ceilings refuse without invoking the callback body or
changing accounting. A call ceiling also bounds zero-cost declarations. Failed and unwound
callbacks keep their full reservations; reserved versus completed calls exposes
incomplete observations. Fixed operation enums and constant-size accounting avoid
unbounded diagnostic metadata or a new telemetry store.
Reservations cover adapter calls; the constant-work gate and caller retry loops
are not represented as measured native cost in those counters.
The constant-work claim covers the ledger-owned decision/accounting only. Caller
closure construction and destruction are outside it: a refused consumed closure
is still dropped, and arbitrary captured Drop code can run or panic. Callback
noninvocation does not establish a universal bound on that caller-owned work.

Declared maxima are an adapter contract, not proof about a native implementation
or resource capacity. The current `NativeOwnerSnapshot` profile is explicitly
refused even if a caller supplies declared maxima: tokenizer strings/export,
AddedToken/configuration and build getters lack a proven pre-call copy/work bound.
`inspect_native_owner` consequently never invokes the snapshot callback body.
The existing advisory
observer is unchanged; this new strict contract is not wired into collection,
history estimation or selection and does not retroactively qualify their costs.

Elapsed ns are separately supplied observations for completed calls, including
failures. Missing values or cumulative overflow stay unknown, while an unwound
call remains incomplete; neither elapsed nor returned byte counts refund a
reservation or establish a wall-time deadline. The source-only synthetic tests
exercise accounting/refusal, not native dispatch performance. A genuinely bounded
native exporter or loader-consumed input proof and a decision on strict collection
integration remain required before a native profile can pass this contract. All
existing inspection/estimation caps and default scheduling remain unchanged.

The CPU domain is process-local, not portable hardware equivalence or capacity.
The native fixtures use small locally generated untrained models; they establish
identity/ACK semantics, not trained-model quality or performance. Retained aliases
to pre-install functions, arbitrary Python/native memory mutation and other
unsupported execution profiles are not universally certified. The bridge still
refuses unknown/injected evidence and cannot substitute labels for owner facts, merge runtime generations, qualify running elapsed time,
or reinterpret these observations as six separate native service phases.

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
not established by the backend tests. The composed diagnostics UI renders typed
group/child/field identities and the backend's repair hints. Refresh and
lifecycle reads share one invalidation epoch; Submit requires the same accepted
response identity before publication and Run, including after asynchronous
session creation. Lifecycle events invalidate authority rather than granting it.

## Verification and remaining rollout gates

The opt-in text owner now consumes a source-qualified bounded WordLevel
component when the pinned Tokenizers provider capability is installed. See the
[provider patch and build procedure](../crates/inference/torch/provider/tokenizers-0.21.4/README.md).
It inspects borrowed native state under nonblocking guards, meters retained
table storage and cumulative buffer copies, and returns an explicitly versioned
binary identity. Available-capability refusal never calls legacy exporters.
The ordinary wheel keeps its existing advisory observation path. The patch is
qualified locally through synthetic native units and the actual tokenizer
wrapper; it is not automatically installed as a runtime dependency. Aggregate
`NativeOwnerSnapshot` remains Unknown/refused until all other collection
components have qualified cost models. The existing selection default is
unchanged.

The bounded-provider consumer now also uses a CPython3.12.3 native primitive
settings component. Original settings enter under continuous GIL custody before
key filtering, UTF8 conversion or sorting, with conservative storage, work and
cumulative copy bounds. Opaque/native settings refuse; accepted native and Python
settings mutations change the versioned identity. This is component accounting:
pre-component wrapper access, configurations, tensors, build/CPU-domain collection
and the aggregate owner remain unqualified. No cancellation or drain contract is
changed. The internal selected-text validator returns borrowed tuple parts so
feature-light consumers can validate without unused backend-only struct fields;
all existing admission checks remain in that common validator.

A deterministic hash-associated development wheel can be packaged locally for
review. Pumas26a has no public local-component ingestion into its validated Torch
bundle custody and forbids in-place dependency repair. The future owner input
must use the existing pending stage, version lock, hash manifest, cleanup and
publication lifecycle, plus bind the actual in-process PyO3 interpreter and loaded
extension to that environment. Neither a managed venv install alone nor package
presence is sufficient. No live runtime registration or replacement is done by
the packaging helper; that owner/interpreter decision remains a rollout gate.

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

`npm run test:group-validation-browser` freshly runs the public Rust validation
tests, hashes their serialized responses/authored graphs/lifecycle events, then
runs the frontend aggregate including compiled Svelte in installed Chromium.
The browser replays actual producer responses through a controlled transport and
stores. It covers rendering, disabled Submit, lifecycle/edit races and calls
refused before frontend effects. The fresh Rust tests separately verify backend
publication/acquisition refusal. This is producer-response-to-browser contract
qualification, not live Tauri RPC, desktop GUI or successful native inference
submission. Chromium and existing Cargo/native prerequisites must be available;
the check does not install them or download models. Without fresh producer
evidence, the ordinary frontend aggregate explicitly skips this replay test.

Default rollout still requires qualified real timing coverage, resource and
adapter capabilities, measured dispatch overhead and completion quality, and
research-simulator differential acceptance. Native inference inside groups,
physical capacity enforcement, preload/staging planning, GPU/native batching
and remote execution require separate implementation and qualification.
