# Focused Audit: Frontend And Accessibility

Implementation plan: [Frontend and accessibility remediation](../../plans/current-standards-remediation/frontend-and-accessibility/plan.md)

## Scope

This audit covers frontend authority, Tauri IPC decoding, async UI lifecycle,
package/application ownership, persisted browser state, interaction semantics,
accessibility, and browser evidence.

Applicable current standards are Frontend, Accessibility, Contracts,
Concurrency, TypeScript, TypeScript Async, IPC, and Verification.

## Assessment

The frontend has good recent decomposition and broad pure TypeScript coverage.
It is only partially migrated to the current contract model. The largest gaps
are treating typed IPC calls as runtime proof, leaking lifecycle-owned work,
and maintaining parallel application/package behavior.

## Findings

### FE-01 — High: IPC values are trusted without complete decoding

[workflowServiceErrors.ts](../../../src/services/workflow/workflowServiceErrors.ts#L145)
returns invoke<T> directly after normalizing only errors.
WorkflowCommandService, WorkflowProjectionService, and TauriWorkflowBackend
then treat commands, events, sessions, graphs, and persisted payloads as typed
values.

tauriConnectionIntentWire.ts fills missing required strings with empty values,
defaults revision state, and casts enum-like fields. The graph mutation parser
is better because it starts with unknown, but nested structures are still
partially cast.

TypeScript generics and assertions are not runtime boundary proof. One
canonical decoder/projection strategy should preserve invalid, unsupported,
and unavailable outcomes.

### FE-02 — High: global graph listeners leak on unmount

Both application and package WorkflowGraph components use onMount(async ...)
and return cleanup only after an await:

- src/components/WorkflowGraph.svelte near lines 307-324;
- packages/svelte-graph/src/components/WorkflowGraph.svelte near lines
  214-231.

Svelte registers lifecycle cleanup synchronously, so the remover for six
window listeners is discarded. Remounts can retain duplicate behavior.

DeviceConfig has a related destruction race, and createViewStores navigation
can apply stale completion from overlapping animations because it has no
generation, cancellation, or serialization owner.

### FE-03 — Medium: connection policy is duplicated and visually fails open

The application and package workflowConnections modules fall back from
backend-authored intent to local port inspection and return true when a
definition is missing. portTypeCompatibility hand-copies rules described as
aligned with Rust.

Backend mutation validation limits durable damage, but the UI can advertise an
invalid operation as valid. Missing authority must remain unavailable rather
than guessed success.

### FE-04 — Medium: package and application owners still compete

The application uses local NodePalette, WorkflowGraph, and WorkflowToolbar
components while @pantograph/svelte-graph exports parallel full components.
Remaining stores synchronize state into a named legacy service, one app module
deep-imports package source, and the hand-authored architecture graph still
names a removed execute_workflow command.

This supports the known mid-transition state. A focused retirement audit must
choose which component, state, type, and service owners remain.

### FE-05 — Medium: persisted browser state is incompletely decoded

Several stores apply direct or partial JSON.parse output to view, session,
graph, undo, or timeline state. Nested actions, identifiers, enum values, and
bounds are not consistently checked. promptHistoryStore is a stronger local
example because it validates every element and applies a size bound.

### FE-06 — Medium: accessibility and interaction evidence is incomplete

Current commands report:

- full ESLint failure in PumaLibNode.svelte;
- critical-pattern failure for direct DOM mutation in IoInspectorPage.svelte;
  and
- three custom accessibility findings.

Some custom findings reflect checker limitations rather than proven user
defects. The checker uses regexes, and one reported generic button already has
a keydown handler. Current standards allow imperative DOM when its ownership
and cleanup are justified; they do not impose a syntax-only ban.

No unit test mounts a Svelte component or exercises DOM, focus, keyboard,
mouse, or pointer behavior. The one Tauri/WebDriver path is a pointer-driven
image-generation happy path and is not in CI.

## Preserved Strengths

- Strict TypeScript checks pass.
- All 90 discovered pure frontend test files pass.
- Backend-owned graph mutation is established for many newer paths.
- Some newer mutation and error paths begin with unknown and decode before use.
- Workbench pages include generation tokens and subscription disposal patterns
  that can be reused.

## Follow-Up Audit Boundaries

1. Inventory every Tauri response/event and select its canonical runtime
   decoder and producer/consumer contract.
2. Inventory timers, onMount(async), subscriptions, observers, refresh loops,
   and navigation work for supersession and teardown.
3. Select the retained @pantograph/svelte-graph public surface and delete
   competing app/package owners and deep imports.
4. Replace syntax-only accessibility confidence with role, name, state, focus,
   keyboard, pointer, and cleanup claims in a representative browser.
5. Audit browser persistence as an untrusted versioned boundary.

Generated-component execution is covered separately in
[Security and dynamic code](01-security-and-dynamic-code.md).
