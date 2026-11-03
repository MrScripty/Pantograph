import { getWorkflowBrowserHost } from './workflowBrowserTransport.ts';
import { invokeWorkflowCommand } from './workflowServiceErrors.ts';

export interface WorkflowBrowserCapabilities {
  model_library_read: boolean;
  model_library_write: boolean;
  model_search: boolean;
  graph_edit: boolean;
  description: string;
}

export async function loadWorkflowBrowserCapabilities(): Promise<WorkflowBrowserCapabilities | null> {
  if (!getWorkflowBrowserHost()) return null;
  return invokeWorkflowCommand<WorkflowBrowserCapabilities>('workflow_browser_host_capabilities');
}

export function browserLibraryActionAllowed(
  browserHost: boolean,
  capabilities: WorkflowBrowserCapabilities | null,
  action: 'search' | 'write',
): boolean {
  if (!browserHost) return true;
  return (action === 'search' ? capabilities?.model_search : capabilities?.model_library_write) === true;
}
