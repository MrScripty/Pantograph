# Plan: Domain Architecture And End-To-End Multimodal Workflows

## Merge note — 2026-11-03

origin/main (46aad5841) was merged into the Pumas RPC branch (`feat/pumas-rpc-on-main`, from `codex/pantograph-rpc-workflow-2026-09-26` at 019caf4aa). On the RPC branch, revision 2 replaced this file in place with "Plan: Isolated Pumas library RPC and Pantograph-owned inference workflows", while origin/main independently kept extending the original "Plan: Domain Architecture And End-To-End Multimodal Workflows" (custom-node adapter, CN-03 tool loop, CI-01, UniFFI and later sections). Both plan bodies are retained verbatim below: Part A is origin/main's plan body and Part B is the RPC branch's revision 2 plan (its title heading is demoted one level; its other headings are unchanged).

Each body declares itself the active sequencing authority, including its own plan status, current phase and next slice. Which body, or a reconciled successor, governs is an unresolved owner decision (decision owner: Puma / MrScripty) that must be settled before this branch is proposed to main. Nothing in either body is withdrawn by this merge. The ledger and issues files in this directory retain both histories likewise.

### Merge source dispositions — 2026-11-03

Where the two lines contradicted each other, the merge kept origin/main's behaviour (the behaviour its newer tests encode) and left the RPC-branch behaviour open for an owner decision. These items need an owner decision before this branch is proposed to main, and the RPC-A/INF-A/DA rows in Part B that cite this evidence need requalification:

- **Selected embedding backend.** Main's Candle CPU (`candle`/`candle.cpu`, HF directory) path is the only selected-embedding route. The RPC branch's llama.cpp GGUF route was removed, together with its dead helpers and tests: the gateway `execute_selected_embedding_with_cancellation` for `llama_cpp.embedding`, the GGUF `SelectedEmbeddingLoad`, the `llamacpp_startup_device_from_canonical` projection, `InferenceBackend::embeddings_with_context`, the embedded runtime-host projection and port branch, and the ignored real Qwen GGUF probes. The RPC branch's typed `EmbeddingVector` contract and its workflow-service mappings remain, but main's producer emits JSON vectors, so no production path emits `EmbeddingVector`.
- **Text generation options.** Graph-authored ports (main) are kept. The RPC branch's projection from scheduler `runtime_trait_settings` was removed.
- **Text runtime identity.** Main's requested-revision and dispatch model-ref projection is kept for text. The image projection keeps the RPC branch's load-target identity because main did not change that code.
- **Package-facts normalization.** Main's privacy projection (no owner-local absolute entry path) and main's requirement that a requested revision be present in producer evidence are kept. The RPC branch's retention of absolute producer entry paths was removed, along with its consumer-side `model_ref_contract_version` validation (`PumasModelRef` custom deserializer). Main's strip-before-decode approach is kept in its three decoders.
- **Batch lifecycle.** Commit 2ffc03792 (completed `DeferredToScheduler` members reported as `RuntimeHostCompleted`) was reverted, because main's newer image-session tests expect `RetryDeferred`.
- **Reconciled rather than chosen:**
  - The intent-first `get` path now backs main's `resolve_for_model` structure for every non-read-only access, including main's serial CPU callers. Read-only access keeps main's owner-fresh request, and main's selected-identity guard still applies after the intent handle.
  - The dependency-requirements coherence rule now applies only when the identity selects bindings. Main's native CPU bootstrap relies on an empty identity selection.
  - Hosted composition accepts owner, authenticated local-client and RPC access.
  - The RPC branch's artifact-identity and path-compatibility rules were applied to main's Candle embedding validator.
- **Pumas dependency.** The crate pin moved to main's `aa1d79ae`, a descendant of the RPC branch's `29242fce` (0.7.0). The retained 0.7.0 fixture artifact therefore no longer matches the compiled client and needs requalification.

## Part A — origin/main plan body

**Plan status:** `Active`

**CPU scheduler policy authority (2026-10-12):** The user separately authorized
the phased [CPU scheduler plan](../cpu-scheduler/plan.md). It owns current
completion-ranking, bounded ready-frontier, residency-policy, app-fairness and
supported CPU batching sequencing/acceptance. This plan retains graph/inference
feature ownership, historical evidence and paused multimodal work; its standards
baseline is unchanged. GPU/split and remote-worker implementation are deferred.

**Current phase:** M3 owner/client consumer source is implemented. Native CPU sampling and the supported inference/embedded host route are qualified separately; the desktop-authored dependent text-to-image graph and full review remain open.

**Preservation successor:** The user selected preservation of functioning rerank
and audio routes over publishing the direct-core removal experiment. The local
`local-feature/scheduler-rerank-preservation-20261007` branch starts from main
`a8483e51` and implements the smaller CPU llama.cpp rerank slice. See the
[rerank preservation report](reports/2026-10-07-scheduler-rerank-preservation.md)
for typed contracts, selected identity/custody, parent comparisons, reopened graph
acceptance and the deferred audio design. Direct rerank/audio handlers remain;
`63f7b4e` stays separate and unpublished. Native/model/display qualification and
publication are outside this local task.

**Next slice (CN-03, 2026-10-22):** Verify and serially integrate the bounded
scheduler-managed pure tool-loop implementation below. CN-02 owns the concurrent
NodeLab/edit-session path; PR92 trained-text readiness remains unchanged.

**Prior chat slice:** The user-selected single-prompt canonical `chat_completion`
graph feature is implemented and locally qualified on isolated
`feat/workflow-chat-completion`; see the [chat report](reports/2026-10-06-workflow-chat-completion.md). Source is
fresh main `c75fa237` with explicit ordinary composition of stop successor
`e570302d` and embedding revision successor `7be490d7`; those frozen candidates
remain separate and their review/publication belongs to the parent. Acceptance
requires exact canonical task/requested revision constraints and an offline
synthetic causal LM through saved/reopened public scheduler graphs, precancellation,
isolation and retained text-generation controls. Independent review and parent
publication remain separate. No pretrained/GPU/desktop or
resident-accounting qualification is implied.

**Prior stop slice:** The user-authorized bounded M2 text stop-string successor is
implemented on `feat/workflow-text-stop-string` from accepted seed/precision
`fae339bdb1eb681a01ea983f1abe0f2f48916349`, which includes main `d61b86fc`.
The [stop-string report](reports/2026-10-06-workflow-text-stop-string.md) records
exact graph text, omission/default layering, generated-only matching, marker
withholding, authored-minimum refusal and conditional backend support. Controlled
CPU and public graph/host checks plus independent medium review decide this
bounded slice; it extends M2 descriptor/validation work and was not an explicit
numbered post-seed milestone. Parent candidate review/publication remains separate.

