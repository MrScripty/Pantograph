import type {
  DiagnosticsNodeStatus,
  DiagnosticsRunTrace,
  DiagnosticsRunStatus,
} from '../../services/diagnostics/types';
import type {
  GraphMemoryImpactSummary,
  NodeMemoryCompatibility,
  WorkflowRuntimeInstallState,
  WorkflowSessionQueueItemStatus,
  WorkflowSessionState,
} from '../../services/workflow/types';

const TIMESTAMP_FORMATTER = new Intl.DateTimeFormat(undefined, {
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
});

export function formatDiagnosticsDuration(durationMs: number | null): string {
  if (durationMs === null) {
    return 'In progress';
  }
  if (durationMs < 1_000) {
    return `${Math.round(durationMs)} ms`;
  }
  if (durationMs < 10_000) {
    return `${(durationMs / 1_000).toFixed(1)} s`;
  }
  if (durationMs < 60_000) {
    return `${Math.round(durationMs / 1_000)} s`;
  }

  const minutes = Math.floor(durationMs / 60_000);
  const seconds = Math.round((durationMs % 60_000) / 1_000);
  return `${minutes}m ${seconds}s`;
}

export function formatDiagnosticsTimestamp(timestampMs: number | null): string {
  if (timestampMs === null) {
    return 'Pending';
  }
  return TIMESTAMP_FORMATTER.format(timestampMs);
}

export function formatDiagnosticsPercent(progress: number | null): string {
  if (progress === null) {
    return 'No progress';
  }
  return `${Math.round(progress * 100)}%`;
}

export function formatDiagnosticsBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KiB`;
  }
  if (bytes < 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
  }
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
}

export function getDiagnosticsStatusClasses(
  status: DiagnosticsRunStatus | DiagnosticsNodeStatus,
): string {
  switch (status) {
    case 'running':
      return 'bg-cyan-950/80 text-cyan-200 border-cyan-800';
    case 'waiting':
      return 'bg-amber-950/80 text-amber-200 border-amber-800';
    case 'completed':
      return 'bg-emerald-950/80 text-emerald-200 border-emerald-800';
    case 'cancelled':
      return 'bg-neutral-900 text-neutral-300 border-neutral-700';
    case 'failed':
      return 'bg-red-950/80 text-red-200 border-red-800';
  }
}

export function getRunNodeStatusCounts(
  run: DiagnosticsRunTrace,
): Record<DiagnosticsNodeStatus, number> {
  const counts: Record<DiagnosticsNodeStatus, number> = {
    running: 0,
    waiting: 0,
    completed: 0,
    cancelled: 0,
    failed: 0,
  };

  Object.values(run.nodes).forEach((node) => {
    counts[node.status] += 1;
  });

  return counts;
}

export interface GraphMemoryImpactCounts {
  preserved: number;
  refreshed: number;
  dropped: number;
  fallback: number;
}

export function getGraphMemoryImpactCounts(
  impact: GraphMemoryImpactSummary | null,
): GraphMemoryImpactCounts {
  const counts: GraphMemoryImpactCounts = {
    preserved: 0,
    refreshed: 0,
    dropped: 0,
    fallback: 0,
  };

  if (!impact) {
    return counts;
  }

  for (const decision of impact.node_decisions ?? []) {
    switch (decision.compatibility) {
      case 'preserve_as_is':
        counts.preserved += 1;
        break;
      case 'preserve_with_input_refresh':
        counts.refreshed += 1;
        break;
      case 'drop_on_identity_change':
      case 'drop_on_schema_incompatibility':
        counts.dropped += 1;
        break;
      case 'fallback_full_invalidation':
        counts.fallback += 1;
        break;
    }
  }

  return counts;
}

export function formatNodeMemoryCompatibilityLabel(
  compatibility: NodeMemoryCompatibility,
): string {
  switch (compatibility) {
    case 'preserve_as_is':
      return 'Preserved';
    case 'preserve_with_input_refresh':
      return 'Refresh Inputs';
    case 'drop_on_identity_change':
      return 'Dropped Identity';
    case 'drop_on_schema_incompatibility':
      return 'Dropped Schema';
    case 'fallback_full_invalidation':
      return 'Fallback Invalidation';
  }
}

export function getSchedulerStateClasses(
  status: WorkflowSessionState | WorkflowSessionQueueItemStatus,
): string {
  switch (status) {
    case 'running':
      return 'bg-cyan-950/80 text-cyan-200 border-cyan-800';
    case 'pending':
      return 'bg-amber-950/80 text-amber-200 border-amber-800';
    case 'idle_loaded':
      return 'bg-emerald-950/80 text-emerald-200 border-emerald-800';
    case 'idle_unloaded':
      return 'bg-neutral-900 text-neutral-300 border-neutral-700';
  }
}

export function getRuntimeInstallStateClasses(
  state: WorkflowRuntimeInstallState,
  available: boolean,
): string {
  if (available) {
    return 'bg-emerald-950/80 text-emerald-200 border-emerald-800';
  }

  switch (state) {
    case 'missing':
      return 'bg-amber-950/80 text-amber-200 border-amber-800';
    case 'unsupported':
      return 'bg-red-950/80 text-red-200 border-red-800';
    case 'system_provided':
      return 'bg-cyan-950/80 text-cyan-200 border-cyan-800';
    case 'installed':
      return 'bg-neutral-900 text-neutral-300 border-neutral-700';
  }
}
