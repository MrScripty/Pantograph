# Plan: Pantograph Binding Platform

## Status
Draft

Last updated: 2026-04-18

## Current Source-of-Truth Summary

This document is the dedicated source of truth for Pantograph's first-class
binding platform planning. It expands the earlier Rustler-only NIF follow-on
into a broader standards-reviewed plan covering:

- curated client-facing binding surface policy
- shared backend-owned binding contract ownership
- C# binding hardening
- Python binding introduction as a real host-consumer lane
- BEAM/Rustler verification as a specialized host lane

The roadmap should summarize status and point here for binding-platform
milestone detail. Binding-specific subplans may exist for individual host
lanes, but this document is the primary source of truth for overall binding
scope, sequencing, support tiers, and verification expectations.

The BEAM/Rustler lane is no longer blocked on NIF compilation. The Rustler
crate now compiles and builds successfully, and the in-repo Mix/ExUnit smoke
harness proves real NIF loading plus initial contract behavior under BEAM. The
remaining BEAM-specific issue relevant to this broader bindings plan is
narrower: raw `cargo test -p pantograph_rustler` is still not an authoritative
host verification path for NIF-bound behavior, so the remaining work is
continued BEAM host-lane coverage breadth and binding-platform reconciliation,
not basic product compile recovery.

## Objective

Turn Pantograph bindings into a first-class product boundary rather than an ad
hoc byproduct of wrapper crates by:

- exposing only the client-facing subset of Pantograph workflows and resources
  that host applications actually need
- keeping canonical semantics in backend Rust crates and shared binding-neutral
  helpers rather than in wrapper-local code
- treating generated host bindings, native libraries, packaging, and testing as
  deliberate product artifacts
- enforcing both native-language and host-language verification for supported
  bindings

The resulting binding platform must stay compliant with the updated generic
binding and testing standards, preserve Pantograph's backend-owned execution
model, and avoid exporting GUI- or transport-local internals as public client
API.

## Scope

### In Scope

- Pantograph headless native binding surface exposed through
  `crates/pantograph-uniffi`
- Rustler/BEAM wrapper surface exposed through `crates/pantograph-rustler`
- Client-facing workflow/session/graph APIs intended for external host
  applications
- Binding support-tier policy for C#, Python, and BEAM
- Shared backend-owned contract extraction when multiple bindings need the same
  semantics
- Native-library packaging, generated binding packaging, and host-language
  smoke/acceptance planning
- Documentation, roadmap, and plan reconciliation required for a first-class
  binding platform

### Out of Scope

- Binding every internal Pantograph function, debug path, or transport helper
- Moving workflow semantics into UniFFI, Rustler, Tauri, or generated host
  code
- Hand-editing generated bindings
- GUI/Tauri-only features that do not belong in headless host-consumer APIs
- Non-binding runtime or scheduler roadmap items except where they affect the
  client-facing contract

## Inputs

### Problem

Pantograph already has a real C# artifact path and smoke coverage for the
headless native library, and it has a narrower Rustler BEAM wrapper with a
dedicated NIF verification problem. But the overall binding story remains
incomplete:

- the repository does not yet define a curated Pantograph binding surface as a
  product contract
- C# is the only host-language lane that currently looks close to first-class
- Python is mentioned as a potential UniFFI host language, but not yet treated
  as a first-class client binding with its own packaged flow and host-language
  tests
- BEAM/Rustler verification has historically been scoped narrowly around NIF
  loading rather than integrated into a broader binding strategy; the baseline
  local smoke harness now exists, but broader contract coverage and platform
  positioning still need to be integrated into the larger bindings plan
- immediate wrapper insertion points are already oversized enough that more
  binding work would deepen existing standards violations if it lands without
  decomposition

Without a dedicated binding-platform plan, Pantograph risks exporting too much,
testing too little, and keeping wrapper-local logic that should instead live in
backend-owned contract layers.

### Constraints

- Pantograph must follow the generic binding architecture in
  `LANGUAGE-BINDINGS-STANDARDS.md`.
