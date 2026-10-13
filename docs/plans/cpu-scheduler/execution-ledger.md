# CPU Scheduler Execution Ledger

## 2026-10-12 — M0 planning admission

- User authorized remaining CPU-testable scheduler work after a standards-compliant
  phased plan and independent design review; GPU/split and remote workers deferred.
- Verified source at main `1fba975ab20cb032949aa7221e373f0719e3264b`, tree
  `6d3c8f96039af1616c3ea3ac97c177b5fdbc922d`. Prior final-main Quality,
  Runtime Separation, Contract and CodeQL workflows passed; no new scheduler
  execution claim follows from that baseline.
- Read repository architecture, ADR-011/017, plan authority and current scheduler
  source/research. Selected explicit canonical standards baseline `188beda1`.
  No applicable local skills/AGENTS files found.
- Hardware discovery: five exposed logical CPUs, four-CPU cgroup quota, 16 GiB
  cgroup memory ceiling, no exposed GPU device paths or vendor tools. CPU use is
  available; GPU execution remains unverified. No hardware provisioning performed.
- Isolated plan branch/worktree from exact main. Existing signed merge objects
  were retrieved through the authorized GitHub Git Data reader and reconstructed
  only after exact SHA verification; no branch/history was rewritten. CLI Git
  authentication remains unavailable; connector publication is the supported route.
- M1 design candidate preserves constructor-owned serial ports, ordinary hosted
  facts/readiness/candidates and linear cleanup; real first-two native evidence
  closes the concrete gap. Cold bootstrap/retaining lease are preconditions.
- Independent feasibility review (`dependency_review`): approved for M1 with no
  remaining blocker. Incorporated the independent raw-duration oracle, fixed graph/
  run cohort, equivalent baseline, caller owner preconditions and numeric bounds.
  Cancellation without matched cleanup remains fenced; no recovery is admitted.
- Independent standards/design review (`scheduler_standards_review`): approved for
  M0 closure and M1 start, no material blockers. Reviewed all six document changes
  and concrete composition/service-config/serial/native-test seams. Incorporated
  conditional persistence/evolution routing for durable diagnostic changes and
  separated controlled C4a from actual CPU C4b evidence. Reviewed substantive plan
  SHA256: `746828c2b9fe7c7c14aae0a85ea911a8826e931ce98efdc22f5a2238e3d29374`;
  subsequent edits record approval/publication lifecycle without changing design.
- Validation: 51 local Markdown targets resolved across the six changed files;
  staged whitespace and decision traceability passed (six changed paths, zero
  mapped production impacts). The new worktree initially lacked `commonmark`;
  reusing the existing ignored node_modules directory allowed the normal gate
  to run without dependency downloads. Declared pre-push `npm test` passed offline:
  node-engine 261 passed/one ignored, workflow-nodes 168 passed. The existing
  inference dead-code warning remains. These baseline tests do not establish new
  scheduler acceptance. Conventional commit-message format was checked; no active
  Git hooks were present and no hooks were disabled. Branch publication does not
  trigger the PR/main-only CI workflows; no PR or main mutation is performed.
- No production code, model acquisition, paid inference, Pumas/private catalog,
  paused multimodal, default-policy or main changes. M1 implementation/evidence
  remains pending after this reviewed plan publication.

This worktree is retained-protected for the plan's continuing implementation
coordination; parent owns its later disposition after reachable handoff.

## 2026-10-12 — M1 implementation and first runnable result

- Created isolated feature worktree/branch
  `feat/cpu-scheduler-retained-ready-2026-10-12` from reviewed/published plan
  `42d03b3fb65f5172be8c956af6e880c1255b354b`. Paused multimodal, PR71, main,
  Pumas and the original plan worktrees remain untouched. The user's continuation
  also authorizes a draft PR for ordinary CI; no merge/deployment/release.
- Preserved constructor-owned execution/batch/cleanup ports through hosted
  composition; ordinary facts/readiness/candidates still install. Mode conflicts
  reject before startup side effects. Shared gateway/registry/selector and a live
  retaining bootstrap session are explicit caller prerequisites.
- Replaced the opaque selection result with a private bounded typed reason and
  proposal, projected through the existing log facade outside store/history
  locks. Added only a read-only test-support view of the existing raw sample ring.
  No durable event/version, IPC, default, Pumas, or execution authority changed.
- Actual qualification exposed a public readiness-resume path that bypassed
  supervised TaskWorker ownership. Before editing that API boundary, independent
  review approved delegating the same existing run through the runtime facade
  only for serial mode. The ordinary path stays unchanged.
