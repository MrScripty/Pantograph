# Stage-End Refactor Gate: 06 Binding Projections And Verification

## Stage

`06-binding-projections-and-verification`

## Plan File

`docs/plans/pantograph-execution-platform/06-binding-projections-and-verification.md`

## Touched-File Source

```bash
git diff --name-only HEAD~1..HEAD
git status --short
```

The committed Stage `06` implementation set came from `HEAD~1..HEAD`. The only
remaining uncommitted files before this report were unrelated asset deletions
and untracked asset files, which are outside the stage boundary and were not
reviewed as Stage `06` implementation files.

## Touched Files Reviewed

- `bindings/beam/README.md`
- `bindings/beam/pantograph_native_smoke/lib/pantograph/native.ex`
- `bindings/beam/pantograph_native_smoke/test/pantograph_native_smoke_test.exs`
- `bindings/csharp/PACKAGE-README.md`
- `bindings/csharp/Pantograph.DirectRuntimeQuickstart/Program.cs`
- `bindings/csharp/Pantograph.NativeSmoke/Program.cs`
- `bindings/csharp/README.md`
- `crates/pantograph-node-contracts/src/README.md`
- `crates/pantograph-node-contracts/src/composition.rs`
- `crates/pantograph-node-contracts/src/lib.rs`
- `crates/pantograph-node-contracts/src/migration.rs`
- `crates/pantograph-node-contracts/src/tests.rs`
- `crates/pantograph-rustler/README.md`
- `crates/pantograph-rustler/src/lib.rs`
- `crates/pantograph-rustler/src/registry_nifs.rs`
- `crates/pantograph-uniffi/src/runtime.rs`
- `crates/pantograph-uniffi/src/runtime_tests.rs`
- `crates/pantograph-workflow-service/src/graph/README.md`
- `crates/pantograph-workflow-service/src/graph/canonicalization.rs`
- `crates/pantograph-workflow-service/src/graph/canonicalization_inference.rs`
- `crates/pantograph-workflow-service/src/graph/canonicalization_legacy_migration.rs`
- `crates/pantograph-workflow-service/src/graph/canonicalization_tests.rs`
- `docs/plans/pantograph-execution-platform/06-binding-projections-and-verification.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/06-binding-projections-and-verification/coordination-ledger.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/06-binding-projections-and-verification/reports/wave-02-worker-rustler-projections.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/06-binding-projections-and-verification/reports/wave-02-worker-uniffi-projections.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/06-binding-projections-and-verification/reports/wave-03-worker-beam-host-tests.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/06-binding-projections-and-verification/reports/wave-03-worker-csharp-host-tests.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/06-binding-projections-and-verification/reports/wave-03-worker-python-host-tests.md`
- `scripts/check-uniffi-embedded-runtime-surface.sh`

Wave `04` closeout additionally touched:

- `docs/adr/ADR-010-binding-projection-ownership-and-support-tiers.md`
- `docs/adr/README.md`
- `docs/headless-native-bindings.md`
- this report

## Applicable Standards Groups

- Planning and documentation
- Architecture and coding
- Rust API and async
- Testing and tooling
- Security and dependencies
- Interop and bindings
- Release and cross-platform

## Outcome

`not_warranted`

## Findings And Decisions

- Binding ownership remains thin. UniFFI and Rustler project backend-owned
  registry, queryable-port, workflow-service, and execution-session facts
  without wrapper-local node semantics.
- Generated host artifacts remain artifacts. The checked-in C# changes are
  smoke fixtures and documentation samples, not generated binding output.
- The Stage `05` refactor pressure exposed by Stage `06` preflight was handled
  before Stage `06` projection work: large composition and canonicalization
  modules were split into focused files and verified.
- The largest remaining touched Rust files are projection or facade modules:
  `crates/pantograph-node-contracts/src/lib.rs` and
  `crates/pantograph-uniffi/src/runtime.rs`. They are not currently duplicating
  domain semantics across layers. Further splitting is deferred until a future
  stage adds another distinct projection family.
- Support-tier claims now match verification evidence: C# has generated/native
  smoke coverage, Python is unsupported, and BEAM remains experimental on this
  host.
- No new dependencies, unsafe blocks, credential handling, listener exposure, or
  broad release manifest changes were introduced by Stage `06`.

## Files Changed By In-Scope Refactor

None. No additional in-scope refactor was warranted after the Stage `05`
module split and Stage `06` verification work.

## Verification

Passed:

```bash
cargo test -p pantograph-node-contracts
cargo test -p pantograph-workflow-service canonicalize_workflow_graph
cargo check -p pantograph-workflow-service
cargo test -p pantograph-uniffi
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
cargo check -p pantograph_rustler
cargo check --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

Rerun after Wave `04` packaged documentation updates:

```bash
PANTOGRAPH_PACKAGE_PROFILE=debug ./scripts/package-uniffi-csharp-artifacts.sh
./scripts/check-packaged-csharp-quickstart.sh
cargo fmt --all -- --check
```

The packaging script emitted the existing CSharpier availability warning while
still producing artifacts.

Host-lane gaps:

```bash
cargo test -p pantograph_rustler
./scripts/check-rustler-beam-smoke.sh
```

`cargo test -p pantograph_rustler` still requires BEAM host-provided `enif_*`
symbols at link time. `./scripts/check-rustler-beam-smoke.sh` cannot run on
this host because `mix` is not installed.

## Residual Risks

- `crates/pantograph-uniffi/src/runtime.rs` remains a broad projection facade.
  If future stages add another unrelated family of host methods, split the
  implementation into focused projection helpers before the file accumulates
  new domain-specific branches.
- BEAM cannot be promoted beyond `experimental` until the smoke command runs in
  a host environment with `mix`.
- Python cannot be promoted beyond `unsupported` until a real generated/native
  artifact and import/load smoke path exist.