- The binding surface must follow the curated export policy and support-tier
  model now defined in `LANGUAGE-BINDINGS-STANDARDS.md`.
- Supported bindings must satisfy the stricter native-language plus
  host-language verification policy now defined in `TESTING-STANDARDS.md`.
- The product-native library remains `pantograph_headless` for headless host
  clients; internal wrapper crate names do not become product identity.
- Backend/service/runtime crates own canonical workflow, session, graph, and
  error semantics.
- UniFFI and Rustler remain Layer 2 wrappers only.
- Generated host bindings remain generated artifacts and are never hand-edited.
- Different host languages may expose different supported subsets if the
  support-tier matrix and consumer contracts make that choice explicit.

### Public Facade Preservation Note

This is a facade-first platform plan. Existing headless host entry points
should remain stable unless an explicit surface freeze decides to deprecate or
rename them with documented compatibility guidance. Default strategy: extract,
classify, and document behind current facades before considering API breaks.

### Assumptions

- C# is the closest current candidate for an initial `supported` host binding
  because it already has generated-artifact packaging and host-language smoke
  coverage.
- Python should be treated as a distinct host-consumer binding lane, not
  conflated with the out-of-process Python worker/runtime separation work.
- BEAM/Rustler should likely remain `experimental` until the current host-side
  smoke baseline is expanded into broader event/error/session coverage and the
  curated exposed surface is reconciled with the overall binding platform.
- The current headless JSON DTO flow through `pantograph-workflow-service`
  remains the most realistic starting point for cross-language bindings.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
