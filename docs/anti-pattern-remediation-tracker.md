# Anti-Pattern Remediation Tracker

Last updated: 2026-02-27 (Phase 4 complete)

## Objective

Track remediation of repo anti-pattern findings with phased, testable changes.

## Phase Status

| Phase | Title | Status | Owner | Exit Criteria |
|---|---|---|---|---|
| 0 | Tracker + baseline | Complete | Codex | Tracker added and scoped issues mapped |
| 1 | Runtime/process correctness | Complete | Codex | No orphan process on timeout; llama lifecycle non-blocking and cross-platform |
| 2 | Svelte DOM manipulation cleanup | Complete | Codex | `svelte/no-dom-manipulating` resolved without regressing generated-component HMR/state |
| 3 | Quality gate realignment | Complete | Codex | `check` blocks critical anti-patterns in app/package code |
| 4 | Store/service efficiency + retention | Complete | Codex | Link sync no longer global 100ms polling; logger bounded |
| 5 | Deferred process-node hardening | Backlog | Codex | Capability gating + policy controls for untrusted workflows |

## Finding-to-Phase Mapping

| Finding | Phase |
|---|---|
| Process timeout leaves child alive | 1 |
| Llama server kill path blocks and uses Unix shell `kill` | 1 |
| Direct DOM manipulation in Svelte nodes | 2 |
| `check` omits full lint coverage | 3 |
| High lint debt on critical rules | 3 |
| Link store global 100ms polling | 4 |
| Logger unbounded in-memory retention | 4 |
| Arbitrary process execution policy gap | 5 |

## Phase 1 Plan

### Scope

- `crates/workflow-nodes/src/system/process.rs`
- `crates/inference/src/server.rs`
- `crates/inference/src/process.rs` (if trait support is needed)
- `src-tauri/src/llm/process_tauri.rs`

### Work Items

1. Ensure process timeout path kills and reaps child process.
2. Remove Unix shell `kill` calls from llama-server lifecycle paths.
3. Remove blocking sleeps from runtime shutdown paths.
4. Make Tauri process termination implementation cross-platform.
5. Add/adjust tests to cover timeout cleanup behavior.

### Validation

- `cargo test -p workflow-nodes process -- --nocapture`
- `cargo check -p inference`
- `cargo check -p pantograph`

### Phase 1 Completion Notes

- `ProcessTask` timeout path now kills and reaps the child process and keeps partial output handling.
- Added regression test ensuring timed-out process does not continue and perform delayed side effects.
- Removed shell `kill` usage and blocking sleeps from `LlamaServer` shutdown/cleanup path.
- Implemented cross-platform `kill()` in Tauri process handle using `CommandChild::kill`.

## Notes on Standards Follow-up

Potential standards improvement identified during Phase 1 work:

- Add an explicit rule to **forbid shelling out to OS process-management commands** (for example `kill`, `taskkill`) from core/runtime code when a native API or abstraction exists.
- Add an explicit rule to **forbid blocking sleeps in async runtime lifecycle paths** (shutdown/startup loops should use async timers or non-blocking checks).

## Phase 2 Plan

### Scope

- `src/components/nodes/workflow/PointCloudOutputNode.svelte`
- `src/components/nodes/workflow/MaskedTextInputNode.svelte`

### Work Items

1. Replace direct DOM writes (`innerHTML`, `appendChild`) with declarative Svelte/element bindings.
2. Preserve node-level interaction behavior (prevent canvas/editor gestures from triggering graph drag/pan).
3. Keep GUI-generator-driven updates compatible with current state flow (no forced full re-mount behavior changes).
4. Add proper runtime cleanup for dynamic rendering resources.

### Validation

- `npm run typecheck`
- `npx eslint src/components/nodes/workflow/PointCloudOutputNode.svelte src/components/nodes/workflow/MaskedTextInputNode.svelte --rule "svelte/no-dom-manipulating:error"`

### Phase 2 Completion Notes

- `PointCloudOutputNode` now binds Three.js rendering directly to a Svelte-managed `<canvas>` instead of mutating container DOM.
- Added explicit lifecycle cleanup for animation frame, resize listener, and Three.js resources.
- `MaskedTextInputNode` no longer uses contenteditable DOM mutation paths; it now uses a textarea + segment model transform helpers.
- Selection-based anchor/mask actions are preserved via textarea selection APIs.

Potential standards improvement identified during Phase 2 work:

- Add a frontend standard requiring reactive/declarative updates over direct DOM mutation in component code, with an explicit exception process for generator/HMR edge cases (documented rationale + scoped suppression).

## Phase 3 Plan

### Scope

- `package.json`
- `scripts/check-critical-antipatterns.mjs`
- `README.md`

### Work Items

1. Add a cross-platform critical anti-pattern gate targeting `src/` and `packages/`.
2. Wire the new gate into the primary `npm run check` path.
3. Keep full lint available (`lint:full`) while avoiding blockage from unrelated baseline lint debt.

### Validation

- `npm run lint:critical`
- `npm run check`

### Phase 3 Completion Notes

- Added `scripts/check-critical-antipatterns.mjs` as a cross-platform Node quality gate.
- Added `npm run lint:critical` and integrated it into `npm run check`.
- Gate now blocks high-risk anti-patterns in app/package code without requiring full-lint debt burn-down first.

Potential standards improvement identified during Phase 3 work:

- Add a standards requirement for **tiered quality gates**: keep `check` blocking critical anti-patterns repo-wide, while allowing broader style/strictness debt to be burned down incrementally via a separate full-lint target.

## Phase 4 Plan

### Scope

- `src/stores/linkStore.ts`
- `src/components/TopBar.svelte`
- `src/components/side-panel/FollowUpInput.svelte`
- `src/services/Logger.ts`

### Work Items

1. Remove global 100ms link-value polling and replace with event-driven updates.
2. Push value change notifications from linkable inputs to keep `LinkedInputNode` previews current.
3. Bound in-memory logger retention to prevent unbounded growth.

### Validation

- `npm run typecheck`
- `npm run check`

### Phase 4 Completion Notes

- Removed interval-based link synchronization from `linkStore` and replaced it with targeted update helpers driven by element notifications/subscriptions.
- Updated `TopBar` and `FollowUpInput` to emit direct value-change notifications so link mappings stay fresh without background polling.
- `Logger` now uses a bounded ring buffer with retention stats (`maxEvents`, `retainedEvents`, `droppedEvents`) to cap memory usage.

Potential standards improvement identified during Phase 4 work:

- Add an explicit frontend/runtime rule to **forbid global high-frequency polling loops for UI state synchronization** when event-driven hooks are feasible, and require explicit retention limits for in-memory observability buffers.
