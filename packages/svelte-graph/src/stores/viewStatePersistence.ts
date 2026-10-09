import type { ViewLevel } from '../types/view.js';

/** Existing unversioned record. Missing legacy fields leave current state intact. */
export interface ViewStateRecord {
  viewLevel?: ViewLevel;
  orchestrationId?: string | null;
  dataGraphId?: string | null;
  groupStack?: string[];
}

export function decodeViewStateRecord(value: unknown): ViewStateRecord | null {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return null;
  const record: ViewStateRecord = {};

  if ('viewLevel' in value) {
    const level = value.viewLevel;
    if (level !== 'orchestration' && level !== 'data-graph' && level !== 'group') return null;
    record.viewLevel = level;
  }
  if ('orchestrationId' in value) {
    if (value.orchestrationId !== null && typeof value.orchestrationId !== 'string') return null;
    record.orchestrationId = value.orchestrationId;
  }
  if ('dataGraphId' in value) {
    if (value.dataGraphId !== null && typeof value.dataGraphId !== 'string') return null;
    record.dataGraphId = value.dataGraphId;
  }
  if ('groupStack' in value) {
    if (!Array.isArray(value.groupStack)) return null;
    const groups: string[] = [];
    for (const group of value.groupStack) {
      if (typeof group !== 'string') return null;
      groups.push(group);
    }
    record.groupStack = groups;
  }
  return record;
}
