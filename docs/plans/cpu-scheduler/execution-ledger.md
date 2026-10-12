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
