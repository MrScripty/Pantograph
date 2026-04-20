# Plan: Pantograph Native Library and Language Binding Separation

## Objective

Refactor Pantograph's foreign-language packaging model so there is one
product-facing native Pantograph headless library per platform and separate,
optional language binding packages that target that library instead of
re-packaging Pantograph itself per language.

## Scope

### In Scope

- Rename and reframe the product-facing shared library/export surface around a
  Pantograph headless identity instead of a binding-framework identity.
- Preserve the existing direct headless API contract for workflow execution,
  workflow sessions, workflow graph inspection, workflow graph editing, and
  workflow persistence.
- Split release artifacts into:
  - one native Pantograph headless library package per platform
  - one optional generated binding package per language
- Update docs, examples, manifests, and CI to reflect the new packaging model.
- Add a migration/compatibility plan for current `pantograph_uniffi` consumers.
- Propose an amendment to the language-binding standards so this packaging model
  is explicit for future repos.

### Out of Scope

- Replacing UniFFI with another binding generator.
- Changing Pantograph workflow/session DTO shapes.
- Bundling models, Puma-Lib assets, Python environments, or managed inference
  runtime payloads into the native library package.
- Redesigning the Rust crate graph beyond what is necessary to separate
  product-native naming from binding-layer naming.
- Introducing a new application SDK surface beyond the existing Pantograph
  headless API contract.

## Inputs

### Problem

Pantograph currently exposes foreign-language consumers through a shared
library and artifact naming scheme centered on `pantograph_uniffi`. That is an
implementation-detail name, not a product name. The result is confusing:

- it suggests the native `.so/.dll/.dylib` is "just bindings"
- it obscures that the library actually contains Pantograph headless runtime
  behavior
- it encourages packaging docs that duplicate the native library for each
  language instead of treating the native library as the shared product payload
- it makes the consumer mental model "download the UniFFI library" instead of
  "download Pantograph native and optionally download a language surface"

The desired architecture is:

- one Pantograph headless native library per platform
- optional host-language binding files/packages generated against that library
- language packages remain thin and replaceable
- the native Pantograph library is downloaded once and reused across languages

### Constraints

- The current direct headless capability surface must remain available:
  workflow execution, sessions, scheduler queue operations, workflow graph
  save/load/list, workflow graph edit sessions, and graph mutation helpers.
- The Rust package roles must remain clear:
  - Pantograph domain/headless behavior stays in the existing core/runtime
    crates
  - the FFI/binding crate remains an adapter layer
  - packaging/CI scripts remain tooling/runtime-entry concerns rather than
    absorbing product logic
- Existing Rust crate consumers should continue to use the Rust crates directly;
  this refactor is about foreign-language/native packaging identity.
- Existing foreign-language consumers may already expect the current generated
  method names and/or native filename; migration must be deliberate.
- Current CI already proves direct C# generation, compile, and runtime smoke.
  The refactor must preserve or replace those checks with equivalent or better
  coverage.
- Artifact documentation must stay explicit that models, Python sidecar
  dependencies, and managed inference runtimes are separate runtime
  dependencies, not hidden inside the binding package.
- The repo currently has unrelated local changes; any implementation must avoid
  touching them.

### Assumptions

- The existing Pantograph headless implementation can continue to be exposed
  through UniFFI while changing the product-facing native library identity.
- Some form of compatibility bridge or transition artifact will be needed for
  current `pantograph_uniffi` consumers, at least for one migration window.
- The generated host bindings can remain language-specific outputs under
  `bindings/` or `target/` while artifact names and docs shift to a
  Pantograph-native naming model.
- Consumers benefit more from a single native Pantograph artifact plus optional
  generated language layers than from a language-specific "native runtime"
  package.

### Dependencies

