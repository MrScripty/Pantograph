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
