/** Data shapes the shared workbench parts accept. Presenters build these. */

export interface SegmentOption<T extends string = string> {
  value: T;
  label: string;
  /** Shown in mono after the label; omit when the count is unknown. */
  count?: number | string;
  /** Disabled option; `title` says why. */
  disabled?: boolean;
  title?: string;
}

export interface KeyValueRow {
  key: string;
  value: string;
  /** Render the value in the mono face (ids, timestamps, sizes). */
  mono?: boolean;
  /** Makes Ctrl+C over the value copy this run id (see shell/runContextClipboard.ts). */
  runId?: string;
}

export type DeviceLaneBlockKind = 'running' | 'waiting' | 'queued' | 'free';

export interface DeviceLaneBlock {
  id: string;
  kind: DeviceLaneBlockKind;
  /** Memory in GB; the block's width is proportional to it. */
  gb: number;
  /** Visible text inside the block; empty for free space. */
  label: string;
  /** Tooltip, for example "Waiting: needs 8 GB, 6.8 GB free". */
  title?: string;
}

export interface DeviceLane {
  id: string;
  /** Device name, for example "cuda:0 RTX 4090". */
  name: string;
  /** Usage line, for example "17.2 of 24 GB in use". */
  meta: string;
  blocks: DeviceLaneBlock[];
}