- `crates/pantograph-embedded-runtime`
- `crates/pantograph-workflow-service`
- `crates/pantograph-uniffi`
- current C# smoke harness and packaging scripts
- CI workflow `.github/workflows/headless-embedding-contract.yml`
- Coding standards under
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| UniFFI may couple generated bindings, symbol names, and library naming more tightly than expected | High | Start with a compatibility spike that proves what can be renamed directly vs what needs a facade/alias strategy before refactoring release packaging |
| Renaming the native library without a transition path may break existing consumers | High | Ship compatibility docs and, if needed, a temporary legacy artifact name or compatibility loader guidance for one migration window |
| CI may falsely validate only compile-time host bindings while the runtime loader path changes break actual use | High | Keep runtime smoke plus packaged-artifact compile checks and add packaged native-library loading verification |
| Docs may still imply the native package contains managed runtimes/models | Medium | Explicitly separate native Pantograph library, generated binding layers, and external runtime/model dependencies in every artifact README |
| Packaging refactor could blur Rust crate API vs foreign-language native library API | Medium | Add a public facade preservation note and keep Rust crate guidance separate from FFI/native package guidance |

### Affected Structured Contracts

- Generated C# binding file shape and namespace/package guidance
- Native artifact manifest fields and artifact names
- CI artifact names and download expectations
- Consumer documentation for matching generated bindings to native library
  versions

### Affected Persisted Artifacts

- CI-uploaded zip artifact names and contents
- manifest files included in packaged binding/native artifacts
- documentation links that point at artifact names or package layouts

## Standards Alignment Review

This plan aligns with the current coding and architectural standards if
implemented with the following invariants:

- **Layered/package boundaries stay intact.**
  - `pantograph-workflow-service` and `pantograph-embedded-runtime` remain the
    headless/domain-facing implementation layers.
  - the foreign-language wrapper crate remains an infrastructure/adapter layer.
  - packaging scripts and CI workflows remain tooling/app-composition concerns.
- **Dependencies continue to point inward.**
  - no core/runtime crate should depend on generated bindings, packaging
    scripts, or CI concerns.
  - product-native naming changes must not pull release/loader concerns into
    domain logic.
- **Executable boundary contracts stay explicit.**
  - artifact manifests, generated bindings, and host-facing docs remain treated
    as machine/consumer contracts with version-match rules and compatibility
    notes.
- **File decomposition reviews stay active.**
  - if renamed packaging or compatibility work causes `runtime.rs`,
    packaging scripts, or workflow files to grow materially, perform the
    decomposition review required by `CODING-STANDARDS.md`.
- **Release naming stays standards-compliant.**
  - platform-specific native artifact names must follow
    `RELEASE-STANDARDS.md` and `CROSS-PLATFORM-STANDARDS.md`.
- **Composition-root ownership remains explicit.**
  - runtime wiring, library loading, and compatibility aliasing must be handled
    in entrypoint/tooling layers rather than spread across feature modules.

### Concurrency / Lifecycle Review

- No new long-lived background runtime is required for the refactor itself.
- CI/package scripts own build output creation under `target/`; they must clean
  and recreate only their package-specific staging directories.
- Any compatibility aliasing for native library names must document which
  artifact starts the load process, which filename the host loader resolves, and
  how stale mixed-version binding/native pairs are detected or rejected.

### Public Facade Preservation Note

Facade-first preservation is required.

The Pantograph headless API contract should remain stable while the native
artifact identity and release packaging are refactored underneath it. If a
breaking rename cannot be avoided at the generated host-binding level, the plan
must include a migration window, explicit compatibility notes, and updated CI
coverage for both old and new naming during the transition.

## Clarifying Questions (Only If Needed)

- None at plan creation time.
- Reason: the desired target packaging model is clear enough to sequence work.
- Revisit trigger: the UniFFI compatibility spike shows that the proposed
  native-library/product naming cannot be achieved without a broader API break
  than currently assumed.

## Definition of Done

- Pantograph publishes one product-facing headless native library artifact per
  platform with Pantograph-centric naming.
- Generated language binding artifacts are packaged separately and do not bundle
  the native Pantograph library by default.
- Documentation explains that the native Pantograph library is the shared
  foreign-language payload and the generated language layers are optional extras.
- CI builds the native library artifact and binding artifact separately and
  verifies at least one packaged language binding against the packaged native
  library.
- The repo contains a documented migration story for current
  `pantograph_uniffi`-named consumers.
