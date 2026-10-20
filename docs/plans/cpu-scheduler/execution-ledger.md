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


## 2026-10-13 — reviewed producer source freeze before one CPU cohort

- Applied the admitted embedding probe mode and singleton/pair construction.
  Independent GPT-6.1 Sol / High `m2_qualification_review` approved final producer,
  exact task/digest/Start association, separate numerical references, bounded
  log collection and explicit teardown on execution/export failure. This is
  pre-run source review; no actual cohort evidence yet.
- Release CPU build passed with locked dependencies, no model download and
  `backend-candle,host-dependency-inventory` only. Primary affected example
  Clippy passes with `--no-deps -- -D warnings` under the identical CPU features.
  The first dependency-wide invocation failed in unchanged inference before
  reaching the example; retain `/tmp/m2-native-example-clippy.log` and CPU-17.
  Reviewer admitted scoped owner lint consistent with existing workflow gate;
  it does not claim the dependency configuration is lint-clean.
- The corrected admission was published as `cccf299c` before construction code
  changes. Freeze this source and compiled binary before exactly one fresh
  isolated AABBAABBAABBA singleton calibration plus one actual two-Ready pair.
  Preserve all raw failures; no timing/winner tuning or automatic retry.


## 2026-10-13 — one actual M2 CPU cohort and independent evidence review

- Executed exactly one invocation from clean frozen/pushed `7a7165d4`, release
  binary SHA `dd6c01eb30bb11c8b71c3d528532a10d109155dbe840ccfefdad292292d25fc2`.
  Four CPU equivalents /16 GiB, no accessible GPU; calibrated Candle CPU F32
  and one private thread. No model acquisition or winner retry.
- AABBAABBAABBA singleton calibration plus one two-Ready pair passed. Exact
  task/sample/Start populations and workload digests match, generation30,
  four conditioned counts3 each. Raw-only maxima yield A→B terminal35,474,725ns,
  sum51,214,065ns versus B→A terminal36,273,953ns, sum56,009,338ns. Advice and
  unchanged M1 actual dispatch both select A. No order-reversal/speedup claim.
- All15 cohort vectors pass the separate original numerical reference, maximum
  absolute error8.940696716308594e-08. All15 CPU Starts have successful matching
  completions and all14 runs succeed. No cleanup errors/reservations/native
  serial owner remain. Existing registry stopping/residency metadata persists;
  physical eviction is unqualified.
- Independent GPT-6.1 Sol / High `m2_final_review` approved whole-range source,
  task/digest/owner/freshness/output/cleanup evidence and raw-only hand arithmetic
  without consulting producer oracle/summary. All35 archive files match raw
  originals/member hashes; archive SHA
  `4a5eb82caac045ac2c864b904d2add515944395379585640208f755c0c16dd42`.
- The [actual report](reports/2026-10-13-m2-cpu-results.md) supersedes the core
  checkpoint's coordination/unexecuted-producer handoff; original M1 evidence
  and hashes remain unchanged. M2-C1/C2/C3 satisfied; C4 awaits final ordinary
  PR CI. PR75 sources remain untouched; CPU-17 remains disclosed source debt.

- Final supporting logs are packaged with the actual report. Scheduler205
  (one existing ignore) and workflow988 (zero ignores) pass again; their
  respective all-target warning-deny owner Clippy gates pass. Three existing
  actual native worker lifecycle regressions pass, separately from the fixed
  cohort: normal linear cleanup, post-drain cancellation without training and
  lost-receipt fencing. No wider native/desktop qualification is inferred.

- Final independent report/plan/ledger publication review approved: arithmetic,
  truthful scope/supersession, all12 delivered-file checksums, ten supporting-log
  hashes and32 relative links pass. Staged traceability and range no-new lint
  pass. M2 remains Verifying solely for ordinary final-head PR76 CI.


## 2026-10-14 — accepted M2 and authorized PR76 integration

