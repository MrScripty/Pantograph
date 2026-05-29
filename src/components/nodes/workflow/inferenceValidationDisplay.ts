import type {
  InferenceInterfaceDriftReport,
  InferenceInterfaceUpdateProposal,
  WorkflowGraphValidationSummary,
} from '../../../services/workflow/types.ts';

export type InferenceValidationTone = 'neutral' | 'info' | 'warning' | 'error' | 'success';

export interface InferenceValidationDisplay {
  label: string;
  detail: string | null;
  tone: InferenceValidationTone;
}

export interface InferenceUpdateApplyDisplay {
  detail: string | null;
  enabled: boolean;
  label: string;
}

export interface InferenceUpdatePreviewDisplay {
  extraCount: number;
  operationSummary: string | null;
  rows: string[];
  title: string;
}

const STATUS_LABELS: Record<WorkflowGraphValidationSummary['status'], string> = {
  pending: 'Pending validation',
  stale: 'Stale validation',
  unresolved: 'Interface unresolved',
  unavailable: 'Interface unavailable',
  blocked: 'Validation blocked',
  executable: 'Executable',
};

const STATUS_TONES: Record<WorkflowGraphValidationSummary['status'], InferenceValidationTone> = {
  pending: 'info',
  stale: 'warning',
  unresolved: 'warning',
  unavailable: 'error',
  blocked: 'error',
  executable: 'success',
};

export function buildInferenceValidationDisplay(
  summary: WorkflowGraphValidationSummary | null | undefined,
): InferenceValidationDisplay | null {
  if (!summary) {
    return null;
  }

  return {
    label: STATUS_LABELS[summary.status],
    detail: formatValidationDetail(summary),
    tone: STATUS_TONES[summary.status],
  };
}

export function buildInferenceDriftDisplay(
  driftReport: InferenceInterfaceDriftReport | null | undefined,
  updateProposal: InferenceInterfaceUpdateProposal | null | undefined,
): InferenceValidationDisplay | null {
  if (!driftReport) {
    return null;
  }

  const changeCount = driftReport.changes?.length ?? 0;
  const operationCount = updateProposal?.operations?.length ?? 0;
  return {
    label: driftReport.blocking ? 'Interface drift' : 'Interface changed',
    detail: operationCount > 0
      ? formatCount(operationCount, 'proposed update')
      : changeCount > 0
        ? formatCount(changeCount, 'interface change')
        : 'review required',
    tone: driftReport.blocking ? 'error' : 'warning',
  };
}

export function buildInferenceUpdateApplyDisplay(
  updateProposal: InferenceInterfaceUpdateProposal | null | undefined,
): InferenceUpdateApplyDisplay | null {
  if (!updateProposal) {
    return null;
  }

  if (updateProposal.destructive) {
    return {
      detail: 'Requires preview',
      enabled: false,
      label: 'Review',
    };
  }

  const operations = updateProposal.operations ?? [];
  if (
    operations.length === 1
    && operations[0]?.operation === 'replace_authored_snapshot'
  ) {
    return {
      detail: null,
      enabled: true,
      label: 'Apply',
    };
  }

  return {
    detail: 'Unsupported update',
    enabled: false,
    label: 'Review',
  };
}

export function buildInferenceUpdatePreviewDisplay(
  driftReport: InferenceInterfaceDriftReport | null | undefined,
  updateProposal: InferenceInterfaceUpdateProposal | null | undefined,
): InferenceUpdatePreviewDisplay | null {
  if (!driftReport && !updateProposal) {
    return null;
  }

  const changes = driftReport?.changes ?? updateProposal?.drift_report?.changes ?? [];
  const rows = changes.slice(0, 3).map((change) =>
    change.port_id ? `${change.port_id}: ${change.message}` : change.message,
  );
  const operationCount = updateProposal?.operations?.length ?? 0;

  return {
    extraCount: Math.max(changes.length - rows.length, 0),
    operationSummary:
      operationCount > 0 ? formatCount(operationCount, 'backend operation') : null,
    rows,
    title: driftReport?.blocking ? 'Review interface changes' : 'Interface update preview',
  };
}

function formatValidationDetail(summary: WorkflowGraphValidationSummary): string | null {
  if (summary.blocking_diagnostics_count > 0) {
    return formatCount(summary.blocking_diagnostics_count, 'blocking diagnostic');
  }

  if (summary.diagnostics_count > 0) {
    return formatCount(summary.diagnostics_count, 'diagnostic');
  }

  if (summary.enqueue_disabled_reasons && summary.enqueue_disabled_reasons.length > 0) {
    return formatCount(summary.enqueue_disabled_reasons.length, 'queue block');
  }

  return null;
}

function formatCount(count: number, singular: string): string {
  return count === 1 ? `1 ${singular}` : `${count} ${singular}s`;
}
