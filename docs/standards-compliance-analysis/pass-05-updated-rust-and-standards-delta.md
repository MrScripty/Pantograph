# Pass 05: Updated Standards Delta and Rust-Specific Findings

Audit date: 2026-04-21

## Scope
This pass reread the standards directory after updates were made and reconciled
the existing audit against the new and sharpened requirements. The largest
change is that Rust now has a dedicated language-specific standards set under
`languages/rust/`, which is authoritative for this repository's Rust workspace.

## Standards Applied
- `README.md`: language-specific standards are now part of the standards index.
- `CODING-STANDARDS.md`: decomposition review now also considers public
  function count and responsibility count.
- `ARCHITECTURE-PATTERNS.md`: executable boundary contracts and structured
  producer-consumer contracts are now more explicit.
- `DOCUMENTATION-STANDARDS.md`: README sections require concrete content or an
  explicit `None` statement with reason and revisit trigger.
- `TESTING-STANDARDS.md`: integration tests must isolate global and durable
  state, and binding tests must cover native and host-language sides.
- `DEPENDENCY-STANDARDS.md`: package-local dependency ownership and CI tool
  bootstrap are now explicit.
- `LAUNCHER-STANDARDS.md`: GUI release smoke requirements and generated
  desktop/script escaping requirements are now explicit.
- `languages/rust/RUST-*.md`: Rust API, async, dependency, interop, binding,
  security, cross-platform, unsafe, release, and tooling standards.

## Additional Verification Results
- `cargo check --workspace --all-features` passed, but emitted warnings.
- `cargo check --workspace --no-default-features` passed, but emitted warnings.
- `rg` found no repo-owned `unsafe { ... }` blocks or `unsafe fn` definitions
  in Rust source at the time of this pass.
- `rg` found no workspace lint policy in root `Cargo.toml`; member crates do
  not opt into `[lints] workspace = true`.
- `rg` found no `rust-version` metadata and no `publish = false` declarations
  in member crate manifests.

## Findings

### P05-F01: Rust Workspace Lint Policy Is Missing
Severity: High

Evidence:
- Root `Cargo.toml` does not define `[workspace.lints.rust]` or
  `[workspace.lints.clippy]`.
- Member `Cargo.toml` files do not opt into workspace lint policy.
- Current Rust checks pass only because warnings are non-blocking.

Standards conflict:
- `RUST-TOOLING-STANDARDS.md` requires shared workspace lint policy where
  possible, including default-deny unsafe policy and warning ratchets such as
  `unwrap_used`, `todo`, and missing docs.

Required direction:
- Add root workspace lint policy.
- Opt member crates into workspace lints.
- Start with a documented warning baseline if immediate `-D warnings` adoption
  is too disruptive, but make the ratchet explicit.

### P05-F02: Rust Crate Release Metadata and Publish Control Are Incomplete
Severity: High

Evidence:
- Root `[workspace.package]` contains `edition` and `license`, but not
  `version`, `rust-version`, or `repository`.
- Many crates have local `version = "0.1.0"` and either local `edition`/license
  or partial workspace inheritance.
- No member crate declares `rust-version`.
- No binary-only, wrapper, or internal crate declares `publish = false`.

Standards conflict:
- `RUST-RELEASE-STANDARDS.md` requires Rust toolchain pinning,
  `rust-version`, complete metadata for publishable/reusable crates, and
  explicit `publish = false` for crates that should never go to crates.io.

Required direction:
- Define workspace package metadata intentionally.
- Move shared crate metadata to workspace inheritance where appropriate.
- Set `publish = false` for app, binding-wrapper, internal tooling, and
  workspace-only crates unless they are deliberately publishable.
- Add `rust-toolchain.toml` after confirming the intended MSRV/toolchain.

### P05-F03: Required Rust Verification Is Not a Canonical Gate
Severity: High

Evidence:
- Local ad hoc checks show `cargo check --workspace --all-features` and
  `cargo check --workspace --no-default-features` pass with warnings.