- Actual Ready execution then refused omitted VRAM accounting. Before changing
  the probe declaration, review confirmed existing known-zero semantics and the
  calibrated backend's CPU-only tensors. Kept all candidate claims and validation;
  declared retained RAM 32 MiB (operator envelope), VRAM 0. No free-capacity guess.
- First completed dev cohort: eight identical baseline runs and eight serial
  runs; all 32 returned vectors match independent reference (maximum absolute
  error `8.940696716308594e-08`). Three FIFO training runs, then five observed
  second-position starts, independently matching raw maxima and actual ledger
  order. Serial run 0 resumes readiness with the same final run identity. No
  adaptive retries/winner selection; earlier failed integrations are not accepted
  cohorts. Completed evidence root: `cpu-m1-real-20261012g`.
- Independent review recomputed output/sample/order oracles and found no material
  implementation blocker. A clock regression now checks typed InvalidClock
  discards partially accumulated ranking evidence; injection is private cfg(test).
- New native cancellation tests request cancellation after actual native drain
  and before cleanup, using the untouched real response/receipt. Matched cleanup
  allows another real forward; a deliberately lost receipt preserves charge and
  fences the next attempt. Metadata is controlled and explicitly separate from
  ordinary hosted C2 proof; no cancellation-during-computation claim from these
  new wrappers. Both passed, as did 25 focused composition tests.
- Shutdown returns with no reservations/native serial owner. Registry status can
  conservatively remain `stopping` with declared resident accounting; do not
  mistake `shutdown_ok` for complete accounting-row removal.
- Release cohort, final broader gates, publication and CI are recorded in the
  [M1 report](reports/2026-10-12-retained-ready-m1.md) and final handoff once
  complete. C5 measures selector-only scheduling cost alongside task/queue/wall
  times; total scheduler CPU remains unisolated.
- Final independent source review identified unsupported supervised serial
  deadlines. Standards review approved fail-closed timed run/resume refusal
  before admission/enqueue; default EmbeddedRuntime resume retains its old path.
  The new public facade adds no deadline enforcement to generic supervised
  recovery. The final probe uses no workflow timeout and labels its caller
  watchdog separately. The earlier dev cohort proves output/order, not deadlines.
- Final optimized build used cached dependencies (14m 24s; exact-source rebuild
  0.43s), then one predeclared release cohort with no concurrent compilation/model
  work. Both compositions/producers were alive for baseline and serial phases.
  All 32 vectors and all raw-sample/proposal/actual-order oracles pass. However,
  all five ranked runs correctly choose position zero from observed maxima. The
  final-source second-position criterion remains pending and the probe exits 1;
  no cohort retry, graph/input alteration or oracle weakening follows. Both
  reviewers require C3 Partial and M1 not fully accepted.
- Release measurements: baseline wall median 54.031 ms, serial 63.955 ms; selector
  cost 16.505–41.403 µs. C5 passes as a descriptive scoped report, not a speedup.
  Total scheduling CPU is unisolated. Raw compressed JSON/JSONL, per-run summaries
  and SHA-256 manifest are retained under the report's `m1-evidence` directory.
- Full final workflow-service suite: 979 passed, including new clock and timeout
  regressions. Final embedded/pre-push gates and external CI follow in handoff.
- Full final embedded-runtime suite: 592 passed, two existing tests ignored;
  this includes the new actual native cancellation and composition regressions.
  Independent reviewer inspected final release originals/hashes and 979-test log,
  approved source/report with C3 Partial and C5 scoped acceptance.
- Final declared pre-push `npm test`: node-engine 261 passed/one existing ignored;
  workflow-nodes 168 passed. Workspace rustfmt, staged whitespace and decision
  traceability pass; no hook was disabled. Remaining ignores are existing cost
  probes/node-engine exclusions. No new ignore/suppression/dependency acquisition.
- Two independent reviewers approve draft publication with M1 Partial: C3 final
  second-position criterion remains open, C5's measurement obligation is accepted,
  and C6 awaits exact published-head/ordinary CI evidence. Source/evidence remain
  reviewable; no further milestone is started by this draft.

## 2026-10-12 — published draft CI correction

- Published and independently verified draft PR #73 at
  `743e783cad979940bb0eea12fb642d36bd1614f2`, tree
  `67f4d8127c3758afa4f501227185c77428d0654e`. All blobs/tree/ref and local
  fast-forward matched; published main-to-head traceability passed 34 paths.
- Normal CI found one new test-only `await_holding_lock` lint at timeout-refusal
  assertions. Replaced explicit guard drop with a lexical assertion block,
  preserving production behavior and adding no suppression. Measurement/source
  manifests remain bound to the initial qualified snapshot; no new model cohort
  is run for this test-only correction. Independent review and targeted lint/tests
  precede the ordinary fast-forward publication. C3 remains Partial.

