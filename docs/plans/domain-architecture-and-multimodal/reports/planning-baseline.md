# Planning Baseline — 2026-09-08

This is a bounded planning inspection, not the full architecture or compliance audit.

## Inspected evidence

- `docs/plans/README.md`, old remediation portfolio/ledger and image-generation plan: old sequencing admits security M0 alone and image acceptance remains blocked.
- `ARCHITECTURE.md`: durable task scheduling is the target; desktop and bindings project headless Rust contracts.
- Root `Cargo.toml`: Rust workspace spans workflow, scheduler, inference, runtime, bindings and desktop; Pumas is pinned to an external revision.
- Root `package.json` and `docs/development.md`: frontend tests are manually enumerated; current guidance reports incomplete coverage and failing gates. These are historical claims to rerun, not fresh test results.
- `crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`: existing task orchestration is an important seam to trace rather than replace speculatively.
- `crates/pantograph-embedded-runtime/src/workflow_service_composition.rs`: existing factory composes registry, gateway, Pumas, dependency readiness and dispatch; M1 must assess how much caller knowledge it requires.
- Trust-policy references exist across inference contracts and Python worker code; their existence does not establish complete enforcement. The old trust finding needs producer-to-loader tracing.
- Both local Pumas proposals identify model-fact/load-target ownership concerns; they remain untouched.

## Standards used for this planning artifact

Source root: `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards`.

Core, Router, Planning, Implementation, Verification, Development Proportionality,
Documentation, Architecture, Contracts and Code Design govern the plan design.
Product implementation must route language, frontend, IPC, persistence,
concurrency, security, performance, diagnostics and other profiles from each
slice's actual facts. No language implementation changed in this planning pass.

Material differences from the old approach: composed-design admission requires
actual caller/change-path evidence; coherence matters more than counts; a new
checker or registry needs independent deciding value; bounded reversible
implementation should proceed once decision-relevant uncertainty is resolved;
multiple independent milestones may proceed with one integration owner.

## Limits

No complete call-graph inventory, external consumer inventory, benchmark,
hardware/model availability probe, baseline test run, or production design
admission was performed. The plan deliberately makes those bounded execution
tasks and does not assert that any particular old diagnosis remains true.
