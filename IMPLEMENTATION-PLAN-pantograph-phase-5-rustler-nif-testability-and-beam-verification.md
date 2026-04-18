# Plan: Pantograph Phase 5 Rustler NIF Testability And BEAM Verification

## Status
In progress

Last updated: 2026-04-18

## Current Source-of-Truth Summary

This document now serves as the BEAM-specific execution lane within the broader
Pantograph binding platform planning in
`IMPLEMENTATION-PLAN-pantograph-binding-platform.md`. It keeps the narrower
Rustler NIF problem statement and BEAM-hosted verification detail available,
but the binding-platform plan is now the primary source of truth for overall
binding policy, support tiers, and cross-language sequencing across C#, Python,
and BEAM.

Use the binding-platform plan first when scope, support-tier, or cross-language
questions arise. Use this document when the work being discussed is
specifically the BEAM/Rustler lane.

The earlier native compile blocker caused by the `node-engine` / Rustler
`Send` mismatch is now resolved in backend-owned code. The remaining distinct
verification problem for this lane is the real BEAM-hosted NIF boundary:
`cargo test -p pantograph_rustler` still requires a host that provides the
`enif_*` symbols, so the outstanding work here is now focused on pure-Rust
extraction for cargo-friendly coverage and BEAM-hosted acceptance for the
true wrapper boundary.

## Objective

Resolve the current Rustler NIF verification gap with a two-part follow-on:

- Option 1: refactor NIF-adjacent contract shaping into pure Rust modules or
  backend-owned helpers so most workflow-event envelope logic can be exercised
  with ordinary Rust tests and without BEAM link-time requirements
- Option 2: add a real BEAM-hosted verification harness so the remaining
  Rustler-specific term conversion, NIF loading, and runtime-boundary behavior
  are tested in the environment that provides the missing `enif_*` symbols

The resulting code must preserve backend ownership of workflow semantics, avoid
moving business logic into Rustler or Tauri, and improve the immediate
codebase surroundings enough that further Phase 5 work does not compound
existing standards violations.

## Scope

### In Scope

- Rustler workflow-event and workflow-error boundary logic in
  `crates/pantograph-rustler`
- Extraction of pure-Rust helper modules for serializer, envelope, and
  contract-shaping logic when that logic does not require BEAM terms
- Shared backend-owned helper extraction if the logic is canonical and reused
  by multiple wrapper crates
- A BEAM-hosted integration harness for real NIF loading and contract
  verification
- README, roadmap, and plan updates required by the new boundary shape
- Immediate refactors required to keep touched insertion points compliant while
  this work lands

### Out of Scope

- New workflow semantics unrelated to the existing Phase 5 event contract
- Moving canonical workflow lifecycle logic into Rustler, UniFFI, Tauri, or
  Svelte
- Replacing existing UniFFI or Rustler public facades with a breaking rewrite
- Broader scheduler, KV-cache, or runtime-registry roadmap work
- Shipping production Elixir application code beyond the minimum host harness
  needed to verify the NIF boundary

## Inputs

### Problem

Pantograph currently has Rustler-side serializer parity coverage for some Phase
5 envelopes, but parts of that work remain trapped behind Rustler linkage and
cannot be exercised with ordinary `cargo test` when the BEAM runtime is not
providing the `enif_*` symbols. That leaves two gaps:

- pure contract-shaping logic is harder to test than it should be because it
  still lives too close to the NIF wrapper boundary
- true NIF loading and term/error conversion are not yet pinned by a real
  BEAM-hosted acceptance harness inside this repository

Without a dedicated plan, the likely failure modes are predictable: more logic
accumulates inside the oversized Rustler wrapper, helper extraction happens in
the wrong ownership layer, or the team relies on cargo-only tests for behavior
that can only be trusted when the NIF is loaded by a BEAM host.

### Constraints

- Canonical workflow semantics stay in backend Rust crates, not in Rustler.
- Rustler remains a Layer 2 wrapper crate under
  `LANGUAGE-BINDINGS-STANDARDS.md`, not the owner of workflow meaning.