- A standards-change proposal exists that codifies this product-native plus
  optional binding-package model for future binding work.
- The resulting implementation preserves the existing layer/package dependency
  direction and does not move Pantograph domain logic into the binding/package
  tooling layers.

## Milestones

### Milestone 1: Confirm Naming and Compatibility Strategy

**Goal:** Prove the technically correct boundary between product-native naming,
UniFFI internals, and generated host bindings before changing packaging.

**Tasks:**
- [ ] Inventory where `pantograph_uniffi` is used as:
  - Rust crate/package name
  - native library filename
  - UniFFI metadata crate identifier
  - generated namespace/module identifier
  - CI artifact identifier
- [ ] Spike whether the native library filename can be renamed independently of
  the generated host-binding namespace, or whether a compatibility alias/facade
  is required.
- [ ] Choose a product-facing native identity, for example
  `pantograph_headless`, and define:
  - native filenames by platform
  - package names by platform
  - compatibility strategy for legacy `pantograph_uniffi`
- [ ] Record whether the chosen rename can be confined to packaging/export
  identity or whether it would also force a Rust crate/package role change.
- [ ] Record migration notes for current consumers.

**Verification:**
- Targeted build/bindgen spike proving the chosen naming strategy actually
  generates and loads.
- Explicit architecture review confirming the chosen naming strategy preserves
  inward dependency direction and existing package roles.
- Update the plan execution notes with the chosen compatibility approach.

**Status:** Completed

**Implementation decision:** Changing the Rust package name was not required.
Changing the `crates/pantograph-uniffi` library target name to
`pantograph_headless` makes Cargo emit `libpantograph_headless.so` /
`pantograph_headless.dll` / `libpantograph_headless.dylib` and causes generated
C# to use `uniffi.pantograph_headless`. This is a deliberate foreign-language
artifact rename; Rust workspace consumers continue to use the existing Rust
crates directly.

### Milestone 2: Separate Product-Native Packaging From Binding Packaging

**Goal:** Make the native Pantograph library the primary platform artifact and
make language binding packages thin optional layers.

**Tasks:**
- [ ] Refactor the native build/export configuration so the shipped native
  library uses Pantograph-centric product naming.
- [ ] Update packaging scripts to emit:
  - `pantograph-headless-native-<platform>.zip`
  - `pantograph-csharp-bindings.zip`
- [ ] Remove the native library from the language binding package by default.
- [ ] Update manifest files so binding packages reference the required native
  package instead of embedding it.
- [ ] Ensure package docs describe version matching between generated bindings
  and the native Pantograph library.
- [ ] Keep packaging logic in scripts/CI layers and avoid moving packaging
  concerns into the domain/runtime crates unless required for export metadata.

**Verification:**
- Package script runs locally and produces separate native/binding artifacts.
- Artifact listing checks confirm expected contents and absence of the native
  library in the binding package.
- Review changed files for decomposition thresholds if any script/runtime file
  crosses the coding-standard review triggers.

**Status:** Completed

### Milestone 3: Preserve Headless Contract and Consumer Guidance

**Goal:** Keep the Pantograph headless API stable while rewriting the consumer
mental model and examples around the new artifact split.

**Tasks:**
- [ ] Update binding docs to distinguish:
  - Rust crate API
  - native Pantograph headless library
  - generated host-language layers
- [ ] Update packaged quickstart examples so they assume the native Pantograph
  library is supplied separately and document runtime dependency expectations.
- [ ] Add migration notes for old artifact names and old download assumptions.
- [ ] Update README/module docs so UniFFI is described as an implementation
  detail, not the product identity.
- [ ] Update any affected directory README files when package/layout boundaries
  change or new non-obvious directories are introduced.

**Verification:**
- Documentation review against `DOCUMENTATION-STANDARDS.md`.
- Packaged quickstart compile check still passes offline.
- Packaged example/native library runtime path is validated for the
  non-model/non-bundled authoring path.

**Status:** Completed

### Milestone 4: Update CI and Release Flow

**Goal:** Make CI publish and verify the correct product-native plus optional
  binding-layer artifact model.

