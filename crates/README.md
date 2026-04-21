# crates

Rust workspace members for Pantograph backend contracts, runtime integration,
workflow execution, and host-language bindings.

## Purpose
This directory groups Rust crates by architectural role so backend-owned
workflow, runtime, inference, and binding contracts can be built and tested
without depending on frontend source layout. The boundary exists to make Cargo
workspace ownership explicit and to keep reusable Rust logic out of the Tauri
app crate unless the app is the actual composition root.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `inference/` | Multi-backend inference and managed-runtime infrastructure used by embedded runtime and app hosts. |
| `node-engine/` | Core graph execution, task descriptors, validation, and workflow graph primitives. |
| `pantograph-workflow-service/` | Host-agnostic workflow application service, scheduler, graph-edit, trace, and diagnostics contracts. |
| `pantograph-embedded-runtime/` | Pantograph runtime composition layer binding workflow service to inference, Pumas, Python, RAG, and runtime registry helpers. |
| `pantograph-runtime-registry/` | Backend-owned runtime residency, reservation, admission, reclaim, warmup, and technical-fit state machine. |
| `pantograph-runtime-identity/` | Shared runtime id, backend key, display label, and alias normalization helpers. |
| `pantograph-frontend-http-adapter/` | Optional HTTP transport adapter implementing workflow host contracts for frontend-modular surfaces. |
| `pantograph-uniffi/` | UniFFI wrapper crate and bindgen entrypoint for generated host-language bindings. |
| `pantograph-rustler/` | Rustler NIF wrapper crate for BEAM/Elixir integration. |
| `workflow-nodes/` | Built-in workflow node descriptors and task implementations registered into `node-engine`. |

## Problem
Pantograph has multiple Rust consumers: the Tauri desktop app, native embedding
surfaces, binding-generation workflows, and backend test harnesses. Without an
explicit workspace map, shared contracts can drift into app crates, adapters can
grow policy, and binding wrappers can accidentally become sources of canonical
workflow behavior.

## Constraints
- Core/domain crates must not depend on app, transport, or binding crates.
- Binding wrappers must stay thin and delegate behavior to backend-owned Rust
  contracts.
- Cargo dependency ownership must match the crate that executes or exposes the
  behavior.
- Feature flags are public contracts for reusable crates and must remain
  documented as the workspace hardening milestone lands.
- Runtime policy belongs in backend/runtime crates, not in Tauri or generated
  host bindings.

## Decision
Keep Rust backend code in dedicated workspace members with role-oriented crate
boundaries. App and binding crates compose these crates, while reusable backend
crates own contracts and state machines. This preserves facade-first refactors:
large files can be split inside a crate without changing the crate role or
forcing consumers to import implementation modules directly.

## Alternatives Rejected
- Put all Rust code under `src-tauri/`: rejected because it would make the
  desktop app the owner of reusable backend and binding contracts.
- Move binding behavior into core crates: rejected because UniFFI/Rustler
  runtime details would leak into domain and workflow-service code.
- Keep package roles implicit in `Cargo.toml` only: rejected because standards
  compliance and review need human-readable ownership and dependency rules.

## Invariants
- `node-engine`, `pantograph-workflow-service`, and runtime-registry crates own
  backend contracts before adapters project them outward.
- `pantograph-uniffi` and `pantograph-rustler` expose curated binding surfaces
  and must not own canonical workflow semantics.
- `src-tauri` composes the desktop app but should not become the reusable Rust
  runtime layer.
- Workspace-level dependency and lint policy should be adopted without hiding
  crate-local ownership.

## Revisit Triggers
- A crate starts importing an app, binding, or transport implementation that
  should be outside its role.
- A reusable crate gains features that are only needed by one leaf app or one
  binding wrapper.
- Workspace lint or feature-contract work identifies a crate whose current role
  is too broad to document truthfully.
- Public native artifacts are renamed or split for release packaging.

## Dependencies
**Internal:** root `Cargo.toml`, `src-tauri/`, `bindings/`, workflow docs, and
the source READMEs inside each member crate.

**External:** Cargo workspaces, Rust toolchain, UniFFI, Rustler, Tauri, Pumas
Library, and backend runtime dependencies declared by member manifests.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
Run a focused crate check from the workspace root:

```bash
cargo check -p pantograph-workflow-service
```

Use a crate by its package name rather than by reaching into another crate's
implementation directory:

```rust
use pantograph_workflow_service::workflow::WorkflowRunRequest;
```

## API Consumer Contract
- Inputs: consumers depend on each crate's public Rust API and documented
  Cargo features, not on private module paths.
- Outputs: crates expose Rust types, generated binding artifacts, or adapter
  implementations according to their role.
- Lifecycle: app and binding crates create runtime resources; reusable service
  crates expose contracts and lifecycle hooks but do not own desktop startup.
- Errors: public errors should be typed in reusable crates and flattened only
  at binding or transport boundaries.
- Versioning: workspace crates currently ship together as Pantograph-internal
  packages; publishable status and version policy are part of the Rust
  workspace hardening milestone.

## Structured Producer Contract
- Stable fields: `Cargo.toml` membership, package names, public feature names,
  and generated artifact names are machine-consumed by Cargo, CI, and binding
  scripts.
- Defaults: root `default-members` defines which crates participate in ordinary
  workspace verification.
- Enums and labels: Cargo feature names are public labels and must not be
  renamed without compatibility review.
- Ordering: workspace member order is not a runtime contract.
- Compatibility: generated binding outputs and native library names must be
  version-matched when packaged for consumers.
- Regeneration/migration: changes to crate roles, feature names, or binding
  artifact names must update Cargo metadata, release scripts, binding docs,
  and this README in the same implementation slice.

## Testing
Baseline workspace checks are being made canonical by the standards compliance
plan. Current focused examples:

```bash
cargo check -p node-engine
cargo test -p pantograph-workflow-service
```

## Notes
- Per-crate README files are added in follow-up M1 slices.
- Rust-specific lint, metadata, feature-contract, and release hardening work is
  tracked under M5 of the standards compliance plan.