- Any logic that is FFI-framework-neutral and useful to multiple wrappers must
  live in a backend-owned crate or pure backend module, not be duplicated or
  trapped in Rustler-specific code.
- Any logic that is genuinely Rustler-specific may remain in
  `crates/pantograph-rustler`, but it must be decomposed into focused modules.
- New BEAM-hosted verification must not become a second implementation of the
  contract; it is a harness for the wrapper boundary.
- Existing public binding entry points should stay facade-first and additive
  unless an explicit break is approved.
- New directories created under `src/` must include `README.md`, and any
  external-host harness directory with 3+ files or non-obvious purpose must be
  documented per `DOCUMENTATION-STANDARDS.md`.

### Public Facade Preservation Note

This is a facade-first refactor and verification plan. Existing exported
Rustler entry points should keep their external names and high-level behavior
stable while the internal implementation is split into smaller modules and
while new verification harnesses are added behind the current boundary.

### Assumptions

- The missing linker symbols discussed earlier are the BEAM-provided `enif_*`
  functions expected by Rustler-built NIF artifacts when loaded or linked
  outside a BEAM host.
- No in-repo Mix/ExUnit harness exists today; one must be introduced if Option
  2 is implemented.
- The current Phase 5 workflow-event/error envelopes are the contract to pin,
  not a temporary scaffold.
- Some Rustler helper code may be appropriate to move into a backend-owned
  crate if the same canonical envelope semantics should also be consumed by
  UniFFI or other wrappers.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- `crates/pantograph-rustler`
- `crates/pantograph-uniffi`
- Any backend crate chosen to own extracted canonical envelope semantics
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CODING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/LANGUAGE-BINDINGS-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DEPENDENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TOOLING-STANDARDS.md`

### Affected Structured Contracts

- Rustler workflow-event payload shaping and any helper DTOs extracted to pin
  canonical backend-owned event semantics
- Rustler workflow-error serialization and structured envelope mapping
- Any backend-owned intermediate contract extracted for reuse by Rustler and
  UniFFI
- Real NIF host-loading expectations for the BEAM verification harness
- Any new harness-side assertions for stable event/error field names, labels,
  and terminal semantics

### Affected Persisted Artifacts

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-real-workflow-event-contract.md`
- This dedicated NIF plan
- `crates/pantograph-rustler/src/README.md` if the directory structure or
  API-consumer contract changes
- Any new harness README or test guidance documents created for Option 2
- Any checked-in contract fixtures added as part of acceptance coverage

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate insertion points already exceed decomposition thresholds from
`CODING-STANDARDS.md`:

- `crates/pantograph-rustler/src/lib.rs` is approximately 2511 lines
- `crates/pantograph-uniffi/src/lib.rs` is approximately 1522 lines
- `crates/pantograph-uniffi/src/runtime.rs` is approximately 915 lines

This plan therefore requires decomposition review and focused extraction before
either Option 1 or Option 2 adds more behavior to those files. If canonical
helper extraction touches UniFFI reuse paths, that work must also avoid
deepening the existing oversized wrapper files.

### Concurrency / Race-Risk Review

- Workflow-event and error contract assertions must remain stable under queued,
  cancelled, retried, and resumed runs; extracted helpers cannot accidentally
  erase execution identity or terminal semantics.
- Any BEAM-hosted integration harness must document who starts the host
  runtime, how the NIF is loaded, how test isolation is enforced, and how
  repeated runs avoid cross-test global-state leakage.
- If the harness uses compiled native artifacts or temp directories, each test
  path must own or isolate those resources to satisfy
  `TESTING-STANDARDS.md`.

### Ownership And Lifecycle Note

- Backend crates own canonical workflow-event and error semantics.
- `crates/pantograph-rustler` owns Rustler-specific wrapper code,
  term-conversion glue, and boundary validation only.
- A BEAM host harness, if added, owns bootstrapping and shutting down the host
  runtime for integration verification only; it does not own contract meaning.