## 2026-10-12 — C3 causal review and prospective supplement

- Final published implementation `938bc89ea381e089ecf4273d578324698959ddb4` passed
  all four ordinary CI workflows; independent review confirmed C6. C3 stayed
  partial. The worktree and remote draft PR #73 matched, with no merge/deployment.
- A bounded read-only causal audit independently recomputed all eight release
  raw sample/proposal/Start oracles and verified all 13 preserved checksums.
  All five ranked choices followed the lower observed maximum; no policy or
  wiring defect was found. Phase-specific causes remain unisolated.
- Independent standards review rejected oversized raw-input proposals after
  finding the stricter 1,024-byte ordinary String embedding limit. No capability
  change, TranscriptText workaround or exploratory inference was performed.
- The user authorized one supplemental protocol, `c3-768b-v1`: exactly 768-byte A
  and five-byte B, independent reference verification, eight baseline then eight
  serial runs, raw duration/output/order evidence, and an unconditional stop with
  no retries or tuning. The original exit-1 and slower serial baseline remain
  unchanged and visible. The full prospective amendment is in the canonical plan.
- This documentation publication precedes any new reference/model execution.
  Independent confirmation of the published amendment is required before the
  fixed experiment. C3 remains partial; no further milestone is admitted.

## 2026-10-12 — single c3-768b-v1 execution and review

- Published the prospective amendment at `b158dc24e959924bb0481ef18d1be35705833bfd`,
  tree `cc507dd52b4811ec1183919e1a456a1774f15b75`. Independent review verified the
  remote commit/tree/parent and exact document blobs before reference execution.
- Added only an explicit supplemental example mode and independent reference
  script; no production/default/capability change. Review caught a failure-recording
  gap and approved saving captured attempts/reference outputs before propagating
  failure. Published reviewed source `09b1aaa58c39fdf976e98aaa130abda890fd3776`,
  tree `8479f35b5b8f857a7419372d4aca53c45ab0b5a5`, before execution.
- Corrected release build succeeded in 1m30s; formatting, whitespace, traceability,
  declared pre-push tests (261 + 168 passed, one existing ignore) and focused
  warning-deny example Clippy pass. A 16:15 disconnect notification did not
  interrupt an experiment: the 16:16:29 command succeeded, build was finished and
  no invocation marker existed. No build or experiment was restarted in response.
- One independent reference execution passed exact input IDs/masks/types and one
  CPU forward, error 0. One fresh hosted owner/bootstrap then completed exactly
  eight baseline plus eight serial runs. Exclusive invocation markers and exit
  records prevent accidental retry. No tuning, seed choice or subsequent cohort.
- All 32 hosted vectors pass (max error `8.940696716308594e-08`). Independent review
  checked all 32 retained input bodies, every sample/count/maximum, both Starts
  and append-only history. Runs 0–2 were FIFO; runs 3–4 selected B then A from
  maxima 13.471875/12.854651 ms. Run 4's B sample 21.126173 ms caused runs 5–7 to
  correctly select A then B. Probe exit 0; reviewer accepts the missing C3 case.
- Supplemental wall medians: baseline 55.0731265 ms, serial 58.903377 ms. No speedup
  or isolated preprocessing-causality claim. Original exit-1 cohort, slower
  54.031291/63.954510 ms medians, original report and all 13 checksums are unchanged.
- New summary/reference/source manifest and full raw exports/retained bodies are
  packaged separately under `reports/c3-768b-v1`; every run is reported in the
  [supplemental report](reports/2026-10-12-c3-768b-results.md). Final source/evidence
  review, published-head verification and normal CI govern handoff. No further
  experiment, M2, GUI, merge/deployment, GPU or remote work is admitted.


## 2026-10-13 — M1 reconciliation and bounded M2 admission

- Fetched main and independently confirmed PR73 is merged, merge commit
  `07b23d47b2cdfae5669bbda78515c9c728081031`, final M1 head `8ca768e2`.
  GitHub metadata confirms all four final M1 workflows passed. The original
  negative/slower cohort and one accepted C3 supplement remain unchanged.
- The new saved checkout uses branch `feat/cpu-scheduler-m2-lookahead` at that
  main base. Initial worktree/index were clean; original `work` and integration
  branches are preserved. No applicable AGENTS.md exists. Current canonical
  standards `188beda1` remain the selected baseline.
- Independent GPT-6.1 Sol / High reviewer `m2_contract_review` admitted the
  bounded predecessor-conditioned retained-CPU service advisory in the current
  plan. It required exact owner snapshot + generation Arc association,
  read-only workload/predecessor evidence, and all remaining runtime obligations
  checked inside advice. These conditions were recorded before source changes.
