import type { WorkflowGraphMutationResponse } from '../types/workflow.js';

export type WorkflowGraphMutationResultStatus = 'applied' | 'failed' | 'skipped' | 'stale';

export interface WorkflowGraphMutationResult {
  action: string;
  error?: unknown;
  response?: WorkflowGraphMutationResponse;
  sessionId: string | null;
  status: WorkflowGraphMutationResultStatus;
}

export interface WorkflowMutationDispatch {
  getActiveSessionId: () => string | null;
  isActiveSession: (sessionId: string) => boolean;
  setActiveSessionId: (sessionId: string | null) => void;
  syncGraphMutationFromBackend: (
    action: string,
    mutate: (sessionId: string) => Promise<WorkflowGraphMutationResponse>,
  ) => Promise<WorkflowGraphMutationResult>;
}

export function createWorkflowMutationDispatch(params: {
  applyBackendMutationResponse: (
    sessionId: string,
    response: WorkflowGraphMutationResponse,
  ) => boolean;
}): WorkflowMutationDispatch {
  let activeSessionId: string | null = null;

  function setActiveSessionId(sessionId: string | null): void {
    activeSessionId = sessionId;
  }

  function getActiveSessionId(): string | null {
    return activeSessionId;
  }

  function isActiveSession(sessionId: string): boolean {
    return activeSessionId === sessionId;
  }

  async function syncGraphMutationFromBackend(
    action: string,
    mutate: (sessionId: string) => Promise<WorkflowGraphMutationResponse>,
  ): Promise<WorkflowGraphMutationResult> {
    if (!activeSessionId) {
      console.warn(`[workflowStores] Ignoring ${action} without an active session`);
      return { action, sessionId: null, status: 'skipped' };
    }

    const requestSessionId = activeSessionId;

    try {
      const response = await mutate(requestSessionId);
      if (!params.applyBackendMutationResponse(requestSessionId, response)) {
        return { action, response, sessionId: requestSessionId, status: 'stale' };
      }
      return { action, response, sessionId: requestSessionId, status: 'applied' };
    } catch (error) {
      if (!isActiveSession(requestSessionId)) {
        return { action, error, sessionId: requestSessionId, status: 'stale' };
      }

      console.error(`[workflowStores] Failed to ${action}:`, error);
      return { action, error, sessionId: requestSessionId, status: 'failed' };
    }
  }

  return {
    getActiveSessionId,
    isActiveSession,
    setActiveSessionId,
    syncGraphMutationFromBackend,
  };
}
