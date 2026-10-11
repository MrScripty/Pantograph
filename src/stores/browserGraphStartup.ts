export function graphSessionStorageKey(browserHost: string | null): string {
  return browserHost ? `pantograph.lastGraph:${browserHost}` : 'pantograph.lastGraph';
}

/** Restore only workflows advertised by this host; never invent an empty session. */
export function selectBrowserStartupWorkflow(
  workflows: Array<{ id?: string; name: string }>,
  last: { id: string; type: string } | null,
): string | null {
  const ids = workflows.map(workflow => workflow.id ?? workflow.name);
  return last?.type === 'workflow' && ids.includes(last.id) ? last.id : ids[0] ?? null;
}
