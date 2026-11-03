// Existing unversioned browser record; this decoder does not migrate or write it.
export type UndoableAction = { type: 'COMMIT_SOFT_DELETE'; hash: string };

export interface UndoEntry {
  id: string;
  action: UndoableAction;
  timestamp: number;
}

export interface UndoState {
  history: UndoEntry[];
  position: number; // Index of next action to undo (-1 means nothing to undo)
}

export const MAX_UNDO_HISTORY = 32;
export const UNDO_STORAGE_KEY = 'pantograph-unified-undo';

type UndoDecodeResult =
  | { ok: true; state: UndoState }
  | { ok: false; reason: string };

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isIdentifier(value: unknown): value is string {
  // IDs are opaque: do not impose a new hash format or length limit.
  return typeof value === 'string' && value.trim().length > 0;
}

export function decodeUndoState(value: unknown): UndoDecodeResult {
  if (!isRecord(value) || !Array.isArray(value.history)) {
    return { ok: false, reason: 'Undo history must be an array in a record.' };
  }
  if (value.history.length > MAX_UNDO_HISTORY) {
    return { ok: false, reason: 'Undo history exceeds its 32-entry limit.' };
  }

  const history: UndoEntry[] = [];
  for (const entry of value.history) {
    if (!isRecord(entry) || !isIdentifier(entry.id)
      || typeof entry.timestamp !== 'number' || !Number.isSafeInteger(entry.timestamp)
      || entry.timestamp < 0 || !isRecord(entry.action)
      || entry.action.type !== 'COMMIT_SOFT_DELETE' || !isIdentifier(entry.action.hash)) {
      return { ok: false, reason: 'Undo history contains an invalid entry or action.' };
    }
    history.push({
      id: entry.id,
      timestamp: entry.timestamp,
      action: { type: 'COMMIT_SOFT_DELETE', hash: entry.action.hash },
    });
  }

  // Preserve the existing omitted-position fallback for recognized legacy records.
  const position = Object.hasOwn(value, 'position') ? value.position : history.length - 1;
  if (typeof position !== 'number' || !Number.isInteger(position)
    || position < -1 || position >= history.length) {
    return { ok: false, reason: 'Undo position must be an integer within the history.' };
  }
  return { ok: true, state: { history, position } };
}