- If extracted helpers are reused by UniFFI and Rustler, ownership must sit in
  a backend crate or FFI-neutral helper module with both wrappers adapting it
  separately.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Pure helper extraction leaves canonical semantics stranded in Rustler-specific code | High | Move reusable, FFI-neutral contract shaping into a backend-owned module or crate and keep Rustler-specific translation thin |
| Wrapper files grow further instead of shrinking | High | Make decomposition an explicit early milestone and refuse to add new logic to the existing oversized wrapper entry file without extraction |
| Cargo-only tests create false confidence for real NIF loading behavior | High | Treat Option 2 as required completion work for Rustler-specific boundary verification, not an optional nice-to-have |
| BEAM harness duplicates business logic or becomes production-owned code | Medium | Keep the harness minimal, host-only, and contract-assertion-focused with README-documented ownership |
| Shared helper extraction accidentally couples core semantics to Rustler or UniFFI implementation details | High | Keep backend-owned helper APIs framework-neutral and isolate FFI conversion in thin wrapper modules |

## Standards Review Passes

### Draft Pass

Initial draft written from the current Phase 5 roadmap gap and the known
Rustler linker-symbol problem:

- Option 1 captures pure-Rust extraction for cargo-test coverage
- Option 2 captures real BEAM-hosted verification for the remaining wrapper
  semantics
- Immediate decomposition work is treated as part of the plan, not a future
  cleanup wishlist

### Pass 1: Plan Structure And Traceability

Reviewed against:

- `PLAN-STANDARDS.md`

Corrections applied:

- Added explicit objective, scope, risks, completion criteria, re-plan
  triggers, and milestone verification sections.
- Recorded this file as a dedicated source of truth instead of leaving the NIF
  follow-on buried inside a roadmap bullet.
- Kept roadmap and implementation-plan artifacts in the affected persisted
  artifact list for later reconciliation.

### Pass 2: Architecture, Ownership, And Decomposition

Reviewed against:

- `CODING-STANDARDS.md`

Corrections applied:

- Locked workflow semantics to backend Rust crates and kept Rustler as a thin
  wrapper boundary.
- Added explicit decomposition pressure from the oversized Rustler and UniFFI
  insertion points instead of assuming new code can land in place.
- Required single-owner lifecycle rules so host harness code does not take over
  business-state ownership.

### Pass 3: Bindings And Interop Boundaries

Reviewed against:

- `LANGUAGE-BINDINGS-STANDARDS.md`
- `INTEROP-STANDARDS.md`

Corrections applied:

- Distinguished reusable backend-owned contract shaping from Rustler-specific
  term conversion so the three-layer architecture remains intact.
- Constrained Option 2 to host-boundary verification rather than a second
  implementation of the contract.
- Required explicit startup/shutdown and test-isolation ownership for any BEAM
  harness.

### Pass 4: Testing And Acceptance Coverage

Reviewed against:

- `TESTING-STANDARDS.md`

Corrections applied:

- Required both pure unit coverage for extracted helpers and a true cross-layer
  acceptance path through a BEAM host for NIF-specific behavior.
- Added isolation requirements for native artifacts, temp roots, and repeated
  test runs.
- Treated replay, cancellation, and terminal-envelope preservation as explicit
  acceptance concerns, not just serializer details.

### Pass 5: Documentation, Dependency, And Tooling Hygiene

Reviewed against:

- `DOCUMENTATION-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`
- `TOOLING-STANDARDS.md`

Corrections applied:

- Required README updates for any new source or harness directories.
- Kept new dependencies out of scope by default; a BEAM harness should use the
  minimum host-side tooling necessary rather than broad new runtime additions.
- Preserved traceability expectations so future implementation commits can tie
  code and documentation changes together cleanly.

## Recommendations

- Execute Option 1 before Option 2. Shrinking and clarifying the Rustler
  boundary first will make the BEAM-hosted acceptance harness smaller, more
  stable, and less likely to pin accidental implementation details.