- Exact reviewed head `8f9afd72a9e4b83b43e049722161eeaac5e933d3` passed all
  required CI: [Quality, all13 jobs](https://github.com/MrScripty/Pantograph/actions/runs/38484792616),
  [Headless](https://github.com/MrScripty/Pantograph/actions/runs/38484792619),
  [Runtime Separation](https://github.com/MrScripty/Pantograph/actions/runs/38484792765).
  Independent fresh integration review confirmed unchanged base `07b23d47`,
  coherent branch history and disjoint PR74/75 scopes. M2-C1/C2/C3/C4 satisfied.
- Earlier merge attempts were rejected by automatic approval review because
  generalized or quoted delegation did not establish trusted user authority.
  No alternative route or main mutation followed those denials. A fresh user
  approval specifically answered the PR76-to-main question; the normal pinned
  GitHub merge then succeeded. This does not authorize merging other proposals.
- PR76 merge commit `0f6972af3f70da75479f176d12c3bc57ea0d318e` preserves parents
  `07b23d47b2cdfae5669bbda78515c9c728081031` and
  `8f9afd72a9e4b83b43e049722161eeaac5e933d3`. Remote main, exact ancestry and
  tree `129c5471ced07a4ce1f2adb637d64ca7860a75c2` match. The entire merge tree
  equals the reviewed PR head; all12 delivered-file checksums remain valid.
- The feature branch is integrated; no history rewrite, source changes, extra
  model cohort, prediction-authorized cleanup or default activation occurred.
  PR75 remains draft with its T3 real-model hold; no text-owner code was edited.
  CPU-17 and existing conservative stopping/residency accounting stay disclosed.
- This documentation-only follow-up owns acceptance/authority reconciliation
  and actual merge identity. Overall plan stays Active; M3–M6 need separately
  reviewed contracts. [Integration report](reports/2026-10-14-m2-integration.md)
  separates successful PR CI from post-merge push CI; no unavailable check is
  reported as passed. Follow-up merge permission is not inferred from PR76.

- Independent GPT-6.1 Sol / High `m2_final_review` approved the four-file
  documentation-only publication: all36 relative links resolve, source/raw
  evidence is unchanged, acceptance/authority/PR75 hold and separate main CI
  observation are accurate. Formatting, whitespace, staged traceability and
  no-new lint pass. This review approves publication, not the follow-up merge.

- Final post-merge verification: exact main `0f6972af` also passes all normal
  push workflows: [Quality, all13 jobs](https://github.com/MrScripty/Pantograph/actions/runs/38492288367),
  [Headless](https://github.com/MrScripty/Pantograph/actions/runs/38492288374),
  [Runtime Separation](https://github.com/MrScripty/Pantograph/actions/runs/38492288359).
  Remote main remained the recorded merge; no code/evidence change or new
  qualification claim. The approved follow-up records these final observations.


## 2026-10-19 — bounded M3 admission

- Fetched current main `4aec0d5c4927a6af2832d40e224be5c2136fc9ef`; original
  clean `work` checkout at `ca9edde6` remains untouched. Created separate
  `feat/cpu-scheduler-m3-bounded-service` worktree `/workspace/pantograph-m3`.
  No applicable AGENTS.md exists; standards `188beda1` read from exact checkout.
- Independent GPT-6.1 Sol / High reviewer `m3_design_review` admitted the M3
  contract before production edits. It identified a concrete stale-key-ticket
  starvation counterexample: three backlogged keys can cycle forever ahead of
  a fourth. The contract refreshes service-order tickets per grant separately
  from member FIFO, and includes an explicit regression trace.
- Opt-in post-Ready entitlement opportunities use existing authenticated
  client/bucket session attribution, 64 pending registrations / 32 units and
  two other-unit bypasses. Active-run base priority applies at this boundary;
  existing per-session priority/FIFO/starvation/warm policy stays intact.
- Preparation is outside the serial permit in this opt-in only. Its CheckOnly
  readiness/non-runtime progression grants no native execution authority. The
  waiter bound does not bound all supervised preparations. Source review must
  prove RAII removal, no lost wakeups, checked identities, final representative
  fences and turn retention through serial scope release/poison.
- M3-C1..C5 remain pending. No native cohort, speedup/default, M4 or PR75
  adapter/model work is claimed. User authorizes focused PR publication; parent
  coordinates integration and no merge/branch deletion is performed.


## 2026-10-19 — M3 source and deterministic review

- Implemented bounded entitlement policy and owned async waiters in separate
  scheduler/workflow owners. The native serial permit is released before the
  fairness turn wakes another waiter. No physical accounting or cleanup contract
  changed. Missing attribution refuses before preparation/runtime claims.
- Independent GPT-6.1 Sol / High `m3_source_review` found a final-Start gap after
  awaited candidate preparation. Fixed it by carrying the original attribution,
  priority and first Ready member through to the existing final store lock,
  alongside the current M1 pair and selected-task fences. Its no-pair async
  mutation regression proves refusal with no attempt/native forward/cleanup.
- Reviewer approved the corrected source and deterministic evidence: workflow
  library1008 passed,0 failed,0 ignored. This includes13 hand-oracle policy tests,
  2 async registration tests and5 fair-worker tests. Existing priority/FIFO/boost,
  M1 ranking, M2 advisory and native-owner controlled regressions remain green.
- Native producer review exposed that cancellation before forwarding yields no
  drain receipt. Prospectively choose a genuine post-drain public cancellation
  request instead; do not claim compute interruption or fabricate Cancelled.
  Five sessions across two authenticated units (4 foreground pairs+1 background
  pair) ensure a competing foreground waiter at protected background selection.
  Minimum8 samples fixes M1 to insufficient-sample FIFO for this cohort.
- Independent `m3_design_review` admitted the final producer SHA256
  `6d4e4d4a2cb512c44a661c56b032217c40f67e39a9701fb1f74cee0ce603fdc6`,
  including direct attribution, charged-lease and cancellation/cleanup ordering
  oracles. Release build/affected lint/source freeze precede any model execution.
- The initial documentation checkpoint had no installed Git hooks in this saved
  checkout. Its manual traceability call first failed due to missing node_modules;
  linked the existing pinned dependency directory and reran the declared range
  check successfully. No hook was disabled, credentials/settings changed, or
  unrelated product work overwritten. Subsequent checks use explicit inputs.

- Core checkpoint `dd9b800cc0dc2ee7169235e67fb1a5526b1f5f25` is pushed and
  remote SHA verified; workflow all-target `--no-deps -D warnings` Clippy passes.
  Declared pre-push tests pass261+168 (one existing node-engine ignore);
  scheduler205 pass with one existing cost probe ignored. Formatting, critical/
  accessibility lint and explicit staged traceability pass.
- The saved environment disconnect callback at23:07 was checked against actual
  shell/repository/process state. Access remained intact, local+remote checkpoint
  matched, producer edits were preserved and the original release/lint processes
  continued. No duplicate worker or test restart occurred because of the callback.
- The affected CPU example warning-deny Clippy passes after one lint-only nested
  if collapse. Independent reviewer verified reversing only that edit reproduces
  the prior reviewed digest; final producer SHA256 is
  `864d371a4d5c5abb90186b9deef2859a14e86c9ec99862b7d3e44a8170b18d1e`.
  This remains pre-run admission. Existing inference optional-consumer warnings
  CPU-17 are unchanged; no other-owner code or suppression was added.

## 2026-10-19 — M3 fixed cohort and oracle disposition

- Frozen clean source `e9d89eacf686b163539c08ebc6225a6c2750bc96`, tree
  `f7ec94bd6f550975b58f30b96603bf0b80401ec7`; release binary SHA256
  `74ad1349267b2251299e10c7c6b650575669893402b0fd00009db380766cb756`.
  The prospectively reviewed five-session/two-unit cohort ran exactly once,
  after independent numerical reference and source/binary freeze. All ten
  grants/Starts/native outputs/matching cleanup receipts agree. Background was
  protected at positions3 and7 despite waiting foreground priority100.
- Actual post-drain cancellation retained the charged lease through matching
  cleanup; completed output was allowed. Shutdown cleanup errors/reservations
  empty and native owner absent; CPU-14 metadata remains disclosed. Three actual
  native lifecycle regressions pass in the accepted default-feature offline
  configuration. Unsupported Candle-only whole-library build failure retained.
- Original process exit1 and failed oracle retained: the sample checker confused
  local task names reused across runs with full identities. Both independent
  reviewers admitted saved-data-only correction. Four pure checker regressions
  cover repeated names, missing/extra counts, foreign/duplicate full identities
  and actual cancellation state; no model/native retry occurred.
- Corrected analysis matches nine aggregate samples to the serial uncancelled
  return sequence after full-identity result/return join. Training exclusion is
  source-assisted using frozen cancellation/appending checks and lifecycle tests;
  raw samples expose no run/attempt IDs and cannot independently attribute timings.
- Evidence/report in `reports/m3-service-v1` and the M3 report preserve all original
  outcomes, hashes, supporting logs and correction provenance. M3-C1..C4 satisfied
  within admitted scope; C5 exact published-head CI pending.
- After independent M3 source review, identified a bounded M4 dependency: actual
  receipt-backed last-lease stop exists; serial retained cleanup/cold-load wiring
  and separate load/stop costs are missing. RetainIfNextSame else acknowledged
  StopThenColdLoad requires its own contract and actual BERT8/BERT12 evidence.
  No M4/default policy, overlapping load, recovery or native batching was added.

- Independent final report/evidence review approved the bundle: all41 raw members
  are byte-identical to retained originals, archive/log hashes match, production
  source is unchanged since freeze and the corrected checker/source-assisted
  limits are accurate. Design review approved corrected producer SHA256
  `8fa52e568df966dd783d0d3f6930a17789765667b371e0e04e8276525e626820`.
  Final staged traceability, formatting and whitespace checks pass.

## 2026-10-19 — M4 dependent contract and fixture preparation

- Parent explicitly authorized the next bounded retain/acknowledged-stop/cold-load
  slice on a separate dependent branch. Created `feat/cpu-scheduler-m4-serial-residency`
  in `/workspace/pantograph-m4` from preserved PR81/M3 head
  `b63aa3ebe5fab1531aef952a35ce74932bf76b30`. Parent monitors M3 CI;
  production edits wait for M3 acceptance and independently admitted M4 contract.
- Independent source/design research found consumed-task-receipt and cold-load
  authority gaps. Chose one native idle capability over actual cold-load-started
  guard custody, preserving existing registry last-lease stop/ACK invariants.
  No stale next-task peek, fake warm drain or separate generic registry is needed.
- Canonical draft records exact write set, actual post-grant/pre-candidate load-only
  preparation, prepared-owner final Start fencing, private facade exclusion,
  explicit ephemeral-session disposition and shutdown ordering. T4 helper/text
  paths, M3 tested head, PR75 model/adapters and PR79 root README remain untouched.
- Read-only fixture script verified all14 original manifest files and BERT8/12
  golden shapes/finite normalization, recording exact hashes and prospective
  A,A,B,B,A load/retain/stop oracle. No model forward, acquisition or regeneration.
- The23:33:44 disconnect callback was checked against actual shell/worktree/process
  state. Executor remained available, all heads/branch files intact, and no duplicate
  workers, builds or model invocations were started.

- Independent GPT-6.1 Sol High source/design reviewers admitted the revised M4
  contract after load-started guard reuse, private-facade exclusion, ordered
  shutdown and separate pre-Start native cancellation clarifications. Source
  reviewer checked plan SHA256 `aeb814ad14428783bf1366f422562e61891b1e16653f8076270de961af4ba2a3`
  before admission-status edits. Producer reviewer verified fixture script SHA256
  `d35d1ee9a406640cc65186775e17ba098891788d0c8f774589b432fe7e880a97`.
- Static accounting witness preserves existing per-kind conservative task hints
  and proves a conditional fitting additive envelope; it is not a native capacity
  freeze or memory measurement. Actual hints/future resident headroom must be
  validated before physical work. Producer phase boundaries and separate case
  owners/counts freeze before any native invocation. M3 acceptance still pending.

- Final source reviewer approved the conditional accounting witness and checked
  RAM32+48+1024=1104MiB and VRAM0+0+1024=1024MiB arithmetic against exact
  existing estimator/consumer source. It is a logical admission witness, not
  discovered hardware capacity or native memory qualification. All14 original
  fixture hashes and helper/manifest hashes match; Python parses and staged
  traceability/whitespace checks pass. No Rust production/test source changed.

## 2026-10-20 — Exact-head M3 review and independent M4 fixtures

- Retained all15 successful substantive CI job logs for PR81 head
  `b63aa3ebe5fab1531aef952a35ce74932bf76b30`. Fetched PR merge
  `a9292d48a49ac05ddec711397159077b3e68f9f0` has identical tree
  `defca3d306f4824726fa6094b2bba04a3128e570`; source-pinned CPU jobs check out
  the head directly. Actual format/Clippy command outcomes are success, despite
  advisory wrappers. Independent final source/evidence review approved C1–C4;
  all original native data/failure/reanalysis hashes remain intact.
- Quality summary118394536482 is still queued/null. M3 acceptance and M4 production
  remain pending; no status is promoted from queued to success. Detailed review
  and complete CI logs are in the2026-10-20 acceptance report and M3 CI bundle.
- Independent fixture work added one controlled registry composition regression:
  original load-started guard custody survives two separate task cleanups, full
  charges150running/140idle, mutation refusal, sole guard stopACK and stale frame
  rejection. Sole guard retained-task validation correctly refuses LastLease;
  native cap minting needs successor-free proof. This performs no native actions.
  Scoped registry13 tests pass; tests warning-deny Clippy, rustfmt and whitespace
  checks pass. Independent source reviewer approved SHA256
  `7eba158fe0e825beda3fac8c2295b9f44a09c81c16e6f95d6c49a6ff330bf298`.
- Facade feasibility refinement retains one execution-runtime owner across all
  calls/shutdown and keeps ordinary auto-resume disabled; scalar diagnostics leak
  no writable generation evidence. Native API research found existing supervised
  load-only Candle primitive and exact private receipt transfer path. All remain
  within the admitted write set; no M4 production code or model run occurred.

- Independent design review admitted a narrow four-path preparation amendment:
  host-contract Rust export, existing PUMAS package/load-target wrappers and target
  tests. Model-reference methods preserve all legacy selected-model/runtime-family,
  identity/OwnerFresh/error/privacy behavior. Actual intent+bounded text goes into
  preparation without a fake Started/candidate/lease/public-cancellation context.
  This closes the pre-candidate resolver feasibility gap before production edits.

- Independent final packaging review approved the pending-evidence checkpoint:
  all15CI log hashes/original bytes, compact and archived queued-summary metadata,
  fixture source/log hashes and unchanged original M3 bundle verified. Explicit
  staged traceability and whitespace checks pass. Final summary remained queued
  at01:22:35 UTC; no M3 acceptance or M4 production start is claimed.

## 2026-10-20 — M3 acceptance and local M4 production gate

- At02:23 UTC all4 exact-head workflows and all13 Quality jobs completed success,
  including actual summary118394536482. Independent final source review accepts
  M3-C1–C5 within the existing CPU/opportunity/source-assisted evidence limits.
  Frozen b63aa3eb/tree defca3d3 and original failed oracle/native cohort are unchanged.
- Added the16th actual summary log and full final API status to the CI archive;
  retained the earlier queued checkpoints as history and regenerated hashes.
- Parent explicitly authorized local M4 production on the existing separate
  branch. Native capability, preparation contract/resolver and workflow lanes
  now implement the independently admitted contract; no M4 cohort has run.
- Publication is still blocked by automatic review's nonpublic source/test egress
  rejection for local07138639 payload to MrScripty/Pantograph. Exact-payload approval
  remains unanswered; no retry, alternative upload, merge or deletion is authorized.
- Harmless pwd/status checks verified recovered execution access after the02:25
  disconnect; existing checkpoint and in-progress native files were preserved.

## 2026-10-20 — Local M4 implementation/review checkpoint

- Native linear residency implementation passes26 scoped actual CPU tests
  (8new plus18 existing warm/last-lease), with separate load/publication/stop/ACK
  intervals. Independent source review found no native/workflow lifecycle blocker.
- Workflow7new tests pass; full suite1015library+65integration pass,0ignored.
  Original missing-import test compile failure is preserved separately from its
  corrected successful rerun. Actual M3 grant/Ready/owner proof remains authority.
- Root port initial6tests passed. Independent port review found two P1 integration
  defects: pre-load accounting substituted package size for actual per-kind intent
  maxima, and generic lifecycle refusal rejected DispatchStarted. Fixes now use
  actual RAM/VRAM MAX hints (unknown/shrunk refuses), acknowledge exact Start without
  release, and permit only exact unstarted task lease cleanup without generic stop.
  Added regression tests await affected recompilation; these are not claimed passing.
- Independent design review admitted five separate acknowledged private facade
  lifetimes for the fixed stop-between-tasks comparison, versus one retained owner.
  Counts/outputs/ownership are comparable; startup/fairness/latency are not claimed
  equivalent. This avoids adding a production eviction API for a measurement.
- Private facade integration and fixed producer are in progress. No fixed M4 native
  protocol has run; source/producer/input/environment freeze and independent review
  are still required. Local-only publication restriction remains unchanged.

## 2026-10-20 — Local source/producer gate complete, execution not started

- Current contracts65, metadata/package14, port/facade11, native26 and workflow
  1015+65 regressions pass. Producer8pure saved-JSON oracle tests pass after
  independent findings corrected acquired claims, exact ACK, identities/order,
  pressure terminal/refusal-stage and bounded shutdown evidence.
- Independent source reviews accept private facade/supervised shutdown, common
  real-signal preparation refusal, optional native timing and scalar logs, typed
  input-bound refinement and the two prospective cancellation fixtures.
- All local failures/success logs retained in m4-local-v1 pre-qualification archive.
  No fixed protocol or either new qualification case has executed. Clean tracked
  source/tree, binary/input/environment freeze is the next gate. Local-only hold
  remains; no publication retry or alternative route is authorized.

## 2026-10-20 — Fixed M4 main failed; native prefix preserved

- Frozen7139b2ad/tree75852b54 release binaryc10590a7 ran the main protocol once
  at03:22UTC and exited1 after first A: `M4 retained vector artifact missing`.
  All71 original nonbinary files are archived with hashes in m4-main-failure-v1.
- Independent read-only review confirms a valid golden-matching vector, one
  load/forward/taskACK/finalstopACK and zero final charges. Empty projections
  caused the readback failure; optional Started lease assumptions were also wrong.
  No remaining main steps, comparison owners or pressure ran. C4 is Partial/Blocked.
- Metadata-only explicit refresh and corrected Started/Completed joins are under
  local repair/review. The two separately admitted cancellation cases have never
  run and require a new reviewed source/binary/input/environment freeze. Original
  failed main evidence stays unchanged; no main retry/continuation is authorized.
- Publication remains blocked by the same unanswered automatic-review rejection.
