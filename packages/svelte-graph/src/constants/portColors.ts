// Canonical port type color mapping
// Consolidates the typeColors constant previously duplicated across 5+ component files

import type { PortDataType } from '../types/workflow.js';

/** Color mapping for each port data type, used for handles, edges, and indicators */
export const PORT_TYPE_COLORS: Record<PortDataType, string> = {
  string: '#22c55e',
  prompt: '#3b82f6',
  number: '#f59e0b',
  boolean: '#ef4444',
  image: '#8b5cf6',
  audio: '#f472b6',
  stream: '#06b6d4',
  json: '#f97316',
  component: '#ec4899',
  document: '#14b8a6',
  tools: '#d97706',
  embedding: '#6366f1',
  vector_db: '#a855f7',
  any: '#6b7280',
};

/** Get the color for a port data type, falling back to 'any' */
export function getPortColor(dataType: string): string {
  return PORT_TYPE_COLORS[dataType as PortDataType] ?? PORT_TYPE_COLORS.any;
}
