# src/services/agent

Frontend agent service boundary.

## Purpose
This directory owns frontend services for agent request orchestration, activity
logging, stream handling, and agent type contracts.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Agent service exports. |
| `types.ts` | Agent service TypeScript contracts. |
| `AgentService.ts` | Agent command/service orchestration. |
| `ActivityLogger.ts` | Activity log state and formatting helpers. |
| `StreamHandler.ts` | Stream event handling for agent responses. |

## Problem
Agent UI code needs a service layer that isolates command invocation, streaming
updates, and activity logging from components.

## Constraints
- Backend/Tauri agent commands own execution behavior.
- Stream handling must preserve event order.
- Activity records should remain structured for UI display.

## Decision
Keep agent frontend orchestration in this service directory and expose stable
types through `index.ts`.

## Alternatives Rejected
- Call Tauri agent commands directly from leaf components: rejected because
  stream and activity behavior would duplicate.
- Put activity logging in backend only: rejected because frontend UI needs
  session-local presentation state.

## Invariants
- Agent service preserves backend error categories where possible.
- Stream events are applied in order.
- Components consume service state/callbacks, not raw command wiring.

## Revisit Triggers
- Agent protocol becomes generated.
- Streaming moves to a shared event bus.
- Agent tools become workflow graph runtime nodes.

## Dependencies
**Internal:** agent feature exports, side-panel components, Tauri command
bindings, and activity UI.

**External:** Tauri JavaScript APIs and TypeScript.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```ts
import { AgentService } from './AgentService';
```

## API Consumer Contract
- Inputs: user prompts, command options, stream events, and backend responses.
- Outputs: agent responses, activity records, and stream state.
- Lifecycle: services live for app/session UI lifetimes and should clean up
  stream subscriptions.
- Errors: command and stream failures should remain distinguishable for UI.
- Versioning: service methods/types must migrate with components and commands.

## Structured Producer Contract
- Stable fields: activity record keys, stream event keys, and agent DTO fields
  are machine-consumed by UI components.
- Defaults: service defaults should match backend command defaults or document
  overrides.
- Enums and labels: activity types and stream status labels carry behavior.
- Ordering: activity and stream events preserve source order.
- Compatibility: side panel and agent views depend on service contract shapes.
- Regeneration/migration: update services, components, backend commands, and
  tests with DTO changes.

## Testing
```bash
npm run typecheck
```

## Notes
- Keep agent execution policy in backend/Tauri owners.
