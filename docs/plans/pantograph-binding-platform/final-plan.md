# Plan: Pantograph Binding Platform

## Status
Draft

Last updated: 2026-04-23

## Current Source-of-Truth Summary

This document is the dedicated source of truth for Pantograph's first-class
binding platform planning. It expands the earlier Rustler-only NIF follow-on
into a broader standards-reviewed plan covering:

- curated client-facing binding surface policy
- shared backend-owned binding contract ownership
- C# binding hardening
- Python binding introduction as a real host-consumer lane
- Elixir/BEAM Rustler verification as a specialized host lane

The roadmap should summarize status and point here for binding-platform
milestone detail. Binding-specific subplans may exist for individual host
lanes, but this document is the primary source of truth for overall binding
scope, sequencing, support tiers, and verification expectations.

Canonical plan path:
`docs/plans/pantograph-binding-platform/final-plan.md`.

The Elixir/BEAM Rustler lane is no longer blocked on NIF compilation. The
Rustler crate now compiles and builds successfully, and the in-repo Mix/ExUnit
smoke harness proves real NIF loading plus initial graph/error contract
behavior under BEAM. The remaining Elixir/BEAM-specific issue relevant to this
broader bindings plan is narrower: raw `cargo test -p pantograph_rustler` is
still not an authoritative host verification path for NIF-bound behavior, and
the existing BEAM harness is not yet broad enough to justify the current
Rustler README's broad `supported` tier without a support-tier reconciliation
pass.

Since the first version of this plan was written, binding-adjacent docs and
wrappers have moved forward:

- `crates/pantograph-uniffi/README.md` now defines supported, experimental, and
  internal-only surfaces for the native headless library.
- `crates/pantograph-rustler/README.md` now defines support tiers, but its
  broad `Supported` row is ahead of the available host-language verification.
- `bindings/beam/pantograph_native_smoke` now exists and covers NIF load,
  version, workflow JSON roundtrip, validation errors, and parse errors.
- `bindings/csharp/Pantograph.NativeSmoke` now exercises the generated C#
  binding against the real native library through session create, run, status,
  queue, keep-alive, and close paths.
- `bindings/csharp/PACKAGE-README.md`,
  `bindings/csharp/Pantograph.DirectRuntimeQuickstart/README.md`, and the
  generated package manifests are now shipped as part of the C# artifact
  contract and therefore need explicit support-tier reconciliation rather than
  being treated as incidental examples.
- `crates/pantograph-uniffi/src/runtime_tests.rs` now covers direct-runtime
  invalid-request session envelopes plus persisted graph/edit-session behavior,
  so the native-side baseline for the direct headless runtime is stronger than
  earlier drafts of this plan assumed.
- Headless binding CI currently packages and uploads Linux C# and native
  artifacts only; Windows and macOS artifact expectations are documented but
  not yet backed by equivalent CI artifact verification.

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
- Rustler-backed Elixir/BEAM wrapper surface exposed through
  `crates/pantograph-rustler`
- Client-facing workflow/session/graph APIs intended for external host
  applications
- Binding support-tier policy for C#, Python, and Elixir/BEAM
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

- the repository now has partial support-tier documentation, but the plan,
  roadmap, UniFFI docs, Rustler docs, and binding harnesses do not yet agree on
  which surfaces are actually `supported`, `experimental`, or `internal-only`
- C# is the only host-language lane that currently looks close to first-class,
  and its generated-binding smoke already covers real session-based execution
- Python is mentioned as a potential UniFFI host language, but not yet treated
  as a first-class client binding with its own packaged flow and host-language
  tests
- Elixir/BEAM Rustler verification has historically been scoped narrowly
  around NIF loading rather than integrated into a broader binding strategy;
  the baseline local smoke harness now exists, but broader event, callback,
  session, and error-envelope coverage plus support-tier positioning still need
  to be integrated into the larger bindings plan
- the plan artifact itself must now live under `docs/` and stop using a
  repository-root path if it is going to remain standards-compliant source of
  truth
- shipped C# package docs, quickstart docs, and generated manifests are part
  of the external consumer contract, but earlier drafts of this plan did not
  treat them as first-class affected artifacts
