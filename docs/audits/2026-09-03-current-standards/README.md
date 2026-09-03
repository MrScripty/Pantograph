# Pantograph Current-Standards Audit

Audit status: baseline complete

Audit date: 2026-09-03

Pantograph revision: 97c04827

Coding-Standards revision: 82a0ddf315a08364357f6564018e37bdbeb72a1a

## Purpose

This is the current repository-wide standards baseline for Pantograph. It
compares the codebase with the canonical standards selected through
STANDARDS-ROUTER.md at the revision above.

The audit is intentionally broad rather than exhaustive. It identifies
systemic findings and delegates detail to focused audits. It does not authorize
or sequence refactors; an implementation plan should be derived after the
critical and high findings are accepted or corrected.

The February-April 2026 compliance work remains useful history, but it is not
the current compliance authority. The standards library has changed
substantially since that work.

## Executive Assessment

Pantograph is not currently standards-compliant, but it is also not an
untouched legacy codebase. Recent work established strong crate boundaries,
typed scheduler/runtime contracts, explicit architecture decisions, strict
TypeScript, workspace lint inheritance, filesystem containment, and a large
test inventory.

The highest risk is concentrated around old and transitional surfaces:

- the active Diffusers image path unconditionally enables model-supplied
  Python code without a trust decision;
- generated Svelte component validation can fail open before code executes in
  the main renderer;
- Rustler and UniFFI retain binding-owned runtime, lossy conversion, and
  unobservable event-delivery behavior;
- direct inference execution remains alongside the target scheduler-only path;
- test discovery, release evidence, and several quality claims do not match
  what the repository actually runs; and
- documentation/traceability automation still enforces rules retired by the
  current standards.

The correct next move is not a blanket file-splitting or check-adding campaign.
It is a small number of boundary-focused remediation plans, beginning with
dynamic-code trust and truthful verification.

## Scope And Method

The audit covered:

- 23 Rust workspace members, including the Tauri application;
- 768 Rust, 298 TypeScript, 126 Svelte, and 15 Python tracked source files;
- workflow, scheduler, runtime-host, inference, persistence, diagnostics, and
  runtime-registry ownership;
- Tauri, UniFFI, Rustler, C#, HTTP, Python-worker, and generated-component
  boundaries;
- frontend projection, state, async lifecycle, and accessibility;
- local commands, hooks, CI, test discovery, dependencies, launcher,
  packaging, release, licensing, ADRs, plans, and operational documentation.

The audit used source inspection and existing local checks. It did not perform
GPU/model execution, release builds, real multi-platform packaging, GUI
accessibility testing, or destructive security testing. Claims requiring those
environments remain unresolved rather than being inferred from lower-fidelity
checks.

The two pre-existing Pumas proposal worktree changes were excluded from the
audit and left untouched.

## Standards Route

The repository facts select:

- Core and the Standards Router;
- Architecture, Contracts, Concurrency, Resilience, Security, Diagnostics,
  Dependencies, Licensing, Cross-Platform, Performance, and Accessibility
  topics where their observable conditions apply;
- Library, Frontend, and Launcher application profiles;
- IPC, Persistence, Generated Contract, Interop, and Language Binding boundary
  profiles;
- Rust, Rust API, Rust Async, Rust Dependency, Rust Tooling, Rust Security,
  Rust Cross-Platform, Rust Interop, and Rust Binding profiles;
- TypeScript and TypeScript Async profiles; and
- Verification, Documentation, Tooling, Build, Planning, and Release workflows
  for the repository mechanisms they own.

Godot and concurrent-plan-integration profiles were not selected.

The current standards do not impose universal file-size limits, a README in
every directory, fixed README headings, or a universal test-command catalog.
Those mechanisms must be justified by actual ownership and evidence needs.

## Severity

| Level | Meaning |
| --- | --- |
| Critical | A verified path can cross a security or authority boundary without the required decision or proof. |
| High | A systemic contract, lifecycle, or evidence gap can produce wrong behavior or false confidence. |
| Medium | A real maintainability, drift, or incomplete-proof problem should be addressed after higher-risk boundaries. |
| Unresolved | The risk is plausible, but the required consumer, environment, or lifecycle facts were unavailable. |

## Overarching Findings

