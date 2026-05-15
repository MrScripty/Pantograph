import { invoke } from '@tauri-apps/api/core';
import type { PortOptionsCommandArgs, PortOptionsResult } from './types';

export type PortOptionsInvoker = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export interface LoadPortOptionsOptions {
  forceRefresh?: boolean;
}

const cachedResults = new Map<string, PortOptionsResult>();
const inflightResults = new Map<string, Promise<PortOptionsResult>>();

export function invalidatePortOptionsCache(args?: PortOptionsCommandArgs): void {
  if (!args) {
    cachedResults.clear();
    inflightResults.clear();
    return;
  }

  const key = portOptionsCacheKey(args);
  cachedResults.delete(key);
  inflightResults.delete(key);
}

export async function loadPortOptions(
  args: PortOptionsCommandArgs,
  options: LoadPortOptionsOptions = {},
  invoker: PortOptionsInvoker = invoke,
): Promise<PortOptionsResult> {
  const key = portOptionsCacheKey(args);

  if (options.forceRefresh) {
    cachedResults.delete(key);
    inflightResults.delete(key);
  }

  const cached = cachedResults.get(key);
  if (cached) {
    return cached;
  }

  const inflight = inflightResults.get(key);
  if (inflight) {
    return inflight;
  }

  const request = invoker<PortOptionsResult>('query_port_options', args)
    .then((result) => {
      cachedResults.set(key, result);
      return result;
    })
    .finally(() => {
      inflightResults.delete(key);
    });
  inflightResults.set(key, request);
  return request;
}

export function portOptionsCacheKey(args: PortOptionsCommandArgs): string {
  return stableStringify({
    nodeType: args.nodeType,
    portId: args.portId,
    search: args.search ?? null,
    limit: args.limit ?? null,
    offset: args.offset ?? null,
    context: args.context ?? null,
  });
}

function stableStringify(value: unknown): string {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(',')}]`;
  }

  const record = value as Record<string, unknown>;
  const entries = Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`);
  return `{${entries.join(',')}}`;
}