- the headless graph-authoring surface still relies partly on out-of-band node
  knowledge because the direct binding lane does not yet expose backend-owned
  node-definition and port-option discovery from the registry
- immediate wrapper insertion points are already oversized enough that more
  binding work would deepen existing standards violations if it lands without
  decomposition
- product-native artifact docs mention Linux, Windows, and macOS library
  layouts, while current headless binding CI only proves and uploads Linux
  artifacts

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
- Supported graph-authoring bindings must expose backend-owned node-definition
  and dynamic port-option discovery rather than requiring host-maintained node
  catalogs.
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

- C# remains the closest current candidate for a first explicitly reconciled
  `supported` host binding because it already has generated-artifact packaging,
  host-language smoke coverage, and packaged quickstart checks.
- Python should be treated as a distinct host-consumer binding lane, not
  conflated with the out-of-process Python worker/runtime separation work.
- Elixir should be the product-facing host-language framing for the BEAM lane;
  Rustler remains the implementation mechanism, and BEAM/OTP lifecycle details
  remain part of that lane's runtime contract.
- Elixir/BEAM should be treated as unreconciled until the current broad
  Rustler `Supported` README row is either narrowed/downgraded or backed by
  expanded host-side event, callback, error-envelope, and session coverage.
- The current headless JSON DTO flow through `pantograph-workflow-service`
  remains the most realistic starting point for cross-language bindings.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
- `docs/headless-native-bindings.md`
- `bindings/csharp`
- `bindings/beam`
- `crates/pantograph-uniffi`
- `crates/pantograph-rustler`
- `.github/workflows/headless-embedding-contract.yml`
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
- Headless graph-authoring discovery contracts for backend-owned node
  definitions, queryable ports, and port-option lookup
- Binding support-tier matrix for C#, Python, and Elixir/BEAM
- Generated host binding package expectations and native library pairing rules
- Wrapper-local error and event projection contracts
- Any shared backend-owned binding helper contracts extracted for reuse across
  UniFFI and Rustler