| ID | Severity | Finding | Focused audit |
| --- | --- | --- | --- |
| CSA-01 | Critical | Active Diffusers execution hard-codes remote model code trust, while the Rust request rejects a trust field. | [Security and dynamic code](01-security-and-dynamic-code.md) |
| CSA-02 | High | Generated UI validation can fail open and the accepted module runs in the main renderer without a demonstrated isolation contract. | [Security and dynamic code](01-security-and-dynamic-code.md) |
| CSA-03 | High | Direct inference execution remains a second authority beside the accepted scheduler/runtime-host path. | [Architecture, lifecycle, and bindings](02-architecture-lifecycle-and-bindings.md) |
| CSA-04 | High | Binding and process adapters create runtimes, detach work, erase invalid input, or report success after dropping delivery. | [Architecture, lifecycle, and bindings](02-architecture-lifecycle-and-bindings.md) |
| CSA-05 | High | Frontend IPC payloads are usually trusted through TypeScript generics instead of runtime decoding. | [Frontend and accessibility](03-frontend-and-accessibility.md) |
| CSA-06 | High | Quality documentation claims clean gates that are currently red, and canonical commands cover inconsistent subsets. | [Verification and tooling](04-verification-and-tooling.md) |
| CSA-07 | High | Manual frontend and Rust test registration silently omits tracked suites; no claim/evidence map justifies the selected coverage. | [Verification and tooling](04-verification-and-tooling.md) |
| CSA-08 | High | The release smoke does not execute the selected release binary, and packaging lacks verified target/version identity. | [Dependencies, release, and documentation](05-dependencies-release-and-documentation.md) |
| CSA-09 | High | Decision-traceability and active-plan machinery encode the retired standards model and can accept or demand unrelated documentation. | [Dependencies, release, and documentation](05-dependencies-release-and-documentation.md) |
| CSA-10 | Medium | Frontend accessibility gates are red, global listener/async ownership has defects, and browser-level interaction evidence is narrow. | [Frontend and accessibility](03-frontend-and-accessibility.md) |
| CSA-11 | Medium | Dependency, lockfile, Pumas revision, Python provenance, license, and SBOM authorities are duplicated or incomplete. | [Dependencies, release, and documentation](05-dependencies-release-and-documentation.md) |

## Strong Foundations To Preserve

- ARCHITECTURE.md and the accepted ADRs clearly name the target scheduler,
  runtime-registry, runtime-host, workflow-service, and adapter ownership.
- New scheduler queue and inference-validation task owners track handles,
  close admission, signal cancellation, drain work, and report join failures.
- The path-security crate centralizes containment and has traversal and symlink
  escape tests.
- Rust toolchain, Node, npm, and Python versions are pinned; workspace members
  inherit unsafe-code denial and publication control.
- TypeScript strictness and a large pure helper/store test corpus are in place.
- Newer frontend mutation paths begin from unknown input and decode errors.
- Tauri declares a narrow capability set, and CI includes real Rustler and C#
  binding smoke paths.

These are migration anchors. Remediation should deepen or reuse them rather
than creating parallel replacements.

## Baseline Evidence

Observed on 2026-09-03:

| Check | Result |
| --- | --- |
| cargo fmt --all -- --check | Pass |
| cargo check --workspace --no-default-features | Pass |
| npm run typecheck | Pass |
| npm run test:frontend | Pass for its manually listed 71 files |
| all 90 discovered frontend test files | Pass in an audit-only invocation |
| npm test | Pass: node-engine 258 passed, 1 ignored; workflow-nodes 168 passed |
| cargo test -p pantograph-scheduler | Pass: 99 tests |
| npm run lint:full | Fail: PumaLibNode.svelte |
| npm run lint:critical | Fail: IoInspectorPage.svelte |
| npm run lint:a11y | Fail: three reported findings |
| npm run traceability | Fail and demonstrates obsolete mapping behavior |
| cargo clippy --workspace --all-targets --all-features -- -D warnings | Fail across multiple crates |

Passing checks are evidence only for their stated scope. No overall green
baseline is claimed.

## Recommended Order

1. Contain model-supplied and generated-component code execution. Establish
   explicit trust, provenance, isolation, and fail-closed outcomes.
2. Make verification truthful: define the supported claims, fix discovery,
   remove false clean-baseline statements, and separate required evidence from
   advisory debt.
3. Complete the scheduler-only execution cutover and binding cleanup. Remove
   alternate runtime ownership and lossy boundary conversions.
4. Audit all process, task, callback, subscription, and shutdown owners against
   one lifecycle model.
5. Establish dependency, release-unit, platform, license, and final-artifact
   provenance before making release claims.
6. Replace retired documentation and planning enforcement with exact
   impact-to-authority mappings.

## Focused Audits

- [Security and dynamic code](01-security-and-dynamic-code.md)
- [Architecture, lifecycle, and bindings](02-architecture-lifecycle-and-bindings.md)
- [Frontend and accessibility](03-frontend-and-accessibility.md)
- [Verification and tooling](04-verification-and-tooling.md)
- [Dependencies, release, and documentation](05-dependencies-release-and-documentation.md)

## Implementation Plans

The [current-standards remediation portfolio](../../plans/current-standards-remediation/plan.md)
owns integration order and aggregate acceptance. Each focused plan owns its
domain decisions, milestones, evidence, ledger, and issues:

- [Security and dynamic code](../../plans/current-standards-remediation/security-and-dynamic-code/plan.md)
- [Architecture, lifecycle, and bindings](../../plans/current-standards-remediation/architecture-lifecycle-and-bindings/plan.md)
- [Frontend and accessibility](../../plans/current-standards-remediation/frontend-and-accessibility/plan.md)
- [Verification and tooling](../../plans/current-standards-remediation/verification-and-tooling/plan.md)
- [Dependencies, release, and documentation](../../plans/current-standards-remediation/dependencies-release-and-documentation/plan.md)