- Prefer backend-owned extraction when helper logic expresses canonical event
  or error semantics that UniFFI may also need. Keep wrapper-local extraction
  only for Rustler-specific term shaping.
- Keep the BEAM harness intentionally narrow. Its job is to verify NIF loading
  and boundary contracts, not to become a second workflow runtime.

## Milestones

### Milestone 1: Contract Freeze And Decomposition Map

#### Tasks

- Inventory the current Rustler-exported workflow-event and workflow-error
  entry points, helper paths, and envelope shapes that Phase 5 needs to pin.
- Mark which logic is canonical and reusable versus which logic is
  Rustler-specific term conversion or validation.
- Decide the extraction target for each helper slice:
  backend-owned module/crate for FFI-neutral semantics, or focused
  `pantograph-rustler` submodule for wrapper-only behavior.
- Define the facade-preserving split for `crates/pantograph-rustler/src/lib.rs`
  so new work does not deepen the existing 2511-line file.
- Record any nearby UniFFI extraction that must happen in parallel if shared
  helper reuse would otherwise deepen oversized wrapper files there.

#### Verification

- Review the decomposition map against `CODING-STANDARDS.md` thresholds before
  implementation starts.
- Confirm the planned helper ownership keeps the three-layer binding
  architecture intact.
- Update this plan and the roadmap if the decomposition map changes the
  expected sequencing.

#### Current status note

- `workflow_event_contract.rs`, `workflow_host_contract.rs`, and
  `elixir_data_graph_executor.rs` already exist as focused Rustler boundary
  modules.
- `resource_registration.rs` is now also carved out of the NIF facade so
  load-time resource registration no longer deepens the main Rustler entry
  file or relies on ignored must-use results in `load`.
- `workflow_graph_contract.rs` now owns the workflow graph JSON CRUD and
  validation helpers behind the public NIF facade, reducing another large
  Rustler insertion point before deeper host-side coverage lands.
- `mix` and `elixir` are not installed in the current environment, so BEAM
  host verification scaffolding can be authored here but not executed locally
  until host tooling is available.

### Milestone 2: Option 1 Pure-Rust Extraction And Cargo-Test Coverage

#### Tasks

- Extract pure serializer, envelope, and contract-shaping logic out of the
  Rustler entry file into focused modules or backend-owned helpers.
- Keep Rustler boundary modules limited to argument validation, thin
  term-conversion glue, and wrapper-local error adaptation.
- Add Rust unit tests for the extracted pure logic so the main Phase 5
  contract behavior can run under normal `cargo test`.
- If canonical helper extraction benefits UniFFI or other wrappers, route them
  through the shared helper interface instead of duplicating logic.
- Update module README content when new `src/` directories or materially new
  consumer contracts are introduced.

#### Verification

- `cargo test` for the package(s) that now own the extracted pure logic
- `cargo check -p pantograph-rustler`
- Any wrapper-level serializer parity tests that can now run without a BEAM
  host

#### Current status note

- The earlier `Send`-safety compile blocker is already resolved outside this
  milestone in backend-owned `node-engine` code.
- Option 1 remains open for the separate goal of shrinking the Rustler wrapper
  and moving cargo-testable contract shaping away from BEAM-bound code.

### Milestone 3: Option 2 BEAM-Hosted NIF Acceptance Harness

#### Tasks

- Add a minimal BEAM-hosted harness, likely a Mix/ExUnit project or equivalent
  host-specific test scaffold, in a documented directory with clear ownership.
- Configure the harness to build or locate the Pantograph Rustler NIF artifact
  and load it in the host runtime that provides `enif_*`.
- Add end-to-end contract assertions for the Rustler-specific boundary:
  representative workflow-event terms, structured error envelopes, and the
  loading/error behavior that cargo-only tests cannot prove.
- Ensure harness tests isolate temp artifacts, environment variables, and any
  process-global runtime state.
- Document how local developers and CI should run the harness, including host
  prerequisites and expected artifact paths.

#### Verification

- Host-side NIF acceptance run, such as `mix test`, once the chosen harness
  exists
