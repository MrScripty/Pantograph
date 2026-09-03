# Plan: Editor-To-Artifact Image Generation

**Plan status:** `Blocked`

**Current phase:** Milestone 1 — Restore trustworthy validation and execution prerequisites

**Next slice:** After the security plan's model-code authorization milestone is accepted, capture the complete typed inference-validation diagnostics for the saved `tiny-sd-turbo-diffusion` workflow and fix only their canonical producers.

**Acceptance status:** `blocked`

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** `none`; detailed pre-consolidation history is available in Git.

**Related ADRs:** [ADR-002](../../adr/ADR-002-runtime-registry-ownership-and-lifecycle.md), [ADR-011](../../adr/ADR-011-scheduler-only-workflow-execution.md), [ADR-013](../../adr/ADR-013-workflow-version-registry-and-run-snapshots.md), and [ADR-014](../../adr/ADR-014-run-centric-workbench-projection-boundary.md)

## Objective

A user submits the saved image-generation workflow from the real desktop
editor; backend validation admits it; scheduler-owned task orchestration selects
runtime/device resources; the PyTorch/Diffusers worker produces an image; and
the retained artifact is visible in the I/O Inspector.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| IMG-A01 | The saved workflow's displayed ports, executable validation, scheduler materialization, and pre-dispatch validation derive from one backend-owned inference interface and preserve exact blocking diagnostics. | `integration` | `representative` | `automated` | `blocked` | Security authorization and validation repair pending |
| IMG-A02 | Every valid non-empty compatible assignment group of `1..N` dispatches through one scheduler/runtime-host batch path; a group of one does not expire merely because no peer arrived. | `integration` | `simulated` | `automated` | `pending` | Scheduler contract and orchestration evidence pending |
| IMG-A03 | Workflow-service and embedded-runtime affected suites pass without direct/request-scoped execution or fabricated model/runtime/device facts. | `integration` | `representative` | `automated` | `pending` | Baseline rerun pending |
| IMG-A04 | The real WebKit/Tauri editor submits the configured workflow and presents the retained generated image in I/O Inspector. | `user-workflow` | `required-real` | `automated` | `blocked` | Trusted local model/runtime and desktop environment required |
| IMG-A05 | No supported image path uses raw model paths, frontend/Tauri policy, alternate direct execution, or legacy fallback behavior. | `focused` | `not-applicable` | `automated` | `pending` | Bounded removal scan pending |

## Scope

### In Scope

- Backend-owned inference interface and executable validation for the saved image workflow.
- Canonical scheduler dispatch for compatible groups of one or more tasks.
- Runtime-host PyTorch/Diffusers execution and retained artifact projection.
- One real editor-to-I/O-Inspector acceptance path.

### Out Of Scope

- Alternative direct, request-scoped, script-only, or template fallback paths.
- Frontend/Tauri ownership of model, validation, scheduler, runtime, device, or artifact policy.
- General scheduler optimization, another image backend, or release packaging.
- Generated-component execution and broader workbench cleanup.

## Constraints And Assumptions

- Pumas owns model/package/artifact facts; Pantograph owns scheduling, runtime
  selection, admission, diagnostics, and workflow behavior.
- Workflow service owns validation/orchestration, scheduler owns admission and
  grouping, runtime registry owns residency, runtime host owns execution,
  ArtifactStore owns retained bodies, and the frontend owns projection only.
- Runtime reuse carries no workflow inputs or run-local state across runs.
- Missing prerequisites produce typed outcomes and never select fallback behavior.
- The 2026-07-28 failure inventory is historical until rerun on the current revision.

## Binding Decisions

| Decision | Owner | Rule |
| --- | --- | --- |
| Model-code authorization | [Security remediation](../current-standards-remediation/security-and-dynamic-code/plan.md) | Must be accepted before another real Diffusers run. |
| Inference interface | Workflow service and inference contracts | One descriptor governs editor, validation, materialization, and dispatch. |
| Dispatch cardinality | Scheduler | One batch envelope accepts a non-empty compatible `1..N` group. |
| Runtime handoff | Runtime host | Executes the scheduler decision without choosing policy or recovering missing facts. |
| User result | ArtifactStore and frontend projection | Retained bytes are backend-owned; UI only presents the typed artifact. |

## Systemic Finding Audit

