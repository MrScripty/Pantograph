# Plan: Current-Standards Remediation Portfolio

**Plan status:** `Planned`

**Current phase:** Portfolio Milestone 0 — contain active model-code trust

**Next slice:** Execute only [security and dynamic code Milestone 0](security-and-dynamic-code/plan.md#milestone-0-apply-model-code-authorization), record SDC-A1/A2 evidence in its ledger, and then record the portfolio gate here. No other child milestone is admitted concurrently.

**Acceptance status:** `pending`

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** `none`

**Related ADRs:** [ADR index](../../adr/README.md)

**Source audit:** [2026-09-03 current-standards audit](../../audits/2026-09-03-current-standards/README.md)

## Objective

Bring the audited Pantograph repository to the current routed standards through five bounded remediation plans, preserving one domain owner, one evidence meaning, and one serial integration authority for shared files.

This portfolio owns order, handoffs, aggregate acceptance, and shared-write coordination. The child plans own implementation detail:

- [Security and dynamic code](security-and-dynamic-code/plan.md)
- [Architecture, lifecycle, and bindings](architecture-lifecycle-and-bindings/plan.md)
- [Frontend and accessibility](frontend-and-accessibility/plan.md)
- [Verification and tooling](verification-and-tooling/plan.md)
- [Dependencies, release, and documentation](dependencies-release-and-documentation/plan.md)

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| CSR-A01 | Model-supplied code requires exact authorization, component admission fails closed, and generated modules do not execute in the main renderer. | `system` | `representative` | `automated` | `pending` | Accepted [SDC-A1–SDC-A6](security-and-dynamic-code/plan.md#objective-acceptance) on the final revision |
| CSR-A02 | Scheduler/runtime-host ownership, native bindings, processes, and shutdown have the single-authority and observable lifecycle required by the architecture plan. | `integration` | `representative` | `automated` | `pending` | Accepted [ALB-01–ALB-06](architecture-lifecycle-and-bindings/plan.md#objective-acceptance) on the final revision |
| CSR-A03 | Frontend IPC, async work, graph composition, connection intent, persisted state, and selected accessible tasks satisfy their owned contracts in the WebKit/Tauri host. | `user-workflow` | `representative` | `automated` | `pending` | Accepted [FE-A01–FE-A06](frontend-and-accessibility/plan.md#objective-acceptance) on the final revision |
| CSR-A04 | Claim discovery, traceability, static gates, commands, CI, browser evidence, and exact-artifact evidence report only what their oracles decide on the declared target set. | `system` | `required-real` | `automated` | `pending` | Accepted [VT-A01–VT-A07](verification-and-tooling/plan.md#objective-acceptance) on the final revision |
| CSR-A05 | Dependency, license, documentation, release-unit, target, metadata, and candidate identities have one authority and are verified against the final artifact set. | `release-artifact` | `required-real` | `either` | `pending` | Accepted [DRD-A01–DRD-A08](dependencies-release-and-documentation/plan.md#objective-acceptance) on the final candidate revision |
| CSR-A06 | CSA-01–CSA-11 each has one closed child-plan disposition; all five child plans are `Accepted`, their final evidence names one revision/candidate, and no critical/high finding remains open or silently downgraded. | `focused` | `not-applicable` | `automated` | `pending` | Child ledgers, this ledger's cross-plan reconciliation, and final claim-portfolio validation |

A child status, passing supporting check, simulated substitute, or evidence from a different revision cannot satisfy an aggregate claim. Missing required capability is `unavailable` and blocks the affected claim. Contradictory ownership or malformed evidence is `invalid` and stops integration. A target is `unsupported` only when the accepted release authority says so.

## Scope

### In Scope

- Sequencing and aggregate acceptance of the eleven findings in the source audit.
- Cross-plan dependencies, shared-file handoffs, and final-revision reconciliation.
- Portfolio status, evidence links, blockers, and re-plan decisions.

### Out Of Scope

- Repeating child tasks, write sets, evidence designs, or domain decisions here.
- Product features unrelated to CSA-01–CSA-11, release publication, credentials, or external-channel mutation.
- Rewriting historical audits/plans, introducing plan-owned Git topology, or treating a clean build/lint count as standards compliance.
- Restoring generated-component execution; the security plan deliberately leaves live execution unsupported pending a separately accepted isolation design.

## Current Child Status

| Child authority | Plan status | Acceptance | Portfolio admission |
| --- | --- | --- | --- |
| Security and dynamic code | `Planned` | `pending` | Milestone 0 is the sole admitted next slice |
| Verification and tooling | `Planned` | `pending` | Wait for Portfolio Milestone 1 |
| Dependencies, release, and documentation | `Planned` | `pending` | Wait for Portfolio Milestone 1 |
| Architecture, lifecycle, and bindings | `Planned` | `pending` | Wait for Portfolio Milestone 2 |
| Frontend and accessibility | `Planned` | `pending` | Wait for Portfolio Milestone 3 |

Child-local “next slice” fields describe readiness, not permission to run concurrently. Portfolio admission is the integration authority.

## Constraints And Assumptions

### Constraints

- Child plans remain the only owners of domain implementation and evidence details.
- Shared paths have one active writer, and publication or external-channel mutation remains out of scope.
- Unavailable required-real evidence blocks acceptance rather than selecting a weaker substitute.

### Assumptions

- The five child plans bound all current CSA-01–CSA-11 authority and consumer populations; implementation evidence that expands a population triggers re-planning.
- Serial handoffs are sufficient for the currently known shared write sets. Concurrent proposal integration is not selected unless mutable proposals can become stale before integration.

## Binding Decisions

| Boundary | Canonical owner | Portfolio rule |
| --- | --- | --- |
| Model/generated-code trust and execution | Security plan | Security Milestone 0 precedes architecture edits to the image runtime; disabling renderer execution is not rolled back. |
| Scheduler, runtime host, bindings, processes, shutdown | Architecture plan | Security's applied authorization is preserved; no direct-execution fallback survives. |
| IPC projection, graph Interface, browser state, selected accessible tasks | Frontend plan | It consumes backend trust/lifecycle outcomes and does not redefine them. |
| Claim identity, evidence kind/environment, discovery, schedules, CI adapters | Verification plan | `verification/claims.json` becomes evidence authority; this plan only aggregates child acceptance IDs. |
| Dependency/release identity, licensing, artifact plan, current documentation | Dependency/release/documentation plan | It consumes verified product behavior and cannot infer missing legal, platform, or publication authority. |
| Integration order and shared-write handoff | This plan | Exactly one implementation slice owns a shared path at a time; handoff and post-handoff evidence are recorded in this ledger. |

## Systemic Finding Audit

The systemic population is bounded by the five child plans' declared authorities, consumers, and expansion facts. Every occurrence must be fixed, retained with evidence, removed, or explicitly blocked by its child owner. The stopping condition is CSR-A01–CSR-A06 accepted with all five child plans `Accepted`; spot fixes do not close a systemic finding.

## Shared-Write Coordination

- A portfolio milestone admits only the selected child milestone's current allowed write set, that child's three plan-control files, and this portfolio's three plan-control files. The child write set is canonical by reference; its union with later milestones is not pre-authorized.
- Before a slice starts, compare its intended paths with the worktree and all later child write sets. A user-owned or active-plan edit requires an explicit handoff; it is never overwritten or absorbed implicitly.
- `Cargo.lock` is serialized in this order: security's Boa removal, architecture dependency-edge changes, then dependency-resolution reconciliation. Re-run the dependency contract after every handoff.
- `package.json`, `package-lock.json`, launcher, CI, and verification documentation are owned first by verification foundations, then by the admitted security/frontend or dependency slice, and finally by verification closeout. Each handoff reruns portfolio validation.
- Runtime-host/Tauri paths pass from security to architecture, then to frontend/security desktop evidence. Root README, policy, ADR-index, and active-plan migration remain with the documentation child except for an explicitly mapped child-owned document.
- A conflict is resolved by serial order or re-planning, never by broadening both write sets. Parallel work is allowed only after this plan records disjoint write sets and independent gates.

## Simplicity And Ownership Review

**Applicability:** `applicable`

Five independently owned remediation concepts compose across runtime, renderer,
tooling, and release boundaries.

- Independent concepts and dimensions: trust/execution, runtime lifecycle, frontend projection, verification meaning, and release identity remain separate child authorities.
- State, identity, value, time, policy, and mechanism: runtime and UI state remain with product owners; model/source/revision/digest, claim ID, package/ABI/target, and candidate identity remain distinct. Evidence is valid only for its recorded revision/environment and is invalidated by a changed contract, identity, target, or consumer population.
- Caller and composition-root knowledge: this portfolio invokes a child by plan path and milestone operation. It knows acceptance IDs and handoffs, not child-internal loaders, schedulers, selectors, or packaging mechanics.
- Representative change paths and forced owners: a new loader goes to security; a new runtime entry point to architecture; a Tauri/UI field to frontend; a new test root or schedule to verification; a target/artifact/license promise to dependency/release/documentation. Cross-boundary changes require both owners' claims without creating a sixth semantic owner.
- Stable Interfaces versus hidden knowledge: child acceptance claims and typed product boundaries are stable Interfaces. Tool syntax, process mechanics, selectors, and artifact layout stay hidden in their child plans.
- Independent evolution, testing, failure, and replacement: child gates can fail, roll back, or re-plan independently until a declared shared handoff. Aggregate acceptance is intentionally not independent of their final evidence.
- Necessary complexity and containment: this portfolio adds no runtime registry, wrapper, generator, framework, or duplicate claim manifest. Its only extra mechanism is the documented serial handoff needed by demonstrated shared writes.
- Deletion and cumulative machinery result: removing this portfolio would leave usable child plans but lose authoritative order, collision control, and aggregate closure. After acceptance it remains a decision/evidence index, not a live product authority.

## Milestones

Each child milestone is a small implementation slice with its own gate. Within a portfolio milestone, execute the listed operations serially and do not mark the portfolio gate complete from partial child evidence.

### Portfolio Milestone 0: Critical Trust Containment

**Goal:** Close the active Diffusers authorization bypass before adjacent runtime changes.

**Order:** Security Milestone 0 only.

**Allowed write set:** exactly the security plan's Milestone 0 write set plus the security and portfolio plan-control files.

**Gate:** SDC-A1 and SDC-A2 accepted; negative authorization/cache cases and supporting format/check results recorded. Rollback must keep every Python loader at `trust_remote_code=False`.

**Status:** `Planned`

### Portfolio Milestone 1: Evidence And Identity Foundations

**Goal:** Make claims, traceability, static debt, command outcomes, dependency units, and consumed identities explicit before broad refactoring.

**Order:** Verification Milestones 0–3, then dependency/release/documentation Milestones 0–1; rerun affected verification contracts after dependency handoff.

**Allowed write set:** for each operation, exactly that child milestone's write set plus its and the portfolio plan-control files. Shared package/workflow/launcher files transfer serially in the stated order.

**Gate:** every listed child milestone gate passes; unresolved required claims remain visibly pending rather than satisfied by weaker evidence.

**Status:** `Planned`

### Portfolio Milestone 2: Runtime Authority And Lifecycle

**Goal:** Complete scheduler-only execution, thin bindings, owned processes, and observable shutdown after trust is carried into the runtime path.

**Order:** Architecture Milestones M1–M7 serially.

**Allowed write set:** exactly the active architecture milestone's write set plus architecture and portfolio plan-control files.

**Gate:** ALB-01–ALB-06 accepted; affected verification and dependency contracts rerun after shared-file handoff.

**Status:** `Planned`

### Portfolio Milestone 3: Generated Boundary And Frontend Contracts

**Goal:** Fail closed at generated-source admission, remove renderer execution, and repair the frontend's decoded, lifecycle-owned, accessible user paths.

**Order:** Security Milestones 1–2; frontend Milestones 0–4; verification Milestone 4 harness/claim handoff; frontend Milestones 5–6; security Milestone 3; then final verification Milestone 4 acceptance.

**Allowed write set:** exactly the currently active child milestone's write set plus that child and portfolio plan-control files. Security, frontend, and verification exchange `package*.json`, desktop harness, and hotload ownership only at recorded handoffs.

**Gate:** SDC-A1–SDC-A6 and FE-A01–FE-A06 accepted, including representative WebKit/Tauri and hostile-component non-execution evidence. Missing desktop/browser capability is `unavailable` and blocks.

**Status:** `Planned`

### Portfolio Milestone 4: License, Documentation, And Candidate Identity

**Goal:** Resolve legal/current-document authority and construct the final immutable artifact set from stable product and dependency boundaries.

**Order:** Dependency/release/documentation Milestones 2–4.

**Allowed write set:** exactly the active dependency/release/documentation milestone's write set plus its and the portfolio plan-control files.

**Gate:** DRD-A03–DRD-A06 accepted for the complete required target set; owner-selected license terms are recorded before packaging acceptance.

**Status:** `Planned`

### Portfolio Milestone 5: Exact Evidence And Closure

**Goal:** Prove the final product and exact candidate, then reconcile every child and audit disposition on one revision.

**Order:** Verification Milestone 5; dependency/release/documentation Milestone 5; child final-evidence reruns where their recorded revision differs; portfolio reconciliation last.

**Allowed write set:** exactly the active child closeout milestone's write set plus that child and portfolio plan-control files. Portfolio reconciliation writes only this directory.

**Gate:** CSR-A01–CSR-A06 accepted. A required-real lane that is unavailable blocks; source checks, stale artifacts, or another revision are not substitutes. Publication remains out of scope and unavailable without separate authority.

**Status:** `Planned`

## Migration, Recovery, And Evidence Rules

- Each child owns its atomic data/API/dependency migration and rollback. Failed integration returns to the last accepted child boundary without restoring unsafe execution, dual resolution, direct inference authority, or silently successful lifecycle outcomes.
- Preserve user work and active-plan history. Git history is recovery evidence, not an authorized runtime fallback or permission to reset unrelated changes.
- Record command/procedure, revision or candidate identity, environment, result, and artifact/log reference in the owning child ledger. This ledger records only admission, handoff, aggregate gate, and deviation facts.
- A child may be `Accepted` only on its own evidence. The portfolio remains nonterminal until the final-revision rerun and CSR-A06 reconciliation.

## Blockers

- `none` for the sole next slice.
- [PORT-I01](issues.md#port-i01) blocks dependency/release/documentation Milestone 2 and later license/release acceptance until the repository owner chooses project license terms.
- [PORT-I02](issues.md#port-i02) blocks incompatible binding/package deletion if a supported external consumer is discovered.
- [PORT-I03](issues.md#port-i03) blocks aggregate acceptance wherever representative or required-real evidence is unavailable; no lower-fidelity fallback is accepted.
- Any unhanded shared-path work blocks that slice, not unrelated read-only planning.

## Re-Plan Triggers

- A new loader/executor, runtime entry point, binding/package consumer, test root, target, artifact role, licensed material, or public compatibility promise expands a child population.
- A child changes an acceptance claim's meaning, evidence environment, canonical owner, milestone order, or shared write set after an upstream handoff.
- Required provenance, legal authority, process control, browser/desktop runner, GPU/model lane, target runner, or exact candidate identity cannot decide its claim.
- Two plans require simultaneous ownership of a shared path, or implementation needs a file outside the active child's allowed write set.
- The claim registry or portfolio starts duplicating domain state, or the integration mechanism grows beyond serial handoffs and evidence links.

## Implementation Invocation

Start with: **Implement `docs/plans/current-standards-remediation/security-and-dynamic-code/plan.md`, operation “Milestone 0: Apply Model-Code Authorization” only. Preserve unrelated work, stay within that milestone's write set, run its SDC-A1/A2 gate, and update its ledger/issues plus this portfolio ledger before requesting another slice.**

## Final Acceptance

- Acceptance status: `pending`
- Deferred: live generated-component execution and general desktop/network hardening under the explicit triggers in the security plan.
- Publication: not authorized by this plan.
- Final status: `Planned`
