import type {
  LibraryUsageProjectionRecord,
  ProjectionStateRecord,
} from '../../services/diagnostics/types';

export type LibraryAssetCategory =
  | 'model'
  | 'runtime'
  | 'workflow'
  | 'node'
  | 'template'
  | 'connector'
  | 'pumas'
  | 'pantograph'
  | 'unclassified';

export function classifyLibraryAsset(assetId: string): LibraryAssetCategory {
  const normalized = assetId.toLowerCase();
  const [prefix] = normalized.split(/[:/]/, 1);
  switch (prefix) {
    case 'model':
    case 'models':
      return 'model';
    case 'runtime':
    case 'runtimes':
      return 'runtime';
    case 'workflow':
    case 'workflows':
      return 'workflow';
    case 'node':
    case 'nodes':
      return 'node';
    case 'template':
    case 'templates':
      return 'template';
    case 'connector':
    case 'connectors':
      return 'connector';
    case 'pumas':
    case 'puma':
      return 'pumas';
    case 'pantograph':
      return 'pantograph';
    default:
      return 'unclassified';
  }
}

export function formatLibraryAssetCategory(assetId: string): string {
  switch (classifyLibraryAsset(assetId)) {
    case 'model':
      return 'Model';
    case 'runtime':
      return 'Runtime';
    case 'workflow':
      return 'Workflow';
    case 'node':
      return 'Node';
    case 'template':
      return 'Template';
    case 'connector':
      return 'Connector';
    case 'pumas':
      return 'Pumas';
    case 'pantograph':
      return 'Pantograph';
    case 'unclassified':
      return 'Unclassified';
  }
}

export function isLibraryAssetLastUsedByRun(
  asset: Pick<LibraryUsageProjectionRecord, 'last_workflow_run_id'>,
  workflowRunId: string | null | undefined,
): boolean {
  return Boolean(workflowRunId && asset.last_workflow_run_id === workflowRunId);
}

export function formatLibraryBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) {
    return `${(bytes / 1_073_741_824).toFixed(1)} GiB`;
  }
  if (bytes >= 1_048_576) {
    return `${(bytes / 1_048_576).toFixed(1)} MiB`;
  }
  if (bytes >= 1_024) {
    return `${(bytes / 1_024).toFixed(1)} KiB`;
  }
  return `${bytes} B`;
}

export function formatLibraryProjectionFreshness(state: ProjectionStateRecord | null): string {
  if (!state) {
    return 'Projection unavailable';
  }
  const cursor = `seq ${state.last_applied_event_seq}`;
  switch (state.status) {
    case 'current':
      return `Current at ${cursor}`;
    case 'rebuilding':
      return `Rebuilding at ${cursor}`;
    case 'needs_rebuild':
      return `Needs rebuild at ${cursor}`;
    case 'failed':
      return `Failed at ${cursor}`;
  }
}
