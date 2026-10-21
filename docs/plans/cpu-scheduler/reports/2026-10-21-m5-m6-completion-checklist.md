# Bounded M5/M6 completion checklist

Operation `continue` under [the CPU plan](../plan.md). The user authorized PR89
integration and required a finite production/qualification boundary before more
instrumentation. This reconciliation supersedes an open-ended sequence of
allocation-observer slices; it does not accept overall M5/M6 from diagnostics.

## Supported production outcome and integrator decision

The supported CPU envelope is **ordinary per-member execution with bounded pure
workload-key reuse**, explicitly selected by an owning Rust host. Every actual
member still takes its own authenticated M3 opportunity, preparation, reservation,
final Start fence, native execution/drain and matched cleanup. There is no batching
wait, grouped native forward, shared grant, forecast-based admission or new memory
policy. Ordinary construction keeps its current behavior.

As serial integrator, select the ordinary-member option left open by the
[C2 boundary decision](2026-10-20-m5-bounded-grouping-contract-proposal.md#required-decision-before-runtime-integration).
This is the supported-envelope M5 target, not acceptance of native/continuous
batching or every possible C1 consumer. The existing C2a contract already defines
its cost-free compatibility projection and exact immutable-key cache. Accept this
target only after the actual CPU gate below passes; do not silently count an
advisory grouping proposal as executing a group.

## Finite required checklist

| ID | Required outcome and deciding evidence | Current disposition / stop condition |
| --- | --- | --- |
| INT-89 | PR89 exact head `48717570004be87735c4b693212fdadf154d8738`, tree `2d59d369f11b89e119c54afce9f0a6df86ada4bd`, independently approved for merge; normal merge preserves both parents; actual merge CI terminal. | Source had 22 successful checks/four PR workflows. Merged as `dc64b895180260ceef8619bedd1ac6d8b756a3ec`, parents `dc926783` and `48717570`, identical tree. Post-merge CI is pending; the integration receipt owns final run identities. Stop integration on an actual failed required check and diagnose permitted fresh evidence. |
| M5-SW | Bounded C1 library API and C2a ordinary-worker key reuse preserve fresh per-member authority, fallback, bounds and cleanup. | Implemented/merged PR85/86; independently reviewed focused and controlled-worker evidence exists. Do not add another scheduler, forecast store or allocator observer to close this row. |
| M6-SW | One supported owning host can explicitly choose that existing key-reuse mode and read its immutable configuration without exposing private authority. Existing constructor remains unchanged; shutdown/reconstruction is rollback. | Concrete missing production capability after PR89: `EmbeddedSerialCpuResidency::new` fixes fair+residency with reuse off and offers no choice/query. The reviewed implementation adds the explicit constructor and delegates the existing query. Default/inventory consumer tests pass; publication and required exact-head CI remain to decide this row. |
| M5-CPU | One prospectively fixed actual CPU baseline-versus-key-reuse comparison through public sessions verifies exact outputs, member/attempt identities, separate grants/Starts and matched cleanup, with fresh owner/proof checks. Capture actual cache behavior and all five existing phase spans; report elapsed observations descriptively. | Pending software qualification producer/protocol admission, then one retained execution using committed untrained BERT fixtures. Existing controlled 30-versus-16 encoding counts and isolated local times are not native acceptance. Missing instrumentation in this producer is software work, not an external-input blocker. Stop on the fixed protocol's terminal outcome, any missing criterion or infrastructure failure; preserve all outcomes and do not tune/retry for a winner. |
| M6-CLOSE | Independent exact source/consumer/evidence review, required exact-head CI and post-merge checks support the bounded host choice and operator instructions. | Pending M6-SW and M5-CPU. Close only the named ordinary-member envelope; retain other qualifications below. No UI, IPC, persistence, deployment or default activation is implied. |

These five rows are the completion boundary. After they pass, stop implementation
of this envelope and mark only these scoped claims accepted. If a row fails,
repair the evidenced defect within its contract or record a concrete blocked
disposition. Do not create another diagnostic domain instead of deciding the row.

## Validation that needs representative workloads or hardware

The committed BERT fixtures can qualify native ownership, numerical output and
the supported mechanism on accessible CPU hardware; they require no downloaded
model or invented measurements. They do not establish trained-model quality,
production throughput, tail latency, forecast accuracy, power use or practical
memory savings. Those claims require frozen representative model/tokenizer
revisions, inputs/shapes, settings, baseline and target CPU/hardware/environment,
with an independently reviewed measurement protocol. No such claim is an input
to current admission. GPU/remote qualification needs the separately authorized
suitable devices/hosts already excluded by the plan.

## Separate optional or deferred work

- Complete incremental allocation: PR89 completely follows its *registered
  original completed-result payload* domain after materialization, including
  Shared/Arc retention; it does not observe original allocation starts or an
  instantaneous physical peak. Construction, allocator metadata, inputs/tokenizer,
  model residency, Candle intermediate/shared aliases, GEMM scratch, Rayon/workers
  and caller copies remain outside that domain. Dependency allocation/drop hooks,
  alias identity and scratch/worker context are necessary for a **future complete
  memory claim**, not for the cost-free C2a path. Do not substitute RSS or
  reservation estimates. No new allocator slice is selected by this checklist.
- A full C1 memory-aware dispatch consumer, native/continuous batching, general
  preemption/load overlap, automatic recovery, UI/persisted selection and default
  activation/deployment require their own concrete consumer, design admission and
  qualification. None is silently reclassified as completed.
- Broader real-model, multimodal, GPU and remote acceptance remains with its
  existing authority. Optional telemetry expansion must name a decision it changes
  before it delays this finite checklist.

## Next software slice and review boundary

Write set: `serial_cpu_residency.rs`, its existing tests, the host-options guide,
this checklist and canonical CPU plan/ledger/issues. Add
`EmbeddedSerialCpuResidency::new_with_bounded_key_reuse(config)` and
`scheduler_configuration()` returning the existing value snapshot. Reuse one
private constructor implementation and existing builders; preserve config struct
literal compatibility and all existing lifecycle/feature gates. No new options
framework, stored snapshot, policy, dependency, manifest/lockfile, IPC, native
allocator or binding mechanism is needed. This is one usable consuming-path change.

Composed-design admission is not applicable to a replacement composition: the
same protected facade/service/port/registry/native owners and shutdown protocol
remain, with one existing constructor-owned boolean selected explicitly. This
adds no ownership/lifecycle boundary, state owner or operational obligation. If implementation needs to
change ownership, grants, Start fences, resource claims or shutdown, stop and
re-admit that design instead of inheriting this bounded decision.

Baseline strict Rustdoc and narrow-feature lint limitations recorded in the
[PR89 report](2026-10-21-result-payload-attribution.md) remain unsuppressed.
Historical denied log reads remain denied evidence, including PR88 main jobs
`119033798758`/`119033798732`; no retries or alternate routes are authorized.

## Consumer implementation checkpoint

The implemented public factory is `new_with_bounded_key_reuse(config)`; existing
`new(config)` still selects false through the same private constructor. The
immutable query delegates directly and remains readable after acknowledged,
idempotent shutdown. Config struct literals, public run/resume inventory gates,
private owners, resource policy and native cleanup code are unchanged.

Local verification: default facade module8/8; full embedded library607passed,
2existing ignored controlled cost probes; inventory-feature facade module6/6;
all-feature/all-target own-package warning-deny Clippy; warning-deny own-package
Rustdoc; formatting;33 resolved local documentation links; declared pre-push
261passed/one existing ignored+168passed. These checks establish this consumer's
configuration and regression behavior. The existing native lifecycle case uses
its original constructor; no new key-reuse CPU comparison is claimed.

Preserved failures: the first no-default/Candle compilation selected a profile
incompatible with existing llama-specific test helpers and caught one corrected
new-test Option access; no cases ran. The first default test execution passed2
and failed6 because Pumas could not create its default metadata directory under
the restricted home path. Pinned Pumas supports `PUMAS_REGISTRY_DB_PATH`; using
that process-local override for fresh scratch metadata made both corrected
module runs pass. No dependency code, HOME, authentication, permissions or
security settings changed. An exec transport interruption was followed by a
successful workspace recovery check before one retry of the failed read-only process creation;
local edits and active test processes remained intact.

Both independent pre-code reviews admitted this boundary, and independent
source/test review approved the implementation. The published draft owns exact
head/tree and hosted evidence. This checkpoint does not accept M5-CPU or overall
M5/M6, authorize the native protocol, or supply independent review credit for a
CodeRabbit status. Stop adding instrumentation; close the finite rows instead.
