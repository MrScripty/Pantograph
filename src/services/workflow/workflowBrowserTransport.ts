/** Explicit loopback browser host for the real workflow service (never a mock). */
export function resolveWorkflowBrowserHost(search: string, nativeHost = false): string | null {
  if (nativeHost) return null;
  const configured = new URLSearchParams(search).get('workflowHost');
  if (!configured) return null;
  const url = new URL(configured);
  if (
    url.protocol !== 'http:' ||
    !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname) ||
    url.username || url.password || url.search || url.hash || url.pathname !== '/'
  ) {
    throw new Error('workflowHost must be an HTTP loopback origin without credentials or a path');
  }
  return url.origin;
}

export function getWorkflowBrowserHost(): string | null {
  if (typeof window === 'undefined' || '__TAURI_INTERNALS__' in window) return null;
  return resolveWorkflowBrowserHost(window.location?.search ?? '');
}

export async function invokeBrowserWorkflowCommand<T>(
  host: string,
  command: string,
  args: Record<string, unknown> = {},
  request: typeof fetch = fetch,
): Promise<T> {
  const response = await request(`${host}/api/workflow/commands/${encodeURIComponent(command)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(args),
    signal: AbortSignal.timeout(30_000),
  });
  const body: unknown = await response.json();
  if (!response.ok) {
    // Preserve the backend's structured diagnostics link for the existing error UI.
    if (body && typeof body === 'object' && 'code' in body && 'message' in body) throw body;
    throw new Error(`Workflow host request failed: HTTP ${response.status}`);
  }
  return body as T;
}
