# src/components/server-status

## Purpose
This directory contains focused subcomponents that make up the `ServerStatus`
Settings panel. It exists so external connection controls, runtime snapshot
inspection, and health-monitor presentation do not continue to accumulate inside
one large shell component.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `ExternalConnectionPanel.svelte` | External-server connection form for URL and optional API key entry. |
| `HealthStatusPanel.svelte` | Health-monitor status disclosure with on-demand check and recovery actions. |
| `RuntimeSnapshotGrid.svelte` | Read-only runtime lifecycle summary cards for active and embedding runtimes. |

## Problem
`ServerStatus.svelte` had grown past the preferred size for a UI component while
mixing unrelated concerns: external connection form state, sidecar settings
shell, runtime lifecycle snapshots, and health-monitor disclosure. That made the
file harder to review while the runtime-manager work still needed to add more UI
under the same Settings section.

## Constraints
- `ServerStatus.svelte` remains the mounted Settings shell entry point.
- These subcomponents must stay presentation-focused and receive orchestration
  callbacks from their parent rather than creating new service owners.
- External connection and health recovery flows must keep semantic buttons and
  standard input controls.

## Decision
Split the reusable presentation blocks out of `ServerStatus.svelte` and leave
the parent component responsible for service subscriptions and high-level mode
switching. This reduces the size of the mounted shell component without moving
transport or orchestration ownership into child components.

## Alternatives Rejected
- Keep all server-status concerns in `ServerStatus.svelte`.
  Rejected because the file had already grown beyond the preferred component
  size and would have become worse once the runtime-manager surface was mounted.
- Move service subscriptions into each child component.
  Rejected because that would duplicate state ownership and scatter lifecycle
  control across multiple UI files.

## Invariants
- Parent components retain ownership of service subscriptions and mutation
  callbacks.
- Child components render semantic form and button controls.
- Runtime lifecycle cards remain read-only summaries; they do not mutate backend
  runtime state directly.

## Revisit Triggers
- The server-status shell gains another major concern that warrants a second
  decomposition pass.
- A reusable package-level server-status surface is introduced outside the
  Pantograph app shell.

## Dependencies
**Internal:** `src/components/ServerStatus.svelte`, `src/services/ConfigService`,
`src/services/HealthMonitorService`.
**External:** Svelte 5.

## Related ADRs
- None identified as of 2026-04-19.
- Reason: this directory is a local UI decomposition of an existing Settings
  shell rather than a new cross-cutting architecture boundary.
- Revisit trigger: the server-status shell becomes shared across hosts or
  requires backend-facing contract changes.

## Usage Examples
```svelte
<RuntimeSnapshotGrid
  activeRuntime={llmState.status.active_runtime}
  activeModelTarget={llmState.status.active_model_target}
  embeddingRuntime={llmState.status.embedding_runtime}
  embeddingModelTarget={llmState.status.embedding_model_target}
  fallbackActiveRuntimeId={llmState.status.backend_name}
/>
```

## API Consumer Contract
- `ExternalConnectionPanel.svelte` expects parent-owned bindable `externalUrl`
  and `apiKey` values plus `onConnect`/`onDisconnect` callbacks.
- `HealthStatusPanel.svelte` expects a parent-owned `HealthMonitorState` and
  callbacks for check-now and recovery actions.
- `RuntimeSnapshotGrid.svelte` renders read-only runtime lifecycle snapshots and
  tolerates `null` values for unavailable runtimes.

## Structured Producer Contract
- None identified as of 2026-04-19.
- Reason: these components render Settings UI state and do not publish machine-
  consumed structured artifacts.
- Revisit trigger: the directory begins generating saved diagnostics or
  exported runtime metadata.
