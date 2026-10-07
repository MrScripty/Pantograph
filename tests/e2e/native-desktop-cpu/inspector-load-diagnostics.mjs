// These functions execute in the actual webview; keep them self-contained.
export function installInspectorLoadDiagnostics() {
  // Tauri's invoke property is read-only. Observe JavaScript failures without
  // replacing the transport or changing request/response behavior.
  const failures = [];
  window.__nativeInspectorLoadFailures = failures;
  const record = (value) => failures.push({ capturedAt: new Date().toISOString(), ...value });
  window.addEventListener('error', (event) => record({ phase: 'javascript_error',
    error: event.message, filename: event.filename, line: event.lineno, column: event.colno }));
  window.addEventListener('unhandledrejection', (event) => record({ phase: 'unhandled_rejection',
    error: String(event.reason) }));
  return true;
}

export function readInspectorLoadDiagnostics() {
  const page = document.querySelector('[data-testid="io-inspector-page"]');
  const text = page?.textContent ?? '';
  return {
    javascriptFailures: window.__nativeInspectorLoadFailures ?? [],
    selectedRunHeader: page?.querySelector('h1 + div')?.textContent?.trim(),
    loadingSnapshot: text.includes('Loading run snapshot'),
    projectionUnavailable: text.includes('Projection unavailable'),
    capturedGraphPresent: Boolean(page?.querySelector('svg[aria-label="Captured workflow graph"]')),
    artifactCardCount: page?.querySelectorAll('[data-testid="io-artifact-card"]').length ?? 0,
    previewCount: page?.querySelectorAll('[data-testid="io-artifact-card"] pre').length ?? 0,
  };
}