- At least one cross-layer acceptance path from native Rust implementation
  through the Rustler wrapper into the BEAM consumer assertions
- Re-run harness suites to catch global-state leakage or artifact reuse bugs

#### Current status note

- This milestone now owns the remaining `enif_*` linker-symbol verification
  gap for true Rustler NIF loading and host-boundary assertions.
- A first documented BEAM harness scaffold now exists under `bindings/beam/`,
  with a minimal Mix/ExUnit smoke project that loads the compiled NIF through
  `PANTOGRAPH_RUSTLER_NIF_PATH` and exercises real exported Rustler functions.
- The local environment now has a working Erlang/Elixir toolchain through the
  existing `mise` install path, and the new Mix/ExUnit smoke harness passes
  against the compiled `pantograph_rustler` NIF.

### Milestone 4: Source-of-Truth And Contract Reconciliation

#### Tasks

- Reconcile the roadmap and the main Phase 5 plan to reflect Option 1 and
  Option 2 completion status with no stale wording.
- Update any touched README files with API-consumer or structured-producer
  contract details required by the new boundary shape.
- Record any residual follow-on work discovered during harness implementation,
  including whether more wrapper-neutral helper extraction should be promoted
  into a shared backend-owned contract layer.

#### Verification

- Confirm roadmap, this plan, and the main Phase 5 plan agree on milestone
  status and remaining scope.
- Confirm new or changed directories satisfy README requirements.

## Re-Plan Triggers

- The extracted helper inventory shows that currently Rustler-local logic is
  actually canonical and should move into a backend-owned crate.
- The chosen BEAM harness requires new tooling, packaging, or CI assumptions
  beyond the current minimal host-verification scope.
- Existing Rustler public facades cannot be preserved without an explicit
  contract break.
- UniFFI reuse or nearby wrapper refactors turn out to be required sooner than
  expected to keep the immediate surroundings standards-compliant.

## Completion Criteria

- Option 1 is complete: the non-NIF contract-shaping logic required by Phase 5
  runs under normal Rust tests without depending on BEAM-provided linker
  symbols.
- Option 2 is complete: the remaining Rustler-specific NIF boundary behavior is
  pinned by a real BEAM-hosted acceptance harness inside the repository.
- The Rustler wrapper remains a thin adapter, not the owner of workflow
  semantics.
- Immediate touched insertion points are decomposed enough that this work does
  not deepen the known oversized-wrapper standards violations.
- The roadmap and implementation-plan files reflect the same accurate status.

## Execution Notes

- 2026-04-18: The backend-owned `Send` mismatch between `node-engine` and the
  Rustler orchestration bridge was repaired in the dedicated unblock slice.
  `cargo check -p pantograph_rustler` now succeeds, while
  `cargo test -p pantograph_rustler` remains blocked only by the separate
  BEAM-provided `enif_*` linker-symbol boundary this plan is intended to
  address.
- 2026-04-18: Milestone 1 decomposition continued inside
  `crates/pantograph-rustler`: resource registration moved into a focused
  `resource_registration.rs` module so `lib.rs` remains facade-first while the
  NIF load boundary becomes easier to evolve for later BEAM harness work.
- 2026-04-18: Milestone 3 scaffolding started under `bindings/beam/` with a
  documented Mix/ExUnit smoke harness that targets the compiled
  `pantograph_rustler` NIF through an explicit `PANTOGRAPH_RUSTLER_NIF_PATH`
  contract.
- 2026-04-18: Milestone 2 extraction also started inside
  `crates/pantograph-rustler`: workflow graph JSON CRUD and validation helpers
  moved behind a focused `workflow_graph_contract.rs` module so the public NIF
  facade stays stable while the Rustler entry file keeps shrinking.
- 2026-04-18: The BEAM smoke harness now loads the compiled
  `pantograph_rustler` NIF successfully under local Mix/ExUnit execution and
  passes version plus workflow graph round-trip smokes after aligning the
  Elixir shim with the full default Rustler export surface.
