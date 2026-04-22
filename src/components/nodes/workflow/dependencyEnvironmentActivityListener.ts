import type { DependencyActivityEvent } from './dependencyEnvironmentTypes.ts';

export type DependencyEnvironmentActivityUnlisten = () => void;

export type DependencyEnvironmentActivityEventListener = <Payload>(
  eventName: string,
  handler: (event: { payload: Payload }) => void,
) => Promise<DependencyEnvironmentActivityUnlisten>;

export interface DependencyEnvironmentActivityListenerSetup {
  listenEvent: DependencyEnvironmentActivityEventListener;
  matchesActivityEvent: (payload: DependencyActivityEvent) => boolean;
  renderActivityEvent: (payload: DependencyActivityEvent) => string;
  appendActivityLine: (line: string) => void;
  persistNodeState: () => void;
  shouldRunModeAction: () => boolean;
  runModeAction: () => Promise<void>;
}

export function formatDependencyEnvironmentListenerError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return `listener: error="${message}"`;
}

export async function setupDependencyEnvironmentActivityListener({
  listenEvent,
  matchesActivityEvent,
  renderActivityEvent,
  appendActivityLine,
  persistNodeState,
  shouldRunModeAction,
  runModeAction,
}: DependencyEnvironmentActivityListenerSetup): Promise<DependencyEnvironmentActivityUnlisten> {
  const unlisten = await listenEvent<DependencyActivityEvent>('dependency-activity', (event) => {
    const payload = event.payload;
    if (!matchesActivityEvent(payload)) return;
    appendActivityLine(renderActivityEvent(payload));
  });

  persistNodeState();
  if (shouldRunModeAction()) {
    await runModeAction();
  }

  return unlisten;
}