- The existing plan already noted that `cargo check` emits many warnings.
- There is no visible general CI workflow that runs:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace`
  - `cargo test --workspace --doc`
  - `cargo check --workspace --all-features`
  - `cargo check --workspace --no-default-features`

Standards conflict:
- `RUST-TOOLING-STANDARDS.md` defines these as baseline Rust verification.
- `TOOLING-STANDARDS.md` requires warnings to become enforceable, not silently
  tolerated.

Required direction:
- Add these commands to the canonical local/CI verification plan.
- Decide whether the first landing uses hard failures or a documented ratchet
  for existing warnings.

### P05-F04: Rust Async Task Ownership Needs a Workspace-Wide Audit
Severity: High

Evidence:
- `tokio::spawn` or `tauri::async_runtime::spawn` occurs in:
  - `crates/workflow-nodes/src/system/process.rs`
  - `crates/inference/src/process.rs`
  - `crates/inference/src/backend/ollama.rs`
  - `crates/pantograph-embedded-runtime/src/model_dependencies.rs`
  - `crates/pantograph-embedded-runtime/src/lib.rs`
  - `crates/pantograph-embedded-runtime/src/runtime_registry.rs`
  - `crates/pantograph-workflow-service/src/workflow.rs`
  - `src-tauri/src/llm/process_tauri.rs`
  - `src-tauri/src/llm/health_monitor.rs`
  - `src-tauri/src/main.rs`
- Some spawns are test-only or appear to store handles; others need explicit
  lifecycle-owner review.

Standards conflict:
- `RUST-ASYNC-STANDARDS.md` requires every spawned task to have a tracked
  `JoinHandle`, `JoinSet`, or `TaskTracker`, plus cancellation and shutdown
  ownership.

Required direction:
- Classify every spawn as test-only, handle-owned, supervisor-owned, or
  non-compliant.
- Introduce a common lifecycle/supervisor pattern for product tasks.
- Add shutdown tests where tasks are part of product runtime behavior.

### P05-F05: Cargo Feature Contracts Are Public but Underdocumented
Severity: Medium

Evidence:
- Features exist in `crates/inference`, `crates/node-engine`,
  `crates/pantograph-embedded-runtime`, `crates/pantograph-uniffi`,
  `crates/pantograph-rustler`, `crates/workflow-nodes`, and `src-tauri`.
- Feature checks pass, but README/crate docs do not consistently document
  public feature semantics, feature costs, or support guarantees.

Standards conflict:
- `RUST-API-STANDARDS.md` treats Cargo features as public contracts for
  reusable crates.
- `RUST-DEPENDENCY-STANDARDS.md` requires heavy optional functionality to be
  behind explicit features and audited for cost.

Required direction:
- Document feature contracts in crate READMEs or crate-level docs.
- Keep default features minimal or justify heavier defaults.
- Add all-features/no-default-features checks to CI and release checklists.

### P05-F06: Binding Surface Support Tiers and Product Artifact Model Are Missing
Severity: High

Evidence:
- `pantograph-uniffi` and `pantograph-rustler` expose large binding surfaces.
- The previous audit already identified large binding facades and missing
  host/native acceptance coverage.
- `pantograph-uniffi` builds a library named `pantograph_headless`, while the
  standards now require product-native artifact naming and version matching to
  be explicit.

Standards conflict:
- `RUST-LANGUAGE-BINDINGS-STANDARDS.md` requires curated binding surfaces,
  support tiers, product-native artifact identity, native tests, host-language
  tests, and version-matched generated bindings.

Required direction:
- Classify exported binding APIs as `supported`, `experimental`, or
  `internal-only`.
- Document native artifact names, generated package names, and version matching.
- Add or document `version()` export behavior for host consumers.
- Ensure every wrapper conversion and error mapping has native tests, plus
  host-language smoke paths for supported bindings.

### P05-F07: Rust Platform `cfg` Placement Needs Thin-Module Review
Severity: Medium

Evidence:
- Platform `cfg` is concentrated in some platform modules such as
  `crates/inference/src/managed_runtime/llama_cpp_platform/mod.rs` and
  `ollama_platform/mod.rs`, which aligns with the standard.
- Inline platform `cfg` also appears in files such as
  `src-tauri/src/llm/port_manager.rs`,
  `src-tauri/src/llm/server_discovery.rs`,
  `crates/inference/src/managed_runtime/archive.rs`, and generated/binding
  runtime paths.

Standards conflict:
- `RUST-CROSS-PLATFORM-STANDARDS.md` allows inline `cfg` only under narrow
  limits; otherwise platform-specific behavior belongs behind platform modules
  or traits.

Required direction:
- Review each non-test inline platform `cfg` against the exception rule.
- Move larger platform logic into named platform modules or adapters.
- Add target checks for required platforms in CI.

### P05-F08: Unsafe Policy Is Not Enforced Despite No Current Unsafe Blocks
Severity: Medium

Evidence:
- `rg` found no repo-owned `unsafe` blocks or `unsafe fn` definitions.
- There is no workspace lint policy denying unsafe code by default.

Standards conflict:
- `RUST-UNSAFE-STANDARDS.md` requires safe Rust by default, with workspace
  `unsafe_code = "deny"` and documented exceptions for legitimate unsafe
  boundary crates.

Required direction:
- Add workspace-level unsafe denial.
- If future FFI or OS-boundary crates require unsafe, relax only that crate
  with an exception checklist and verification plan.

## Additional Issues Outside Pure Standards Compliance
- `cargo check --workspace --all-features` and `--no-default-features` now both
  pass, which is good, but they surface warning debt that would block the
  required clippy `-D warnings` gate.
- `pantograph-rustler` emits `non_local_definitions` warnings from
  `rustler::resource!`; this may require a Rustler update, lint exception, or
  wrapper restructuring.
- Cargo checks were run concurrently and one waited on Cargo's build lock; CI
  should sequence feature checks or use separate jobs with isolated caches to
  avoid misleading local timing.

## Pass 05 Remediation Themes
1. Add Rust workspace lints and explicit unsafe policy.
2. Normalize Rust metadata, publish control, and toolchain pinning.
3. Make required Rust verification canonical in CI and launcher/test docs.
4. Audit all spawned tasks under the Rust async lifecycle rules.
5. Curate and document public Cargo features and binding surfaces.