**Tasks:**
- [ ] Convert CI artifact naming and upload steps to the new native/binding
  package names.
- [ ] Keep or add offline packaged-binding compile verification.
- [ ] Add packaged native-library loading verification against the packaged C#
  quickstart.
- [ ] If supported platform coverage is desired, expand CI to a platform matrix
  for native package generation according to `CROSS-PLATFORM-STANDARDS.md`.
- [ ] Generate release notes/checksums using the product-facing artifact names.
- [ ] Ensure any compatibility/deprecation path follows
  `RELEASE-STANDARDS.md` migration and deprecation guidance.

**Verification:**
- CI workflow passes with new artifact names.
- Uploaded artifacts are separately downloadable and match documented package
  layouts.
- Artifact names and library filenames are checked against release/cross-platform
  naming standards before rollout.

**Status:** Completed

**Implementation decision:** CI continues to build Linux artifacts in the
existing workflow and now uploads `pantograph-csharp-bindings.zip`,
`pantograph-headless-native-linux-x64.zip`, and `checksums-sha256.txt`. A
multi-platform matrix remains optional future release hardening.

### Milestone 5: Standardize the Pattern

**Goal:** Capture the architecture in the standards so future binding work does
not repeat the same ambiguity.

**Tasks:**
- [ ] Draft an amendment to `LANGUAGE-BINDINGS-STANDARDS.md` that explicitly
  separates:
  - product-native library artifacts
  - internal FFI wrapper crates
  - generated host-language binding packages
- [ ] Add rules for product-facing naming so framework names such as UniFFI or
  Rustler do not become shipped artifact identities unless the product itself is
  the framework.
- [ ] Add release packaging guidance requiring one native product library per
  platform with optional separate binding packages.
- [ ] Add compatibility guidance for matching generated host bindings to the
  native library from the same build/release.

**Verification:**
- Proposed amendment reviewed against current `LANGUAGE-BINDINGS-STANDARDS.md`
  and `RELEASE-STANDARDS.md`.
- Proposal text is concrete enough to be inserted into the standard with
  minimal rewriting.

**Status:** Completed

## Execution Notes

Update during implementation:
- 2026-04-09: Plan created after confirming the current repo packages a
  generated C# binding zip and a second zip containing the same docs/example
  plus the `libpantograph_uniffi.so` native library. User clarified the desired
  architecture is one Pantograph native library plus optional separate language
  binding layers.
- 2026-04-09: Compatibility spike confirmed that the Cargo library target name
  controls the generated native library name and generated C# namespace. Chosen
  transition strategy is a deliberate pre-1.0 foreign-language artifact rename
  from `pantograph_uniffi` to `pantograph_headless`, documented in migration
  notes.
- 2026-04-09: Packaging now emits separate C# binding and Pantograph headless
  native library zip artifacts. The C# zip has no `.so`, `.dll`, or `.dylib`.
  The native zip contains one platform native library plus docs/examples.
- 2026-04-09: CI now uploads the separate binding/native artifacts plus
  `checksums-sha256.txt`, and the packaged quickstart check compiles and runs
  against the packaged native library.

## Commit Cadence Notes

- Commit after each verified logical slice:
  - naming/compatibility spike
  - packaging refactor
  - docs/migration update
  - CI/release flow update
  - standards amendment/proposal
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Optional Subagent Assignment

Use only if needed.

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| None planned | N/A | N/A | N/A |

## Re-Plan Triggers

- UniFFI cannot support the desired native-library/product naming without a
  larger public binding break than assumed.
- The chosen native Pantograph library name conflicts with platform loader or
  existing packaging constraints.
- Compatibility support for old `pantograph_uniffi` consumers expands beyond a
  short migration window.
- CI artifact size/platform constraints require a different rollout sequence.

## Recommendations (Only If Better Option Exists)

- Recommendation 1: prefer a product-native library identity such as
  `pantograph_headless` while keeping the Rust package/crate name unchanged if
  that reduces migration cost.
  Why it is better: it fixes consumer-facing naming without forcing an
  unnecessary Rust workspace churn.
  Impact: moderate; depends on the Milestone 1 compatibility spike.

