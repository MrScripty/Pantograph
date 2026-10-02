# Artifact Retention Policy Reopen Evidence

Date: 2026-10-02. Scope: P-05, retained policy across normal host restart.
Parent authority: [active domain plan](../plan.md), continued under the current
implementation instruction. This report is evidence, not a separate plan.

## Admission And Contract

- Branch: `codex/artifact-retention-reopen`; initial PR target:
  `fix/decision-traceability-ownership-2026-10-02`, then `main` after the
  prerequisite qualifies; exact initial base:
  `4938e405c7f656365eefdca492774ccae110c90d` (remote HEAD rechecked before editing).
  The artifact-retention implementer owns this isolated proposal; the domain
  integrator owns sequencing, review and eventual integration. No merge is
  authorized by this change.
- Standards: Coding-Standards `dcc56f26e884ade260770beceba2501d3746200d`, remote
  HEAD rechecked. Selected Core/Router, Implementation, Verification, Commit,
  Documentation, Library, Persistence, Rust/Rust API, Contracts/Evolution,
  Architecture and Code Design. Parent planning/coordination remains with the
  domain integrator. No legacy navigation file supplies policy.
- The manifest remains the authoritative saved policy. Desktop and UniFFI
  defaults initialize a missing store; they must not replace a saved policy,
  including policy identity/version and optional limits.
- `ArtifactStore::open` retains its explicit host-override contract. The new
  `open_with_default_policy` is the ordinary host startup entry point. Settings
  continue to save through `WorkflowService::update_artifact_policy`.
- No persisted schema or binding shape changes. Existing readable manifests
  remain supported. Invalid/unreadable manifests fail through the existing
  typed store errors; they are never reset to defaults.
- This slice owns normal reopen semantics, not interruption-safe publication,
  multi-process writer arbitration, TTL scheduling, or bounded-range I/O.

## Change And Proof Boundaries

The store owns the distinction between supplied policy override and creation
fallback; hosts select one documented operation without inspecting manifest
paths or parsing JSON. Desktop's private artifact startup helper owns its
existing defaults and invokes that operation. It is called by real app startup
and directly by its service-level restart regression. No global setting mirror,
new crate, dependency, or generic startup framework is introduced.

Write set:

- `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`
- `crates/pantograph-workflow-service/tests/artifact_store_policy.rs`
- `crates/pantograph-workflow-service/tests/desktop_artifact_startup.rs`
- `src-tauri/src/app_setup.rs`
- `src-tauri/src/app_setup/artifacts.rs`
- `crates/pantograph-uniffi/src/runtime.rs`
- `crates/pantograph-uniffi/src/runtime_tests.rs`
- this report

The public store tests cover creation defaults, repeated reopen with changed
fallback defaults, the preserved explicit override contract, and rejection of
invalid persisted state without replacement. Desktop regression traverses the
real startup store helper, service policy save, owner drop, disk reload and
service consumers. It verifies exact saved policy, full body reads, disk and
single-artifact limits, cache/spill limits, delete-on-consume, and TTL cleanup
without sleeps. A small workflow-service integration-test wrapper includes that
exact private desktop module, allowing the unpublished coordinated workspace's
existing headless service CI to discover the tests without a Tauri build or a
copied implementation. The claim is desktop startup helper → service → persisted
reopen, not Tauri application startup. The existing UniFFI artifact test checks
saved policy after runtime shutdown/drop and constructor reopen through its
JSON API.

These are focused/integration/persisted-contract claims on a representative
filesystem. They do not claim a launched GUI session, crash durability, actual
model execution or qualified desktop user-workflow acceptance.

## Verification

Status: the focused policy/startup-helper/UniFFI restart claims pass and
independent source review found no blocker. Composition onto reviewed gate
`e22ebb2fe52efc8cfcaac6b23c4a902c7bc6a571` passes staged traceability;
affected-package Clippy remains failing
with the same diagnosed observations as the admitted base;
full-package and desktop qualification are not claimed green.