- `docs/headless-native-bindings.md`
- `bindings/csharp`
- `crates/pantograph-uniffi`
- `crates/pantograph-rustler`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CODING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/LANGUAGE-BINDINGS-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CROSS-PLATFORM-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DEPENDENCY-STANDARDS.md`

### Affected Structured Contracts

- Pantograph headless host-client workflow/session/graph request and response
  contracts
- Binding support-tier matrix for C#, Python, and BEAM
- Generated host binding package expectations and native library pairing rules
- Wrapper-local error and event projection contracts
- Any shared backend-owned binding helper contracts extracted for reuse across
  UniFFI and Rustler

### Affected Persisted Artifacts

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- This binding-platform plan
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
- `docs/headless-native-bindings.md`
- `bindings/csharp/README.md`
- Any future `bindings/python/` or BEAM harness documentation introduced by
  implementation
- Packaging scripts and manifest guidance for generated host-language artifacts

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate wrapper insertion points already exceed decomposition thresholds
from `CODING-STANDARDS.md`:

- `crates/pantograph-rustler/src/lib.rs` is approximately 2511 lines
- `crates/pantograph-uniffi/src/lib.rs` is approximately 1522 lines
- `crates/pantograph-uniffi/src/runtime.rs` is approximately 915 lines

The binding-platform plan must therefore include decomposition review and
shared-helper extraction before significantly expanding the public binding
surface or host-language verification paths.

### Concurrency / Race-Risk Review

- Host-language test lanes will mutate native-library search paths, environment
  variables, temp artifact roots, and compiled output directories; these must
  be isolated per suite to avoid state leakage.
- Python binding tests must not blur the boundary between the Python host
  binding lane and the out-of-process Python worker/runtime lane.
- BEAM harnesses must document NIF load/unload lifecycle ownership and avoid
  global runtime leakage across repeated test runs.
- Event or callback delivery policies must document startup, shutdown, and
  unsubscribe behavior per host lane.

### Ownership And Lifecycle Note

- Backend Rust crates own canonical execution, graph, session, and error
  semantics.
- Shared binding-neutral helpers own reusable contract shaping if more than one
  wrapper needs the same semantics.
- `crates/pantograph-uniffi` and `crates/pantograph-rustler` own wrapper-local
  conversion, validation, and host-runtime bridge code only.
- `bindings/` and host-language harness directories own consumer-facing smoke
  examples and host-language verification assets.
- Packaging scripts own artifact assembly and must pair generated bindings with
  the matching native library from the same build.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Pantograph exports too much internal surface as public binding API | High | Freeze a curated client-facing surface before adding more host-language packaging |
| Wrapper crates continue absorbing canonical semantics | High | Extract reusable contract shaping into backend-owned or binding-neutral helpers |
| Python host bindings are confused with Python worker/runtime concerns | High | Document separate ownership, package identity, and verification lanes |
| Supported bindings drift because only Rust-side or only host-side tests exist | High | Require both native-language and host-language tests for supported bindings |
| Oversized wrapper files make future binding work unreviewable | High | Make decomposition an early milestone before widening the surface |

## Standards Review Passes

### Draft Pass

Initial draft written from the current C# artifact lane, the existing Rustler
NIF verification plan, and the updated generic binding/testing standards.

### Pass 1: Plan Structure And Source-of-Truth

Reviewed against:

- `PLAN-STANDARDS.md`

Corrections applied:

- Recorded this as the primary binding-platform source of truth instead of
  leaving binding policy scattered across roadmap notes and narrower subplans.
- Added explicit milestones, verification, re-plan triggers, and completion
  criteria.

### Pass 2: Binding Architecture And Surface Policy

Reviewed against:

- `LANGUAGE-BINDINGS-STANDARDS.md`
- `CODING-STANDARDS.md`

Corrections applied:

- Locked the plan to a curated client-facing surface rather than blanket
  export of everything wrappers can expose.
- Recorded support tiers and the right to expose different subsets per host
  language when documented.
- Added explicit decomposition pressure for the oversized wrapper files.

### Pass 3: Interop And Lifecycle Boundaries

Reviewed against:

- `INTEROP-STANDARDS.md`
- `CROSS-PLATFORM-STANDARDS.md`

Corrections applied:

- Recorded host-runtime lifecycle ownership, artifact-path isolation, and
  startup/shutdown expectations for the different language lanes.
- Preserved thin-wrapper guidance and avoided moving semantics into host-side
  or wrapper-local code.

### Pass 4: Testing And Verification

Reviewed against:

- `TESTING-STANDARDS.md`

Corrections applied:

- Required native-language plus host-language verification for supported
  bindings.
- Added packaging-pairing verification and cross-layer acceptance obligations.
- Added test-isolation requirements for native library paths, temp roots, and
  host runtime state.

### Pass 5: Documentation And Dependency Hygiene

Reviewed against:

- `DOCUMENTATION-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`

Corrections applied:

- Planned README/doc updates where bindings become first-class artifacts rather
  than repo-internal smoke utilities.
- Kept the plan minimal on new dependencies until each host-language lane is
  justified by the support-tier freeze.

## Recommendations

- Make C# the first `supported` binding lane because it already has the
  strongest artifact and smoke coverage in-repo.
- Treat Python as the next candidate lane, but keep it explicitly separate from
  the Python worker/runtime sidecar concerns.
- Keep BEAM as a specialized lane focused on real host-boundary verification
  and curated surface selection, not blanket parity with UniFFI exports.

## Milestones

### Milestone 1: Binding Product Contract And Support-Tier Freeze

#### Tasks

- Define the Pantograph client-facing binding surface for workflow execution,
  workflow sessions, workflow graph authoring, and shutdown/lifecycle calls.
- Classify each binding lane as `supported`, `experimental`, or
  `internal-only`.
- Record the initial intended tier matrix:
  C# as candidate `supported`, Python as candidate `experimental` moving toward
  `supported`, and BEAM/Rustler as candidate `experimental`.
- Mark non-client/internal capabilities that wrappers must not expose as part
  of the public binding contract without a later explicit decision.
- Reconcile `docs/headless-native-bindings.md` and wrapper READMEs with the
  curated surface policy.

#### Verification

- Confirm the curated surface and support tiers are documented in this plan and
  reflected in the roadmap.
- Confirm every included public capability has an external consumer rationale.

### Milestone 2: Shared Contract Layer And Wrapper Decomposition Map

#### Tasks

- Inventory which current UniFFI and Rustler helpers are canonical and reusable
  versus wrapper-local.
- Design the backend-owned or binding-neutral helper boundary for reusable
  contract shaping and error categorization.
- Define the facade-preserving decomposition for the oversized UniFFI and
  Rustler files before more binding surface is added.
- Record which host-lane differences are legitimate support-tier differences
  versus accidental wrapper drift.

#### Verification

- Confirm the extraction map preserves the three-layer binding architecture.
- Confirm no new milestone work depends on deepening the oversized wrapper
  files without decomposition.

### Milestone 3: C# Supported Binding Lane

#### Tasks

- Promote the existing C# binding lane from smoke-only posture to explicit
  first-class support status if the Milestone 1 matrix confirms it.
- Freeze the supported C# consumer contract around the curated headless
  workflow/session/graph surface.
- Extend verification beyond generation/load smoke to include host-language
  contract assertions for representative success, validation, and error paths.
- Preserve packaged-artifact checks so generated C# and the native library from
  the same build are proven together.

#### Verification

- Rust/native tests cover shared contract shaping and wrapper conversion
  behavior used by the C# lane.
- Host-language C# tests or smokes compile the generated binding, load the
  matching native library, and exercise representative supported calls.
- At least one packaged-artifact acceptance path remains green.

### Milestone 4: Python Binding Lane

#### Tasks

- Define the product identity and support tier for Python host bindings
  distinctly from the out-of-process Python worker/runtime path.
- Add a documented generated-binding package location and host-language test
  strategy for Python.
- Freeze the initial Python-exposed surface to the curated client-facing subset
  instead of mirroring everything UniFFI can technically export.
- Add host-language import/load/smoke expectations and package/native-library
  pairing rules for Python artifacts.

#### Verification

- Rust/native tests cover the shared contract logic used by the Python lane.
- Python host-language tests import or load the generated binding, create the
  native runtime, and exercise representative supported calls.
- Python artifact guidance clearly distinguishes client bindings from Python
  sidecar execution requirements.

### Milestone 5: BEAM / Rustler Lane

#### Tasks

- Keep only the curated client-facing or explicitly justified BEAM surface in
  scope for Rustler.
- Extract pure-Rust contract-shaping logic out of the Rustler boundary where it
  is reusable or backend-owned.
- Add the BEAM-hosted verification harness needed to prove real NIF loading and
  term/error behavior with the `enif_*` runtime present.
- Reconcile the earlier NIF plan as a BEAM-specific execution lane under this
  broader binding-platform source of truth.

#### Verification

- Rust/native tests cover extracted helper logic without BEAM linkage.
- BEAM host-language tests load the real NIF and assert representative contract
  behavior end to end.
- BEAM-only coverage remains focused on the curated supported surface instead
  of wrapper-internal parity for its own sake.

### Milestone 6: Packaging, Documentation, And Source-of-Truth Reconciliation

#### Tasks

- Reconcile the roadmap, this plan, and any language-lane subplans so they do
  not disagree on support tiers, scope, or remaining work.
- Update host-binding docs to describe the curated client-facing surface,
  native-library pairing rules, and per-language expectations.
- Record any remaining follow-on work per language lane after the first-class
  binding platform is established.

#### Verification

- Confirm all source-of-truth docs agree on binding tiers and milestones.
- Confirm artifact/package docs match the actual supported lanes and do not
  overstate parity.

## Re-Plan Triggers

- The curated surface review shows that existing exports should be split into
  multiple support tiers or product identities.
- Python or BEAM host-lane requirements introduce tooling or packaging costs
  that materially change sequencing.
- Shared helper extraction requires a new backend-owned crate instead of
  module-level extraction.
- C# is not ready for `supported` status once stronger host-language contract
  assertions are applied.

## Completion Criteria

- Pantograph has a documented curated binding surface and support-tier policy.
- Supported bindings have both native-language and host-language verification.
- C#, Python, and BEAM each have explicit scope, support tier, and test
  expectations recorded in the source of truth.
- Wrapper crates remain thin enough in ownership that canonical semantics do
  not drift into host-specific implementation layers.
- The roadmap and binding-platform documents are no longer stale or
  contradictory.
