# Pass 02: Architecture and Boundary Findings

Audit date: 2026-04-21

## Scope
This pass reviewed layering, ownership, contract boundaries, and duplicated
state machines across Rust services, Tauri adapters, Svelte packages, and
host-language binding crates.

## Standards Applied
- `CODING-STANDARDS.md`: backend-owned data, service independence, composition root, single owner for stateful flows.
- `ARCHITECTURE-PATTERNS.md`: package roles, backend-owned data, executable contracts, composition root, realtime workflow systems, view model pattern.
- `LANGUAGE-BINDINGS-STANDARDS.md`: core/wrapper/generated binding layering and curated binding surface policy.
- `INTEROP-STANDARDS.md`: cross-language contract maintenance and serialization alignment.

## Findings

### P02-F01: Workflow Execution Identity Still Has Frontend Ownership
Severity: High

Evidence:
- `src/services/workflow/WorkflowService.ts` tracks `currentExecutionId` and
  `currentRunExecutionId`, claims execution IDs from events, and publishes only
  locally filtered state to listeners.
- `src/stores/diagnosticsStore.ts` calls
  `claimWorkflowExecutionIdFromEvent` and `isWorkflowEventRelevantToExecution`
  before applying diagnostics snapshots.
- The existing handoff plan explicitly says recent diagnostics/workflow slices
  should be superseded by Rust-owned contracts.

Standards conflict:
- Workflow execution identity, stale-event policy, and session/run attribution
  are backend-owned workflow semantics, not presentation state.

Required direction:
- Move run/session identity, event relevance, and stale/replay filtering into
  `pantograph-workflow-service` trace or session projection contracts.
- Leave the frontend with selected tab, selected run/node, panel state, and other
  UI-only state.

### P02-F02: Graph Grouping Mutations Are Still Locally Applied in Svelte Stores
Severity: High

Evidence:
- `packages/svelte-graph/src/stores/createWorkflowStores.ts` routes ordinary
  node/edge mutations through backend mutation APIs, but `createGroup`,
  `ungroupNodes`, and `updateGroupPorts` still mutate local nodes/edges/groups
  directly after backend calls.

Standards conflict:
- Group creation changes canonical workflow graph shape. The frontend should
  apply a backend-returned graph mutation response rather than reconstruct graph
  state locally.

Required direction:
- Add backend-owned group mutation responses matching
  `WorkflowGraphMutationResponse`.
- Treat group state as a projection of backend graph/session state.

### P02-F03: Tauri Main Is an Overgrown Composition Root
Severity: Medium

Evidence:
- `src-tauri/src/main.rs` wires long-lived services, app data paths, RAG, runtime
  gateway, executor extensions, workflow session cleanup, invoke handlers, and
  shutdown behavior in one 476-line entrypoint.

Standards fit:
- A composition root belongs near the entrypoint, but the current file mixes
  registration lists, lifecycle startup, ad hoc error handling, and shutdown
  orchestration.

Required direction:
- Keep `main.rs` as the composition facade.
- Extract focused helpers: `compose_workflow_state`, `compose_runtime_state`,
  `initialize_extensions`, `register_tauri_commands`, and `shutdown_runtime_services`.

### P02-F04: Duplicate Workflow Adapter Surfaces Create Drift Risk
Severity: Medium

Evidence:
- `src/services/workflow/WorkflowService.ts` and
  `src/backends/TauriWorkflowBackend.ts` both map frontend calls to Tauri invoke
  commands.
- `src/lib/tauriConnectionIntentWire.ts` normalizes both camelCase and snake_case
  payload shapes between app and package types.
- `src/lib/workflowGraphMutationResponse.ts` and
  `packages/svelte-graph/src/stores/workflowGraphMutationResponse.ts` represent
  similar parsing concerns in separate layers.

Impact:
- Contract normalization logic can diverge between app and package surfaces.
- The package role boundary is blurred between reusable graph UI and Pantograph
  app adapter concerns.

Required direction:
- Define one executable workflow frontend contract module for Tauri wire DTOs.
- Have both app-specific services and reusable package adapters consume the same
  normalizers or generated schemas.

### P02-F05: Host Binding Facades Remain Too Large
Severity: Medium

Evidence:
- `crates/pantograph-uniffi/src/lib.rs` is 1,674 lines.
- `crates/pantograph-rustler/src/lib.rs` is 2,340 lines.
- Their READMEs acknowledge decomposition progress, but both main files remain
  large binding facades with many exported operations.

Standards fit:
- Binding crates can be wrapper-heavy, but wrapper exports should stay thin and
  curated. Large facades make it hard to distinguish supported, experimental,
  and internal-only surfaces.

Required direction:
- Preserve exported names but move implementation families into modules by
  product contract area: runtime object, graph CRUD, sessions, diagnostics,
  Pumas/model catalog, and frontend HTTP compatibility.
- Add support-tier documentation for each exported family.

### P02-F06: Placeholder Tool Execution Violates Runtime Contract Expectations
Severity: High

Evidence:
- `crates/workflow-nodes/src/control/tool_executor.rs` returns successful
  placeholder tool results that say execution requires external implementation.
- `crates/workflow-nodes/src/control/tool_loop.rs` injects a fake tool message:
  `Tool execution not implemented in this task. Please provide a final response.`

Standards conflict:
- `CODING-STANDARDS.md` says unimplemented stubs must not accept requests and
  return dummy data.

Required direction:
- Either remove these nodes from executable registration until real execution is
  available, or mark them disabled/experimental with explicit errors and a
  documented re-enable contract.

## Additional Issues Outside Pure Standards Compliance
- Current architecture still carries old Tauri workflow types and validators
  that are unused after service extraction. This is not only standards debt; it
  is a future bug risk because developers may modify the wrong implementation.
- `USE_MOCKS = false` in `WorkflowService.ts` leaves a compile-time mock branch
  inside the production service. It should either move to test fixtures or be
  selected through an explicit composition root.

## Pass 02 Remediation Themes
1. Make `pantograph-workflow-service` the only owner of workflow run/session identity and graph mutation semantics.
2. Collapse duplicate frontend adapter normalization into one contract boundary.
3. Preserve public facades while extracting large adapter implementation modules.
4. Convert placeholder executable nodes into explicit disabled or real execution paths.