- The existing phase-cost lookahead kernels cannot consume service-only samples;
  no cold/preparation/transfer phase is assigned zero. This M2 subset remains a
  first-order service approximation with two complete orders and unchanged M1
  actual dispatch; general cohort completion is unqualified.
- Workers explicitly use GPT-6.1 Sol / High, with disjoint numeric-kernel and
  serial-policy write sets. This integrator owns the one current plan/evidence.
  The prior turn checkpointed when its model was unverifiable; the fresh turn
  was explicitly configured by the parent, so no model-inspection loop is used.
- Cross-thread messaging to text worker `01a1374d-c69c-74e4-aeab-98aaccad303d`
  is unavailable in the exposed collaboration tool. Requested parent relay of
  the small probe module/argument boundary. No shared readiness-provider edit
  or native owner contract change is planned.
- Plan operation is `continue`. M2-C1..C4 remain pending; one fixed actual
  calibration cohort is prospectively defined. Implementation admission does
  not imply successful CPU evidence, speedup, default activation or M3–M6.


## 2026-10-13 — M2 core implementation and blocked qualification handoff

- Implemented the admitted pure numeric kernel and private serial policy in
  disjoint worker write sets. The host explicitly enables advice; default/M1
  dispatch remain unchanged. One bounded history owns samples and predecessor,
  and only real matched cleanup/current owner restores conditioning.
- Scheduler205 tests pass (one existing ignored probe); workflow-service988
  tests pass (28 serial tests, no ignores). Tests cover the independent hand
  oracle and failure/owner/freshness/population/cancellation invariants with
  controlled protocol fixtures. No actual new model cohort ran.
- Independent GPT-6.1 Sol / High reviewer `m2_source_review` approved the whole
  core source range and final lint-only corrections. Final serial policy
  SHA-256 `f35be2e7e9f7f986906d6670c47e4d74a97968df13c97e24c6caf65fc0ff7df6`.
  No model execution occurred during review.
- Scheduler all-target warning-deny Clippy and workflow package all-target
  `--no-deps` warning-deny Clippy pass. Composed CPU Clippy targeting inference
  fails on unchanged selected-text fields/OpenAI helper dead-code warnings;
  no suppression or other-owner edit was made. CPU-17 tracks the blocker.
- Declared pre-push `npm test`: node-engine261 passed / one existing ignored,
  workflow-nodes168 passed. Workspace formatting/whitespace pass. Exact local
  logs are retained in `/tmp/m2-policy-full-tests.log`,
  `/tmp/m2-policy-package-clippy.log`, `/tmp/m2-policy-cpu-clippy.log` and
  `/tmp/m2-declared-prepush-tests.log` in this saved environment.
- Independent producer review identified and resolved task/Start/key association,
  age coverage and explicit error teardown before any native cohort. The source
  and exact root/helper wiring patch are retained under
  `/workspace/scheduler-m2-pending/`; patch application check passes. Shared
  root/helper files remain unchanged pending required text-worker coordination.
- Cross-thread messaging could not address the supplied text-worker ID. Parent
  relay was requested; no answer arrived before this handoff. M2-C3 remains
  blocked and the scheduler worker retains its qualification responsibility
  after that shared boundary is settled. This is a reviewable core commit,
  not M2 acceptance. Ordinary PR CI also remains pending at publication.
- The [core report](reports/2026-10-13-m2-core.md) records commands, evidence
  boundaries and blockers. No M3–M6, GPU, remote, default activation, general
  completion or speedup claim is made. The feature branch is retained for PR
  review/qualification; parent is integration owner and main is unchanged.


## 2026-10-13 — continued native qualification admission

- Parent explicitly resumed end-to-end CPU qualification under Sol / High.
  PR75's published plan and diff at `2f1e38fd` exclude the embedding probe and
  scheduler write set. Preserve all text-dependency changes; no PR75 merge or
  file replacement is needed. This resolves the prior shared-scope blocker.
- Pre-run independent review found the original singleton-target design invalid:
  target selection does not prune scheduler tasks. No cohort was executed.
  Prospectively replace it with two derived one-branch graphs, preserving exact
  node/data/binding identity and retaining actual workload digest equality as a
  hard acceptance gate. The sequence and one-cohort stop rule remain unchanged.
- Independent `m2_qualification_review` admitted the replacement and the exact
  affected-owner CPU lint gate before code changes. The historical inference
  optional-consumer warning remains outside-slice debt, not a repaired config.
  The compiled source and numerical/evidence producer still require review before
  any model cohort. No GPU/remote or broader scheduler action is admitted.