- Invariant family and canonical owner: one backend-owned validation/execution path from saved graph revision to retained artifact.
- Bounded population: image interface resolution, executable validation, scheduler task materialization/grouping, runtime-host handoff, worker request, ArtifactStore, Tauri transport, and editor/I/O projections.
- Consumer dispositions: each occurrence is retained under its canonical owner, migrated to the canonical path, or deleted; no compatibility fallback remains.
- Stopping condition: IMG-A01–IMG-A05 are satisfied on one recorded revision and required-real environment.

## Simplicity And Ownership Review

**Applicability:** `applicable`

- Independent concepts and dimensions: graph validity, model facts, runtime residency, devices, reservations, grouping, task attempts, run-local input, retained artifacts, and UI projection remain independently owned.
- State, identity, value, time, policy, and mechanism: saved graph revision, model/package identity, runtime/device identity, task/run identity, collection window, policy, and worker transport are distinct and invalidate only their dependent proof.
- Caller and composition-root knowledge: callers submit session/workflow identity and typed input; the composition root supplies service, scheduler, runtime host, and adapters without exposing their mechanisms.
- Representative change paths and forced owners: a model-interface change touches its descriptor/validation consumers; grouping policy touches scheduler evidence; worker transport touches runtime-host contracts; artifact presentation touches the typed projection.
- Stable Interfaces versus hidden knowledge: workflow session, inference descriptor, scheduler handoff, worker contract, and artifact descriptor are stable Interfaces; Pumas layout, Python implementation, placement mechanics, and DOM rendering stay hidden.
- Independent evolution, testing, failure, and replacement: validation, grouping, runtime execution, retention, and UI projection have separate evidence and typed failures while composing in the real workflow.
- Necessary complexity and containment: device admission, runtime selection, model execution, and artifact persistence are inherent and stay in their existing deep owners.
- Deletion and cumulative machinery result: the consolidated plan deletes duplicate plan authorities and requires removal of direct/fallback execution rather than adding another orchestration layer.

## Milestones

### Milestone 0: Consolidate Plan Authority

**Goal:** Replace the append-only plan family and recovery overlay with this current plan, ledger, and issues file.

**Allowed write set:** this plan directory and directly affected documentation links.

**Gate:** Current plan-structure and retained-link checks pass.

**Status:** `Accepted`

### Milestone 1: Restore Validation

**Goal:** Make the real saved workflow reach a correct typed submit decision.

**Allowed write set:** declare the exact workflow-service/inference owner and focused fixtures in the ledger before implementation.

**Gate:** IMG-A01 and affected production-composition tests pass.

**Status:** `Blocked`

### Milestone 2: Canonical Non-Empty Dispatch

**Goal:** Dispatch compatible assignment groups of `1..N` through one scheduler path.

**Allowed write set:** scheduler/task orchestration, focused contracts/tests, and plan-control files declared before implementation.

**Gate:** IMG-A02 passes for solo, compatible multi-task, incompatible, timeout, cancellation, and shutdown cases.

**Status:** `Planned`

### Milestone 3: Restore Integrated Baseline

**Goal:** Reconcile affected fixtures and make canonical workflow/runtime suites green.

**Allowed write set:** affected test owners and canonical producers identified by failures; no broad cleanup.

**Gate:** IMG-A03 passes and failures outside scope are recorded without being hidden.

**Status:** `Planned`

### Milestone 4: Real Desktop Workflow

**Goal:** Produce and present one retained image through the real editor path.

**Allowed write set:** existing GUI harness, exact product owners exposed by the path, and plan-control files.

**Gate:** IMG-A04 and IMG-A05 pass with model/runtime/environment identity recorded.

**Status:** `Planned`

## Blockers

- [IMG-I01](issues.md#img-i01) — the active Diffusers loader bypasses explicit trust authorization.
- [IMG-I02](issues.md#img-i02) — the saved workflow currently reports blocking inference diagnostics.
- [IMG-I03](issues.md#img-i03) — required-real model/runtime/desktop prerequisites must be recorded for final evidence.
- The current-standards portfolio owns shared-path admission; this plan cannot run concurrently on an unhanded write set.

## Re-Plan Triggers

- The security plan changes the worker authorization/request contract consumed here.
- A new model, scheduler, runtime, artifact, or frontend authority enters the bounded path.
- Current reruns contradict the historical failure inventory or require a file outside the declared slice.
- The required-real workflow cannot decide the claimed result without a fallback or weaker environment.

## Final Acceptance

- Acceptance status: `blocked`
- Deferred follow-ups: broader performance, other image families, packaging, and generated UI execution.
- Final status: `Blocked`