PR60's held-text rewrite finding is repaired in a narrow successor of frozen
`19119837`; the [same report](reports/2026-10-06-workflow-text-stop-string.md#pr60-held-text-rewrite-successor)
defines immutable emitted-prefix and suffix-only rewrite semantics, including
crossing-boundary marker refusal and EOS/budget flushing. Parent verification and
fresh hosted CI remain separate; chat graph work resumes on its own candidate.

**Prior embedding slice:** The isolated [canonical CPU embedding graph successor](reports/2026-10-06-workflow-cpu-embedding-graphs.md)
is implemented from freshly fetched main `c75fa237`. Typed embedding/JSON ports,
the existing selected Candle owner, sequential envelope execution and the bounded
vector sink now compose through a saved public scheduler graph. Later independent
review found an explicit requested-revision projection gap in frozen `2961b148`.
The [isolated revision successor](reports/2026-10-06-workflow-cpu-embedding-graphs.md#explicit-requested-revision-successor)
reproduces it with actual Candle CPU output, requires requested/selected revision
agreement before resolution and preserves the original typed request constraint.
Parent verification remains pending. Local qualification and separate candidate
publication are recorded in the report; live Pumas,
pretrained-quality, GPU and desktop execution remain separate. Preserve frozen
stop-string source `19119837`, PR60's separate `e570302d` successor and every
frozen seed/precision/repair/paused history.
Do not resume ranking or resident-accounting work as part of this feature.

**Current-main integration:** PR61 merges freshly fetched main
`7b63535b6b850aa9fd001f96fe95b45f21c32b4c` into reviewed embedding revision
`7be490d7e403a95e2efff7e5a8f21a68d40a42fd` without rewriting either history.
Both additive contract tests and both plan records are retained. No native
desktop/chat source or evidence is included. Fresh composition tests and hosted
CI remain prerequisites for the separately owned review request.

**Integrated stop-string slice:** The user-authorized bounded M2 text stop-string successor is
implemented on `feat/workflow-text-stop-string` from accepted seed/precision
`fae339bdb1eb681a01ea983f1abe0f2f48916349`, which includes main `d61b86fc`.
The [stop-string report](reports/2026-10-06-workflow-text-stop-string.md) records
exact graph text, omission/default layering, generated-only matching, marker
withholding, authored-minimum refusal and conditional backend support. Controlled
CPU and public graph/host checks plus independent medium review decide this
bounded slice; it extends M2 descriptor/validation work and was not an explicit
numbered post-seed milestone. PR60 was merged normally into current main `7b63535b`.

PR60's held-text rewrite finding is repaired in a narrow successor of frozen
`19119837`; the [same report](reports/2026-10-06-workflow-text-stop-string.md#pr60-held-text-rewrite-successor)
defines immutable emitted-prefix and suffix-only rewrite semantics, including
crossing-boundary marker refusal and EOS/budget flushing. The reviewed
`e570302d` source is retained through that main merge.

**Prior seed slice:** Graph-authored text seed is implemented in frozen candidate
`6cf549dd`; see the [seed report](reports/2026-10-06-workflow-text-seed.md). Its
separate [desktop precision successor](reports/2026-10-06-text-seed-desktop-precision.md)
validates authored seeds before numeric persistence and blocks unsafe execution,
retaining raw text and a visible error. Direct desktop NumberInput seed edges support 0 through
`2^53 - 1`; typed Rust/API routes retain full u64. Local component/save/load/replay,
frontend and Rust wire checks pass after independent medium review. PR58 merged
at `d61b86fc` with the exact `d0c9788c` repair tree and is incorporated normally.
Parent publication and fresh hosted CI remain release dependencies. Frozen
composition `9e8cd64c`, evidence `fdb2bf2b`, harness successor `1b9fd0bd`, repair
line, seed candidate, paused seed work and their histories remain preserved.
Preserve full peaks, custody, uncertainty, known zero, source/instance fences and
the no-build-download contract. Native desktop, GPU, pretrained/custom models
and full desktop execution remain separate qualification. Full-u64 desktop
NumberInput entry is explicitly unsupported by this bounded contract.

**Prior native qualification:** Parent reviews frozen text-control source `a8c6970f` and its separate [exact-head native qualification route](reports/2026-10-05-text-control-native-qualification-route.md). Earlier local native host/workflow attempts failed before execution: the pinned ONNX dependency download returned HTTP 403, and no-download builds lacked linker symbols. Subsequent [hosted run 37360010163, job 111931961024](https://github.com/MrScripty/Pantograph/actions/runs/37360010163/job/111931961024) at exact head `815bceffbb0d877644184b71be6584f586be2e6e` passed 145 Rust tests across eight groups and six actual CPU sampler tests; all individual quality checks and all three workflows passed at that head. The focused native host/workflow qualification is complete for that head. Next qualify current owner-produced Pumas identities/facts/load targets/devices and the desktop-authored dependent text-to-image graph (DA-03). Frozen temperature `e0293ebf`, CI correction `fddae90d`, fixture `174c1950`, top-k `3bf3eb45` and composition `60197971` remain unchanged with their stated qualification requirements. Parent holds publication for owner confirmation and integrates review corrections without rewriting history. Current Pumas pin: `26a84e323cae566a46a8f76bef48fa1010aed48b`. DA-03/DA-07 remain open.

The next bounded scheduler capability is a fresh host RAM ceiling for explicitly declared backing pools, after CPU candidate discovery. See [host RAM ceilings](reports/2026-10-05-runtime-owned-host-ram-ceilings.md). It may only lower the configured budget, must preserve live task/resident charges after shrink, and must reread missing or changed owner facts at authoritative admission. It does not infer GPU placement, external-consumer allowance or a resident/transient discount.

CPU `394d4748` passed independent bounded source review. Frozen RAM `d7903010`
received two correctness findings: real cgroup hierarchy roots lack `memory.max`,
and over-capacity RAM incorrectly blocked unrelated VRAM resident publication.
The [narrow review repair](reports/2026-10-05-host-ram-ceiling-review-repairs.md)
distinguishes verified real roots from namespace-visible roots and limits resident
validation to charged pools. Its native source remains unavailable in this
container's unresolved `/..` mount layout. The independent [exact runtime service timing slice](reports/2026-10-05-exact-runtime-service-timing.md)
adds opt-in actual gateway/host observations with strict identity and provenance,
deterministic lifecycle tests, and disabled-by-default collection. Native owners
without immutable content and loaded implementation/configuration/device facts
remain unknown; controlled tests do not qualify real model latency. Ranking and
calibration remain separate until comparable native owner evidence is available.

Independent review accepted RAM repair `43f0a777`. The
[service timing retention repair](reports/2026-10-06-service-timing-correlation-review-repair.md)
replaces raw caller ID retention with bounded correlation digests, preserving
actual IDs through direct gateway and host execution, including rejected calls.
Frozen timing and device checkpoints remain separate. Parent coordinates normal
history-preserving integration with approved main `4153772634269e342a8b0cca797f1cd6716f18a5`;
its prior descriptor/sampler repairs will not be duplicated on feature ancestry.

The [runtime-owned CUDA fact slice](reports/2026-10-05-runtime-owned-cuda-device-facts.md)
uses the existing embedded PyTorch owner for explicit UUID/property observations.
The real CPU-only runtime qualifies the unavailable path; positive UUID tests are
controlled and GPU execution/capacity remain unqualified. Configured labels, Pumas
monitor aggregates and llama.cpp selectors cannot become physical backing facts.
Automatic GPU admission and completion ranking remain pending authoritative owner
placement, shared backing/capacity and comparable timing evidence.

The [bounded integration checkpoint](reports/2026-10-06-runtime-owner-integration-readiness.md)
combines both owner successors with approved main through ordinary merges. The
historical checkpoint records its two suite failures; the descriptor repair
arrives from main. Owner-authorized resumption produced the
[tested integration successor and draft review packet](reports/2026-10-06-runtime-owner-integration-successor.md):
canonical backend registration and real-owner abandoned warmup cleanup now pass
all affected library/host suites. The remaining model-contract failure is closed
by the [Pumas wire/projection qualification](reports/2026-10-06-pumas-wire-projection-qualification.md).
The producer version field remains current; the stale test had bypassed the
existing typed host adapter. Full mixed-backend inference and embedded suites now
pass. Parent coordinates PR publication, review and merge; native GPU, exact-model
and GTK qualification limits remain unchanged.

**Acceptance status:** `blocked`

**Independent scheduler slice:** The parent authorized continued feature delivery
while frozen native sampling qualification is blocked. The
[shared resource admission candidate](reports/2026-10-05-shared-resource-admission.md)
adds explicit backing domains across runtime RAM/VRAM claims, including unified
memory and authoritative commit/custody checks. Its separate
[startup composition successor](reports/2026-10-05-shared-resource-composition.md)
activates explicit persisted declarations through actual AppConfig/desktop setup;
portable config-factory/registry tests execute without ONNX. Native desktop qualification and automatic physical bindings remain pending;
parent owns coordinated review. Explicit-domain capacity now has an independent
host RAM ceiling successor; it does not establish complete available capacity.

The separate [AppConfig qualification successor](reports/2026-10-05-app-config-startup-qualification.md)
preserves frozen `73211ddc`, moves the actual persisted settings/startup composition
owner into `pantograph-app-config`, and fixes `Path::exists` hiding filesystem
failures. Real AppConfig cold-load/save/composition and filesystem regressions
execute locally. Authorized official dependency installation is blocked by OS
permission denial; the real desktop build fails at missing GLib prerequisites.
Tauri setup/IPC and a running desktop remain unqualified.

The independent [resident-resource successor](reports/2026-10-05-resident-resource-accounting.md)
adds explicit model/producer resident envelopes to the registry's existing local
and shared accounting, preserving charges after task release until confirmed
stop. Missing shared-pool resident estimates produce typed unavailable outcomes.
Portable lifecycle/contended admission tests qualify this owner capability;
the [producer bridge](reports/2026-10-05-runtime-producer-resident-estimates.md)
now publishes explicitly configured estimates from the active PyTorch lifecycle
owner with ordered-generation and known-zero handling. The bounded lifecycle repair at `b9e83208` establishes resident accounting or
uncertainty before terminal custody release, reconciles Retain, and makes failed
owned allocations ordinarily reclaimable. `dedbef88` adds acknowledged llama
child shutdown and ordered resident batches across PyTorch and llama owners.
Controlled real-host regressions qualify those paths. A proven resident/transient
split, GPU execution and real model execution remain separate gaps. Frozen
`eabbcc83` and `254aef1` are unchanged.

The [CPU candidate successor](reports/2026-10-05-runtime-owned-cpu-device-candidates.md)
projects available owner CPU variants into automatic selection and real selected
text execution while preserving full peak claims. Its full embedded suite has two
explicit frozen-parent failures: the four-versus-six descriptor assertion and the
warmup-timeout test receiving success. The descriptor repair `9ec3c2e` is already
an ancestor of PR55 qualification `ff84841`, but is absent from CPU `394d4748`.
Do not duplicate it on this ancestry or increase the warmup timeout.

Scheduler thesis priorities after the RAM ceiling slice are comparable exact-model
load/execution observations, actual per-device GPU ownership/backing facts, and
then completion-oriented ranking evaluated on frozen matched cohorts. Existing
technical-fit history ranking is already present. Warm-first heuristics, GPU class
counts as physical devices, and unproven resident/transient discounts do not meet
the thesis's physical-feasibility or timing-evidence requirements.

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** [planning baseline](reports/planning-baseline.md)

**Related ADRs:** [existing decisions](../../adr/README.md), especially ADR-001, ADR-006, and ADR-011–ADR-016. These remain binding until explicitly replaced with consumer and migration evidence.

## Objective

Make Pantograph understandable and maintainable through coherent domain ownership
and small, useful interfaces, while making real text and image model execution
work together in a desktop-authored workflow and return retained results.
Review the complete maintained repository against the newly selected standards;
repair applicable violations and architectural entanglement without rewriting
sound code merely because it was produced by an older agent.

Use **GPT-5.3 Codex Spark** for evaluation on real small, well-defined changes
and bounded repairs; **GPT-5.6 Luna max** for larger enumerated changes with
settled contracts; **GPT-6 Astra low** for complex implementation, integration
and rescue; and **GPT-6 Astra medium** for consequential analysis/design and
substantive independent review. GPT-5.6 Sol medium is additionally authorized for bounded implementation and
verification where it can reduce complete accepted-change costs. These are the user's current routing choices,
superseding the earlier two-model implementation preference. Preserve exact
requested models and record availability or fallback explicitly. Select work by
complete API dollars per accepted change, including checks, review, repairs,
rescue and shared coordination; differing task classes are not a controlled
benchmark. These settings apply to delegated tasks, not the primary session's
configuration.

## Admitted custom-node adapter slice (2026-10-21)

**Admission:** `continue` this Active plan, under the delegated instruction to
implement unfinished custom-node paths. Independent GPT-6.1 Sol High design
review admits the bounded contract below; production implementation starts after
this record. Existing text/runtime/readiness work and README PR79 are excluded.

**Operation and outcome:** host composition registers a trusted, nonblocking,
cancellation-safe in-process Rust `NodeExecutor`; a saved, cold-reopened
source → custom string transformation → sink graph runs through scheduler
sessions. This does not install new executable code without rebuilding.

**Binding contract:** Processing/Batch only, 1..8 required single string inputs
and outputs, bounded UTF-8 payloads, no inferred capabilities, inference,
configuration overlays, dynamic ports, or name/task-ID-based execution authority.
The immutable admission binds node type/version, normalized contract digest,
host-attested implementation digest and environment digest. Graphs pin that
identity; canonical lookup, derivation and invocation use the same admitted
registry. Built-in collisions and duplicate registration are rejected. Registry
publication is immutable; replacement requires a new service/graph identity.
The host attests captured configuration is immutable and callbacks neither block,
spawn detached work nor produce external effects. This is a trust boundary, not
a sandbox or verified binary/environment attestation. Managed invocation owns
input/output validation, scheduler run/task/attempt correlation, panic/error
classification, deadline and cancellation; dropped polling is the cancellation
claim. No result is committed after cancellation. No retry is introduced.

**Write set:** `crates/pantograph-workflow-service/src/{custom_nodes.rs,graph,
workflow,scheduler}`, its public exports and directly related tests; existing
plan/ledger/issues; `docs/custom-nodes.md`, ADR-018, documentation navigation
and the directly required decision-traceability map/guide projection.
No text runtime/readiness source or PR75/80/82 changes. Branch
`feat/custom-node-managed-adapter`, base `9ec7e7f3`, target `main`, task owner custom
node lead; parent owns integration after independent review. Preserve the branch
and worktree pending PR disposition; no merge/deploy or branch deletion.

**Composed-design review: applicable.** Admission/catalog owns trusted callback
identity, shape and immutable lifetime; graph owns pinned authored facts;
scheduler owns run/attempt state and result commitment; adapter owns one call.
Their necessary interleaving is identity and materialized values at dispatch.
Callers know a contract, host trust attestation, executor and identity pin; no
callback names, registry locks, task-type inference or plugin loader internals.
Changing a callback changes registration identity and graphs that choose it;
changing scheduler policy leaves callback code/contract alone. Stable interfaces
carry canonical contracts and identities; scheduler templates reference exact
identity. Adapter failure cannot commit outputs. Deleting the admission/adapter
would distribute validation, trust and lifecycle obligations across graph and
scheduler callers. No second runtime, plugin ecosystem, static tool loop, process
manager or generalized capability layer is added.

**Evidence and next slice:** bounded slice accepted after independent GPT-6.1
Sol High source review and 1,064 library + 65 integration tests passing, with
zero failures/ignored tests. Admission → saved/reopened graph → canonical
scheduler dispatch → result, immutable identity refusal, payload/type/collision
rejection, callback errors/panic, cancellation/commit arbitration and timeout
cleanup are covered. Evidence kinds: focused + integration + contract,
representative Linux CPU, automated; see the execution ledger for exact checks.
The broad plan remains Active; publication is a draft PR for parent integration.
NodeLab remains unavailable; scheduler-managed tool-loop and NodeLab exposure
require this exact registration/identity/managed invocation contract plus their
own lifecycle and authoring admission. Replan for untrusted code, side effects,
blocking work, external workers, or broader ports; those are not implicit grants.
Standards used for this slice: fetched Coding-Standards `0fc92a2` (original local
path unavailable), Core/Router and routed Planning, Implementation, Verification,
Development Proportionality, Commit, Documentation, Architecture/Code Design,
Contracts, Security, Concurrency/Resilience and Library/Rust API/Async guidance.
The rest of this plan retains its historical standards baseline.

## Admitted NodeLab/edit-session slice (2026-10-22)

**Admission:** `continue` this Active plan under the delegated unfinished-path
instruction. CN-01 is integrated in `2e517f58`, README PR79 in current main
`285c4898`. Independent GPT-6.1 Sol High design review admits a bounded NodeLab
graph-authoring surface before implementation. This does not author or install
executable code. PR92's unique trained-text source/evidence and missing T3 gate
remain separate and protected; CN-03 has its own worker and integration gate.

**Outcome and contract:** NodeLab reads a versioned service-owned projection of
host-admitted custom definitions and complete identities. The default desktop
catalog is honestly empty; a composed host supplies nonempty registrations.
Selecting an admitted node creates a separate edit-session draft with one text
source per required input and one text sink per output, derived from host ports.
Users edit source values, inspect backend diagnostics, save and reopen through
existing graph APIs. Saved pins are never silently refreshed. Missing/stale
implementations remain visible and blocking; recovery requires explicit new
authoring. No direct edit-session execution, Local Agent, runtime publishing,
arbitrary loader, new capability grant or static tool loop is enabled.

The draft is constructed by the service from the exact selected identity, not
from a client-supplied custom definition. The same immutable registry must reach
graph-session construction, provider
builders, canonicalization, response diagnostics and connection candidate/commit
validation. Add requires the exact selected identity; custom data patches cannot
change identity or add dynamic definitions. Imported invalid graphs remain
inspectable without healing. Name-only custom insert-on-edge/insert-and-connect
is explicitly unsupported; use pinned add plus explicit connections. The
legacy generic palette stays built-in-only. CN-03 consumes saved graph pins and
CN-01 scheduler invocation, not NodeLab state or a second execution route.

**Composition and write set:** registry/catalog owns admitted facts; graph
sessions own authored state, undo and validation; NodeLab owns local interaction
and observes service responses; existing store owns persistence. Stable
interfaces are the versioned read projection and existing pinned graph/mutation
contracts. This avoids duplicating admission or execution in the frontend.
Change service `custom_nodes`, `graph`, graph API/config and related tests;
NodeLab presenters/controller/page and narrow workflow transport projections;
desktop command wiring; a bounded real-host example/browser probe and affected
documentation. No scheduler-loop core, text runtime/readiness, root README or
PR92 edits. Branch `feat/nodelab-trusted-catalog`, worktree
`/workspace/pantograph-nodelab`, base `285c4898`, target `main`. Preserve source,
branches and evidence; parent coordinates serial integration if CN-03 intersects.

**Acceptance:** focused admission/edit-session refusal tests, real save/reopen
and connection candidate/commit tests, provider-builder registry retention,
frontend decoder/lifecycle tests, and real-browser pointer/keyboard draft →
edit → save → reopen interaction through an injected Rust host. Browser evidence
does not qualify native Tauri startup. Native compile/smoke is attempted only
where available and its actual limits recorded. Required CI, independent source
review and current-main recheck precede merge under the later cleanup authority;
no gate waiver, evidence-weakening rerun or persistent access expansion. Status
`Verifying`; next slice complete independent review, current-main/CI gates and
serial integration. Failed replacement preserves the current draft; failed close
transfers exact session IDs to an app-scoped, user-retryable cleanup owner. Standards baseline `0fc92a2`
continues, adding routed Frontend, TypeScript/Async, Accessibility, IPC,
Persistence, protocol/evolution and GUI verification detail to the CN-01 routes.

**Serial reconciliation (2026-10-22):** CN-03 PR94 is integrated as `46aad584`.
Continue CN-02 on that current-main base with the same production contract.
Preserve both admitted slices and CN-03 source unchanged. Add one edit-session
integration regression for a saved/reopened exact loop pin: its admitted catalog
accepts it, changed/missing catalogs retain it with blocking diagnostics, and an
invalid new pin is rejected without mutation. This tests the shared-registry
handoff rather than adding loop authoring or execution. Rerun both service suites,
frontend/static gates and the real-host browser flow; bind fresh combined-source
evidence, obtain independent review, then pass fresh exact-head CI and current-main
rechecks. PR94 post-merge CI must also pass before PR95 merges normally.
Add explicit hosted discovery/execution of all ten custom-node service tests;
keep CN-03's existing fifteen-test loop gate unchanged and name the new
edit-session regression outside that substring filter.

## Admitted CN-03 pure tool-loop slice (2026-10-22)

**Authority and status:** The user authorized unfinished Pantograph paths and
multiple implementations on 2026-10-21; the parent delegated scheduler-owned
CN-03 execution on 2026-10-22. Continue this plan for that bounded slice.
Status `Verifying`; objective acceptance `partial`. Source is isolated on
`codex/cn03-scheduler-tool-loop` in `/workspace/pantograph-cn03`, fetched main
`285c48989a95caef1bcaac92eac9ff3b1be0a9ad`. The parent owns serial integration
with CN-02. Retain branch/worktree, protected by the named branch, until review,
checks and integration disposition; no force pushes or branch deletion.

**Goal and binding decision:** Execute a saved/reopened source → `tool-loop` →
sink graph with a host-admitted pure controller and explicitly granted trusted
string-transform tools. [ADR-019](../../adr/ADR-019-scheduler-managed-pure-tool-loops.md)
and [the consumer contract](../../tool-loops.md) own authority, bounds, failure,
cancellation, commitment and schema version 9. Inference-backed controllers,
side effects, arbitrary loading, UI/edit sessions and static loop expansion are
excluded. No licensed trained-model or hardware acceptance is claimed.

**Write set:** workflow-service `custom_nodes.rs`, `tool_loops{.rs,/tests.rs}`,
`lib.rs`; graph `registry.rs`; scheduler `mod.rs`, `task_orchestrator{.rs,_tests.rs}`,
`tool_loop.rs`; workflow `task_graph{.rs,_contracts.rs}`,
`non_runtime_task_adapter.rs`, `session_queue_api.rs`, `tests{.rs,/tool_loops.rs}`;
quality-gates tool-loop test step/exact-head Rust checkout; traceability map/guide;
docs tool-loop guide,
custom-node cross-reference, ADR/index and this plan/ledger/issues. No frontend,
CN-02 authoring or PR92 readiness surface is in this write set.

**Composed-design admission:** applicable; inspected artifact probe:

1. Host composition owns what code/grants exist and why they are trusted;
   canonical registry owns graph identity; scheduler owns when execution starts,
   serial nonruntime invocation, attempt lifetime and publication.
2. Identity, budget and capability authority meet only at catalog admission and
   task derivation. Cancellation and commitment deliberately share the existing
   store gate. State is latest-observation only, with no durable partial replay.
3. Hosts learn one registration value and strict turn protocol; graphs carry a
   complete pin. Existing clients need no new executor, runtime or shutdown API.
4. Adding a pure tool changes host registration only. Changing turn protocol
   changes adapter/version and focused tests; adding model/effect capability
   reopens admission and must change the runtime/effect owners first.
5. Dependencies are exact CN-01 identities and the prepared invocation interface.
   Existing scheduler start/commit order is intentionally retained; graph data
   never supplies cancellation, attempt, callback or capability authority.
6. Contract admission and bounded runner can fail and be tested independently;
   final publication/cancellation is verified through the existing orchestrator
   and saved/reopened session consumer.
7. Removing `tool_loops` would spread grant/protocol rules into callers;
   removing scheduler `tool_loop` would spread turn/deadline ownership there.
   No additional registry service, worker, executor trait or static graph exists.
   Schema 9 is necessary to distinguish the coordinated new typed template.
8. Inherent exact authority, bounded state and cancellation ordering live at
   these owners; CN-01 retains primitive safety, scheduler retains lifecycle.

**Acceptance:** pure admission/authority, deterministic bounded progress,
primitive correlation, cancellation/commit race, timeout/disposal and positive
saved/reopened execution require focused tests plus the full service regression
suite. Independent GPT-6.1 Sol High source review and exact-head CI are required
before integration. Evidence and precise pending gates live in the ledger/PR.
The current milestone is verification; the next integration slice is CN-03
source review/checks then serial CN-02 dependency reconciliation. Re-plan if
model turns, external effects, unsupported authoring ports or shared API changes
are needed. An authoring palette cannot grant execution.

## Standards And Plan Authority

Standards source:
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards`

Planning baseline: `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
Read Core and Router, then canonical workflows/topics/profiles selected by actual
task facts and their Requires dependencies. Legacy `*-STANDARDS.md` navigation
files do not restore retired policy. Record material local standards changes or
later revisions before using them; do not silently mix acceptance baselines.

This plan is active under the user's instruction to begin with subagents and
the cost-effectiveness pilot. M0 superseded the [old portfolio](../current-standards-remediation/plan.md),
its five child plans and the [image plan](../current-image-generation-graphs/plan.md).
All old findings/claims are mapped in the [coverage report](reports/repository-review.md);
none are accepted by supersession. The old audit and accepted cleanup remain history.

## Documentation Proportionality

Apply the existing Planning, Documentation, Implementation and Development
Proportionality workflows; this policy requires no standards exception.

- Keep this single plan as sequencing authority. Do not create a child plan per
  agent, crate, finding or commit. Create another plan only when independently
  owned migration or coordination facts actually require it.
- Update current decisions, phase, next slice, blockers and acceptance when
  their meaning changes. Record those changes with the coherent implementation
  or evidence that caused them; no standalone lifecycle commits or per-command,
  per-agent-turn or per-commit documentation updates.
- Keep one concise ledger entry per accepted slice, plus material deviations,
  failed acceptance gates or verification changes. Link deciding evidence rather
  than copying command transcripts, diffs or test output into several documents.
  Successful routine checks need command/scope, result and relevant revision or
  environment, not a permanent raw-log report. Retain detailed evidence only
  where it decides a required claim or explains a failure.
- Agent reports contain source-backed findings, decisions and review coverage;
  no chronological work diaries or separate polished summary of the same facts.
  Consolidate duplicate findings into issues; update their disposition there.
  Completed reports remain dated evidence, not documents kept synchronized with
  implementation. Current policy lives only in its declared owner.
- Keep ordinary reversible design choices in code, tests and the change
  description. Update a canonical guide or contract document only when its
  durable knowledge changes. Use an ADR for a consequential durable decision,
  not every extraction, rename, test or interface-preserving refactor. Do not
  create a source-directory README without a real boundary-documentation need.
- Use one composed-design record per coherent changed composition; downstream
  slices link to it. Revisit affected answers only when ownership, caller
  knowledge, lifecycle, compatibility or observed change propagation changes.
  Do not copy the full probe into every task or repeat unchanged reviews.
- At M0, supersede old plan authority once and leave its history alone. At wave
  closure, compact obsolete narration only when it obscures current decisions;
  avoid cosmetic rewrites and unrelated formatting.

These reductions do not excuse stale current authority, missing coverage,
undispositioned findings, absent migration rationale, or unavailable acceptance
evidence. No new documentation generator, checker or registry is implied.

## Product Contract And Scope

Initial fixture assumption, subject to Milestone 0 confirmation:

`text input -> real text inference -> generated prompt -> real image inference`

The desktop user can author/save or load that graph, select real models, submit
one run, observe progress, and retrieve both generated text and a decodable image
in I/O Inspector. The image task must consume the text task's actual output;
two unrelated successful runs do not prove this contract. Both tasks share the
same workflow run identity, with distinct task/attempt identities.

The concrete port/fixture and policy-qualified cold-reopen procedure is in the
[desktop report](reports/desktop-review.md#executable-fixture-contract). Real
model/artifact IDs and qualified runtime/display remain missing prerequisites;
the selected text output port is `text`, not the retired `response` port.

An image-plus-text input to a vision model is a different capability. Record it
as required if selected by the user; do not claim vision acceptance from image
generation. Model families, supported input representations, streaming promises,
hardware, and persistence lifetime are established from real requirements in M0.

In scope: all maintained first-party Rust, Python, TypeScript/Svelte, bindings,
reusable packages, launchers, scripts, build/CI configuration, tests, and durable
documentation. Inventory generated, vendored, obsolete, and external material
separately, with source owners and reasons for exclusions. Review all maintained
areas, but change only evidenced violations or justified design problems.

Out of scope: publishing releases, changing external repositories without a
separate authorized write set, speculative new model families, restoring live
generated-UI execution without an accepted isolation design, and a wholesale
framework or distributed-services rewrite. Existing release/support obligations
are reviewed even though publication is excluded.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| DA-01 | Every maintained area and reachable interface is reviewed against applicable current routes; each finding has evidence, owner, severity and disposition; no required violation remains open. | focused | not-applicable | either | pending | M1/M5 coverage and findings reconciliation |
| DA-02 | Changed domain interfaces have one owner per invariant/state/lifecycle; representative changes demonstrate reduced unrelated caller knowledge; replaced authorities and unsupported paths are removed or have real migration obligations. | integration | representative | either | pending | M2/M4 composed-design reviews and interface tests |
| DA-03 | One real desktop-submitted mixed graph runs real text and image models, passes generated text along its edge, and exposes both retained outputs with correct run/task identity. | user-workflow | required-real | automated | blocked | EX-04 and D-02 controlled integration accepted; real fixture/runtime prerequisites remain |
| DA-04 | Missing models/dependencies, denied model-code trust, invalid graphs, worker failure, cancellation and shutdown produce the declared terminal outcomes without false success, leaked processes/reservations, or cross-run input leakage. | system | representative | automated | pending | Focused lifecycle contracts and real worker process evidence |
| DA-05 | Accepted saved graphs and retained outputs obey their declared compatibility/lifetime contract through cold reopen; affected bindings and generated/IPC contracts agree with their owners. | contract | representative | automated | pending | Consumer fixtures and cold-process persistence checks |
| DA-06 | Model selection, submit-to-dispatch overhead and runtime reuse meet workload-specific budgets recorded before optimization; measurement separates model computation from orchestration and includes resource use. | system | required-real | automated | pending | M0 baseline and M4 comparison |
| DA-07 | Required supported-target, static, test, accessibility and artifact claims pass on the final integrated material revision; required unavailable evidence remains visibly blocked. | system | required-real | either | pending | M5 claim-specific evidence bundle |

DA-07 aggregates links, not proof kinds: any required release-artifact,
user-workflow, contract, or platform claim retains its own kind and environment
in the acceptance matrix created in M0/M1. A local build cannot close those claims.
An exception is explicitly owned and reported; it is not full compliance with
the overridden obligation. Product success and full review closure are reported
separately until all seven claims are satisfied.

## Constraints And Binding Decisions

| Decision | Owner | Rationale / replaced assumption |
| --- | --- | --- |
| One integrator controls shared contracts, lifecycle and acceptance. | Integrator with Astra medium design/review | Parallel effort must not duplicate authority. |
| Product execution is demonstrated early, before unrelated repository cleanup. | This plan | Replaces the old portfolio's broad prerequisite queue after M0 handoff; necessary trust/contract prerequisites remain. |
| Existing code and ADRs are evidence, not automatic proof of good design or defects. | Domain reviewers | Fresh review replaces inherited diagnosis and blanket rewrite assumptions. |
| Pumas owns model/package/artifact facts; Pantograph owns workflow and execution policy. | Existing architecture and affected contracts | External load-target gaps remain explicit dependency work, never local path guessing. |
| Runtime host executes a scheduler decision; UI and transports project owned outcomes. | Existing ADRs | No special demo executor or frontend-selected runtime policy. |
| Prefer ordinary tests and existing tools over new compliance infrastructure. | Verification owner | Every permanent checker/registry requires distinct deciding value and lifecycle justification. |
| Preserve unrelated local work and real public/persisted contracts. | Integrator | Protected Pumas proposals are outside this plan's write set. |

## Review Method And Domain Map

Start from behavior and trace producers, state transitions, consumers, failures,
and lifecycle. Do not allocate refactors by file size or presume each crate is
already a sound domain. The initial investigation map is:

| Concern | Question to settle |
| --- | --- |
| Graph authoring and node contracts | Who owns graph validity, inference ports, edits, graph revisions and materialization? |
| Workflow run and task orchestration | Who owns run identity, dependency readiness, attempts, cancellation and terminal results? |
| Scheduler and resource admission | Who owns placement, compatible grouping, reservations, waiting, fairness and dispatch? |
| Model library and dependencies | Which facts belong to Pumas, and how are freshness, readiness and authorized load targets conveyed? |
| Runtime host and inference | Who owns execution protocols, model residency, trust, worker processes and shutdown? |
| Media and artifacts | Who owns values versus references, conversions, retention, retrieval and run attribution? |
| Desktop and graph package | Which state is user intent versus backend projection; how are errors, stale updates and accessibility handled? |
| Bindings and infrastructure | Which consumers, compatibility promises, build targets and operator lifecycles are actually supported? |

For every material interface record only useful facts: responsibility and
non-responsibilities; owned values/invariants; callers; inputs/outputs; lifecycle,
ordering and errors; persistence/version obligations; hidden implementation;
and focused acceptance evidence. Put durable rationale in its canonical
architecture/contract document, not duplicated across reports.

Compare keep-and-repair, consolidate/delete, and a new seam only where a material
decision needs alternatives. Once a reversible design satisfies the contract
and standards, implement it. Additional review needs a named unresolved decision
and stopping condition. No recurring review tournament or arbitrary score target.

## Simplicity And Ownership Review

**Applicability:** `applicable`

This planning composition retains current execution owners provisionally; it
does not yet admit a redesigned runtime. M2/M4 must replace these provisional
answers with inspected artifact evidence before each structural implementation.

- **Independent concepts and dimensions:** Graph describes what depends on what;
  a run identifies requested execution; scheduler decides when/where; model facts
  describe available artifacts; runtime host owns how execution occurs; artifacts
  own retained results; desktop owns interaction. The concrete division inside
  workflow-service and embedded-runtime is an explicit M1/M2 investigation.
- **State, identity, value, time, policy, mechanism:** Keep graph revision, run,
  task attempt, model/artifact identity, runtime residency, reservations, user
  input and projection freshness distinct. Scheduling references library facts
  without owning their meaning. Artifact retention does not own run scheduling.
  Document actual version roles and consumer overlaps in M1; do not invent one
  global version. Graph edits must not mutate submitted run snapshots; model
  changes invalidate dependent execution facts, not unrelated UI identity.
- **Caller and composition-root knowledge:** Submission callers should supply
  workflow/session identity and typed inputs, not Python paths, GPU placement or
  worker setup. The existing embedded composition factory is a starting point
  to inspect for excess lifecycle/configuration knowledge, not proof of depth.
- **Representative changes and forced owners:** Changing a model-load protocol
  should affect its adapter/host contract, not graph interaction. Changing
  batching policy should stay with scheduler policy and its evidence. Adding an
  image display should consume an artifact contract without choosing retention.
  Updating a model selector should not require scheduler policy edits. Compare
  actual before/after change propagation for these cases.
- **Stable interfaces versus hidden knowledge:** Graph/session operations,
  inference descriptors, scheduler handoff and artifact retrieval are candidates
  for stable interfaces. Storage layout, Python kwargs and concrete runtime
  wiring remain implementation details unless a real consumer contract says otherwise.
- **Independent evolution, testing, failure and replacement:** Test each owner
  through the same interface its consumers use, then exercise the full desktop
  path. Cross-process execution requires real process failure/lifecycle evidence;
  synthetic inference cannot establish model functionality.
- **Deletion result:** This plan introduces documentation only, no production
  registry, generator, framework or adapter. For each later permanent mechanism,
  record what necessary complexity returns to callers if it is deleted; delete
  pass-through machinery whose complexity simply disappears.
- **Necessary complexity and cumulative machinery:** Resource admission, model
  trust, async execution and retained artifacts are inherent. Keep them contained
  in their owners; reassess the complete composition if their knowledge spreads
  across callers or the refactor adds competing authorities.

## Evidence And Oracle Plan

- DA-03 uses a saved fixture and actual model/runtime identities, a trace of the
  materialized edge value, real text output and image decoding, plus assertions
  on I/O Inspector. Exact generated wording/pixels are not required unless a
  selected model contract promises determinism. Nonempty output alone does not
  prove that the edge was used.
- DA-04 uses deliberate failures at the relevant boundary with assertions on the
  intended diagnostic/terminal state, cleanup, and reservation release. Merely
  observing any exception does not satisfy the negative case.
- DA-05 exercises real consumers and cold reopen against authoritative persisted
  data. A re-created in-memory object is not cold-process evidence.
- DA-06 records machine/model/runtime, cold/warm conditions, workload, latency
  distribution and resource use. Initial engineering targets are warm local
  model-selector rows p95 <=250 ms and already-ready task dispatch overhead p95
  <=500 ms, excluding declared batching/admission wait and model computation.
  These bound interactive overhead, not model speed or universal hardware
  guarantees. Model/runtime/device qualification is still unavailable; measure
  baseline before optimization and revisit targets only with recorded workload
  evidence. Run model work serially when contention would invalidate measurements.
- DA-01/02 use source-backed coverage and concrete change-path comparisons.
  File counts, grep scans and passing lint are supporting evidence only.
- Reuse existing scripts and runners after checking their actual proof scope.
  Extend only what the selected claims need. Keep expensive real runs for
  behavior-affecting integration points and final acceptance.

## Systemic Finding Audit

For a repeated defect, bound its invariant, owner, representations and all
reachable consumers before fixing examples. Each occurrence is repaired,
consolidated, removed, retained with evidence, or blocked with an owner. Expand
only for a newly reachable consumer, authority, material risk or supported
contract. Compare the repaired whole composition and remove superseded evidence
machinery. Stop when the bounded family and its acceptance claims are closed.

## Milestones

### M0 — Establish scope, authority and executable fixture

**Goal:** One current plan authority and a concrete product/verification contract.

**Allowed write set:** this plan directory; `docs/plans/README.md`;
`docs/README.md`; the `plan.md` and `execution-ledger.md` of the old remediation
portfolio, its five child plans, and old image plan. Product source is read-only.

**Tasks:** Reconcile old findings/claims without silently dropping any; retire
overlapping plan authority; record standards revision and local modifications;
inventory maintained source roots, consumers and supported targets; identify
real model IDs, runtime/dependency availability, trust requirements, saved
fixture and desktop runner. Record baseline commands and results only where
they decide immediate prerequisites. Select initial performance budgets and
the actual persistence promise. Route standards from observable facts.

**Gate:** DA-01 coverage population is bounded; DA-03/05/06 acceptance contract
is executable or exact external prerequisites are recorded. Each old claim has
a disposition. Missing model/hardware evidence blocks the dependent real run,
not independent architecture review. Review stops when these decisions are possible.

**Status:** `Accepted`

### M1 — Review the domains and repository coverage

**Goal:** Source-backed ownership map and prioritized findings, with execution-path findings delivered first.

**Allowed write set:** this plan's `reports/`, `issues.md`, `execution-ledger.md`
and `plan.md`; product source remains read-only.

**Tasks:** Trace the mixed graph through every real owner. In parallel review
other maintained areas using the table below. Map applicable standards to
actual evidence; distinguish defects, design weaknesses and optional preferences.
For each finding identify the owned invariant, affected consumers, consequence,
proposed disposition and deciding test. Inventory old tests/checkers for coverage
and obsolete guarantees. Do not defer the first production slice until every
unrelated finding is exhausted.

**Gate:** Execution-path review provides enough facts to admit M2 with exact
files and tests. Full repository review may continue alongside M2/M3 but must
close before M5. Record examined populations and unresolved areas explicitly.

**Status:** `Active`

### M2 — Repair the canonical execution path

**Goal:** One coherent backend path can execute the selected mixed graph safely.

**Allowed write set:** plan reports/control files plus RT-01: new
`crates/inference/torch/worker_diffusion.py`, `crates/inference/torch/worker.py`,
`crates/inference/src/backend/pytorch_worker.rs`,
`crates/inference/src/backend/pytorch_worker_image_python_tests.rs`, and
`scripts/diffusion_cli_smoketest.py` (support/help statement only).
Also admitted: `crates/inference/src/backend/pytorch_tests.rs`, registering the
new real helper and completing the existing batch-projector stub in its text-worker fixture; isolated consumer testing
proved that the new import otherwise breaks four existing text/lifecycle tests.
The admission decision in `reports/runtime-review.md` defines the finite built-in
component construction, preserved scheduler configuration, local weights with restricted
deserialization (see the report correction), typed failures and admission-before-cache invariants. Custom code and
other pipeline variants remain unavailable; no configurable authorization is
invented. Astra low implements; independent Astra medium review and real-loader
regressions decide this slice. Other slices still require exact admission.
Candidate owners: workflow-service, scheduler, embedded-runtime, inference,
runtime-host contracts, node contracts and their existing consumers. This list
is an investigation scope, not blanket source-write authority.

EX-01/EX-03 also admits these five inline-test/source files under
`crates/pantograph-workflow-service/src/workflow/`:
`runtime_branch_task_event.rs`, `runtime_dispatch_assignment.rs`,
`task_execution_worker.rs`, `runtime_branch_batch_execution.rs`, and
`session_execution_api.rs`. The final scoped-live-claims decision in
`reports/execution-review.md` governs ownership transfer, both event and batch
fences, immediate compatible grouping, supervised failure and deterministic
tests. Keep proofs out of immutable snapshots; retain finite expiry for unowned
work. Post-dispatch abandonment with unproved host stop is failed/fenced against
replay, not successful cancellation or resource-release evidence. No new timer,
service, schema or scheduler API is admitted. Missing lifecycle capability
returns to medium design before expanding the slice.

Consumer migration additionally admits
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`:
six session assertions assumed two independently submitted runs always shared
one host request. Validate nonempty bounded groups and exactly-once run/member
identities across all requests, retaining existing prompt, result, recovery and
diagnostic checks. Do not replace the old timing assumption with exactly two calls.

RT-03 is admitted under the runtime report plus the bounded consumer expansion
found during source verification. Core owner (Astra low):
`crates/inference/src/backend/mod.rs`, `backend/pytorch.rs`,
`backend/pytorch_tests.rs`, optional private `backend/pytorch_text_job.rs`,
`gateway.rs`, `gateway_tests.rs`, and `gateway_tests/start_config.rs` (all paths
after the first share `crates/inference/src/`). Replace backend stop with
`async fn stop(&mut self) -> Result<(), BackendError>` and gateway stop with
`pub async fn stop(&self) -> Result<(), GatewayError>`; preserve failures,
producer completion and coherent metadata under existing ownership.

Consumer owner (Luna max): `crates/inference/src/backend/llamacpp.rs` and
`candle.rs`; `crates/node-engine/src/core_executor/kv_cache_test_support.rs`
and `inference_tests.rs`; these files under
`crates/pantograph-embedded-runtime/src/`: `embedded_runtime_lifecycle.rs`,
`runtime_registry_controller.rs`, `runtime_registry_lifecycle.rs`,
`runtime_registry.rs` (exports), `reservation_lifecycle.rs`,
`embedded_workflow_host_helpers.rs`, `lib.rs` (exports only), `lib_tests.rs`,
`runtime_host_execution_port.rs`, `runtime_registry_tests.rs`,
`runtime_registry_tests/lifecycle.rs`, `reservation_lifecycle_tests.rs`,
`workflow_runtime_tests.rs`, `lib_tests/session_runtime_lifecycle_tests.rs`,
`lib_tests/workflow_run_execution_tests.rs`, and
`lib_tests/runtime_lifecycle_capability_tests.rs`; these files under
`src-tauri/src/`: `llm/gateway.rs`, `llm/runtime_registry.rs`,
`llm/rag_sync.rs`, `llm/recovery.rs`, `app_lifecycle.rs`, and
`llm/commands/server.rs`. Only stop-contract propagation, relevant tests and
necessary exports are admitted, including producer/stop-all controller Result
signatures and a local lifecycle error wrapping registry/gateway failures.
Reconcile observed state even on stop failure; do not return successful reclaim,
restart recovery, invalidate residency or log successful shutdown on that failure.
Additional actual callers require bounded source-backed admission.

This is a useful delegation evaluation of different task classes, not a matched
model benchmark. One integrated medium review covers composition; record both
implementation lanes, design, all review/repair/verification agent calls and
root integration/orchestration for RT-03. Shared slice costs remain explicit,
not zero or an invented per-model allocation. API-dollar estimates use the
ledger's verified rate assumptions and disclose the measurement cutoff.

RT-02 is admitted using the runtime report's selected-text contract and the
completed RT-03 lifecycle gate. Inference owner (Astra low), exact paths under
`crates/inference/src/`: `gateway.rs`, `gateway_tests.rs`, `backend/mod.rs`,
`backend/pytorch.rs`, `backend/pytorch_tests.rs`, new `selected_text_execution.rs`,
and `lib.rs` (module/export only). Host owner (Luna max), exact paths under
`crates/pantograph-embedded-runtime/src/`: new `runtime_host_text_execution.rs`,
`lib.rs` (module only), `runtime_host_execution_port.rs`, and
`lib_tests/workflow_run_execution_tests.rs`, and `runtime_host_image_execution.rs`
(only expose the existing Pumas-target converter as `pub(crate)` for shared use).
Additionally admit `lib_tests/runtime_preflight_tests.rs` solely to gate its
Candle-specific test with `cfg(feature = "backend-candle")`; its unconditional
reference prevents the requested no-Candle test matrix from compiling. Astra low
owns the remaining host integration and this bounded repair after the Luna handoff.
Root owns these plan/control files.
No other image projection, worker Python, external Pumas, wire schema, or other source
changes are admitted without a source-backed expansion.

The shared gateway operation is `execute_selected_text_with_cancellation`,
accepting existing `InferenceExecutionRequest`, separate `PumasArtifactLoadTarget`,
`BackendExecutionDecision`, and `InferenceExecutionCancellationHandle`, returning
`Result<InferenceExecutionResult, GatewayError>`. Inference validates identity,
PyTorch runtime, concrete device, directory target and denied custom-code policy
before effects, and owns residency through observed terminal completion and
cancellation cleanup. Host projects exact `prompt: String` to `text: String`,
uses existing generation defaults, rejects unsupported inputs and strings over
1024 bytes without truncation, and requires no image sink for text. Existing
image and batch behavior remains part of verification. Canonical EX-01/03 sends
even singleton text through the batch entrypoint: admit sequential awaited text
member execution there, validating text member shapes before effects, preserving
member identities and existing retry/reservation outcomes. Completed earlier
members remain retained; failed/cancelled members emit no partial output and
remaining cancelled members do not load. This reuses the selected-text operation,
not a new native text batching mechanism. Independent Astra medium review judges
integrated source and deciding tests, including service→batch-host text retention. Cargo ownership is serialized;
implementation, checks, repairs, review and shared root costs are counted together
with an explicit reporting cutoff. Real-model acceptance remains outstanding.

EX-04 is admitted after medium confirmation of the canonical worker path.
Astra low owns exactly these files under
`crates/pantograph-workflow-service/src/workflow/`, including inline tests:
`session_scheduler_runner.rs`, `runtime_branch_task_event.rs`,
`runtime_branch_batch_execution.rs`, and `task_execution_worker.rs`.
Core additionally owns `session_execution_api.rs` for composed recovery to skip
validated scheduler-completed upstream tasks without replay while preserving
mismatch/failed diagnostics, plus its focused recovery regression.
`task_execution_facade.rs` and `task_execution_runtime.rs` are admitted only for
two diagnostic test expectation migrations caused by session validation before
event claim. Pre-proof dependency-pending events remain Ready/unclaimed; this
does not change post-dispatch deferred settlement/retry policy.
Luna max initially owned only
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`
for dependent canonical retained-result fixtures, preserving the accepted singleton
case. Astra low completed the fixture integration, bounded repairs and composed
recovery evidence after the explicit ownership handoff; the shared write set did
not expand. Root owns this plan, issues and ledger. No wire/persisted schema change or
other source file is admitted without source-backed expansion.

Move scheduler progress/admission before claim, select only a Ready task with its
dependency proof, and claim its matching run/task event. Classify next-task
readiness versus all-complete instead of requiring the entire graph Ready.
After observed host completion, retain task results and settle the current
assignment/event; unfinished work is a typed continuation. The worker retains
and rebinds the same run responder under its existing supervised ownership,
then advances downstream inputs and selects the next task. No detached
continuation, premature whole-run success, copied-proof authorization, or lost
cross-run batch responder is allowed. Existing failed/deferred/cancelled and
shutdown/fencing semantics remain. Call the existing whole-run finalizer only
when all tasks complete. Canonical dependent retained text and text→image
fixtures, cancellation/failure and claim-ownership regressions plus independent
medium review decide acceptance; controlled adapters do not prove real models.
Cargo ownership is serialized between core and fixture checks.

**Tasks:** Fix confirmed trust authorization before real model execution;
consolidate inference descriptor/validation authority; settle scheduler versus
run orchestration ownership; repair Pumas load-target/dependency handoff;
preserve task identity, real edge materialization and artifact attribution.
Support solo dispatch without requiring a peer; preserve compatible batch
semantics. Exercise real text and image model adapters behind canonical entry
points. Delete replaced paths after consumer migration is covered.

**Gate:** Focused contracts and affected static checks pass; canonical backend
mixed workflow returns both outputs; denied trust and missing facts fail at
their proper owner. This is intermediate evidence, not DA-03 desktop acceptance.

**Status:** `Active`

### Pumas owner/client integration — implemented source, qualification open

The selected design below is retained as rationale. Its producer operation and
consumer migration are already present at the current pin; they are no longer
the next implementation slice. `PumasSelectorAccess` routes full facts and
targets through Owner/LocalClient, while dispatch, host resolution and hosted
composition consume that facade and preserve ReadOnly execution refusal.
The [full-facts consumer report](reports/2026-10-04-pumas-full-facts-consumer.md)
records the six boundaries, exact producer identity and outstanding native
acceptance. Its earlier source-only receipts do not establish real model or
desktop acceptance. The [configured-owner report](reports/2026-10-04-pumas-configured-owner-client.md)
records accepted attachment and configured-root ownership behavior.

Workflow text generation already supports `max_new_tokens` and `system_prompt`.
The [top-k candidate](reports/2026-10-05-workflow-text-top-k.md) adds the missing
optional integer sampling input through existing U64 values and typed options,
with explicit zero and unchanged omission semantics. This is a bounded input
projection; scheduler redesign and broader backend support are not admitted.
The original feature composition with PR54 repair head `78bc7193` remains
frozen. Its successors preserve both histories: the [public-session fixture repair](reports/2026-10-05-pr54-session-output-discovery.md)
persists real built-in I/O definitions, and the [sampling repair](reports/2026-10-05-top-k-vocabulary-sampling.md)
caps positive k at vocabulary width and preserves non-streaming explicit zero.
Parent review and final native qualification remain outstanding.

**Decision:** Reuse the existing Pumas facts/target operations and Pantograph
access facade. Pumas owns model identity, inspection, freshness, cache and path
resolution. Pantograph owns scheduling, device/runtime choice, code permission,
execution and output retention. The two applications share an existing library
owner; an attached client never owns that process's shutdown.

**Producer change:** Pumas already implements full package facts in its owning
interface, HTTP RPC and an internal dispatch branch. Complete the typed local
IPC operation/client exposure with connection-token validation, bounded model
ID decoding and the existing public diagnostic conventions. Reuse the domain
resolver and cache. Do not add an aggregate execution API, HTTP endpoint,
independent cache, model-copy mechanism or general connection framework.

**Consumer change:** Use the existing Owner/LocalClient/ReadOnly facade for
facts and targets instead of concrete owner dependencies in dispatch and host
composition. Execution accepts Owner or a client attached to the configured
launcher root; ReadOnly remains explicitly non-executable. Match Pumas's
advertised canonical launcher-root identity and try that owner before a
read-only fallback. Never select another library merely because it connects.
Retain lifecycle cleanup only for owners constructed by Pantograph.

**Facts and ordering:** Keep model-list snapshots cheap and hydrate full facts
only for selected execution candidates. `OwnerFresh` target resolution refreshes
external validation but does not hydrate package facts. Validate selected
artifact identity across facts/target and preserve typed stale, missing,
invalid and unsupported outcomes. Schema-version equality is not evidence
identity; the current target supplies no content fingerprint or file lease.
The two operations do not promise atomic immutable contents. File-open failure
remains an execution error; a future immutable-result requirement needs its own
producer-owned contract. Keep executable paths out of scheduler projections.

**Change locality and deletion:** Existing Pumas owner/client adapters contain
transport differences. Completing them removes owner-specific rejection and
branching from Pantograph's facts/target consumers. A Pumas storage/cache change
stays in Pumas; a Pantograph scheduler-policy change stays in Pantograph.
Keep existing internal resolver traits for host tests; no new permanent module
is needed. Separate dispatch and host freshness checks remain until measured
reuse can preserve their semantics; no speculative cache is admitted.

**Bounded implementation candidates:** In Pumas, paths under
`rust/crates/pumas-core/src/`: `ipc/protocol.rs`, `ipc/local_client.rs`,
`api/state.rs` and their existing focused contract tests. In Pantograph:
`crates/workflow-nodes/src/setup.rs`; embedded-runtime's
`pumas_dispatch_package_facts.rs`, `runtime_dispatch_load_target_facts.rs`,
`runtime_host_package_facts.rs`, `runtime_host_load_target.rs`,
`workflow_service_composition.rs`, and affected existing tests; dependency
pins/locks and actual independently pinned consumers must be enumerated before
editing. This design selects scope, but does not authorize overwriting either
repository's active work; freeze the concrete revision and exact test/pin
write set when admitting implementation. The existing domain-architecture plan
remains the sole Pantograph sequencing authority.

**Gate:** Real owner/local-client contract equivalence, authentication/invalid
request/owner-loss failures, correct-root selection, ReadOnly refusal, selected
artifact changes and invalid/missing size/target shape rejection; actual
producer facts through canonical dispatch/host, no scheduler path leakage,
cheap snapshots and measured selected hydration. Target transport completion
is separate from current-model identity/path qualification and real inference.
Pumas path/identity repairs require reproduced current producer evidence.

### M3 — Prove the desktop mixed workflow

**Goal:** The user authors/submits the graph and receives text and image results.

**Allowed write set (D-02):** `src/components/workbench/ioInspectorPresenters.ts`,
`src/components/workbench/ioInspectorPresenters.test.ts`,
`src/components/workbench/IoInspectorPage.svelte`, and the existing
`tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs`
(decode/dimension assertions only). Luna max owns these settled repairs.
Sol medium owns the disjoint controlled browser fixture
`tests/e2e/io-inspector-image-preview/run.mjs` and `fixture.ts`.
Root owns this plan, issues and ledger; Astra medium independently reviews.

D-02 uses the existing full-body request for image Read while retaining bounded
text previews. Both Read and Read Stream reject incomplete images before
preview installation. Inspector owns image decode errors, matching-URL cleanup,
and stale request disposal across selection changes and unmount. Download
remains independent of display decoding. The new harness mounts the actual
Inspector with controlled service responses and a valid PNG larger than 64 KiB
in real WebKit; it supplies the decoder/lifecycle evidence absent from Node
presenter checks without requiring models. Retain it while those browser
contracts lack equivalent coverage in the canonical desktop harness. This
repair preserves artifact storage and IPC contracts. D-02 is accepted after focused requests, browser success/failure/lifecycle
checks, affected static checks and independent medium review; it does not
satisfy DA-03. Further mixed-workflow source writes require
separate concrete admission.

**Tasks:** Integrate actual node ports and model selection, run progress/errors,
artifact retrieval and I/O Inspector. Extend the existing Tauri/WebKit harness
to prove the dependent text/image graph. Verify save/reopen, cancellation and
failure presentation. An inaccessible required interaction is part of this slice.

**Gate:** DA-03 passes on recorded real models/hardware/runtime; affected DA-04
and DA-05 cases pass. Preserve raw bounded evidence in reports. If blocked,
record the exact missing prerequisite and continue unrelated admitted work.

**Status:** `Planned`

### M4 — Complete domain remediation and measured efficiency

**Goal:** Close the remaining full-repository findings while preserving the working product path.

**Allowed write set:** P01/P02 exact paths in First Pilot Admission, plus P03
`crates/inference/src/gateway_tests.rs` for the existing PyTorch alias test's
feature-independent Mock backend constructor only, plus plan
reports/control files. Other finding families require exact production/test/
documentation write sets before implementation.

P04 also admits only
`crates/inference/tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json`:
change nested `payload.model_source.source_contract_version` from 2 to 3.
Medium reviewed all 19 worker JSON fixtures and the source-contract migration;
no other field or validator change is required. This is a low-complexity positive
fixture repair assigned to Astra low, comparable in scope to Luna's P03 fixture
repair. Preserve outer worker version 1 and negative stale-version cases.
Existing load-envelope consumers and invalid-source tests decide acceptance;
independent medium review remains required. It is the fourth useful pilot task,
not a matched test of multi-file architecture implementation.

**Tasks:** Refactor independently changing concerns identified in M1; address
binding/runtime lifecycles, frontend state/accessibility, dependency ownership,
test discovery, launcher/build/release obligations and stale documentation as
applicable. Measure selector performance, scheduling overhead and reuse; fix
demonstrated problems at the owner. Pumas changes require their own authorized
work; do not duplicate its semantics. Remove dead paths and unjustified checks.

**Gate:** DA-01/02/04/05/06 evidence closes for each admitted finding family;
affected tests and static gates pass; the mixed workflow remains functional.
No arbitrary file-size target, new crate quota, or mandatory universal abstraction.

**Status:** `Active`

### CI-01 — Session Capacity Observability Repair

**Decision (2026-10-02):** Explicit keep-alive creation/enablement owns the
existing session-count capacity limit, host-selected eviction and affinity.
Runtime-task dispatch continues to own dependency admission and resource
reservations; non-runtime runs must not acquire a session runtime. The five
legacy capacity tests reached the wrong owner after this split. Moving only
their trigger would lose previously asserted eviction diagnostics and timing:
`session_runtime.rs` currently receives no diagnostic context from its sole
keep-alive caller, while that context requires a run that does not exist.

**Admitted repair:** Emit eviction lifecycle facts with genuine target and
unloaded execution-session identities and optional real run attribution. Extend
the existing typed diagnostic payloads additively, retaining old payload decoding
and validation of run-only events. Add a registered session-runtime error scope
so an unload failure retains its original code/message and reports diagnostic
unavailability without inventing a workflow run. Preserve host selection,
rollback, cleanup, event order and a shared timing-attempt identity. Session-only
facts belong in the ledger; they must not create fabricated run projections.

**Write set:** workflow-service `session_runtime.rs`, `diagnostic_errors.rs`,
`tests/session_capacity.rs`, `tests/session_capacity_faults.rs`, test module
registration and focused diagnostic tests if needed; diagnostics
ledger `event.rs` and `tests.rs`; this plan, its scoped report, and the owning
`docs/headless-workflow.md` guide. No runtime-task admission, resource reservation,
non-runtime execution, public run facade, or unrelated baseline repair changes.
The integrator owns this decision and independent review. Implementation is on
`fix/session-capacity-observability-2026-10-02`, separate from fixture PR #7.

**Gate:** Direct keep-alive lifecycle tests preserve victim selection, all three
affinity cases, explicit target cleanup, rollback and original unload failure;
available and unavailable diagnostics are tested. Ledger tests accept valid
session-only events, reject missing/blank identity, preserve old run-event
contracts, and demonstrate no fabricated run projection. The non-runtime no-load
regression and focused runtime-dispatch tests remain green. Full CI gaps remain
explicit; independent review precedes integration.

### M5 — Integrated acceptance and maintainer handoff

**Goal:** One honest, current product and compliance result.

**Allowed write set:** this plan's files and exact canonical guide paths declared
at M4 closure; source fixes return to a bounded M2/M4 slice.

**Tasks:** Reconcile complete coverage, all old/new findings, supported consumers
and required target claims. Review changed architecture independently from its
implementer. Run the selected integrated checks and real desktop workflow on
the final material revision; update architecture/consumer/run guidance with
actual behavior. Explain remaining limits without labeling them compliant.

**Gate:** DA-01–DA-07 satisfied, no undispositioned area or required violation,
no unavailable evidence represented as passing, and no competing plan authority.

**Status:** `Planned`

## Concurrent Work

### First Pilot Admission (P01/P02)

**Status:** `Accepted` for these two scoped repairs; see ledger for evidence.
This does not accept the complete M4 milestone or establish a cost winner.

Source-backed review establishes these two independent M4 prerequisite repairs
can proceed while M2's execution design is resolved. This changes milestone
ordering only; it does not permit real model execution before trust closure.
The integrator owns current contracts and plan writes; workers deliver code,
not outstanding proposals to mutate shared authority. No conflicting write sets
or stale admission facts have been identified for this pair.

| Slice | Model | Contract and exact allowed write set | Acceptance |
| --- | --- | --- | --- |
| P01: D-01 GUI smoke temporary-root cleanup | Luna max | `scripts/check-workflow-editor-image-generation-gui-smoke.sh`; `scripts/check-workflow-editor-image-generation-gui-smoke.test.mjs`. Keep shell lifecycle ownership through direct child completion, preserve status and remove only its own allocated root. Preserve existing INT/TERM delivery addressed to wrapper PID, wait/reap child before cleanup and return 130/143. Whole driver/app process-tree supervision is not newly promised. | Copied real wrapper in isolated fixture with fake wdio verifies child input/root, success/nonzero exit and cleanup; ready-marker INT/TERM tests assert child receives signal while root exists, terminates before cleanup and leaves no direct child; `bash -n`; independent medium review. This is wrapper contract evidence, not GUI/model acceptance. |
| P02: COV-01 frontend test discovery | Astra low | `package.json`; only if Node's built-in discovery is insufficient, `scripts/run-frontend-tests.mjs` and `scripts/run-frontend-tests.test.mjs`. Discover `.test.ts` under maintained `src/` and `packages/` roots, including new nested files; exclude dependencies/generated output; preserve test failures. Prefer existing Node glob support over a custom runner. | All 90 currently tracked frontend files selected plus a temporary nested regression; intentional failure propagates; excluded-root fixture not selected; complete discovered suite run and independent medium review. Newly exposed failures are reported and assigned, never hidden to make this patch pass. |

These are different tasks with unequal possible repair effort. Their results
start the four-task pilot but cannot establish a causal model ranking. P03/P04
remain unassigned until comparable admitted tasks are available; D-02 large-image
preview and COV-02 packaging are findings, not automatic new write authority.
Per-agent billing is unavailable from current tools; first-pass quality and
repair rounds are observable. Total agent wall time was not instrumented for
this pair; measured test runtime is not model latency. No dollar saving is claimed.

The composition remains the existing shell launcher and Node test runner;
P01 restores an existing resource owner, P02 removes a duplicate manual test
inventory. No new domain owner, public contract or persisted schema is admitted.
Any proposed custom runner must justify why the built-in mechanism cannot own
discovery; it must not become a new test registry.

Use one integrator, Astra medium analysis/review workers, and
implementation workers selected by the policy below. The restarted session exposes ten subagent slots;
that is capacity, not a utilization target. Start with the four bounded review
lanes below and add workers only for independently useful admitted tasks:

| Owner | Primary write set | Adjacent write set | Forbidden/shared | Output | Integration order |
| --- | --- | --- | --- | --- | --- |
| Execution reviewer | `reports/execution-review.md` | none | Product and plan-control files | Run/scheduler/graph ownership and failures | First for M2 |
| Runtime reviewer | `reports/runtime-review.md` | none | Product and plan-control files | Inference, Pumas, trust, worker lifecycle | First for M2 |
| Desktop reviewer | `reports/desktop-review.md` | none | Product and plan-control files | UI/graph package, IPC, artifacts, accessibility | For M3 |
| Coverage reviewer | `reports/repository-review.md` | none | Product and plan-control files | Bindings, tooling, dependencies, docs, remaining coverage | M1/M4 |
| Integrator | Plan-control files and admitted slice | Explicitly declared consumer files | Protected user changes | Current decisions, integrated code/evidence | Serial |

Each delegated task supplies: model/effort, bounded question, relevant standards
routes, authoritative contracts, exact write set, prerequisites, acceptance
command/claim, forbidden shared paths and stop condition. Review reports cite
source locations and uncertainties; they do not authorize a competing redesign.

Implementation parallelism is admitted only after contracts and disjoint writes
are established. One owner edits shared contracts, manifests, composition roots
and plan controls. Reviewers may work alongside that implementation. If multiple
outstanding implementation proposals can become stale before integration, first
route and apply the standards' concurrent-plan-integration profile and record
its required revision/reconciliation facts. Otherwise keep serial integration;
agent count alone does not justify new coordination machinery. Reserve real
model/GPU evidence runs to avoid contention and misleading measurements.

### COV-02 Spark Admission

Under the current four-model policy, Spark owns exactly four literal replacements
of `docs/headless-native-bindings.md` with `docs/headless-workflow.md` in
`scripts/package-uniffi-csharp-artifacts.sh`. Preserve all other bytes and behavior.
The latter is the existing live guide and agrees with the package README. Medium
confirmed the route at `snapshot:v1:1356b67e-26ae-4c5b-9e65-6e3e8f9ef58e`:
Core/Router, implementation/verification/build/documentation/library, zero
unresolved facts. Verify the actual script in an isolated temporary fixture with
stub native build/generation, real archive/checksum tools, and both archived
manifests resolving to the exact guide bytes. No real binding build/release claim.
Independent medium review may be combined with EX-04 review. Root owns plan,
issues and ledger; no other packaging or documentation source changes admitted.

### Cost And Quality Controls

- Route delegated work using the four-model policy above with explicit model
  and supported effort settings and bounded context. Spark begins on real small
  changes; its quality and complete accepted-change cost remain under evaluation.
  Do not silently replace an unavailable requested model or expand a small repair
  into consequential design to fill a model lane.
- A medium analysis task stops once the owner, interface, invariants, relevant
  consumer migrations, write set and acceptance evidence are sufficiently clear
  for reversible implementation. Reuse its findings across dependent slices;
  do not commission duplicate broad investigations.
- An implementation task receives that contract and implements the complete
  coherent slice, including focused tests and affected checks. It may make
  ordinary local choices but must not silently change ownership, compatibility,
  trust, lifecycle or acceptance semantics to make the patch pass.
- If implementation exposes a material design contradiction, return the exact
  evidence and decision to a medium analysis task. Do not repeat blind repair
  attempts or automatically raise all implementation work to medium. After the
  decision is resolved, hand the bounded implementation back to the selected
  implementation model; record any rescue by a different model in pilot evidence.
- Keep root orchestration context and output bounded: avoid dumping tool schemas
  or whole reports, batch independent reads/checks, and delegate routine check
  execution with one consolidated result packet where independent integration
  evidence permits. Preserve medium decision/review gates; reducing their
  necessary coverage is not the cost optimization. RT-03 showed root overhead
  dominates implementation cost.
- Medium review examines the resulting diff against the admitted design and
  applicable standards. Review coherent integrated changes together where safe;
  avoid separate full reviews per file or trivial edit. Re-review only material
  changes and unresolved findings, not unchanged accepted work.
- Run focused checks during implementation and broader evidence at the integration
  points that require it. Neither low reasoning nor cost reduction changes the
  acceptance criteria or permits substituting simulated success for real model runs.
- Prioritize API dollars per accepted change over elapsed time and raw tokens.
  Price recorded uncached input, cache reads/writes and output at verified
  model/service-tier/context rates; count reasoning within output once. Include
  attributable design/review/repair/rescue and show shared overhead separately.
  Distinguish API-equivalent estimates from actual invoices; never infer a cost
  winner from fewer tokens alone. Use existing usage logs and ledger rather
  than building a tracking framework. Use fewer workers when decisions, files
  or hardware are shared.

### Bounded Implementation Model Pilot

**Status:** `Complete` (P01–P04). The ledger records quality, usage and the
verified API-price correction. Luna max has lower observed API-equivalent
implementation cost and lower attributable design/review subtotals in this
unmatched sample; prefer it for qualified bounded repairs. Astra low remains
provisional for consequential classes beyond the pilot, on quality grounds,
not a demonstrated cost advantage. Shared overhead and actual billing remain
unallocated. Continue useful product-task evaluations under the user’s latest instruction;
record quality gates and complete attributable costs without artificial benchmark tasks.

The decision is whether Luna max can lower the total cost of accepted changes
for particular task classes without weakening design or correctness. Official
model positioning and reasoning labels do not establish Pantograph performance.

- After M1 provides admitted production slices, select four useful, reversible
  implementation tasks, two per model, with comparable domain complexity and
  evidence requirements. Include both a local behavior repair and a bounded
  multi-file contract-consumer change per model where available. Do not compare
  trivial Luna edits against difficult Astra redesigns or split coherent work
  artificially to fill the sample. Keep unresolved security, persistence and
  lifecycle design with Astra medium analysis; initially use Astra low for
  consequential implementation outside the pilot's tested scope.
- Record task class/difficulty before assignment. Give both models equally
  explicit contracts, relevant source context, write sets and preselected
  acceptance tests; preserve initial results before repair. Use disjoint tasks
  for useful production progress, not duplicate implementations by default.
- Astra medium reviews both against the same behavior, ownership, failure,
  lifecycle, maintainability and standards criteria. Omit model attribution
  from review packets where practical. Tests are run independently by the
  integrator as required; self-reported success and tests that merely mirror
  implementation do not decide quality.
- Keep one compact table in the existing ledger: task/model/effort, first-pass
  gate results, substantive findings by severity, repair rounds or rescue,
  implementation/review/repair usage when exposed, elapsed time, and final
  acceptance. Record later discovered regressions against their originating
  task. No new benchmark framework or separate recurring report.
- Compare total implementation + review + repair + rescue cost per accepted
  task, including failed attempts. Use actual billed cost when available;
  otherwise distinguish token/latency proxies and API-price estimates from
  session billing. Missing usage means cost is unknown, not proven lower.
- Stop after these four tasks and select a provisional routing policy by task
  class. A small unmatched sample is operational evidence, not a causal model
  ranking. Expand only if one named uncertainty could change the selection and
  a further useful task can resolve it cheaply. Do not delay product progress
  for a statistically strong benchmark.
- Adopt Luna for a tested class only when required gates pass, review reveals
  no unresolved material defects and observed total effort/cost supports it.
  Retain Astra low for classes where Luna needs substantial rescue or design
  repair. One material trust/data/lifecycle violation stops expansion into that
  class pending review; passing local tasks does not establish suitability for
  the scheduler or cross-process architecture. Keep medium review and the same
  acceptance gates for either implementation model.

## Blockers

- No blocker to M0 documentation/read-only work.
- Real mixed-workflow model/runtime/hardware and desktop-runner availability
  have not been verified in this planning pass; M0 owns the check.
- Full consumer/target inventory and project licensing authority remain to be
  reconciled; unresolved external authority blocks only dependent claims.
- Exact production write sets are intentionally not admitted before source-backed
  findings. Later milestones cannot use candidate owner lists as write permission.

## Re-Plan Triggers

Change the current decision when the user selects a different modality contract;
source evidence contradicts an owner/ADR; a real external consumer or persisted
promise changes migration needs; a missing Pumas contract changes sequencing;
required execution evidence is unavailable; or new machinery/change propagation
invalidates the composed-design review. Re-run only affected review and gates.

## Implementation Invocation

`Continue docs/plans/domain-architecture-and-multimodal/plan.md, operation continue.
Admit D-02's complete retained-image retrieval repair from the existing desktop
report after confirming its exact consumer write set; preserve artifact identity,
retention and cancellation semantics. Then qualify concrete owner-fresh Pumas
text/image targets, runtime/device and the existing desktop fixture before real
mixed-workflow execution. EX-04's controlled text→image, pending-response and
readiness-recovery evidence is accepted, not a real-model/desktop substitute.
Continue the four-model policy and record full implementation, review, rescue,
commit/coordination and unknown-rate costs. Full audit/static and existing test
failures remain tracked independently.

Subsequent invocations supply this same canonical plan path with the operation
appropriate to its recorded lifecycle; the next-slice field is not independent
execution authority. The user has now authorized starting the plan with subagents
and the bounded implementation pilot.

## UniFFI Validation Publication Bridge (2026-10-03)

Hosted headless qualification now reaches the UniFFI runtime tests. The FFI
constructor owns an ephemeral attribution store, and execution correctly requires
an owner-published executable validation snapshot. The embedding API exposes
edit sessions but omits validation refresh and snapshot publication, so a valid
text graph cannot currently complete the public create/run flow.

Accepted bounded repair: expose typed EmbeddedRuntime delegates and strict JSON
UniFFI entrypoints for the existing workflow-service current-validation refresh
and graph-session snapshot publication methods. Keep graph/proof construction,
revision checks, descriptor compatibility and dependency-proof freshness at their
existing owners. A client supplies session/version identifiers, never trusted
snapshot content. It loads/saves a graph, creates an edit session, refreshes the
returned graph revision, publishes its current executable validation session,
then creates/runs an execution session. Reopening the ephemeral FFI runtime
requires publication again. Missing, stale and unavailable validation still fail
closed; runtime inference facts and readiness are not fabricated.

Required evidence: actual public JSON text execution; missing/stale/reopened
runtime rejection; existing error-envelope fidelity; UniFFI metadata and generated
C# surface checks. Independent read-only review is required before publication.
Canonical selector fixtures and managed-runtime readiness evidence stay separate.

Independent review found that inference-only snapshots rejected every legitimate
zero-inference graph and that real C# callers had not been migrated. The accepted
correction keeps publication mandatory and checks exact canonical inference-node
coverage at owner publication and executable projection, including imported
records. Empty coverage is valid only for a graph with no inference nodes. Missing,
extra and duplicate runtime projections fail closed; per-node proofs and existing
freshness checks remain. The canonical inference request owner supplies the shared
classification; unsupported inference-bearing task types remain rejected. Both
NativeSmoke and the packaged DirectRuntimeQuickstart must perform the real public
load/edit/refresh/publish lifecycle before execution.

## Final Acceptance

- Acceptance status: `blocked`
- Deferred follow-ups: new model families and live generated-UI execution outside the selected product contract; release publication.
- Final status: `Active`

---

## Part B — Plan: Isolated Pumas library RPC and Pantograph-owned inference workflows (RPC branch revision 2)

**Plan status:** `Active` — continuation; revision 2 changes planning authority, not implemented source.

**Current phase:** R1 / RPC-01 — intent-first Pumas 0.7.0 consumer migration; typed intent availability/handle adapters, bounded hydration and re-observation, explicit desktop RPC operation routing, and runtime identity handoff are implemented and under focused verification.

**Next slice:** `RPC-01` — qualify one retained producer-selected nonempty model through the released intent `get`/handle path in real runtime-host acceptance, then run the dependent image/embedding, scheduler fan-out, desktop and cold-reopen gates. Tiny Aya remains qualified for text only; Tiny SD still has no producer-issued ID, Qwen embedding remains task-facts gated, and Z-Image retains missing text-encoder shard diagnostics and an unadmitted `ZImagePipeline`.

**Acceptance status:** `blocked` — required live artifact/model/desktop evidence is not established here.

**Decision owner:** Puma / MrScripty. **Integration owner:** the Codex primary session, requested GPT-6 Luna Max.

**Execution ledger:** [execution-ledger.md](execution-ledger.md). **Issues:** [issues.md](issues.md).

**Reports:** [audit](reports/2026-09-25-standards-and-rpc-audit.md), [library contract and baseline inventory](reports/2026-09-25-pumas-contract-inventory.md), [acceptance procedures](reports/2026-09-25-end-to-end-acceptance.md).

## Objective

A user can connect Pantograph to an independently retained Pumas Library RPC artifact, discover and select appropriate library models/assets, author/save/run workflows, and retrieve complete, correctly attributed text, images and embeddings. Every inference request executes through **Pantograph's existing unified inference-backend API** and its backend implementations. The desktop must demonstrate all three task families, actual generated text feeding image and embedding tasks, and complete retained outputs after a policy-qualified cold reopen.

Pumas supplies model-library facts, resources, requirements, saved model settings and explicitly authorized acquisition/retention. Pantograph owns workflow semantics, scheduling, resource admission, backend selection, inference runtime/model loading and unloading, execution, cancellation, artifacts and presentation. Pumas is never an inference provider for Pantograph.

Finish the inherited required standards/remediation objective too. Report product functionality and whole-repository compliance separately until both have matching evidence. This planning package certifies neither.

## Authority, history and inspected identities

Keep `docs/plans/domain-architecture-and-multimodal/plan.md` as the sole active sequencing authority. Use operation `continue` after checking actual local lifecycle and reconciling this revision. The older remediation and image plans remain Superseded; accepted documentation consolidation and scoped EX/RT/tooling repairs remain historical evidence. Keep original issue IDs, the [prior-claim mapping](reports/repository-review.md), reports and execution ledger. Do not rewrite terminal historical plans to satisfy newer templates.

Inspected Pantograph: `56caed029a48db66212a953a1677a94449396d52`. Pumas release 0.7.0: `29242fce4ec9043becaeb061587f364ecc4e7177`; upstream main: `51301bd317d7962b51d534590a685b676de835d9`. Current inspected Coding-Standards: `8fef41d18c524c7ac6a1a16f242cc3f549c791a2`, Engine interface 37. These are evidence references, not an exact-HEAD requirement for every Pantograph edit.

Revision 2 supersedes the previous package's permission to delegate inference to Pumas and its use of the active local Pumas checkout/service as a target. It also replaces historical model-routing assignments with the role policy below. Accepted code evidence is retained only within its original scope; affected behavior needs current regression evidence.

## Reconciliation of work after the inspected baseline

The checkout contains committed work after the package's inspected Pantograph revision. Preserve it as history and evidence, but requalify it against revision 2 before treating it as active authority:

- Commit `4938e405` selected a shared Pumas owner/client integration and recorded owner-dependent consumer design. Its source and review evidence remain in the existing ledger and commit history; revision 2 withdraws any active-Pumas or Pumas-inference permission and requires the independent RPC-00 baseline first.
- The accepted D-02 complete-image Inspector repair and browser evidence remain valid within their recorded scope. They do not establish real-model, Pumas, embedding, cold-reopen or Pantograph-only inference acceptance; R3 must recheck the affected boundary where necessary.
- The newer issues and ledger entries are preserved below. They are not overwritten by this update, and their unresolved blockers are carried into RPC-00/R1 rather than silently closed.


## Current Coding-Standards MCP requirement

Use the actual Coding-Standards MCP for planning, implementation, verification and commits. Check its installation/runtime interface, accepted guidance identity and purpose-qualified availability; retrieve Core, route real task facts, and follow the required closure. An online MCP or source checkout is not itself evidence that the selected guidance is current. Repair setup/drift through the supported mechanism; do not manufacture authority through authoring access or use unqualified prose as a substitute.

Record one relevant standards baseline and route per admitted slice. Reconcile material changes at integration boundaries. Missing applicable guidance blocks its dependent write/commit decision while bounded read-only discovery and independently qualified work continue. These planning files summarize project decisions, not a replacement standards corpus. The live authoring MCP was executed for RPC-00/RPC-01 transport admission; final standards/commit closure remains pending.

### Current route refresh — 2026-09-26

The current authoring runtime was rechecked before continuing: interface 37, implementation `0.2.0`, instance `6a638e80-40ef-49d7-82ec-a06a947c8f62`, implementation digest `sha256:f7bd0d1d7cb03bc4ae2eb10aba6d840cbad9ea548c2c8a0b17958314a564c4cb`, catalog digest `sha256:3b985fc1bf612c533ab735962f82a27e081deba0ab09fd05c98e5904c59a032f`, schema digest `sha256:32be44913a5e10913bed5d08ffc0dd4961afb5c4e9bd56514e5773f95a0992f5`, purpose `authoring`, and `installation_state=restart-required` with action `restart-and-reconnect`.

The fresh route snapshot is `snapshot:v1:46d05785-182b-4696-87d6-e739de40925f`. It selected 41 standards with zero unresolved fact categories for the current library/Rust/TypeScript/generated-contract/IPC/interop/persistence implementation, verification, documentation, planning and commit work. Core, Router and every selected target were read through the same snapshot, including the current commit guidance. This route is the current standards evidence for the continuation; the earlier 47-standard route remains historical. Because the runtime still requires restart/reconnect and no such operation is available in the current tool surface, standards-dependent commit closure remains gated.

## Scope and constraints

In scope: external library RPC, consumed contract/dependency migration, isolated reference artifact, library discovery and freshness, model details/requirements/settings, asset/load-target resolution, Pantograph backend/preflight/scheduler paths, all three inference families, complete artifacts, GUI/headless consumers, lifecycle and inherited required standards obligations.

Out of scope: Pumas inference or serving control, Pumas private provider protocols, Whip-Docs integration, a code parser, another inference framework, a second scheduler/catalog, remote/LAN exposure, arbitrary model-family support, automatic model-code trust, upstream source changes and external publication.

### Mandatory execution boundary

Use only library/resource operations on the Pumas client. Do not call Pumas `/v1` routes (including served-model discovery), inference RPC, provider-private endpoints, profile/serving start or Pumas-owned inference through any proxy. This prohibition includes optional paths, compatibility fallbacks and demonstrations. Existing non-Pumas backends may keep their own compatible-provider protocols under Pantograph's support contract; do not globally ban an endpoint path used by unrelated legitimate providers.

All text/image/embedding work uses the existing unified Pantograph inference API. Preserve one canonical runtime route and its typed unsupported outcomes. Pumas metadata does not prove that a Pantograph adapter supports a model, grant custom-code permission or authorize a runtime launch. Asset resolution is not inference. Keep model/library/artifact identity distinct from Pantograph runtime/backend and task identity.

HTTP JSON-RPC `/rpc` and core framed IPC are different contracts. Select the exact supported library operation/transport before implementing it. RPC loss does not authorize claiming the library root, directly opening its database, guessing storage paths, substituting another instance/model, or invoking inference through Pumas.

### Immutable integration fixture, not the active development checkout

Before baseline selection, inspect current upstream HEAD and the relevant release-to-head delta. The initial candidate is the host-appropriate 0.7.0 headless artifact; its exact metadata and provisional rationale are in the inventory. GitHub labels it prerelease and not immutable: retain verified bytes by digest and resolve its source commit rather than trusting a moving tag/name.

Use a newer artifact only for a demonstrated required library contract, correctness, security or compatibility reason. If no suitable remote artifact exists, build an exact upstream commit in an independent checkout and separate build target with controlled dependencies. Retain its binary, hash, source, feature/toolchain and lock/build provenance. Do not use a worktree linked to the active local Pumas repository, its target directory, mutable path override or development service. No unrecorded switch to new upstream bytes mid-slice or acceptance run.

Use an explicit independent endpoint, process owner, library root/database, configuration, registry/cache and fixture lifecycle. Preserve shared/user data and production services. Provision models through supported library operations and qualified asset policies; do not attach an older candidate to a newer live writable model database. A missing required operation changes the recorded candidate decision or creates an upstream blocker; it never authorizes an inference fallback.

### Dependency and execution authority

Necessary dependencies may be installed and repository code run under this goal. Added dependencies require a concrete consumer need, owned source/version/lock/provenance, alternatives and trust/license consideration, lifecycle and relevant tests. Prefer existing mechanisms and isolated environments. This permission does not grant arbitrary global upgrades, new paid services/credentials, large downloads or destructive/shared-system changes without their actual authority.

Retain ownership of async work, inference workers and results through the applicable terminal observation. Elapsed time or silence alone is not failure. Keep connection setup, library reads, Pantograph task duration, cancellation request and confirmed stop distinct. Do not copy Pumas serving lifetime or input limits into Pantograph's backend contracts or replay uncertain inference automatically.

## Binding decisions

| Decision | Owner | Selection and supersession |
| --- | --- | --- |
| Inference | Pantograph unified inference API and backend owners | All T/I/E execution remains in Pantograph; any earlier conditional Pumas-inference path is withdrawn. |
| Pumas responsibility | Pumas library service; Pantograph library client | Library/resources only; validate supported RPC facts and resolve assets without duplicating library authority. |
| Baseline | Integration owner with Astra planning advice | Compare upstream before choosing; initially evaluate pinned 0.7.0 headless bytes; select newer only for a documented relevant need. |
| Isolation | Fixture/dependency owner | Independent retained artifact, service and writable state; active Pumas development cannot become a build/runtime dependency. |
| Standards | Current Coding-Standards MCP | Actual routed and available guidance governs all planning, implementation and commits. |
| Coordination | Luna Max integration role | Use the role policy below and maximize independent concurrency; shared authorities have one writer. |
| Completeness | Product and verification owners | All three real desktop modalities, dependency flow and retained outputs plus inherited required claims; no selector-only or provider-only substitute. |

## Objective acceptance

The new integrated candidate has no satisfied product/workflow acceptance row yet. RPC-A00 has a qualified artifact/isolated-baseline result; the report defines the remaining procedures and records evidence when obtained.

| ID | Observable criterion | Kind | Environment | Mode | Status |
| --- | --- | --- | --- | --- | --- |
| RPC-A00 | Chosen artifact/contract/build is pinned and reproduced independently of the active Pumas development checkout, service and mutable state. | release-artifact | representative | automated | qualified for retained 0.7.0 bytes and isolated root |
| RPC-A01 | A separately running qualified Pumas artifact supplies the actual Pantograph library facts/assets path; Pantograph never owns its root/database. | system | required-real | automated | partial: producer-supplied Tiny Aya selected identity and `ready/ready` directory target now execute through Pantograph's selected-text path; Z-Image also supplies a target but is incompatible, while complete Tiny SD remains identity-gated |
| RPC-A02 | Every consumed library request/result/event preserves producer semantics, outcome, correlation, identity, version and extra-field policy. | contract | representative | automated | partial: positive file target, synthetic and producer-derived directory targets, authenticated IPC selector/load-target/facts, Tiny Aya selected identity and Pantograph target projection, and negative outcomes observed; HTTP selector snapshot remains `-32601/not_found`, while Tiny SD has no producer identity and Z-Image retains missing-shard diagnostics |
| RPC-A03 | Library outage, replacement, cursor gaps and stale selection retain truthful refresh/failure behavior without cross-instance state or replay of uncertain mutations. | integration | representative | automated | partial: same-root producer restart requalified the target, and a stopped producer yielded a typed client transport error without stale replay; cursor gaps, cross-instance replacement and stale-selection recovery remain open |
| INF-A01 | All actual T/I/E executions traverse Pantograph's unified inference API; observed Pumas traffic contains only admitted library/resource operations and zero inference/serving calls. | system | required-real | automated | partial: producer-qualified Tiny Aya text executed through Pantograph after library-only RPC reads; real producer-selected image/embedding execution and complete three-family observation remain open |
| DA-01 | Inherited maintained areas have current applicable-standards review and evidenced dispositions; required violations remain open until resolved. | focused | not-applicable | either | pending |
| DA-02 | Integration has one owner per concern; current composition/change/deletion evidence and real consumer/migration dispositions agree. | integration | representative | either | pending |
| DA-03 | Real desktop produces complete text, decodable images and meaningful embeddings; generated text actually feeds both downstream task families with correct identity. | user-workflow | required-real | automated | blocked |
| DA-04 | Missing facts, invalid data, worker failure, cancellation, disconnect and shutdown preserve accurate terminal/unknown results, resource ownership and unrelated work. | system | representative | automated | pending |
| DA-05 | Saved graphs and complete retained outputs survive the chosen cold-reopen/version/retention procedure; affected IPC/native/host consumers agree. | contract | representative | automated | partial: controlled fresh-process retained-output reopen; desktop/saved-graph scope open |
| DA-06 | Qualified selector/dispatch/reuse measurements distinguish inference cost from orchestration and satisfy recorded workload-specific targets. | system | required-real | automated | blocked |
| DA-07 | Required static/test/security/accessibility/dependency/target/artifact obligations and independent reviews pass; actual MCP-guided process/commit evidence is retained. | system | required-real | either | pending |

DA-03 procedures T/I/E/C are all mandatory. INF-A01 needs both construction/source-route review and observed actual execution; a grep result or an idle Pumas fixture is not sufficient. DA-07 aggregates references and retains the separate original evidence kinds; it cannot downgrade a required artifact/platform claim. Explicit maintainer scope decisions must remain honest, not reclassify missing required evidence as a pass.

## Simplicity and ownership review

**Composed-design review applicability:** `applicable`. These eight answers select the direction; RPC-00 completes the actual consumed-wire and fixture facts before implementation.

1. **Independent concerns:** Pumas owns model-library information/assets/acquisition; Pantograph owns inference backends and workflow execution; the integration client owns library transport projection; the fixture owner owns the frozen dependency's build and process. None owns the others' policy.
2. **State, identity, value, time, policy and mechanism:** separate library/model/artifact revision, wire version, artifact digest, service/root identity, observation cursor, Pantograph runtime/backend/task/attempt, retained result and standards revision. A new Pumas build prompts explicit client qualification, not a new inference engine or rewritten historical result.
3. **Caller and composition knowledge:** the composition root supplies an explicit library endpoint/client lifetime and existing inference runtime. Callers receive validated library facts or Pantograph execution outcomes. They do not know Pumas's registry paths, internal providers, framing, storage layout or build directory.
4. **Representative changes:** library-wire changes affect the client/mapping/tests; inference changes affect Pantograph backends/contracts; model metadata changes trigger existing freshness handling; artifact candidate upgrades affect dependency qualification; presentation changes stay out of execution ownership.
5. **Stable interfaces and hidden knowledge:** stable model references, library result contracts and the existing unified inference API remain distinct. Hide transport and local fixture mechanics. Any retained Rust dependency is commit-pinned and does not grant root ownership or require mutable source adjacency.
6. **Independent evolution, failure and verification:** Pumas development can proceed or disappear while the retained fixture works. Library outage remains an explicit input/readiness outcome; inference availability is separately established through Pantograph. Qualify artifact, adapter, backend and desktop boundaries separately and then together.
7. **Deletion result:** removing the library client would spread transport/version/outcome knowledge into callers; removing a Pumas inference adapter removes forbidden complexity entirely, so none is introduced. Prefer a small ordinary inventory/build record over a new version registry, audit framework or second model catalog.
8. **Necessary and cumulative complexity:** independent deployment, model trust, async lifecycle and retained multimodal values are inherent. Existing owners contain them. A retained binary plus isolated state stabilizes the external boundary without coupling the active upstream checkout or expanding inference authority.

## Milestones and admission

R0 has the exact documentation write set, and the reviewed RPC-01 transport-only slice has its exact source/dependency write set recorded below. Later source sets are selected from real callers when dependencies settle, within this authorized goal; normal refinement does not require approval per file. Independent R2/R3/R4 work with settled contracts may be admitted alongside R1, so the sequence is a dependency order, not a universal serialization barrier.

| Milestone | Goal / preserved contract | Allowed writes now | Gate | State |
| --- | --- | --- | --- | --- |
| R0 / RPC-00 | Route current standards, compare upstream/release, qualify a frozen library artifact, inventory actual operations/callers and native backend tuples, admit the next production slice. | Exact documentation set below; read-only source; controlled isolated dependency provisioning/build/probes are authorized outside application source. | Current MCP route; candidate rationale/digests; actual library observations or precise blockers; bounded consumer dispositions and coherent next source/test write set. | Complete for baseline; external identity/nonempty-library gates remain open |
| R1 / RPC-01 | Library-only RPC from explicit configuration through selector, dispatch/preflight and executable asset resolution; preserve current unified inference API. | Reviewed transport plus the explicit `PumasSelectorAccess::Rpc` projection/configuration and the enumerated dispatch/runtime-host/technical-fit consumer seams; no inference implementation or owner-root fallback. | RPC-A00/01/02 and a useful canonical Pantograph inference path; no forbidden Pumas calls. | In progress: RPC capability seam admitted; artifact identity and real execution remain gated |
| R2 / MOD-01 | Complete Pantograph-owned text/image/embedding routes, input/output contracts and dependent execution. | Admitted embedding runtime-host projection, gateway handoff, typed vector contracts and focused mapping tests; remaining source writes are selected from settled callers. | Real backend T/I/E and C, INF-A01 and complete artifacts. | In progress: embedding source seam admitted; real execution, fan-out and retained-output evidence remain gated |
| R3 / UI-01 | Complete user selection/author/save/run/inspect, D-02 repair, embedding presentation and cold reopen. | None until exact relevant files are selected. | Real desktop DA-03/05 and affected lifecycle/accessibility checks. | Planned |
| R4 / QUAL-01 | Close inherited current-standards/lifecycle/consumer findings and measured efficiency while preserving the usable path. | None until each coherent finding family is admitted. | Required DA-01/02/04/05/06/07 dispositions and evidence. | Planned |
| R5 / ACCEPT-01 | Independent final material review, integrated acceptance, current inventory and coherent commits. | Current plan/evidence and exact affected guides; source repairs return to their owning slice. | All required rows satisfied, retained artifact identities match, reviews complete, history/resources appropriately accounted for. | Planned |

### MOD-01 source slice admitted alongside RPC-01

The next independent source slice now exists for Pantograph-owned embeddings: runtime-host projection, gateway-owned llama.cpp execution, bounded typed vectors, scheduler result/input mapping, workflow JSON projection, and generated-text-to-embedding input coverage. It deliberately preserves the current R1 phase and RPC-01 next slice because executable artifact identity and real execution remain gated. The batch route is semantically complete per member but sequential pending native multi-input qualification; this is implementation progress, not product acceptance.

The source slice now also has a controlled scheduler fan-out regression: one generated text result is consumed by both image and embedding downstream members through the existing runtime-host batch handoff, and text/image/embedding terminal values are retained. The fixture uses a controlled runtime-host port and therefore proves scheduler dependency, correlation, projection and retention behavior only; it is not real backend, desktop or DA-03 evidence. Its parent test now launches a separate test process to reopen the persisted run and all three output bodies, advancing cold-reopen evidence without closing the saved-graph/desktop claim.

That same fixture now drops and recreates file-backed diagnostics and artifact-store services, then queries the persisted completed run and reads the retained text, image-reference and embedding bodies by their projected artifact identities. This is controlled same-process service/store reopen evidence for serialization and projection continuity; it is not a fresh-process saved-graph/desktop cold-reopen result and does not close DA-05.

The RPC baseline was also requalified against a disposable Pumas source clone pinned at `96f859443460ad8e4799d563528aaba113ccff21`, built with `--no-default-features` so Pumas inference plugins were absent. That candidate imported a real Granite embedding GGUF and returned complete library facts, but its HTTP surface still supplied no selected-artifact identity and rejected selector snapshots. The Pantograph RPC seam therefore retains its explicit typed `needs_detail`/fail-closed behavior; this evidence advances the external qualification boundary but does not permit inference, filename guessing or product acceptance.

Direct Pantograph gateway qualification now covers generated text, embeddings and a real local Tiny SD Turbo image through the existing unified API. The text run returned `"Amber Bridge"`, embedding returned two 384-dimensional nonzero vectors, and image returned one 256×256 PNG. These are backend/gateway proofs using explicitly copied local assets, not the missing RPC-selected image target, desktop workflow, or full DA-03 acceptance. The image worker's first-use CUDA metrics failure was repaired with device-context ownership; the focused worker suite is 8/8. Final standards state, independent reviews and coherent commits remain open.

### Producer-qualified Tiny Aya text execution — 2026-09-26

The pinned current-source no-inference Pumas binary (`96f859443460ad8e4799d563528aaba113ccff21`, `--no-default-features`, SHA-256 `3163f4d89d535870cb4ba2549adcbf47af435685576379a862e7ba422badf27c`) indexed a path-normalized Tiny Aya fixture in `/tmp/pantograph-rpc-tinyaya-20260926i`. Library-only `get_models`, package-facts and owner-fresh load-target calls returned model `llm/cohere2/coherelabs--tiny-aya-water__full_repo`, selected artifact `coherelabs--tiny-aya-water__files_b673ab802c36`, contract-3 `hf_compatible_directory` facts, `Cohere2ForCausalLM`, `requires_custom_code=false`, `library_owned`, `valid`, and a `ready/ready` absolute directory target.

The ignored repository acceptance `runtime_host_execution_port::tests::real_rpc_selected_tiny_aya_text_executes_through_runtime_host_port` consumed that exact target through `EmbeddedRuntimeHostExecutionPort`. Its typed Pumas resolvers normalized the `pumas://models/` URI for the library operation, reconciled the producer's omitted revision, and stripped only the owner-local absolute identity path while preserving the selected artifact ID and executable `local_load_path`. Pantograph's PyTorch/Transformers worker loaded both shards with `trust_remote_code=false` and `local_files_only=true`, then returned generated text `"Amber Bridge "` under an explicitly authored eight-token task setting. No Pumas inference, serving, `/v1`, provider or profile operation was called. This closes producer-qualified Pantograph text execution only; image/embedding producer targets, real three-family fan-out, desktop acceptance and cold saved-graph evidence remain open.

### Producer-selected Qwen embedding target remains facts-gated — 2026-09-26

A fresh isolated current-source producer root `/tmp/pantograph-rpc-qwen-embed-20260926a` indexed the retained producer-selected Qwen3 Q4_K_M GGUF. Library-only `get_models`, package facts and owner-fresh load-target calls returned model `embedding/qwen3/qwen--qwen3-embedding-8b-gguf__q4_k_m`, selected artifact `qwen--qwen3-embedding-8b-gguf__q4_k_m`, valid `gguf`/`library_owned` facts, `requires_custom_code=false`, `artifact_state=ready`, `entry_path_state=ready`, and a file target. The exact GGUF SHA-256 is `3fcd3febec8b3fd64435204db75bf0dd73b91e8d0661e0331acfe7e7c3120b85`.

The producer facts declare `task_type_primary=unknown`, so Pantograph's real runtime-host embedding acceptance fails closed with an explicit unsupported package-task diagnostic before any llama.cpp load or inference. It does not infer embedding from the model path or task intent. This is stronger producer-selected embedding boundary evidence, but not successful embedding execution; the producer must supply canonical embedding task facts before RPC-backed embedding can be admitted. No Pumas inference, serving, `/v1`, provider or profile operation was called.

### Runtime-host identity and cancellation hardening — 2026-09-26

The image handoff and planner now require one nonblank producer-selected artifact ID and exact agreement across scheduler selection, package facts and the Pumas load target. The runtime-host Pumas target and package-facts resolvers also reject ready/facts responses whose producer model or conflicting nonempty revision differs from the scheduler selection before any Pantograph identity normalization. Focused planner, handoff, resolver-model, resolver-revision and producer-identity tests cover these cases; this keeps the complete Tiny SD candidate gated when its producer supplies no selected artifact ID.

The text runtime-host path now polls cancellation while both owner-fresh load-target and package-facts RPC futures are in flight, drops a pending future before returning the typed cancellation response, and gives cancellation precedence when a delayed resolver fails. `text_port_cancels_while_waiting_for_load_target_resolution`, `text_port_cancellation_wins_over_in_flight_load_target_failure` and `text_port_cancellation_wins_over_in_flight_package_facts_failure` cover those boundaries. These are lifecycle hardening results, not closure of producer-selected image/embedding execution or desktop/saved-graph cold reopen.

### RPC-00 exact documentation write set

- `docs/plans/README.md`
- `docs/plans/domain-architecture-and-multimodal/plan.md`
- `docs/plans/domain-architecture-and-multimodal/issues.md`
- `docs/plans/domain-architecture-and-multimodal/execution-ledger.md`
- `docs/plans/domain-architecture-and-multimodal/reports/2026-09-25-standards-and-rpc-audit.md`
- `docs/plans/domain-architecture-and-multimodal/reports/2026-09-25-pumas-contract-inventory.md`
- `docs/plans/domain-architecture-and-multimodal/reports/2026-09-25-end-to-end-acceptance.md`

Artifact download/build and disposable fixture state have their own recorded external paths and cleanup/retention owner. They are not writes to the active Pumas development repository. Production client/backend changes begin only after their exact admitted source/test set is recorded.

### RPC-01 transport-seam disposition

The first implementation slice is intentionally limited to a closed HTTP JSON-RPC transport in `workflow-nodes/src/pumas_rpc.rs`. It accepts only literal loopback `/rpc` URLs, disables redirects and ambient proxy routing, rejects credentials and non-loopback hosts, bounds response bodies, applies a request timeout, validates JSON-RPC version/correlation and result-versus-error field presence, and preserves typed remote error codes/classes. Its operation enum contains only library/resource methods: model listing/search, execution descriptor, package facts/summary, summary snapshot, update feed and artifact load-target resolution. It exposes no inference, serving, profile, provider-private or arbitrary-method route.

Focused evidence is `cargo test -p workflow-nodes --features model-library pumas_rpc --locked` with nine passing tests under loopback-capable escalation. The transport source/dependency write set is `crates/workflow-nodes/src/pumas_rpc.rs`, `crates/workflow-nodes/src/lib.rs`, `crates/workflow-nodes/Cargo.toml` and `Cargo.lock`. Astra High and Sol Extra High independently reviewed the seam; the first review found and the implementation repaired credential redaction, literal-loopback enforcement, JSON `null`/absent-field handling and request bounds. The final narrow review required an explicit forward-compatible envelope-field policy, which is now documented and covered by a regression. The current final reviews are complete: the security/lifecycle review found no remaining concrete source defect, while the standards review left only the cross-session Coding-Standards runtime-state discrepancy. Commit remains gated on reconciling that authority. This slice does not migrate `PumasApi` owner callers, establish producer/root identity, or claim successful facts/asset mapping from the empty fixture. Those remain the next RPC-01 gate together with configuration and all affected consumer dispositions.

### Current-source producer-selected directory qualification — 2026-09-26

The pinned current-source no-inference producer was run against a fresh isolated root containing a path-normalized copy of the retained Z-Image-Turbo metadata and read-only symlinks to its actual library bytes. The real Pantograph RPC client decoded `get_models`, package facts and `resolve_model_artifact_load_target`; the facts supplied selected artifact `tongyi-mai--z-image-turbo__bundle_e07edb6c5bfd`, `diffusers_bundle`, `requires_custom_code=false`, `validation_state=valid`, `storage_kind=library_owned`, and the exact directory target. The producer returned `artifact_state=ready` and `entry_path_state=ready`. The same HTTP surface returned `-32601/not_found` for `model_library_selector_snapshot`.

This advances RPC-A01/A02 beyond the synthetic fixture, but it does not admit execution: package facts report missing root-level text-encoder shard paths for Z-Image, and its pipeline is `ZImagePipeline`, outside Pantograph's currently qualified closed Stable Diffusion loader. The retained Tiny SD directory independently has all twelve expected files, `requires_custom_code=false`, valid library-owned facts and `StableDiffusionPipeline`, but its producer metadata has no selected-artifact identity; owner-fresh resolution therefore returns `ambiguous/missing_selected_artifact`. Neither candidate may be promoted by guessing an ID, rewriting metadata or treating a path as selection. The next source/evidence slice is a producer-issued identity for the complete Tiny SD target or another complete producer target matching an admitted Pantograph backend, followed by replacement and outage/reopen checks.

### Complete Tiny SD facts with missing producer identity — 2026-09-26

Normal current-source Pumas indexing was run in isolated root `/tmp/pantograph-rpc-tinysd-20260926e` against a path-normalized copy of the retained Tiny SD metadata and read-only symlinks to its bytes. The producer returned model `diffusion/cc-nms/tiny-sd-turbo` and package-facts contract 3 with `diffusers_bundle`, `library_owned`, `validation_state=valid`, `requires_custom_code=false`, `StableDiffusionPipeline`, Stable Diffusion family evidence, and all twelve expected component/config/weight/tokenizer files present. The facts retain the existing invalid `feature_extractor` diagnostic, which Pantograph's closed loader already treats as an absent optional image-processor component.

The producer's `get_models` record and package facts did not carry a selected-artifact ID. An owner-fresh load-target request containing only the model identity returned typed `artifact_state=ambiguous`, `entry_path_state=ambiguous`, and diagnostic `missing_selected_artifact`; supplying the observed absolute path without an identity returned `artifact_missing` rather than authorizing path promotion. The HTTP selector snapshot remained `-32601/not_found`. This is the strongest current complete-admitted candidate, but it is still a fail-closed identity blocker; no inference was run and the isolated owner was stopped with an empty registry.

The available library-only acquisition routes were also exercised in fresh disposable roots: `import_external_diffusers_directory` returned success but left the indexed model without a selected-artifact ID, `import_model_in_place` generated idempotent metadata/facts with the same missing identity, and the lower-level `import_model` route successfully indexed a retained Granite GGUF but likewise exposed no selected artifact while reporting `task_type_primary=unknown`. The analogous direct directory import failed during producer reconciliation at a safetensors weight path before producing a selectable package. All operations were stopped before any inference route; neither authorizes Pantograph to derive or promote an ID locally or infer the embedding task. The producer identity/task gate therefore remains genuine rather than an omitted import step.

A read-only scan of the current shared-library diffusion metadata found no alternate complete admitted candidate: Tiny SD is the only record declaring `pipeline_class=StableDiffusionPipeline`, and it is the same record without `selected_artifact_id`. Other diffusion records with producer-selected identities declare different pipelines or do not have executable package facts. This closes the candidate-search branch without changing the fail-closed identity rule.

### Authenticated framed-IPC qualification — 2026-09-26

The pinned current-source no-inference producer was also exercised through its separate authenticated framed local-IPC contract, using a fresh registry file under `/tmp/pantograph-rpc-zimage-20260926d` and the same isolated Z-Image root. The framed request used the producer's documented 4-byte big-endian length prefix and JSON-RPC operation `model_library_selector_snapshot`; the registry supplied the ephemeral connection token, which is intentionally not retained in project evidence. The producer returned one selector row with the producer-selected identity `tongyi-mai--z-image-turbo__bundle_e07edb6c5bfd`, `artifact_state=ready`, `entry_path_state=ready`, `storage_kind=library_owned`, and a complete cached summary. Authenticated framed `resolve_model_artifact_load_target` returned the same ready directory target, and the package-facts batch route regenerated the same contract-3 facts.

This explains the transport split rather than closing it: the same producer's HTTP `/rpc` route returns `-32601/not_found` for selector snapshots, while its owner-local authenticated IPC route supports selector snapshots and the typed load-target/facts operations. Pantograph's production seam remains the explicitly configured closed HTTP JSON-RPC client; the IPC evidence is retained as a separate qualified Pumas contract and is not silently treated as an HTTP fallback or a reason to discover/claim a Pumas root. The producer process was stopped cleanly and its isolated registry had no remaining instance row. The Z-Image target still reports missing text-encoder shard paths and an unadmitted `ZImagePipeline`, so the next gate remains a complete producer-provided directory target matching an admitted Pantograph backend.

### Linked generated-text fan-out through Pantograph — 2026-09-26

A disposable single-process qualification harness used the existing Pantograph `InferenceGateway` for all three executions. It first called `execute_typed` through the copied llama.cpp `10883` runtime and LFM2.5 GGUF, receiving generated text `"Amber Bridge"`. That exact returned string was then passed unchanged as both the image plan's `prompt` and the single embedding input; the image used the previously serialized synthetic ready Tiny SD directory target and Pantograph's PyTorch backend, while embedding used Pantograph's llama.cpp embedding mode. The run returned one nonzero 384-dimensional embedding and one nonempty 256x256 `image/png` payload (`bytes_base64=145876`), with each runtime stopped cleanly.

This is the required generated-text dependency at the Pantograph gateway boundary: downstream inputs came from the text result rather than a second literal fixture prompt. The text/embedding assets were the independently copied local GGUFs (SHA-256 `67b42337951a2b3140c0c2eb49b5bbbdd14653114337ac15397f68dc664648d4` and `d4b41f5d7db712806722103a1c6aba2f0fe99f77740501d6f313c8240641f145`), and the image run used the existing `.venv` Torch/Diffusers environment (`torch 2.10.0+cu128`, `diffusers 0.37.0`). The image target remains synthetic contract evidence rather than a producer-selected target, so this advances Pantograph fan-out and backend evidence without closing DA-03, INF-A01, or the real RPC-selected execution gate.

## Agent roles and coordination

| Role | Model/effort | Contract |
| --- | --- | --- |
| Primary orchestration and integration | GPT-6 Luna Max | Verify state, maintain constraints/inventory, assign disjoint sets, routine coordination, concise handoff inspection, final acceptance and coherent commits. |
| Planning and final review | GPT-6 Astra High | Design/admission recommendations for integration; final material review is read-only. |
| Implementation | GPT-6 Sol Medium | Admitted code and tests, especially complex/sensitive work. |
| Document discovery | GPT-6 Luna High | Read-only, bounded: latest inventory first, then relevant architecture/plan and `context.md` passages; include `architecture-plan` if present. Return constraints, concrete gaps and exact file/line evidence. |
| Independent architecture/security/lifecycle review | GPT-6 Sol Extra High | Read-only; reuse for narrow repair verification. |
| Preferred suitable implementation | Passeur MCP / Muse Spark 1.3 Contributor | Settled, bounded implementation/fixtures to reduce cost; never planning or review. |

These are requested assignments, not claims that a model or service is available. Verify exact capabilities/effort and primary-model state; report mismatches. If the Muse lane is unavailable, use Sol Medium for appropriate implementation and record it. Missing required review is an unsatisfied gate, not permission for self-review. Reviewers never edit or commit; implementation owners repair and reviewers inspect.

Verify Passeur's canonical Pantograph root/worktree binding and actual registered Muse capability before submission. Give each assignment goal, relevant facts/standards, material/base identity, exact primary and adjacent write sets, shared/forbidden files, acceptance, output and escalation. Shared contracts, generated files, lockfiles, common fixtures, plan and index are single-owner writes. Worker commits require explicit branch/commit ownership under the same standards; the root owns final integration.

Maximize independent parallelism among discovery, bounded planning, stable-slice review and admitted implementation. Do not implement an unsettled dependency or treat review of changing bytes as final. Apply concurrent-plan safeguards when outstanding proposals can become stale. Coordinate builds/GPU and fixture state to preserve valid evidence. Keep worker lifecycle control evidence-based rather than timer-based. Reuse concise findings and reviewers, not duplicated broad scans.

## Evidence, inventory and commits

Use [the acceptance procedures](reports/2026-09-25-end-to-end-acceptance.md). Extend existing tests before adding new evidence machinery. Tests must reach and discriminate the affected behavior under existing failures; unchanged failure counts alone are insufficient. A fake model, Pumas provider output, selector list or complete metadata envelope cannot establish a real Pantograph workflow.

Update the existing inventory/plan/issues/ledger at material change and acceptance boundaries, including actual artifact and standards identities, consumer mappings, roles, review dispositions, checks and exactly one next slice. Keep dated detail in history; no parallel inventory or per-agent diary. Obtain Sol Extra High review for important architecture/security/lifecycle changes and Astra High final material review; Luna Max runs integrated final acceptance on reviewed bytes. Separate API-equivalent cost estimates from real bills and use only observed data if costs are recorded.

Through current MCP-provided Commit guidance, inspect status, stage only the coherent admitted set, inspect the staged diff and sensitive/generated effects, run required checks/hooks, then commit the implementation and materially changed inventory/evidence. No arbitrary commit count, forced commit topology, implicit hook bypass, history rewrite or unrequested push. Branch/worktree isolation follows actual concurrency/risk and terminal resource ownership.

## Blockers and re-plan triggers

Unverified: complete consumed-operation parity; selector discovery; executable directory-target mapping; qualified image/embedding model/runtime/device and display; full scheduler/desktop lifecycle, cold-reopen and target evidence; final standards/commit closure. Tiny Aya now provides one real runtime-host text-generation acceptance through the unified Pantograph path. Qwen embedding remains correctly blocked by producer task evidence `unknown`, while the complete producer-selected image target and desktop/cold-reopen evidence remain open. Each item blocks only the decision or claim that depends on it.

Replan for material contract/ownership/trust/compatibility/lifecycle/output/acceptance changes, a required library operation missing from the selected artifact, unsafe deployment sharing, or an actual new affected semantic owner. Refine write sets without restarting planning when the original decisions remain sound. A new upstream commit is a review input, not an automatic replan or artifact update. Preserve the immutable selected candidate during a run.

## Completion

Continue through the complete goal, not merely RPC discovery or plan creation. Final acceptance requires all mandatory claims, independent review and truthful inventory/commit/resource state. Preserve required-real blockers and report separate product/compliance status; none of text, image, embeddings or Pantograph-only inference may be dropped to close the plan.

### Intent-first Pumas 0.7.0 consumer migration — 2026-09-26

The consumer dependency is now pinned to the reviewed Pumas Library 0.7.0 source revision `29242fce4ec9043becaeb061587f364ecc4e7177`; `Cargo.lock` records that revision and its release dependency graph. The retained Linux release artifact used for producer qualification is `/tmp/pumas-library-release-linux/linux-unpacked/resources/pumas-rpc`, SHA-256 `61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80`. A disposable run of that actual binary against an empty independent root accepted `intent_query_models`, `intent_get_model`, and `intent_get_model_status`: query returned `matches` with zero candidates, while get/status returned typed `missing` with `artifact_missing`; no inference, serving or `/v1` operation was issued.

The release source and the inspected current Pumas checkout have no relevant intent-contract difference. The released contract is sufficient for this consumer slice: Pantograph now has typed closed-RPC operations and selector-access methods for query/get/status across owner, authenticated local client and explicit RPC roles. Durable ensure/release/declaration operations are intentionally not exposed to Pantograph callers until a real durable consumer lifetime is established.

Selected-model runtime resolution now constructs a `LocalModel`/`LocalOnly` requirement from the scheduler's explicit Pumas identity, uses intent get for owner/RPC access, and adapts an `Available` `ModelHandle` at the runtime load-target boundary. Identity, artifact kind, executable path kind, storage, validation, source-fingerprint and cache-observation evidence are preserved in Pantograph-owned target fields; the executable path is not promoted into scheduler identity. If intent reports package facts as missing, invalid or stale, Pantograph performs one targeted package-facts operation and one bounded re-observation, then preserves the resulting typed state. Package-facts resolution remains complementary because intent availability does not establish pipeline, task or backend suitability. Dispatch load-target evidence uses the same intent handle and retains the verification projection. Read-only access retains its existing load-target path in `ReadOnlyIndexed` mode because the released `PumasReadOnlyLibrary` has no intent facade. A scheduler selection may omit an artifact ID; Pumas must supply the uniquely resolved handle identity, while a supplied scheduler artifact ID is still checked for exact agreement.

Desktop hosted startup now requires the explicitly configured `workflow.pumas_rpc_endpoint` or `PANTOGRAPH_PUMAS_RPC_ENDPOINT` and uses `RpcEndpoint`; it no longer searches for a sibling Pumas checkout, opens a local database, or discovers another instance. Existing desktop admin/Hugging Face commands that require a raw owning `PumasApi` remain a separate complementary-operation gap and are not silently routed through inference or an untyped RPC fallback.

The current blockers are reclassified as follows: missing typed intent consumers was Pantograph migration; private old IPC test fixtures were an obsolete test dependency exposed by the 0.7.0 release and were removed without changing production local-client code; Tiny SD's missing selected artifact and Qwen's `task_type_primary=unknown` remain producer metadata/provisioning evidence; Diffusers-directory upstream acquisition remains an actual released intent layout limitation; and desktop endpoint/process identity plus real intent-backed nonempty model execution remain environment evidence. No Pumas source change is justified.

Current phase: **R1 / RPC-01 intent-first consumer migration**, implementation in progress. Exactly one next slice: qualify a retained producer-selected nonempty model through the released intent get/handle path in Pantograph's real runtime-host acceptance, then preserve the resulting state in this plan, inventory, issues and ledger before admitting further desktop/provisioning work. DA-03, INF-A01, desktop cold reopen and final standards state remain open.

### 2026-09-26 — intent outcome/evidence repair

Fresh standards/spec review found that the first consumer slice flattened non-available intent outcomes into unbounded debug strings and dropped `verification.observed_from_cache_at` while adapting a handle. The repair replaces those strings with a bounded typed state/candidate-count/diagnostic projection, carries source fingerprint and cache-observation evidence through Pantograph's owned load-target and dispatch-facts contracts, and adds bounded hydration/re-observation semantics for package-facts diagnostics. The review also identified that the released read-only facade rejects `OwnerFresh`; the fallback now explicitly uses `ReadOnlyIndexed`. The remaining nonempty `Available` handle, real image/embedding, scheduler fan-out, desktop and cold-reopen evidence are still open and are not promoted by these contract repairs.

### 2026-09-26 — intent-first handoff and explicit desktop operation repair

The consumer repair now preserves the producer-selected model identity and revision instead of overwriting them with scheduler values. When the scheduler leaves optional revision or artifact identity unset, the Pantograph-owned reconciliation fills only those fields from a validated intent-resolved target; any supplied mismatch fails closed. Executable paths and verification evidence remain runtime-boundary data. Package-facts/load-target coherence is checked before dispatch, and the exact intent source fingerprint participates in the safe load-target key so evidence changes cannot reuse a prior target.

Intent diagnostics now retain bounded typed outcome state, candidate count, producer diagnostics and any one-shot targeted package-facts hydration error. Hydration is attempted only for explicit missing/invalid/stale package-facts diagnostics, followed by one re-observation; observational intent calls are not used as an unbounded repair loop. The RPC selector result no longer manufactures a snapshot cursor because `get_models` is not a producer update feed. Existing supported desktop search, download, delete, review and effective-metadata operations now route through the explicit RPC selector access rather than bypassing the configured library endpoint.

This slice changes Pantograph only; the Pumas 0.7.0 release and inspected current source remain read-only, and no Pumas inference/serving/provider route was used. Focused checks passed for workflow-nodes (218), load-target (7), dispatch facts (4), candidate provider (14), runtime-host execution port (17 executable, 2 ignored), image projection (8), text projection (7) and embedding projection (2), plus the locked offline workspace check. The standards runtime still reports `restart-required`; the supported host action remains `restart-and-reconnect` and is unavailable in this session.

Acceptance remains partial. Exactly one next slice is retained: qualify a nonempty `Available` intent handle from the frozen release in a real Pantograph runtime-host run. Producer-selected image/embedding execution, generated-text scheduler fan-out, desktop configuration/inspection, complete retained outputs and cold reopen remain dependent gates.

### 2026-09-26 — final review repair boundary

The final implementation review corrected four consumer-side defects without changing the Pumas source or weakening the ownership boundary. Released download and review RPC calls now use the exact 0.7.0 wire shapes; the handle adapter validates Pumas's model-reference contract version; LocalClient load-target dispatch is admitted through the released intent/load-target surface while full package-facts dispatch remains explicitly unavailable because the 0.7.0 facade does not expose that operation; and image plus image-batch runtime-host resolution polls cancellation while waiting for an intent-derived load target.

The earlier scheduler-reconciliation wording is superseded: Pantograph does not mutate scheduler admission requests or readiness proofs to make producer data fit. Scheduler identity remains the admission proof; the validated producer target supplies runtime projection identity, and producer package-facts revision is preserved when the scheduler omitted one. Supplied scheduler identity mismatches still fail closed. Focused repaired suites pass, including load-target (6), dispatch facts (4), candidate provider (14), and execution port (19 executable, 2 ignored). Exactly one next slice remains: qualify a nonempty released-intent `Available` handle through a real Pantograph runtime-host run.

### 2026-09-26 — second final review repair boundary

The second independent review found and the implementation repaired four additional consumer defects. Summary-only RPC catalog rows marked 'NeedsDetail' remain selectable as model identities for the normal intent resolver; known missing, partial, invalid, ambiguous and stale states remain disabled. Released HF search envelopes now preserve success=false and the producer error instead of returning a successful empty list. Candidate coherence rejects only conflicting revisions when both observations provide one, preserving the distinction between an omitted package revision and a contradiction. Image and image-batch runtime-host paths now poll cancellation during package-facts resolution and re-check cancellation before reporting resolver errors.

Verification now passes workflow-nodes (221), load-target (6), dispatch facts (4), candidate provider (15), and execution port (23 executable, 2 ignored), plus workspace check and formatting. Exactly one next slice remains: qualify a nonempty released-intent 'Available' handle through a real Pantograph runtime-host run.

### 2026-09-26 — text settings and optional revision repair boundary

The latest review repair preserves the distinction between an omitted producer revision and a contradictory revision in the selected text and embedding validators: a missing package revision is compatible with an intent/load-target revision supplied by Pumas, while two supplied revisions must still agree. Runtime-host text projection no longer imposes the acceptance fixture's eight-token budget on authored production requests. It maps the existing scheduler trait settings for `max_new_tokens`, `temperature`, `top_p`, `top_k`, and `seed` (including the existing text-generation namespaces) into Pantograph's typed inference options; absent authored settings remain unbounded by this projection, while the small acceptance fixture explicitly authors `max_new_tokens=8`.

The 1,024-byte prompt/result bound remains enforced. Focused text projection verification passes (8 tests), and selected text/embedding revision compatibility tests pass. The real released-intent `Available` handle, producer-selected image and embedding execution, actual scheduler fan-out, desktop inspection, cold reopen, and standards reconnect remain open evidence gates; no Pumas source change is justified.

### 2026-09-26 — third review repair and producer path-contract reclassification

The third review repair closes the remaining consumer-side coherence defects without changing Pumas. Revision observations now agree across every supplied request, scheduler, package-facts and target reference; an omitted target/package revision is still an unknown observation, but it cannot mask conflicting nonempty revisions elsewhere. Intent-omitted legacy artifact paths remain compatible only as omissions; conflicting supplied identity fields still fail closed.

The released Pumas package-facts contract was rechecked before prescribing a path fix. Its `artifact.entry_path` is the producer's executable local path, not the root-relative `PumasModelRef.selected_artifact_path` identity field. Pantograph therefore retains that producer path through package facts and the image execution plan at the runtime boundary, while continuing to validate the scheduler/intent identity separately. The earlier consumer rejection of absolute package-facts paths was removed; the load target remains the authoritative executable path and no parent-directory guess or scheduler identity substitution is introduced.

Authored graph `generation_options` and typed `trait_settings` now flow into the existing scheduler trait-settings field through inference-interface publication, validation state, executable snapshot and runtime text projection. Supported settings are mapped deliberately; unknown options remain ignored and malformed supported values fail closed. Image single and batch publication now re-check cancellation after the gateway returns and before media retention. The existing 1,024-byte runtime-host prompt/result contract remains enforced; only the accidental production eight-token default was removed, with eight tokens still explicit in the tiny acceptance fixture.

Focused verification for this boundary passes: inference selected text (3), selected embedding (2), image planning (28), runtime-host package facts (10), runtime-host execution port (25 executable, 2 ignored), workflow graph request resolution (15), formatting, and locked offline package checks. The standards runtime remains `restart-required`; its supported `restart-and-reconnect` host action is unavailable in this session. The release probe still has no nonempty `Available` handle, so producer-selected image/embedding execution, actual scheduler fan-out, desktop inspection, complete retained outputs/cold reopen, and final standards reconnect remain open. Exactly one next slice is retained: qualify the frozen release's nonempty intent handle through the real Pantograph runtime-host path.

### 2026-09-26 — independent review repair: version, path, settings and retention boundaries

The Astra and Sol Extra High read-only reviews found four consumer defects and two findings that remain intentionally outside this slice. Pantograph now rejects unsupported supplied Pumas model-reference contract versions at the shared deserialization boundary, and package-facts/dispatch adapters no longer strip the producer version before validation. Supported version `1` and explicitly unversioned legacy inputs remain accepted. Selected text and embedding validation also compares all supplied identity paths independently of whether the intent target includes the optional legacy path field.

When both observations carry absolute executable paths, image planning, selected text and selected embedding execution now require the producer package-facts path and intent load-target path to identify the same artifact. Relative fixture-era package paths remain observationally non-comparable; Pantograph does not derive or guess an absolute path. Nested `generation_options.length` and `.sampling` containers now fail closed when present but malformed. Authored settings continue through the existing scheduler trait-settings contract; the canonical runtime contract has retired static `generation_options` ports, so the stale frontend mock port is not reintroduced as a new execution prerequisite.

Media retention now has an explicit terminal race policy: cancellation observed before a sink write rejects without retention; once a member's sink write begins, that completed member remains inspectable, and a batch marks only unprocessed members cancelled. Regression tests cover single-image completion during a sink-triggered cancellation and batch partial completion after the first member is retained. The existing 1,024-byte text prompt/result contract remains deliberate and unchanged; the review suggestion to widen it is not admitted because it conflicts with the canonical plan's bounded contract. Useful authored generation is provided by scheduler settings within that contract, not by an acceptance-only token default.

The full embedded-runtime package remains at its known baseline boundary: 472 tests passed and 3 older production-composition/fixture tests failed (`runtime branch dispatch unavailable` and two executable-snapshot drift assertions), while the changed runtime-host suite passes 27 executable tests and 2 ignored. Focused graph, planner, package-facts, selected text/embedding and version checks pass. The independent reviewer observed a newer standards MCP session reporting interface 38/current/authoring, while the root session still reports interface 37/restart-required; this disagreement is recorded as a standards reconnect gate, not silently resolved from a checked-out revision. The release still has no nonempty `Available` handle. Exactly one next slice remains: qualify the frozen release's nonempty intent handle through real Pantograph runtime-host execution, then continue producer-selected image/embedding, scheduler fan-out, desktop inspection and cold reopen evidence.

### 2026-09-26 — released intent qualification and blocker reclassification

The frozen Pumas 0.7.0 release binary (`61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80`, source `29242fce4ec9043becaeb061587f364ecc4e7177`) was run with independent writable root `/tmp/pantograph-rpc-intent-release-20260926` and endpoint `http://127.0.0.1:38985/rpc`. Through supported `import_model`, the root imported the exact Qwen fixture into `llm/qwen/qwen3-tiny-rpc-fixture`. Release `get_models` returned one complete, clean model, and `resolve_model_package_facts` returned contract-3 valid `hf_compatible_directory` facts with `Qwen3ForCausalLM`, library-owned storage and the producer executable path.

The supported `submit_model_review` operation was then used with exact config/tokenizer evidence to correct only task metadata: `task_type_primary=text-generation`, text input/output modalities, review source `pantograph-qualification-review`, confidence `1.0`. The review succeeded with overlay `b799d7f1-0443-4635-a7fe-e2631146e69d`; effective metadata is reviewed and the review queue is empty. This proves the existing producer review mechanism can establish authoritative task evidence without Pantograph editing producer state.

After that correction, `intent_query_models`, `intent_get_model`, and `intent_get_model_status` for a `LocalModel`/`LocalOnly` requirement all returned `Incomplete` with the sole diagnostic `artifact_evidence_missing` at `artifact.selected_artifact_id`. Supplying the observed absolute path without an identity returned `artifact_mismatch`; the path is not an identity substitute. This is a Pumas import/metadata identity capability gap, not a Pantograph migration defect, observational retry problem, or permission to derive an ID from a filename/path/display name. No Pumas inference, serving, `/v1`, provider, profile or fallback route was called; the release process was stopped cleanly.

Current phase remains **R1 / RPC-01 intent-first consumer migration**, implementation in progress. The next slice is revised to obtain a producer-issued selected-artifact identity through an existing supported release operation (or, if the release exposes no such operation for a local import, record the precise owner-level upstream issue and preserve the fail-closed consumer boundary), then rerun the real Pantograph intent-handle/runtime-host qualification. Producer-selected image/embedding execution, scheduler fan-out, desktop inspection, complete retained outputs/cold reopen and standards reconnect remain dependent gates.

### 2026-09-26 — path-free intent requirement repair

The released intent qualification exposed one Pantograph consumer defect independent of the producer identity gap: `build_runtime_host_model_requirement` forwarded `selected_artifact_path` from legacy/scheduler state into the intent requirement. Because Pumas treats that field as a supplied path constraint, this could turn a valid local model into `artifact_mismatch`. The builder now preserves model ID, revision and selected artifact ID but omits the path; the producer-issued executable path is consumed only after an `Available` handle crosses the runtime boundary. A regression covers a legacy absolute path and proves it is not promoted into intent identity.

The focused runtime-host load-target suite passes 7/7, and the affected execution-port suite passes 27 with 2 ignored. Astra's read-only repair review found no concrete issue. This is a Pantograph migration correction; it does not relax producer identity checks or authorize local artifact-ID derivation. The next slice remains the supported producer-issued identity/upstream issue decision followed by real released intent-handle qualification.

### 2026-09-26 — D-02 incomplete image-body guard

The desktop Inspector now shares one Pantograph-owned completeness guard between image preview and image download. If either the retained artifact or the response identifies image media, an incomplete response is rejected before decoding, preview replacement or download-object creation. Text preview behavior and the existing byte bounds are unchanged. The desktop smoke assertion now requires a decoded image to be backed by a nonempty response marked complete and to have no projected access error.

This is a Pantograph presentation/integration repair, not a Pumas capability change. Presenter tests, TypeScript typecheck, targeted lint, desktop test syntax and diff checks pass. The WebKit Inspector fixture did not reach readiness in the available run, so this repair does not claim real desktop acceptance. The single next implementation slice remains obtaining a producer-issued selected-artifact identity through a supported Pumas release operation or recording the precise owner-level issue, followed by released intent-handle/runtime-host qualification.

### 2026-09-26 — resolved-empty dependency admission and composition baseline

The no-custom-dependency clarification is now implemented at the owning admission boundaries. A valid Pumas `Resolved` result with empty requirements, bindings and selected binding IDs is accepted as “no additional model requirements”; the dependency registry no longer rejects that valid projection, and its readiness producer drains it without invoking a package probe. Catalog browsing also stops manufacturing `dependency_bindings: []` when dependency resolution was never observed. A dependency-query/probe failure remains typed unavailable, and declared unsatisfied bindings remain fail-closed through the existing readiness evidence.

Real session admission regressions now cover both sides of the distinction: a standard image workflow with an empty Pumas dependency set reaches Pantograph's runtime-host port once the ordinary backend is ready, while the same empty-set admission reports a runtime-host `RuntimeUnavailable` failure when the backend path is unavailable. These tests do not call Pumas inference or require a model-specific dependency profile.

The three stale embedded-runtime composition/fixture failures were rerun. The two composition fixtures now satisfy the current `runtime_source_context` graph contract. The image runtime fixture no longer supplies an unsupported `revision=main`; it now reaches the released Pumas intent/load-target path and defers because the directly seeded Diffusers import still has no producer-issued `artifact.selected_artifact_id`. The full embedded-runtime package is therefore 479 passed, 1 failed, and 2 ignored. This is preserved as a producer identity/provisioning blocker, not hidden or converted into a Pantograph path workaround.

The live Coding-Standards authoring runtime remains interface 37 / implementation 0.2.0 with `installation_state=restart-required` and required action `restart-and-reconnect`; that host action is unavailable in this session. The current route snapshot used for this slice is `snapshot:v1:9f202e7c-f527-428d-aca9-0d160bc8c34f`; applicable implementation, verification, Rust, dependency, contract, resilience, diagnostics and commit guidance was read. This does not change the current phase or next slice: obtain a producer-issued nonempty identity/`Available` handle through a supported Pumas operation, then rerun real runtime-host and dependent multimodal acceptance.

### 2026-09-26 — dependency coherence, readiness synchronization and scheduler lifecycle repair

This repair slice resolves three Pantograph-owned verification defects. The dependency payload boundary now requires the identity's selected-binding set to equal the payload's selected-binding set; the resolved-empty fixture derives its identity from a genuinely empty planning request, and a contradictory identity regression fails closed. Readiness lifecycle tests now wait for the observable snapshot and empty work queue with a bounded timeout instead of assuming a fixed elapsed delay. The artifact-store path-leak assertion binds its serialized string and temporary path explicitly, removing a test-target type-inference failure without changing the contract.

The embedded image failure was reclassified from producer identity loss to scheduler lifecycle mapping. The controlled fixture reached Pumas package-facts/load-target resolution, Pantograph's unified image gateway, and retained the `hello` image body. Scheduler-created batch requests deliberately use `DeferredToScheduler` reservation policy; the orchestrator previously converted that policy into `RetryDeferred` before examining the completed member state. The early return is removed, and a completed response remains terminal under the actual scheduler policy. The full host-filesystem suite passes 480 tests with 2 ignored; the sandbox-only run is not accepted because its Pumas-backed tests fail before assertions with SQLite readonly/permission errors.

The real WebKit Inspector fixture also passed under the required host loopback environment: PNG decoding, complete-body reads, malformed/partial/failure handling, text bounds, stale reads and URL lifecycle checks all passed. The sandbox's non-escalated run cannot bind `127.0.0.1:5173`; this is recorded as an environment restriction, not desktop acceptance of a real Tauri/model workflow.

The frozen Pumas release/source identities and no-inference boundary are unchanged. The live Coding-Standards runtime remains interface 37 / implementation 0.2.0 / `restart-required`, with the exact supported action `restart-and-reconnect` unavailable here; Passeur is configured but runtime readiness and Pantograph repository binding were not proven, so no Muse assignment is claimed. Exactly one next slice remains: obtain a producer-issued selected-artifact identity and nonempty released-intent `Available` handle through a supported Pumas operation, then rerun real Pantograph runtime-host qualification. Producer-selected image/embedding execution, generated-text scheduler fan-out, desktop inspection, complete retained outputs/cold reopen and standards reconnect remain open.

### 2026-09-26 — frozen-release intent qualification and current standards authority

The retained Pumas 0.7.0 baseline was qualified from source revision `29242fce4ec9043becaeb061587f364ecc4e7177`. The independently retained Linux `pumas-rpc` artifact at `/tmp/pumas-library-release-linux/linux-unpacked/resources/pumas-rpc` has SHA-256 `61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80` and was run only against independently owned writable roots. No Pumas inference, serving, provider, profile, proxy or `/v1` operation was issued; both qualification processes were stopped cleanly.

Two bounded release probes reclassified the remaining blockers. Tiny Aya, copied as immutable model bytes into `/tmp/pantograph-pumas-release-tinyaya-20260926`, initially returned `Incomplete/package_facts_missing`. The supported `resolve_model_package_facts` operation returned a valid text-generation package with producer-selected identity `coherelabs--tiny-aya-water__files_b673ab802c36`, but its `hf_compatible_directory` facts expose a shard file as `entry_path`; subsequent intent query/get/status returned `Incomplete/artifact_path_invalid` because the indexed directory target no longer had the declared filesystem kind. The release intent acquisition path also rejects directory/shard layouts. This is a precise Pumas package-facts/load-target contract issue, not permission for Pantograph to guess a parent directory or edit producer state.

The independent standard Qwen copy at `/tmp/pantograph-pumas-release-qwen-20260926` returned a complete, clean GGUF model from `get_models` with producer identity `empero-ai--qwen3_8-4b-distill-gguf__q4_k_m`. Package facts were valid, and `resolve_model_dependency_requirements` returned `Resolved`, empty validation errors and `bindings: []`; this is valid absence of additional Pumas dependency bindings, not a missing backend requirement. After bounded package-facts hydration and a controlled inspection-manifest adjustment, intent query/get/status still returned `Incomplete/package_facts_stale`. The remaining Qwen blocker is released producer cache/coherence state, separate from optional dependencies and Pantograph backend readiness.

The current Coding-Standards MCP was rechecked directly. Its runtime is authoring interface `39`, implementation `0.2.0`, instance `ce5e3d74-9b9f-4a52-bc87-4a00fc8e27d1`, implementation digest `sha256:6acec181069ed7a28cba559a7ec1095327ccbd10236ad803cd084a46f601f5bf`, catalog digest `sha256:dd421b8316d692478c44b8c40460a3a1f077603f10c2cf7615b635daa517b42b`, schema digest `sha256:f13e5fe1733523236ec2c4cbc5a5ef834d57c0309cd92eb7f4bd1499473faab1`, installation `current`, and action `reuse`. Historical interface-37/restart-required entries are superseded by this runtime recheck. Current guidance requires exact staged-scope review, affected verification, sensitive/generated-file review and one atomic conventional commit; it also requires preserving unavailable evidence as blocked rather than upgrading local agreement into acceptance.

Current phase remains **R1 / RPC-01 intent-first consumer migration**, with acceptance partial. Exactly one next slice is retained: obtain or qualify a producer-issued nonempty released-intent `Available` handle through a supported Pumas operation, then run Pantograph's real runtime-host path. Do not derive artifact IDs, guess filesystem parents, bypass intent, add a model-specific dependency profile for the standard Qwen case, or modify Pumas source. Producer-selected image/embedding execution, generated-text scheduler fan-out, desktop inspection, complete retained outputs/cold reopen and final product acceptance remain open.

### 2026-09-26 — released Available handle and Pantograph embedding qualification

The frozen release qualification now has a producer-issued nonempty handle. An independently owned copy of the identity-bearing single-file GGUF model `embedding/qwen3/qwen--qwen3-embedding-8b-gguf__q4_k_m` was used under `/tmp/pantograph-pumas-release-qwen8-20260926`; the exact `Qwen3-Embedding-8B-Q4_K_M.gguf` SHA-256 is `3fcd3febec8b3fd64435204db75bf0dd73b91e8d0661e0331acfe7e7c3120b85`, and the model bytes were read-only. The frozen Pumas binary and source identities remain unchanged.

Supported package-facts resolution established valid GGUF facts and the exact producer artifact identity. The existing Pumas `submit_model_review` operation corrected only the exact artifact's task evidence to embedding/text→embedding, with evidence tied to the producer model type, GGUF architecture/dimension and exact file digest. One bounded package-facts re-observation then made the intent requirement ready. `intent_query_models` returned a ready match; `intent_get_model` and status returned `Available` with the producer model-reference contract, artifact ID, file-kind load path, library-owned storage, valid validation state, package-facts contract and source-fingerprint evidence. The same artifact's dependency resolution returned `Resolved`, empty validation errors and `bindings=[]`; no model-specific Pumas dependency profile was required.

Pantograph's real runtime-host acceptance consumed that handle through the existing unified gateway and Pantograph-owned llama.cpp process. The retained backend binary was `/tmp/pantograph-llama-runtime-b10883/llama-b10883/llama-server`, SHA-256 `53746ea4be345401c4544f952211fb892c010596bd3d167d0b78bce2fae552d2`, launched through the existing wrapper and isolated data directory. The run completed with a finite, nonzero 4,096-dimensional embedding. It initially exposed a Pantograph adapter defect: the scheduler's canonical `cpu` was passed directly to llama.cpp, which requires the backend-local `none` selector. The gateway now performs this explicit canonical-to-backend projection (with focused CPU/CUDA regressions); Pumas remains a library/resource authority and is never used for inference. Both isolated processes shut down cleanly with no retained listener or orphan runtime.

This closes the released intent-handle/runtime-host embedding boundary only. It does not claim scheduler fan-out, generated-text dependency flow, image execution, desktop inspection, retained-workflow cold reopen or whole-repository standards compliance. Current phase remains **R1 / RPC-01 intent-first consumer migration**, implementation in progress. Exactly one next slice: connect producer-issued text/image/embedding selections to the actual scheduler and desktop-authored workflow, beginning with the qualified embedding path and preserving the outstanding image identity/layout blocker as a dependent producer-owned gate.

### 2026-09-26 — explicit release-root requalification and desktop-harness boundary

The retained Pumas 0.7.0 binary was requalified using its documented `--launcher-root` contract against independently owned writable root `/tmp/pantograph-pumas-release-qwen38-20260926`. The standard Qwen text GGUF was catalogued with producer artifact identity `empero-ai--qwen3_8-4b-distill-gguf__q4_k_m` and file SHA-256 `dec96e8cf2e11b613bb46513dec485377f9ca5a351e71712ee0e244f287c6790`. Supported intent query/get/status first reported `Incomplete/package_facts_missing`; the one permitted `resolve_model_package_facts` hydration returned valid GGUF facts, and bounded re-observation returned `Incomplete/package_facts_stale` at `package_facts.inspection_manifest`. This confirms a released producer cache/coherence blocker, not an optional-dependency or Pantograph backend-readiness blocker. No further manifest mutation, retry loop, ID derivation or Pumas source change is admitted.

The same explicit startup contract was used to rerun the released Qwen embedding handle through Pantograph's real runtime-host acceptance. The test again completed with a finite nonzero 4096-dimensional vector through the unified Pantograph gateway and Pantograph-owned llama.cpp process. The GUI wrapper regression passed all five success/failure/signal-cleanup cases under host execution; the earlier sandbox failures were `EPERM` child-process restrictions. The host WebKit Inspector fixture also passed separately, but neither is desktop/model workflow acceptance. Exactly one next slice remains: connect producer-issued text/image/embedding selections through the actual scheduler and desktop-authored workflow, with image identity/layout and Qwen text package-facts coherence still dependent producer gates.

### 2026-09-26 — scheduler attribution repair and current authority boundary

The existing scheduler fan-out/reopen regression now verifies that retained text, image and embedding outputs carry the selected dispatch model identity and runtime identity both before and after cold reopen. Pantograph derives this metadata from the completed runtime-dispatch assignments; workflow-input artifacts remain unattributed when no inference task owns them. This preserves the producer model reference and the selected runtime spelling (`llama_cpp.embedding`) without promoting an executable path into scheduler identity.

An ignored host acceptance was added for the real Qwen embedding workflow-session path. It uses the producer-issued intent handle, the existing scheduler, the runtime-host adapter and Pantograph's unified llama.cpp gateway, with a genuinely resolved empty Pumas dependency result. It is compiled under the standalone feature but is not accepted from the current retained service state: the Qwen package currently reports `Incomplete/package_facts_stale` because its cached inspection evidence does not match the retained package layout. No Pumas metadata or marker was fabricated, and no exhausted probe was repeated.

Current phase remains **R1 / RPC-01 intent-first consumer migration**, with acceptance partial. Exactly one next slice remains: qualify a fresh producer-coherent identity-bearing text/image package or use a supported producer rehydration path, then run the real scheduler/desktop workflow and required saved-workflow cold reopen. The resolved embedding boundary, optional-dependency regressions, controlled scheduler fan-out, image retrieval repairs and existing backend evidence remain preserved within their demonstrated scope.

The latest direct Coding-Standards MCP identity check reports authoring interface `39`, implementation `0.2.0`, the previously recorded instance and digests, but `installation_state=restart-required` with action `restart-and-reconnect`. This session cannot perform that host action; the exact required action is to restart the Coding-Standards MCP host, reconnect the session and rerun `runtime_info`. No standards reconnect or whole-repository compliance claim is made from this slice.

### 2026-09-26 — fresh release import requalification and producer identity boundary

The retained Pumas 0.7.0 binary was started with its documented `--launcher-root` contract against independent writable root `/tmp/pantograph-pumas-release-qwen8-import-20260926`. Supported `import_model` imported the read-only Qwen3 Embedding 8B Q4_K_M file (4,676,804,928 bytes; SHA-256 `3fcd3febec8b3fd64435204db75bf0dd73b91e8d0661e0331acfe7e7c3120b85`) as `embedding/qwen3/qwen3-embedding-8b-gguf`. `get_models`, package facts and the execution descriptor reported complete/coherent GGUF facts, a valid library-owned executable file target and `Resolved` empty dependency bindings.

The exact LocalOnly intent requirement then returned `Incomplete` with `artifact_evidence_missing` at `artifact.selected_artifact_id`; the effective imported metadata had no selected artifact identity and unknown task evidence. A single supported `submit_model_review` request using exact GGUF architecture/dimension/digest evidence returned producer JSON-RPC `-32603`/`internal` on this fresh import. This is not a reason to retry, synthesize an ID, promote a path, or relax task evidence. It narrows the producer owner issue to a supported identity/provenance path for local imports (and, separately, the review operation's fresh-import error); Pumas source remains read-only.

Pantograph therefore preserves the intent-first fail-closed adapter and the already accepted identity-bearing embedding handle/runtime-host boundary. The focused Pantograph repairs remain green, including resolved-empty optional dependencies, artifact-store typing, embedded image lifecycle, readiness synchronization, warmup, and the host WebKit Inspector fixture. The current Coding-Standards runtime is interface `39`/implementation `0.2.0` with the known instance/digests but `restart-required`; the exact required action remains restart the MCP host, reconnect, and rerun `runtime_info`.

Current phase remains **R1 / RPC-01 intent-first consumer migration**, with acceptance partial. Exactly one next slice: use one supported producer acquisition/provenance path that issues a coherent identity-bearing text/image package, then run the real text→image/embedding scheduler workflow, desktop inspection and saved-workflow cold reopen. Do not derive artifact IDs, guess filesystem parents, add a Qwen-specific dependency profile, repeat exhausted local-import probes, or modify Pumas source. Product acceptance and whole-repository standards compliance remain open.

### 2026-09-26 — delegation authority verification

The connected Passeur service is not Pantograph-bound: its reported project input and repository are `/media/jeremy/OrangeCream/passeur_cache/pumas-coordinator-source` / `Pumas-Library`, with expected repository ID `6aaae9e5ae2b753918ac7478`; its coordination state is `frozen` and execution is `blocked` under `PROJECT_NEEDS_RECONCILIATION` for task `e4323220-4f72-4b5e-9194-4e7c308abcc4`. No Passeur/Muse implementation or review assignment is claimed. Native reviewer slots were also unavailable earlier because the agent service reported its thread limit. This does not alter the single next product slice or authorize work against Pumas source.

### 2026-09-26 — metadata-bearing package qualification environment gate

A distinct producer hypothesis was prepared: Pumas 0.7.0's supported in-place import preserves existing `metadata.json`, and the retained Qwen text package contains producer-issued artifact identity. The model bytes were copied read-only into a fresh independent workspace root with the retained metadata; no Pumas source or retained package was edited. The first launch attempt was prevented by `/tmp` having only 333 MB available and SQLite reporting `readonly`; after redirecting HOME/XDG/TMP state to the workspace filesystem, the release indexed one model but terminated with `Operation not permitted` before exposing RPC. Consequently no intent result or acceptance claim is made for this hypothesis.

The disposable workspace root was removed; retained model assets and unrelated `/tmp` data were preserved. The precise environment action before retry is to free `/tmp` capacity and/or provide a host filesystem/runtime location where the frozen release can complete its startup effects. Current Coding-Standards MCP identity remains interface `39`/implementation `0.2.0` with `restart-required`; restart the host, reconnect, and rerun `runtime_info`. The product next slice remains one supported producer acquisition/provenance path for a coherent identity-bearing text/image package, then real scheduler, desktop and cold-reopen acceptance.

### Current slice boundary — scheduler diagnostics and producer coherence — 2026-09-26

The implementation slice repaired a Pantograph-owned observability defect: runtime-host batch member diagnostics are now retained when deferred scheduler responses are converted into Pantograph outcomes. The affected 18-test module passes, and the real scheduler failure is now classifiable rather than generic. A wrong inner llama.cpp binary-root invocation is recorded as an environment failure; the corrected parent-root invocation reaches the producer intent gate and stops on `Incomplete/package_facts_stale`.

A fresh independent release-root qualification proves that the metadata-bearing Qwen package can be indexed and yields valid producer package facts, a producer artifact identity and `Resolved` empty dependency bindings. It does not prove intent availability: the released intent result remains stale at `package_facts.inspection_manifest` because its cached evidence references a missing `.pumas_download` entry. A bounded producer safety rejection for a digest-conflicting in-place import is preserved as a safety boundary, not treated as a successful repair or an upstream feature verdict.

Current phase remains **R1 / RPC-01 intent-first consumer migration**, implementation in progress with acceptance partial. The next slice is singular: obtain a producer-authorized coherent identity-bearing text/image handle, then execute the corrected real text→image/embedding scheduler workflow, desktop inspection and saved-workflow cold reopen. The diagnostic repair, optional-dependency semantics, controlled attribution/reopen evidence and image retrieval repairs are not promoted beyond their demonstrated scope. Standards remains open pending the exact host `restart-and-reconnect` action; Pumas source remains read-only.

### 2026-09-26 — producer-issued Tiny SD handle and runtime-profile boundary

An independently retained Pumas 0.7.0 binary (source `29242fce4ec9043becaeb061587f364ecc4e7177`, SHA-256 `61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80`) was run against root `/tmp/pantograph-rpc-dir-target-20260926b` at explicit endpoint `http://127.0.0.1:39871/rpc`. The complete producer-issued Tiny SD bundle `diffusion/cc-nms/tiny-sd-turbo` has selected artifact `cc-nms--tiny-sd-turbo__bundle_qualified`, directory load kind, library-owned storage, valid validation, and source fingerprint `20b0d0a06025953cbdb4a79f60d5cfd1bd3f7dfe7898f5d27994f57ef3338017`. The exact `LocalModel`/`LocalOnly` intent get returned `Available` with that handle; this is the first positive image handle in the current slice.

The handle's execution descriptor declares a resolved Diffusers binding with five producer requirements: `accelerate==0.31.0`, `diffusers==0.32.0`, `safetensors==0.3.1`, `torch==2.5.1`, and `transformers==4.41.2`. Pantograph's ignored real runtime-host acceptance now consumes the handle through the RPC load-target/package-facts resolvers, Pantograph's unified PyTorch gateway and the retained artifact sink. With the ordinary host Python it reaches backend startup and reports the truthful missing-`torch` failure. A bounded isolated qualification install of the exact profile is itself rejected by the package resolver because `transformers==4.41.2` requires `safetensors>=0.4.1`, conflicting with the producer pin `safetensors==0.3.1`. This is a producer-owned profile compatibility issue, not permission to use `--no-deps`, add a Pantograph profile, call Pumas inference, or weaken readiness.

The readiness fixture repair replaced elapsed sleeps with observable bounded synchronization and passed lifecycle 8/8 plus auto-resume 6/6. It is committed as `b1f3d95a`; the intent-backed Tiny SD acceptance harness is committed as `999e41bc` and remains correctly ignored until the producer-declared environment is coherent. The current Coding-Standards runtime still reports `restart-required`/`restart-and-reconnect`; this session cannot perform the host action. Current phase remains **R1 / RPC-01 intent-first consumer migration**, with acceptance partial. Exactly one next slice: obtain a producer-supported coherent Diffusers environment/profile for this already-qualified handle (or file the precise Pumas owner issue), then rerun real image runtime-host and text→image/embedding scheduler acceptance. Desktop inspection, complete retained outputs/cold reopen and whole-repository standards compliance remain open.

### 2026-09-26 — real scheduler embedding and complete-output retention

The real ignored workflow-session acceptance now passes against the retained Pumas 0.7.0 release and an independently owned Qwen3 Embedding 8B GGUF root. The producer-issued intent `Available` handle, resolved empty dependency result, existing Pantograph scheduler, runtime-host adapter and unified llama.cpp backend produced a finite nonzero 4,096-dimensional vector. The run queried the retained workflow artifact, verified model/runtime attribution, read the complete body, reopened the artifact store and diagnostics ledger, and read the same body after cold reopen.

That run exposed a Pantograph retention defect rather than a producer or backend defect: the serialized vector exceeded the generic 64 KiB fallback threshold and was projected as metadata-only, causing the body read to fail. Commit `9a06d95f` retains runtime-attributed generated outputs through the existing artifact-store policy and adds a focused regression that verifies exact body bytes and model/runtime attribution. Unattributed oversized workflow values retain their existing bounded metadata-only behavior; the published 1,024-byte text contract is unchanged.

The frozen baseline remains Pumas source `29242fce4ec9043becaeb061587f364ecc4e7177` and binary SHA-256 `61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80`. A read-only inspection of the newer active Pumas candidate found no changed Tiny SD dependency profile or demonstrated compatibility requirement, so it was not adopted. No Pumas source or inference route changed.

Current phase remains **R1 / RPC-01 intent-first consumer migration**, with acceptance partial. The next slice is producer-coherent text/image handle qualification followed by real generated-text→image/embedding scheduler fan-out, desktop authoring/inspection and saved-workflow cold reopen. The embedding scheduler/cold-reopen result is accepted only within its demonstrated scope; whole-product acceptance and whole-repository standards compliance remain open. The current Coding-Standards MCP still requires the host action `restart-and-reconnect`; this session cannot perform that action.

### 2026-09-26 — required-model readiness repair and suite boundary

Pantograph technical-fit preflight now treats a non-empty required-model set as runtime-readiness evidence even when the backend list is omitted. This preserves the normal backend readiness failure for an unavailable standard model while still reporting incomplete candidate state when a ready runtime is present. The task-classification regression was aligned with the current registry: retired `expand-settings` is treated as an absent contract and remains unsupported.

Focused regressions pass for the changed technical-fit and task-classification behavior. The current workflow-service library result is **862 passed / 18 failed**; the remaining failures are the graph-session validation/publication family (13) and session-capacity fixtures (5), not the intent client, retention repair, or these changed paths. The full embedded-runtime library remains **480 passed / 1 ignored**. This is not whole-repository acceptance.

The primary next slice remains singular: qualify a producer-coherent identity-bearing text/image handle or supported producer profile/acquisition result, then run the real generated-text→image/embedding scheduler path and desktop/cold-reopen acceptance. The released HF acquisition operation was inspected but not started because it provisions a new managed destination and would initiate an unbounded multi-gigabyte transfer rather than rehydrate the retained imported fixture. Pumas source remains read-only; no producer metadata or identity was fabricated.

### 2026-09-26 — workflow-service suite restored and runtime-admission fixture boundary

The previously recorded workflow-service failures were fixture drift, not current product blockers. Graph-session fixtures now provide the required explicit `runtime_source_context`, and the revision-change validation test sequences its first and automatic revalidation lookups before draining validation tasks. Capacity fixtures now exercise the explicit runtime-admission/keep-alive boundary; they no longer use an empty one-shot non-runtime request as a proxy for runtime loading. The diagnostics-unavailable case preserves the underlying unload error without rewriting it as a diagnostics-lock failure.

Verification is now `cargo test -p pantograph-workflow-service --locked --offline --lib`: **880 passed, 0 failed**; the focused graph session suite is 66/66 and capacity/lifecycle coverage is 9/9. The host-filesystem embedded-runtime library remains **480 passed, 1 ignored**, and `cargo fmt --all -- --check` passes. This closes local suite drift without weakening assertions or changing Pumas ownership. The current release baseline, no-Pumas-inference boundary, intent-backed embedding scheduler/cold-reopen evidence, optional-dependency semantics, image retrieval repairs and retention repairs remain accepted only within their demonstrated scope.

The full product goal remains incomplete: no accepted generated-text→image/embedding scheduler run, real image inference, desktop-authored inspection, or saved-workflow cold reopen exists. The decisive external gates remain a producer-coherent text handle (`package_facts_stale`), the Tiny SD declared profile conflict, and the Coding-Standards MCP host action `restart-and-reconnect`; this session cannot perform the host restart. Current phase remains **R1 / RPC-01 intent-first consumer migration**, with exactly one next slice: obtain a supported producer-coherent text/image handle or profile, then execute the real text→image/embedding workflow and desktop/cold-reopen acceptance.

### 2026-09-26 — current verification of optional-dependency and fixture boundaries

The current HEAD (`ab05e7e67`) reverified the previously repaired Pantograph-owned boundaries without changing Pumas source or the retained baseline. The dependency registry contract suite passes 11/11, including an empty resolved result whose identity is derived from its genuinely empty planning request and a contradictory identity/payload negative. The real workflow admission regression for a standard model with no selected Pumas bindings passes: a resolved-empty result remains pending until ordinary backend readiness, then dispatches through the runtime-host port; the pending-path regression preserves the dependency-query failure as `RuntimeNotReady` and does not dispatch or treat it as successful empty requirements. Existing declared-binding and provider-failure tests remain fail-closed.

The affected consumer suites are green on the current host filesystem: workflow-service library 880/880; embedded-runtime standalone library 480 passed, 0 failed, 3 intentionally ignored; production embedded image runtime-host test 1/1; artifact-store integration target 13/13; readiness lifecycle 6/6; auto-resume 6/6; and `cargo fmt --all -- --check`/`git diff --check`. The prior embedded-image identity report is reclassified within Pantograph: the production fixture now preserves the producer-selected model/artifact through the intent/load-target and scheduler lifecycle path, reaches the unified image gateway, and retains the complete image body. This does not promote the fixture to real image acceptance; the released Tiny SD handle remains blocked at the producer-declared contradictory environment/host backend gate.

The host-level GUI/WebKit fixture passes all 5 launcher/readiness/signal/cleanup cases. The non-escalated sandbox failure occurs before the fixture assertions at child-process creation (`spawnSync /bin/bash ... EPERM`), so it is not desktop evidence. The direct Coding-Standards MCP check remains `installation_state=restart-required` with required action `restart-and-reconnect`; no current standards closure or whole-repository compliance claim is made. Exactly one next slice remains: obtain a producer-coherent text/image runtime result or supported profile, then run the real generated-text→image/embedding scheduler workflow and desktop/saved-workflow cold reopen.

### 2026-09-26 — released Tiny SD handle and PyTorch worker boundary

The retained Pumas 0.7.0 release binary (`61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80`, source `29242fce4ec9043becaeb061587f364ecc4e7177`) was run against independent root `/tmp/pantograph-rpc-dir-target-20260926b` on `http://127.0.0.1:35777/rpc`. Library-only `get_models` and package-facts resolution reported a complete, clean `diffusion/cc-nms/tiny-sd-turbo` package with producer-selected artifact `cc-nms--tiny-sd-turbo__bundle_qualified`, valid Diffusers-directory facts, `StableDiffusionPipeline`, and a producer diagnostic for the malformed `feature_extractor` reference. The exact LocalOnly intent query/get/status returned `ready`/`Available`; its handle preserved the producer model/artifact identity, `diffusers_bundle` kind, directory load kind, `library_owned` storage, valid verification and source fingerprint `20b0d0a06025953cbdb4a79f60d5cfd1bd3f7dfe7898f5d27994f57ef3338017`.

The existing real runtime-host image acceptance then passed 1/1 in 149.50 seconds using the repository's already-installed Python environment (`torch 2.10.0+cu128`, `diffusers 0.37.0`, `transformers 4.52.4`, `accelerate 1.12.0`). It consumed the intent handle through Pantograph's unified PyTorch/Diffusers backend and retained a complete 441,767-byte PNG. The acceptance fixture now preserves the producer's omitted revision instead of forwarding the generic `main` constraint, and omits the generic display-class scheduler option because the current image contract accepts primitive IDs and this runtime family does not support scheduler overrides. These are fixture/consumer-boundary corrections, not producer identity or profile fabrication.

The run exposed and repaired one Pantograph backend seam: intent verification evidence remains in the Rust runtime plan, while the PyTorch worker projection removes only `verification_source_fingerprint` and `verification_observed_from_cache_at`, which are not part of the worker's deny-unknown-fields contract. The worker projection regression passes 13/13 and the image-generation envelope subset passes 6/6. The full inference library run against the repository venv is 679/690; 11 PyO3/Python-shim tests remain environment/version-sensitive failures and are not promoted to a full-suite pass. No Pumas inference, serving, provider, profile-startup, proxy or `/v1` route was called, and the release process stopped cleanly.

A separate independent Qwen3-8B sharded safetensors candidate was also exercised only to classify a new hypothesis. Its actual binary was an AppImage-extracted candidate (`1b42b6cbedc0bf842990a895cf759537f60ba1564e054a8c8b5f7f022e4a2114`), not the frozen release; supported package-facts hydration and in-place import left intent at `Incomplete/artifact_path_invalid` because Pumas indexed the first shard as a file entry for a directory package. This candidate is not adopted and no path, ID or metadata was synthesized.

Current phase remains **R1 / RPC-01 intent-first consumer migration**, with the image intent/runtime-host boundary accepted within scope. The primary next slice is now the real scheduler workflow: qualify a coherent text handle or supported producer rehydration, connect generated text to image and embedding tasks, then complete desktop authoring/inspection and saved-workflow cold reopen. Qwen text remains blocked by released `package_facts_stale`/directory evidence, and the Tiny SD producer-declared dependency profile remains separate from the now-qualified backend environment. The Coding-Standards MCP still requires host `restart-and-reconnect`; whole-repository standards compliance remains unclaimed.

### 2026-09-26 — current-source text candidate host qualification gate

The already-qualified pinned no-inference Pumas candidate at commit `96f859443460ad8e4799d563528aaba113ccff21` (binary SHA-256 `3163f4d89d535870cb4ba2549adcbf47af435685576379a862e7ba422badf27c`) was prepared for a bounded scheduler text/embedding qualification using retained read-only Tiny Aya and Qwen embedding bytes. The host's global registry database was redirected to the disposable root after the system root reported read-only state. Pumas initialized its model index, but `PumasApi::builder` then terminated with `Operation not permitted` before publishing an RPC endpoint. The `/media` qualification volume is currently remounted read-only (`errors=remount-ro`), so no external writable service root could be created.

This is an environment gate on reusing the independently qualified newer text candidate, not a consumer or producer-contract result. No Pantograph source, Pumas source, model bytes, metadata or inference route changed, and no scheduler acceptance claim was made. The smallest next action is a host filesystem/runtime location that permits the pinned candidate's registry, model-library lock and startup effects; then run one bounded text→embedding scheduler qualification against that frozen candidate. The release text/image gates and desktop/cold-reopen gates remain open.

### 2026-09-26 — optional-dependency regression and composite release recheck

Pantograph now has a focused regression for the ordinary standard-model case: the GGUF text-generation fixture is compatible with a `llama_cpp` backend, produces an execution-evidence candidate, and causes neither a Pumas model-specific package-readiness probe nor synthetic dependency facts. The real workflow-session admission regression also passes 1/1: a resolved-empty dependency result remains pending until ordinary backend readiness, then dispatches through the runtime-host port. Existing failed-query and declared-binding paths remain fail-closed. This confirms that absent optional Pumas bindings do not become a backend profile requirement; it does not weaken backend readiness, artifact identity, task evidence or custom-code trust.

The frozen Pumas 0.7.0 release was rechecked from retained bytes (source `29242fce4ec9043becaeb061587f364ecc4e7177`, binary SHA-256 `61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80`) in independent composite root `/tmp/pantograph-pumas-intent-multimodal-20260927`, endpoint `http://127.0.0.1:34617/rpc`, with a separate Qwen-embedding comparison root at endpoint `http://127.0.0.1:40551/rpc`. Tiny Aya LocalOnly intent after one bounded package-facts hydration remained `Incomplete/artifact_path_invalid` because the producer-selected directory/shard entry no longer had the indexed filesystem kind. Qwen text and Qwen embedding package-facts hydration returned valid identity-bearing facts, and dependency resolution for the Qwen text candidate returned `Resolved`, empty validation errors and `bindings=[]`; subsequent intent observations remained `Incomplete/package_facts_stale` at `package_facts.inspection_manifest`. These are producer package-facts/layout/cache-coherence results, not permission to derive an artifact ID, guess a parent directory, add a Qwen profile or bypass intent. Both release services stopped cleanly and no Pumas inference/serving/provider route was used.

The focused readiness regression passed 2/2, the real workflow-session admission regression passed 1/1, and the previously rerun registry/artifact-store, embedded image lifecycle and host GUI/WebKit checks remain green within their recorded scope. The direct Coding-Standards MCP still reports authoring interface `39`, implementation `0.2.0`, the recorded instance and digests, `installation_state=restart-required`, and action `restart-and-reconnect`; this session cannot perform that host action. Passeur remains unbound to Pantograph and blocked under `PROJECT_NEEDS_RECONCILIATION`; no delegated assignment or unavailable reviewer result is claimed.

Current phase remains **R1 / RPC-01 intent-first consumer migration**, with acceptance partial. Exactly one next slice remains: obtain a producer-authorized coherent identity-bearing text/image handle or supported acquisition/provenance result, then run the real generated-text→image/embedding scheduler workflow, desktop inspection and saved-workflow cold reopen. The standard no-custom-dependency case is verified within the Pantograph admission scope; the full product goal and whole-repository standards compliance remain open.

### 2026-09-26 — fail-closed producer-detail query boundary and acquisition environment gate

The Pantograph `puma-lib` execution path no longer converts a failed selected-model detail query into a successful node carrying only saved model identity. A red regression reproduced the prior success with `{model_id, pumas_model_ref}` and no dependency state when the producer returned a descriptor error; the consumer now returns a typed execution failure and preserves the producer diagnostic. Workflows without selector access retain their saved-data behavior. No model-specific profile was added and resolved `bindings=[]` semantics are unchanged.

The focused `puma-lib` tests pass 7/7, embedded-runtime library tests pass 482 with 1 ignored, workflow-service library tests pass 880/880, the resolved-empty admission regression passes 1/1, and formatting/diff checks pass. This repair does not close real text/image scheduler fan-out, desktop inspection, or saved-workflow cold reopen.

A bounded released-Pumas single-GGUF acquisition qualification did not reach RPC: the exact release source/binary exited during SQLite startup with `attempt to write a readonly database` in fresh disposable roots and when checking the retained Qwen text root. No intent/acquisition request, metadata change, inference route or Pumas source change occurred. This is a host/runtime storage gate, not a contract rejection. The next action is one bounded acquisition retry on a host-provided writable service root; do not repeat the failed startup probe or synthesize identity/path evidence.

The Coding-Standards MCP remains `restart-required` with host action `restart-and-reconnect`, which this session cannot perform. Current phase remains **R1 / RPC-01 intent-first consumer migration** with exactly one next slice: obtain a coherent text/image handle or supported acquisition/provenance result, then run generated-text scheduler fan-out, desktop inspection and cold reopen. Whole-repository standards compliance remains unclaimed.

### 2026-09-26 — release-configured acquisition, supported metadata review, and remaining artifact coherence gate

The frozen Pumas 0.7.0 release was qualified with explicit, independently owned writable state rather than the active Pumas checkout. The exact source is `29242fce4ec9043becaeb061587f364ecc4e7177` and the retained release binary is SHA-256 `61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80`.

- **Intent acquisition:** root `/tmp/pantograph-pumas-acquisition-config-probe2`, registry override `PUMAS_REGISTRY_DB_PATH`, endpoint `http://127.0.0.1:32943/rpc`. `intent_get_model` for the explicit upstream Qwen2.5 0.5B GGUF Q4_K_M requirement acquired 491,400,032 bytes and returned a producer-resolved model/artifact/revision. The resulting local package facts were identity-bearing and valid at the selected-file level, but intent re-observation remained `Incomplete/package_facts_stale` at `package_facts.inspection_manifest`.
- **Complementary supported acquisition:** root `/tmp/pantograph-pumas-direct-qwen2-20260927`, endpoint `http://127.0.0.1:44999/rpc`. The released `start_model_download_from_hf` operation acquired the same explicit file and producer artifact identity. The supported `submit_model_review` operation removed only the producer-created `metadata.json.bak` entry, preserving the selected GGUF identity, task and file evidence; overlay `7a51136a-9733-4145-882d-d8a36b806a9e` was accepted and marked reviewed.
- **Deciding producer evidence:** after review, `resolve_model_package_facts`, `refresh_model_index`, summary resolution and repeated LocalOnly intent observations still reported stale inspection facts. The package facts also reported a logical-size mismatch (`491400032` selected bytes versus metadata total `491403935`) and parsed the GGUF header as `MOSTLY_Q5_K_M` while the producer identity/request is Q4_K_M. In contrast, `resolve_model_artifact_load_target` independently returned a ready file target with the exact selected path, `gguf` kind, `file` load kind, library-owned storage and valid verification.
- **Classification:** this is a released-Pumas package-facts/cache and artifact-content coherence defect, not a Pantograph selector, dependency-profile or parent-directory problem. Pantograph must not invent an artifact ID, promote a path, guess a directory, substitute a Q5 artifact, or weaken identity validation. Pumas source remains read-only. The smallest next action is a producer-authorized coherent artifact/cache result, followed by one bounded intent re-observation.

The optional-dependency boundary remains accepted within its demonstrated scope. Effective Qwen metadata has `dependency_bindings:[]`, no custom-code requirement and valid text-generation task evidence; Pantograph’s standard-model regression does not request package readiness or fabricate dependency facts, and resolved-empty admission still waits for ordinary backend readiness before runtime-host dispatch. A failed dependency query remains fail-closed. This does not certify the unresolved Qwen artifact or establish the full scheduler workflow.

The deterministic pending assertion in `dependent_text_to_image_keeps_original_response_pending_until_downstream_finishes` now polls the public future once with a noop waker rather than relying on elapsed timing. The focused test passes 1/1. The registry (11/11), artifact-store (13/13), production embedded image runtime-host (1/1), and host WebKit image fixture remain green within their recorded scopes; the sandbox WebKit failure is pre-assertion `EPERM`. The primary next slice remains singular: obtain the producer-coherent handle, then execute real generated-text fan-out to image and embedding, desktop inspection, and saved-workflow cold reopen. Standards host restart/reconnect and full product acceptance remain open.

### 2026-09-27 — bounded released-model requalification and current image runtime-host result

The retained Pumas 0.7.0 binary (source `29242fce4ec9043becaeb061587f364ecc4e7177`, SHA-256 `61abcd708edea18d7b9cec0fd089c9c45e18b311dcd82b65aa191f5f10038b80`) was used only from independently owned writable roots and explicit loopback endpoints. No active Pumas checkout, producer source, inference/serving route, `/v1` endpoint or proxy was used.

The bounded Tiny Aya re-observation at `http://127.0.0.1:34813/rpc` returned `Incomplete/artifact_path_invalid` for the producer-selected first-shard entry, while complementary package facts identify an `hf_compatible_directory` with two shards and a logical-size mismatch. A separate supported single-file TinyLlama GGUF acquisition completed 668,788,096 bytes and returned producer model/artifact/revision identity. Review overlay `211de3bc-9de1-4345-a230-f4fad7595328` supplied text-generation/llama.cpp evidence and left `dependency_bindings=[]`, but intent remained `Incomplete/package_facts_stale`; package facts also report a GGUF header `MOSTLY_Q5_K_M` against the requested/identified Q4_K_M artifact. These are released-producer directory/cache/content coherence gates, not missing optional dependencies or Pantograph selection defects.

The corrected current-host Tiny SD run at `http://127.0.0.1:34823/rpc` passed the existing ignored real runtime-host acceptance 1/1. Pantograph consumed the intent-backed producer handle through the unified PyTorch/Diffusers gateway and retained a complete 441,767-byte PNG. This accepts only the real image runtime-host boundary, not scheduler text fan-out, desktop inspection or cold reopen. The real Qwen embedding scheduler recheck reached the intent/load-target boundary and failed closed on producer `Incomplete/package_facts_stale`; no inference was called.

Focused consumer evidence remains green: requirements registry 11/11, artifact store 13/13, production embedded image runtime-host 1/1, full embedded-runtime library 482 passed/1 ignored, workflow-service library 880/880, and host WebKit fixture checks passed. The Coding-Standards runtime still reports `restart-required`/`restart-and-reconnect`, and Passeur remains unbound; no current standards closure is claimed. Current phase remains **R1 / RPC-01 intent-first consumer migration**, with exactly one next slice: obtain one producer-coherent text handle, then execute generated text feeding image and embedding through the real scheduler, followed by desktop inspection and saved-workflow cold reopen. The full product goal and whole-repository standards compliance remain open.