### Affected Persisted Artifacts

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `docs/plans/pantograph-binding-platform/final-plan.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
- `docs/headless-native-bindings.md`
- `bindings/csharp/README.md`
- `bindings/csharp/PACKAGE-README.md`
- `bindings/csharp/Pantograph.DirectRuntimeQuickstart/README.md`
- `bindings/beam/README.md`
- `bindings/beam/pantograph_native_smoke/README.md`
- `crates/pantograph-uniffi/README.md`
- `crates/pantograph-rustler/README.md`
- Any new `bindings/python/` documentation or expanded Elixir/BEAM harness
  documentation introduced by implementation
- `.github/workflows/headless-embedding-contract.yml`
- `scripts/package-uniffi-csharp-artifacts.sh`
- `scripts/check-packaged-csharp-quickstart.sh`
- Generated package manifests and manifest guidance for host-language artifacts

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate wrapper insertion points still exceed decomposition thresholds
from `CODING-STANDARDS.md`, although they are materially smaller than in the
earlier draft of this plan:

- `crates/pantograph-rustler/src/lib.rs` is approximately 889 lines after
  substantial extraction into focused helper modules
- `crates/pantograph-uniffi/src/lib.rs` is approximately 720 lines
- `crates/pantograph-uniffi/src/runtime.rs` is approximately 588 lines

The binding-platform plan must therefore include decomposition review and
shared-helper extraction before significantly expanding the public binding
surface or host-language verification paths. New exported surface should not be
added to these oversized files unless the same slice also moves an equivalent
or larger responsibility into a focused module or records an explicit
decomposition exception with a revisit trigger.

### Current Documentation Contract Gaps

- `crates/pantograph-uniffi/README.md`, `crates/pantograph-uniffi/src/README.md`,
  `crates/pantograph-rustler/README.md`, and
  `crates/pantograph-rustler/src/README.md` now mostly satisfy host-facing and
  structured-producer README expectations, but their support-tier content must
  be reconciled with this plan and the roadmap.
- `bindings/csharp/README.md` and `bindings/beam/README.md` remain lighter than
  the current README template expectations for host-facing directories, and the
  shipped C# artifact docs in `bindings/csharp/PACKAGE-README.md` and
  `bindings/csharp/Pantograph.DirectRuntimeQuickstart/README.md` are not yet
  reconciled with the same support-tier and platform-support language.
- `bindings/beam/pantograph_native_smoke/README.md` documents the current BEAM
  smoke surface, but it does not yet explain how that narrow harness maps to
  the much broader Rustler README `Supported` row.
- `docs/headless-native-bindings.md` documents C# and platform artifact layouts
  but does not yet state a support-tier matrix or distinguish CI-backed
  platform artifacts from documented future platform layouts.

### Current Cross-Platform Artifact Gap

`docs/headless-native-bindings.md` describes Linux, Windows, and macOS native
library layouts. Current headless binding CI builds, packages, smoke-tests, and
uploads the Linux artifact path only. Before the plan can claim cross-platform
native binding support, it must define:

- required versus best-effort host platforms
- expected native library names per target
- which platform artifacts are produced in CI
- which host-language smoke or package checks run per platform
- whether unsupported targets are documented as future work rather than shipped
  product artifacts

### Concurrency / Race-Risk Review

- Host-language test lanes will mutate native-library search paths, environment
  variables, temp artifact roots, and compiled output directories; these must
  be isolated per suite to avoid state leakage.
- Python binding tests must not blur the boundary between the Python host
  binding lane and the out-of-process Python worker/runtime lane.
- Elixir/BEAM harnesses must document NIF load/unload lifecycle ownership and
  avoid global runtime leakage across repeated test runs.
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
| Elixir product binding semantics are conflated with Rustler implementation details | Medium | Document Elixir as the host lane and Rustler as the wrapper mechanism |
| Supported bindings drift because only Rust-side or only host-side tests exist | High | Require both native-language and host-language tests for supported bindings |
| Oversized wrapper files make future binding work unreviewable | High | Make decomposition an early milestone before widening the surface |
| README support tiers overstate host-language verification | High | Reconcile support tiers before promoting or packaging public binding contracts |
| Platform artifact docs overpromise beyond CI coverage | High | Add a platform support matrix and CI/package checks before claiming support |

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
- Moved the plan to `docs/plans/pantograph-binding-platform/final-plan.md` so
  the source-of-truth artifact follows the current documentation artifact
  layout.
- Added explicit milestones, verification, re-plan triggers, and completion
  criteria.
- Refreshed the plan after the Rustler split, BEAM smoke harness, C# smoke
  expansion, direct-runtime native tests, shipped C# package docs, and binding
  README support-tier updates so execution starts from current repo state.

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
- Added a reconciliation gate for existing UniFFI and Rustler support-tier
  tables so documentation cannot claim `supported` status ahead of verification.

### Pass 3: Interop And Lifecycle Boundaries

Reviewed against:

- `INTEROP-STANDARDS.md`
- `CROSS-PLATFORM-STANDARDS.md`

Corrections applied:

- Recorded host-runtime lifecycle ownership, artifact-path isolation, and
  startup/shutdown expectations for the different language lanes.
- Preserved thin-wrapper guidance and avoided moving semantics into host-side
  or wrapper-local code.
- Added a cross-platform artifact matrix requirement before docs or artifacts
  can imply Windows/macOS parity with the current Linux-backed package path.

### Pass 4: Testing And Verification

Reviewed against:

- `TESTING-STANDARDS.md`

Corrections applied:

- Required native-language plus host-language verification for supported
  bindings.
- Added packaging-pairing verification and cross-layer acceptance obligations.
- Added test-isolation requirements for native library paths, temp roots, and
  host runtime state.
- Reclassified BEAM work from "create a harness" to "expand and integrate the
  existing harness" because the Mix/ExUnit NIF smoke baseline now exists.

### Pass 5: Documentation And Dependency Hygiene

Reviewed against:

- `DOCUMENTATION-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`

Corrections applied:

- Planned README/doc updates where bindings become first-class artifacts rather
  than repo-internal smoke utilities.
- Kept the plan minimal on new dependencies until each host-language lane is
  justified by the support-tier reconciliation.
- Added explicit README contract work for `bindings/csharp` and `bindings/beam`
  so host-facing harness/package directories meet documentation standards.

## Recommendations

- Reconcile the existing support-tier tables before changing code. In
  particular, either narrow/downgrade the current broad Rustler `Supported`
  surface or add the missing Elixir/BEAM host-language verification needed to
  justify it.
- Treat C# as the first likely `supported` binding lane after reconciliation
  because it has generated/package/runtime smoke coverage and CI packaging.
- Treat Python as the next candidate lane, but keep it explicitly separate from
  the Python worker/runtime sidecar concerns.
- Keep Elixir/BEAM as a specialized lane focused on real host-boundary
  verification and curated surface selection, not blanket parity with UniFFI
  exports.
- Do not advertise Windows or macOS native binding artifacts as supported until
  the plan defines their target status and CI/package verification path.

## Milestones

### Milestone 1: Binding Product Contract And Support-Tier Reconciliation

#### Tasks

- Define the Pantograph client-facing binding surface for workflow execution,
  workflow sessions, workflow graph authoring, shutdown/lifecycle calls, and
  every other surface currently labeled `supported` in host-facing READMEs or
  shipped package docs.
- Decide whether backend-owned node-definition discovery, queryable-port
  discovery, and port-option discovery are part of the minimum supported graph
  authoring surface for each host lane, and document any temporary gaps
  explicitly instead of relying on implicit out-of-band node knowledge.
- Reconcile existing support-tier claims in `crates/pantograph-uniffi/README.md`
  and `crates/pantograph-rustler/README.md` with this plan, the shipped C#
  artifact docs, and the roadmap.
- Classify each host lane and exported surface as `supported`,
  `experimental`, or `internal-only`.
- Inventory the full current Rustler `Supported` row, including workflow JSON
  graph helpers, executor resources, orchestration store operations, node
  registry operations, callback response/error NIFs, and Pumas API resource
  operations, then either justify each surface with an external consumer
  rationale and verification plan or downgrade/reclassify it.
- Record the initial reconciled matrix. Expected starting point:
  C# over the direct `pantograph_headless`/UniFFI runtime path is candidate
  `supported`; Python host bindings are candidate `experimental`;
  Elixir/BEAM over Rustler is candidate `experimental` unless the existing
  broad Rustler `Supported` row is backed by expanded Elixir/BEAM host
  verification in the same milestone.
- Mark non-client/internal capabilities that wrappers must not expose as part
  of the public binding contract without a later explicit decision.
- Mark legacy in-memory workflow engine, frontend-HTTP exports, debug/admin
  helpers, and host-harness-only functions as either explicitly experimental or
  internal-only unless a documented external consumer rationale exists.
- Reconcile `docs/headless-native-bindings.md`, wrapper READMEs, binding
  harness READMEs, shipped C# package docs, generated manifest expectations,
  and the roadmap with the curated surface policy.
- Remove the repository-root copy of this plan once all live references point
  to the canonical `docs/plans/` path.

#### Verification

- Confirm the curated surface and support tiers are documented in this plan,
  wrapper READMEs, binding harness/package READMEs, and the roadmap without
  contradictory tier labels.
- Confirm every included public capability has an external consumer rationale.
- Confirm no README claims `supported` for a surface that lacks both native-side
  and host-language verification required by the testing standards.
- Confirm any graph-authoring surface labeled `supported` does not depend on a
  host-maintained node catalog when the same backend registry data can be
  exposed through bindings.
- Confirm any Elixir/BEAM surface that remains `supported` is explicitly
  justified and no longer inherited implicitly from the older broad Rustler
  README row.
- Confirm current `supported`, `experimental`, and `internal-only` labels state
  packaging, versioning, and host-test expectations.
- Confirm the canonical source-of-truth path for this plan is under `docs/`
  and all live references resolve to that path.

### Milestone 2: Shared Contract Layer And Wrapper Decomposition Map

#### Tasks

- Inventory which current UniFFI and Rustler helpers are canonical and reusable
  versus wrapper-local.
- Design the backend-owned or binding-neutral helper boundary for reusable
  contract shaping and error categorization.
- Define the facade-preserving decomposition for the oversized UniFFI and
  Rustler files before more binding surface is added. Current targets:
  split UniFFI runtime workflow/session/graph/lifecycle methods out of
  `runtime.rs`; continue moving Rustler callback/event/host/resource groups out
  of `lib.rs`; keep `lib.rs` files as export facades.
- Record which host-lane differences are legitimate support-tier differences
  versus accidental wrapper drift.
- Add a decomposition exception note for any file that remains over 500 lines
  after a milestone, with a concrete reason and revisit trigger.
- Require each new binding export slice to either reduce oversized facade
  responsibility or explain why no further split is possible in that slice.

#### Verification

- Confirm the extraction map preserves the three-layer binding architecture.
- Confirm no new public binding surface is added to
  `crates/pantograph-uniffi/src/lib.rs`,
  `crates/pantograph-uniffi/src/runtime.rs`, or
  `crates/pantograph-rustler/src/lib.rs` without a same-slice decomposition or
  recorded exception.
- Confirm all extracted modules have README coverage where required by the
  documentation standards.
- Confirm native tests cover extracted pure helper logic without requiring
  host runtimes unless the helper is explicitly host-boundary-only.

### Milestone 3: C# Supported Binding Lane

#### Tasks

- Promote the existing C# binding lane from current generated/package/runtime
  smoke posture to explicit first-class support status if the Milestone 1 matrix
  confirms it.
- Freeze the supported C# consumer contract around the curated headless
  workflow/session/graph surface.
- Add backend-owned node-definition discovery and dynamic port-option discovery
  to the direct headless graph-authoring contract before that graph-authoring
  surface is classified as fully supported.
- Extend verification beyond the current session success smoke to include
  host-language contract assertions for representative validation, malformed
  request, missing workflow/session, and backend-owned error-envelope paths.
- Preserve packaged-artifact checks so generated C# and the native library from
  the same build are proven together.
- Reconcile shipped C# artifact docs and manifests with the actual supported
  surface so the artifact README, quickstart README, generated manifest, and
  repository docs describe the same contract.
- Decide whether C# support is Linux-only at first or cross-platform. If
  cross-platform, add Windows/macOS package and smoke verification; if Linux-only
  initially, document that explicitly in package docs and release notes.

#### Verification

- Rust/native tests cover shared contract shaping and wrapper conversion
  behavior used by the C# lane.
- Host-language C# tests or smokes compile the generated binding, load the
  matching native library, and exercise representative supported calls.
- Supported graph-authoring smoke coverage proves clients can discover node
  definitions and queryable port options without an out-of-band node catalog.
- At least one packaged-artifact acceptance path remains green.
- The headless binding CI matrix matches the documented C# platform support
  claim.
- Generated C# and native library artifacts are version-matched from the same
  build and manifests say so.

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
- Decide whether Python generation uses the repo-owned UniFFI bindgen helper,
  upstream `uniffi-bindgen`, or another pinned tool, and document the dependency
  and version policy before adding package scripts.
- Document platform support separately from the C# lane; do not inherit C#
  platform claims automatically.

#### Verification

- Rust/native tests cover the shared contract logic used by the Python lane.
- Python host-language tests import or load the generated binding, create the
  native runtime, and exercise representative supported calls.
- Python artifact guidance clearly distinguishes client bindings from Python
  sidecar execution requirements.
- Python generation, import/load, and package/native pairing are reproducible
  from documented commands and do not require hand-editing generated files.

### Milestone 5: Elixir / BEAM / Rustler Lane

#### Tasks

- Define Elixir as the product-facing host binding identity and Rustler as the
  implementation mechanism for the BEAM lane.
- Reconcile the current Rustler README support-tier table with Milestone 1.
  Either narrow/downgrade broad `Supported` claims or add the host-language
  verification needed to justify them.
- Keep only the curated client-facing or explicitly justified Elixir/BEAM
  surface in scope for Rustler.
- Extract pure-Rust contract-shaping logic out of the Rustler boundary where it
  is reusable or backend-owned.
- Expand the existing Elixir/BEAM-hosted verification harness beyond NIF load,
  workflow graph roundtrip, validation errors, and parse errors to cover the
  reconciled Elixir/BEAM support tier.
- Add host-side Elixir/BEAM assertions for representative event, callback,
  error-envelope, resource lifecycle, and session behavior if those surfaces
  remain public or supported.
- Add CI or documented local gating for the BEAM harness where the required
  Elixir/Erlang runtime is available.
- Reconcile the earlier NIF plan as a BEAM-specific execution lane under this
  broader binding-platform source of truth.

#### Verification

- Rust/native tests cover extracted helper logic without BEAM linkage.
- Elixir/BEAM host-language tests load the real NIF and assert representative
  contract behavior end to end.
- Elixir/BEAM-only coverage remains focused on the curated supported surface
  instead of wrapper-internal parity for its own sake.
- The Rustler README no longer says direct `cargo test -p pantograph_rustler`
  is waiting on a harness without acknowledging the current Mix/ExUnit baseline.
- Any Elixir/BEAM surface left as `supported` has native helper coverage plus
  host-language smoke/acceptance coverage through the real NIF.

### Milestone 6: Packaging, Cross-Platform, Documentation, And Source-of-Truth Reconciliation

#### Tasks

- Reconcile the roadmap, this plan, and any language-lane subplans so they do
  not disagree on support tiers, scope, or remaining work.
- Update host-binding docs to describe the curated client-facing surface,
  native-library pairing rules, and per-language expectations.
- Add or update `API Consumer Contract` and `Structured Producer Contract`
  sections for `bindings/csharp`, `bindings/beam`, and any new
  `bindings/python` directory.
- Update shipped C# artifact docs and generated manifest content so
  `bindings/csharp/PACKAGE-README.md`,
  `bindings/csharp/Pantograph.DirectRuntimeQuickstart/README.md`,
  `docs/headless-native-bindings.md`, and package `manifest.json` semantics stay
  aligned.
- Add a platform support matrix covering Linux, Windows, macOS ARM, and macOS
  Intel with each target marked `supported`, `best-effort`, or `future`.
- Align `docs/headless-native-bindings.md`, packaging scripts, manifests, and
  CI artifact uploads with that platform support matrix.
- Record any remaining follow-on work per language lane after the first-class
  binding platform is established.

#### Verification

- Confirm all source-of-truth docs agree on binding tiers and milestones.
- Confirm artifact/package docs match the actual supported lanes and do not
  overstate parity.
- Confirm CI or documented release checks exist for every platform/language
  combination marked `supported`.
- Confirm unsupported or best-effort platform artifacts are documented as such
  and are not marketed as supported product artifacts.
- Confirm the repository decision-traceability script recognizes updated
  binding documentation directories or the plan records why they are excluded.

## Re-Plan Triggers

- The curated surface review shows that existing exports should be split into
  multiple support tiers or product identities.
- Python or Elixir/BEAM host-lane requirements introduce tooling or packaging
  costs that materially change sequencing.
- Shared helper extraction requires a new backend-owned crate instead of
  module-level extraction.
- C# is not ready for `supported` status once stronger host-language contract
  assertions are applied.
- Current Rustler `Supported` claims cannot be justified without substantial
  BEAM harness expansion.
- Cross-platform package verification is not feasible in the current CI budget,
  requiring a Linux-only initial support posture.
- A wrapper file would grow further past the decomposition threshold without a
  same-slice split or documented exception.

## Completion Criteria

- Pantograph has a documented curated binding surface and support-tier policy.
- Supported bindings have both native-language and host-language verification.
- C#, Python, and Elixir/BEAM each have explicit scope, support tier, and test
  expectations recorded in the source of truth.
- The canonical binding-platform source of truth lives under `docs/plans/`, and
  active roadmap/subplan references point there instead of the repository root.
- Platform support for native binding artifacts is explicit and matches CI or
  release verification.
- Binding harness/package directories, shipped C# artifact docs, and generated
  manifest expectations meet host-facing and structured-producer documentation
  requirements.
- Wrapper crates remain thin enough in ownership that canonical semantics do
  not drift into host-specific implementation layers, and remaining oversized
  files have accepted decomposition exceptions with revisit triggers.
- The roadmap and binding-platform documents are no longer stale or
  contradictory.
