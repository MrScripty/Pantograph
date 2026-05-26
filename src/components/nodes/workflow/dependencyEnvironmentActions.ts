import type {
  DependencyEnvironmentActionResponse,
  DependencyEnvironmentNodeAction,
} from './dependencyEnvironmentTypes.ts';

export type DependencyEnvironmentActionInvoker = () => Promise<DependencyEnvironmentActionResponse>;

export interface DependencyEnvironmentActionRunnerInput {
  action: DependencyEnvironmentNodeAction;
  invokeAction: DependencyEnvironmentActionInvoker;
  appendActivityLine: (line: string) => void;
  setBusy: (busy: boolean) => void;
}

export function formatDependencyEnvironmentActionError(
  action: DependencyEnvironmentNodeAction,
  error: unknown,
): string {
  const message = error instanceof Error ? error.message : String(error);
  return `${action}: error="${message}"`;
}

export function formatDependencyEnvironmentActionResult(
  response: DependencyEnvironmentActionResponse,
): string {
  if (response.status === 'request_ready') {
    return `${response.action}: request ready`;
  }

  const diagnosticMessages = response.diagnostics
    ?.map((diagnostic) => diagnostic.message.trim())
    .filter((message) => message.length > 0);
  const message = diagnosticMessages?.[0] ?? 'blocked by backend validation';
  return `${response.action}: blocked="${message}"`;
}

export async function runDependencyEnvironmentActionRequest({
  action,
  invokeAction,
  appendActivityLine,
  setBusy,
}: DependencyEnvironmentActionRunnerInput): Promise<boolean> {
  setBusy(true);
  try {
    const response = await invokeAction();
    appendActivityLine(formatDependencyEnvironmentActionResult(response));
    return response.status === 'request_ready';
  } catch (error) {
    appendActivityLine(formatDependencyEnvironmentActionError(action, error));
    throw error;
  } finally {
    setBusy(false);
  }
}
