import type { PortOption } from '../../../services/workflow/types';

export interface PumaLibSelectionNodeData {
  modelName: string;
  model_id: string;
  pumas_model_ref: Record<string, unknown>;
}

export function pumasModelRefFromOption(option: PortOption): Record<string, unknown> | null {
  const metadataModelRef = option.metadata?.pumas_model_ref;
  if (isObjectRecord(metadataModelRef)) return metadataModelRef;
  if (isObjectRecord(option.value)) return option.value;
  return null;
}

export function pumasModelIdFromOption(option: PortOption): string | null {
  const metadataId = readNonEmptyString(option.metadata?.id);
  if (metadataId) return metadataId;

  const modelRef = pumasModelRefFromOption(option);
  return readNonEmptyString(modelRef?.model_id);
}

export function isSelectablePumasModelOption(option: PortOption): boolean {
  return pumasModelIdFromOption(option) !== null && pumasModelRefFromOption(option) !== null;
}

export function pumasModelOptionKey(option: PortOption): string {
  return pumasModelIdFromOption(option) ?? option.label;
}

export function findPumasModelOptionById(
  options: PortOption[],
  modelId: string | null | undefined,
): PortOption | null {
  const cleanedModelId = readNonEmptyString(modelId);
  if (!cleanedModelId) return null;
  return options.find((option) => pumasModelIdFromOption(option) === cleanedModelId) ?? null;
}

export function buildPumaLibSelectionNodeData(option: PortOption): PumaLibSelectionNodeData {
  const modelId = pumasModelIdFromOption(option);
  const pumasModelRef = pumasModelRefFromOption(option);
  if (!modelId || !pumasModelRef) {
    throw new Error('Puma-Lib model option is missing canonical pumas_model_ref identity');
  }

  return {
    modelName: option.label,
    model_id: modelId,
    pumas_model_ref: pumasModelRef,
  };
}

function isObjectRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function readNonEmptyString(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}
