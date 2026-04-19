# ADR-003: Runtime Redistributables Manager Boundary

## Status
Accepted

## Context
Pantograph needs installable runtime binaries such as `llama.cpp`, and those
runtime dependencies are now part of workflow safety rather than just desktop
convenience. A missing or partially installed redistributable can block runtime
startup, invalidate workflow preflight, or break restore/reuse flows after a
session has been checkpointed.

Pantograph already had fragments of this problem implemented in multiple
places:
- `crates/inference` owned low-level managed-binary detection, download, and
  command resolution helpers.
- `crates/pantograph-embedded-runtime` introduced Pantograph-specific
  runtime-manager view contracts for GUI and workflow-facing consumers.
- `src-tauri/src/llm/commands` exposed desktop commands for install/remove/
  inspect operations.
- Svelte GUI components rendered redistributable state and had started to
  accumulate their own refresh assumptions.

Without an explicit architecture decision, future work could drift in one of
three non-compliant directions:
- Tauri or Svelte layers rebuild runtime install/readiness truth locally.
- Workflow and scheduler code bypass the same managed-runtime contract that the
  GUI uses.
- Additional runtime families such as `Ollama` fork the `llama.cpp` path
  instead of reusing one backend-owned runtime-manager boundary.

## Decision
Adopt the following ownership boundary for Pantograph runtime redistributables:

1. Backend Rust owns redistributable truth.
- Runtime catalog, install state, selected/default/active version policy,
  retained artifacts, restart reconciliation, readiness validation, and command
  resolution remain backend-owned.
- Those facts live behind backend crates and exported managed-runtime
  contracts, not inside Tauri command handlers or Svelte components.

2. `crates/inference::managed_runtime` remains the infrastructure owner.
- It owns runtime definitions, platform install/finalization helpers, durable
  runtime state, and executable command resolution.
- Per-runtime and per-platform differences stay behind backend adapter modules
  rather than bleeding into host code.

3. `crates/pantograph-embedded-runtime` owns Pantograph-facing manager views.
- It projects backend runtime state into additive GUI/workflow contracts such
  as runtime snapshots, selection state, install history, and job progress.
- It does not replace backend runtime lifecycle ownership; it exposes that
  backend state in Pantograph-consumable form.

4. Workflow, scheduler, and diagnostics consume the same backend-owned facts.
- Preflight, execution, restore, reuse, and diagnostics must use the shared
  managed-runtime readiness/view contracts.
- No execution path may silently bypass redistributable readiness checks by
  resolving binaries directly from host-local assumptions.

5. Tauri remains adapter/composition only.
- Tauri commands may forward install/remove/select/inspect requests and return
  backend-owned payloads.
- Tauri may compose app services that subscribe to those payloads, but it must
  not become the owner of runtime install policy, version selection logic, or
  readiness rules.

6. GUI services project; they do not decide.
- Frontend services may cache backend-owned runtime snapshots and fan them out
  to multiple GUI views.
- Any frontend projection must stay additive and traceable back to backend
  payloads instead of inventing new lifecycle truth.

7. Reuse for future managed runtimes must be additive.
- New managed runtime families such as `Ollama` should plug into the same
  backend runtime-manager contracts, state model, diagnostics path, and
  transport conventions.
- Runtime-specific special cases belong in backend definitions/platform
  adapters, not in Tauri or workflow consumers.

## Consequences

### Positive
- Workflow safety, runtime install UX, and host transport now align on one
  backend-owned redistributable contract.
- GUI state synchronization can become event-driven without moving business
  logic out of Rust.
- Future managed runtimes have a defined reuse path instead of starting from a
  new binary-specific transport flow.
- Plans, READMEs, and reviews have a stable architecture reference for this
  boundary.

### Negative
- More runtime-manager state must be maintained and projected consistently
  across backend crates.
- Implementers need to keep Pantograph-facing view contracts additive while the
  underlying backend manager grows.
- Some existing host-layer refresh or probing behavior must be removed when it
  duplicates backend truth.

### Neutral
- This ADR freezes the ownership boundary, not every future runtime family or
  release-source policy decision.
- Desktop hosts still own app composition and user-triggered command transport,
  but that no longer implies policy ownership.
