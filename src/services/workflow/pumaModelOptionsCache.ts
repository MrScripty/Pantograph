import { invoke } from '@tauri-apps/api/core';
import type { PortOption, PortOptionsResult } from './types';

export interface ModelLibraryUpdateEvent {
  cursor: string;
  model_id: string;
  change_kind: string;
  fact_family: string;
  refresh_scope: string;
  selected_artifact_id?: string | null;
  producer_revision?: string | null;
}

export interface ModelLibraryUpdateFeed {
  cursor: string;
  events: ModelLibraryUpdateEvent[];
  stale_cursor: boolean;
  snapshot_required: boolean;
}

export type WorkflowInvoker = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export interface LoadPumasModelOptionsOptions {
  forceRefresh?: boolean;
}

interface PumasModelOptionsSnapshot {
  options: PortOption[];
  cursor: string | null;
}

let cachedSnapshot: PumasModelOptionsSnapshot | null = null;
let inflightSnapshot: Promise<PortOption[]> | null = null;

export function extractPumasSelectorCursor(options: PortOption[]): string | null {
  for (const option of options) {
    const cursor = option.metadata?.package_facts_summary_cursor;
    if (typeof cursor === 'string' && cursor.trim().length > 0) {
      return cursor;
    }
  }
  return null;
}

export function extractPumasSelectorCursorFromResult(response: PortOptionsResult): string | null {
  const resultCursor = response.metadata?.package_facts_summary_cursor;
  if (typeof resultCursor === 'string' && resultCursor.trim().length > 0) {
    return resultCursor;
  }
  return extractPumasSelectorCursor(response.options);
}

export function selectorUpdateFeedRequiresRefresh(feed: ModelLibraryUpdateFeed): boolean {
  return feed.stale_cursor || feed.snapshot_required || feed.events.length > 0;
}

export function invalidatePumasModelOptionsCache(): void {
  cachedSnapshot = null;
  inflightSnapshot = null;
}

export async function loadPumasModelOptions(
  invokerOrOptions: WorkflowInvoker | LoadPumasModelOptionsOptions = invoke,
  maybeOptions: LoadPumasModelOptionsOptions = {},
): Promise<PortOption[]> {
  const { invoker, options } = normalizeLoadPumasModelOptionsArgs(invokerOrOptions, maybeOptions);

  if (options.forceRefresh) {
    cachedSnapshot = null;
    inflightSnapshot = null;
  }

  if (cachedSnapshot) {
    if (!cachedSnapshot.cursor || !(await snapshotNeedsRefresh(invoker, cachedSnapshot.cursor))) {
      return cachedSnapshot.options;
    }
    cachedSnapshot = null;
  }

  if (inflightSnapshot) {
    return inflightSnapshot;
  }

  inflightSnapshot = loadFreshPumasModelOptions(invoker).finally(() => {
    inflightSnapshot = null;
  });
  return inflightSnapshot;
}

function normalizeLoadPumasModelOptionsArgs(
  invokerOrOptions: WorkflowInvoker | LoadPumasModelOptionsOptions,
  maybeOptions: LoadPumasModelOptionsOptions,
): { invoker: WorkflowInvoker; options: LoadPumasModelOptionsOptions } {
  if (typeof invokerOrOptions === 'function') {
    return { invoker: invokerOrOptions, options: maybeOptions };
  }
  return { invoker: invoke, options: invokerOrOptions };
}

async function loadFreshPumasModelOptions(invoker: WorkflowInvoker): Promise<PortOption[]> {
  let snapshot = await fetchPumasModelOptionsSnapshot(invoker);

  if (snapshot.cursor && (await snapshotNeedsRefresh(invoker, snapshot.cursor))) {
    snapshot = await fetchPumasModelOptionsSnapshot(invoker);
  }

  cachedSnapshot = snapshot;
  return snapshot.options;
}

async function fetchPumasModelOptionsSnapshot(
  invoker: WorkflowInvoker,
): Promise<PumasModelOptionsSnapshot> {
  const response = await invoker<PortOptionsResult>('query_port_options', {
    nodeType: 'puma-lib',
    portId: 'model_path',
  });
  return {
    options: response.options,
    cursor: extractPumasSelectorCursorFromResult(response),
  };
}

async function snapshotNeedsRefresh(invoker: WorkflowInvoker, cursor: string): Promise<boolean> {
  try {
    const feed = await invoker<ModelLibraryUpdateFeed>('list_model_library_updates_since', {
      cursor,
      limit: 100,
    });
    return selectorUpdateFeedRequiresRefresh(feed);
  } catch {
    return false;
  }
}