Environment: Linux x86_64, rustc/cargo 1.92.0, rustfmt 1.8.0. The initial ORT
native download failed before tests ran. The shared setup owner then verified
Microsoft's official ONNX Runtime 1.24.2 release archive against its release
asset SHA256 (`43725474ba5663642e17684717946693850e2005efbd724ac72da278fead25e6`).
Tests use ort-sys's supported `ORT_LIB_LOCATION`, `ORT_PREFER_DYNAMIC_LINK=1`
and matching `LD_LIBRARY_PATH`. No Cargo dependency, feature declaration or
lockfile changed.

Results:

- Affected-file `rustfmt --check --edition 2021 --config skip_children=true`
  and `git diff --check`: pass
- `cargo test -p pantograph-workflow-service --test artifact_store_policy --test desktop_artifact_startup --locked`:
  5 policy/store tests and 2 actual desktop-helper/service/reopen tests pass
- `cargo test -p pantograph-uniffi --no-default-features --features embedded-runtime direct_runtime_exposes_artifact_store_contract_surface --locked`:
  the existing API test, extended through shutdown/drop/constructor reopen,
  passes; 20 unrelated tests filtered out
- `cargo test -p pantograph-workflow-service --no-fail-fast --locked`:
  unit tests 852 pass / 20 fail; integration contract tests 26 pass / 3 fail;
  all artifact contract/settings/store/policy tests and the new startup helper
  tests pass. The same command against a separate clean checkout of the exact
  base reproduces all 23 failing test identities. Failures concern graph
  validation, session capacity, task classification, technical fit and contract
  fixtures. Of their diagnostics, 21 match after thread IDs are removed, one
  differs only by a generated workflow-run UUID, and the graph test
  `refresh_current_validation_summary_rejects_revision_changed_during_fact_lookup`
  reports `Missing` at base versus `Invalid` in the candidate where both expect
  `Current`. That graph test creates only a GraphSessionStore, not an artifact
  store; this slice does not claim to diagnose or close its differing states
- Original staged traceability failed the retired README-per-directory rule.
  The composed `TRACEABILITY_STAGED_ONLY=1 ./scripts/check-decision-traceability.sh`
  now passes on the reviewed replacement, without boilerplate or a hook bypass
- Affected-package Clippy:
  `cargo clippy -p pantograph-workflow-service --locked --offline --no-deps --lib --tests --message-format=json -- -D warnings`
  fails for both this candidate and the separate exact-base checkout. Comparing
  target kind, diagnostic code/level/message, primary file and source text gives
  the same 284 diagnostic observations, with no additions or removals. These
  include 134 library and 149 library-test errors plus one dependency warning.
  This discriminator does not make the warning-deny gate green; compilation
  stops before the separate integration-test Clippy targets are reached
- Desktop-package compile/execution: locally unavailable because GTK/WebKit/
  libsoup development packages are absent. Existing native `rust-check` CI
  installs these dependencies and checks the workspace; the headless workflow
  contract CI already runs all workflow-service tests. The source-inclusion
  test does not replace full desktop compilation or GUI evidence

Independent review covered saved-policy authority and optional-field semantics,
explicit override compatibility, all production store-open callers, cache and
retention consumers, real service/drop/reopen tests, the UniFFI JSON path and
headless wrapper placement. Test checks remain scoped to the boundaries above.

## Separately Dispositioned Findings

P-06 interruption-safe body/manifest publication and P-07 bounded-range reads
remain separate follow-on slices. This patch neither changes their mechanisms
nor claims to close them. Inspector full-image reads remain unchanged.

## Composed Publication Evidence

The branch fast-forwarded onto gate [PR #2](https://github.com/MrScripty/Pantograph/pull/2)
at `e22ebb2fe52efc8cfcaac6b23c4a902c7bc6a571`, including its independently reviewed
PR-range correction. The entire staged implementation patch stayed byte-for-byte
unchanged. Fresh staged traceability, whitespace and formatting checks pass.
The previously qualified unchanged store/startup and UniFFI test binaries reran
5 policy tests, 2 real desktop-helper/service reopen tests and the UniFFI reopen
case successfully. These are existing-binary reruns, not a fresh Cargo compile
of the composed tree. Original full-package and Clippy comparisons retain their
stated scope; they remain failing. Exact-head hosted CI and applicable hosted
review must qualify both this draft and its gate prerequisite before merging.
