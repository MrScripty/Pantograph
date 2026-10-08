import type { WorkflowStores } from '@pantograph/svelte-graph';

export type JsonFilterMutationResult = Awaited<ReturnType<WorkflowStores['updateNodeData']>>;

/** Display saved data without coercing an invalid backend-owned value. */
export function jsonFilterPathState(path: unknown): { text: string; error: string | null } {
  if (path === undefined) return { text: '', error: null };
  if (typeof path === 'string') return { text: path, error: null };
  return { text: '', error: 'The saved path must be a string. Apply a replacement to repair it.' };
}

export function jsonFilterApplyError(result: JsonFilterMutationResult): string | null {
  switch (result.status) {
    case 'applied': return null;
    case 'stale': return 'The graph session changed. Reopen the node and apply the path again.';
    case 'skipped': return 'No graph edit session is active. Open the workflow and try again.';
    case 'failed': return result.error === undefined
      ? 'The path could not be applied. Your draft is retained.' : String(result.error);
  }
}
