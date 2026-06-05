import type {
  NodeDefinition,
  PortDefinition,
  PortOption,
  PortOptionsCommandArgs,
  PortOptionsResult,
} from '../../../services/workflow/types';
import {
  loadPortOptions,
  portOptionsCacheKey,
} from '../../../services/workflow/portOptionsCache.ts';
import type { SelectionInputOption } from './selectionInputState.ts';

export interface SelectionInputTargetNode {
  id: string;
  nodeType?: string;
  node_type?: string;
  data?: Record<string, unknown> & {
    definition?: NodeDefinition;
  };
}

export interface SelectionInputProviderQuery {
  args: PortOptionsCommandArgs;
  requestKey: string;
}

export type SelectionInputProviderLoadStatus = 'applied' | 'stale';

export interface SelectionInputProviderLoadResult {
  status: SelectionInputProviderLoadStatus;
  options: SelectionInputOption[];
}

export type SelectionInputPortOptionsLoader = (
  args: PortOptionsCommandArgs,
) => Promise<PortOptionsResult>;

export function buildSelectionInputProviderQuery(
  targetNode: SelectionInputTargetNode | null,
  targetPort: PortDefinition | null,
): SelectionInputProviderQuery | null {
  const provider = targetPort?.options_provider;
  if (!targetNode || !provider) return null;

  const args: PortOptionsCommandArgs = {
    nodeType: provider.node_type,
    portId: provider.port_id,
    context: compactContext({
      targetNodeId: targetNode.id,
      taskKind: extractTaskKind(targetNode, targetPort),
      descriptorFingerprint: extractDescriptorFingerprint(targetNode.data),
      selectedModelRef: extractSelectedModelRef(targetNode.data?.pumas_model_ref),
      packageFactsSummaryCursor: extractString(targetNode.data?.package_facts_summary_cursor),
      requestedRuntimeId: extractString(targetNode.data?.runtime),
      requestedDeviceId: extractString(targetNode.data?.device),
    }),
  };

  return {
    args,
    requestKey: portOptionsCacheKey(args),
  };
}

export async function loadLatestSelectionInputProviderOptions(
  query: SelectionInputProviderQuery,
  latestRequestKey: () => string | null,
  loader: SelectionInputPortOptionsLoader = defaultPortOptionsLoader,
): Promise<SelectionInputProviderLoadResult> {
  const result = await loader(query.args);
  if (latestRequestKey() !== query.requestKey) {
    return {
      status: 'stale',
      options: [],
    };
  }

  return {
    status: 'applied',
    options: normalizePortOptions(result.options),
  };
}

export function normalizePortOptions(options: PortOption[]): SelectionInputOption[] {
  return options.map((option) => {
    const normalized: SelectionInputOption = {
      label: option.label,
      value: option.value,
    };
    if (option.disabled === true) normalized.disabled = true;
    if (option.unavailableState) normalized.unavailableState = option.unavailableState;
    if (option.unavailableReasonCode) normalized.unavailableReasonCode = option.unavailableReasonCode;
    if (option.unavailableReason) normalized.unavailableReason = option.unavailableReason;
    return normalized;
  });
}

function defaultPortOptionsLoader(args: PortOptionsCommandArgs): Promise<PortOptionsResult> {
  return loadPortOptions(args);
}

function compactContext(
  context: NonNullable<PortOptionsCommandArgs['context']>,
): NonNullable<PortOptionsCommandArgs['context']> | undefined {
  const compacted = Object.fromEntries(
    Object.entries(context).filter(
      (entry): entry is [string, string] => typeof entry[1] === 'string' && entry[1] !== '',
    ),
  );
  return Object.keys(compacted).length > 0 ? compacted : undefined;
}

function extractTaskKind(targetNode: SelectionInputTargetNode, targetPort: PortDefinition): string | undefined {
  return (
    extractString(targetNode.data?.task_kind) ??
    targetPort.inference_payloads?.find((payload) => typeof payload.task_id === 'string')?.task_id
  );
}

function extractDescriptorFingerprint(data: Record<string, unknown> | undefined): string | undefined {
  if (!data) return undefined;

  return (
    extractStringFromRecord(data.inference_interface_update_proposal, 'current_descriptor_fingerprint') ??
    extractStringFromRecord(data.inference_interface_drift_report, 'current_fingerprint') ??
    extractStringFromRecord(data.inference_interface_snapshot, 'descriptor_fingerprint')
  );
}

function extractSelectedModelRef(value: unknown): string | undefined {
  const direct = extractString(value);
  if (direct) return direct;
  if (!value || typeof value !== 'object') return undefined;

  const record = value as Record<string, unknown>;
  const nested = extractSelectedModelRef(record.pumas_model_ref);
  if (nested) return nested;

  const modelId = extractString(record.model_id ?? record.modelId ?? record.id);
  if (!modelId) return undefined;

  const revision = extractString(record.revision);
  const selectedArtifactId = extractString(record.selected_artifact_id ?? record.selectedArtifactId);
  return [modelId, revision, selectedArtifactId].filter(Boolean).join('@');
}

function extractString(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  return trimmed === '' ? undefined : trimmed;
}

function extractStringFromRecord(value: unknown, field: string): string | undefined {
  if (!value || typeof value !== 'object') return undefined;
  return extractString((value as Record<string, unknown>)[field]);
}
