# Anti-Pattern Remediation Tracker

Last updated: 2026-02-27 (Phase 2 complete)

## Objective

Track remediation of repo anti-pattern findings with phased, testable changes.

## Phase Status

| Phase | Title | Status | Owner | Exit Criteria |
|---|---|---|---|---|
| 0 | Tracker + baseline | Complete | Codex | Tracker added and scoped issues mapped |
| 1 | Runtime/process correctness | Complete | Codex | No orphan process on timeout; llama lifecycle non-blocking and cross-platform |
| 2 | Svelte DOM manipulation cleanup | Complete | Codex | `svelte/no-dom-manipulating` resolved without regressing generated-component HMR/state |
| 3 | Quality gate realignment | Pending | Codex | `check` blocks critical anti-patterns in app/package code |
| 4 | Store/service efficiency + retention | Pending | Codex | Link sync no longer global 100ms polling; logger bounded |
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