- Recommendation 2: treat "convenience bundles" that contain both a language
  binding and the native Pantograph library as optional secondary artifacts, not
  the primary release model.
  Why it is better: it preserves the single-native-library mental model while
  still supporting one-click consumer downloads when needed.
  Impact: low to moderate; can be added after the main refactor if demand
  exists.

- Recommendation 3: update the standards to distinguish product-native library
  naming from internal wrapper-crate naming.
  Why it is better: the current standard explains layers well but does not
  explicitly prevent framework/tool names from becoming the shipped product
  identity.
  Impact: low; documentation-only.

## Proposed Standards Change

Add a new section to `LANGUAGE-BINDINGS-STANDARDS.md` named
`Product-Native Artifact Model` with text along these lines:

### Proposed Addition

```markdown
## Product-Native Artifact Model

When a library is shipped to foreign-language consumers, distinguish three
separate identities:

1. Product-native shared library
2. Internal FFI wrapper crate/tooling
3. Generated host-language binding package

### Rules

1. Ship one product-native shared library per platform target.
2. Generated host-language bindings must be packaged separately by default.
3. Do not name shipped artifacts after the binding framework (`uniffi`,
   `rustler`, etc.) unless the product itself is the framework.
4. Binding packages must document which native product library they require.
5. Binding packages and native libraries must be version-matched from the same
   build or release.
6. If convenience bundles include both native library and generated bindings,
   document them as optional secondary artifacts, not the primary architecture.

### Example Layout

release/
|-- pantograph-headless-native-linux-x64.zip
|-- pantograph-headless-native-win-x64.zip
|-- pantograph-csharp-bindings.zip
|-- pantograph-python-bindings.zip
`-- checksums-sha256.txt

### Rationale

This keeps the product identity tied to the product, avoids duplicating the
native library across language packages, and makes it clear that generated host
bindings are optional surfaces over a shared native implementation.
```

Why this change is needed:

- the current standard explains the wrapper crate layer but does not clearly
  separate internal wrapper naming from shipped product/native artifact naming
- it does not explicitly say whether binding packages should or should not
  bundle the native product library
- it does not give downstream consumers a strong rule for "download the native
  product once, then add optional language layers"

## Completion Summary

### Completed

- Plan created.
- Standards amendment proposal drafted and applied to
  `LANGUAGE-BINDINGS-STANDARDS.md`.
- Pantograph headless native library renamed to the product-facing
  `pantograph_headless` identity.
- C# docs, examples, smoke checks, packaging scripts, and CI artifact names
  updated to the separated native-library plus optional binding-package model.
- Migration notes added for pre-refactor `pantograph_uniffi` consumers.

### Deviations

- No temporary legacy native-library alias was added. The current C# binding
  artifacts are pre-1.0 and the cleaner product-facing rename was simpler and
  less ambiguous than shipping duplicate loader names.
- CI remains Linux-only for this contract workflow; broader platform matrix
  packaging is left as optional release hardening.

### Follow-Ups

- Consider adding release workflow matrix jobs for macOS and Windows native
  package generation when those release targets are ready.

### Verification Summary

- Reviewed `PLAN-STANDARDS.md`
- Reviewed `templates/PLAN-TEMPLATE.md`
- Reviewed `LANGUAGE-BINDINGS-STANDARDS.md`
- Checked current Pantograph binding/package/CI state before drafting this
  plan
- `cargo test -p pantograph-uniffi`
- `cargo test -p pantograph-uniffi --no-default-features`
- `cargo test -p pantograph-uniffi --features frontend-http`
- `./scripts/check-uniffi-embedded-runtime-surface.sh`
- `./scripts/check-uniffi-csharp-smoke.sh`
- `PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh`
- `./scripts/check-packaged-csharp-quickstart.sh`

### Traceability Links

- Module README updated: N/A
- ADR added/updated: N/A at plan creation time
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A at plan
  creation time

## Brevity Note

Keep the plan concise. Expand detail only where execution decisions or risk
require it.
