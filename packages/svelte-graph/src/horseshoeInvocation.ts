export type HorseshoeBlockedReason =
  | 'not_editable'
  | 'no_active_drag'
  | 'insert_not_supported'
  | 'candidates_pending'
  | 'no_insertable_nodes'
  | 'missing_anchor_position';

export interface HorseshoeOpenContext {
  canEdit: boolean;
  connectionDragActive: boolean;
  supportsInsert: boolean;
  hasConnectionIntent: boolean;
  insertableCount: number;
  anchorPosition: { x: number; y: number } | null;
}

export interface HorseshoeOpenResolution {
  action: 'open' | 'queue' | 'blocked';
  reason: HorseshoeBlockedReason | null;
}

export function isSpaceKey(event: Pick<KeyboardEvent, 'code' | 'key'>): boolean {
  return (
    event.code === 'Space' ||
    event.key === ' ' ||
    event.key === 'Space' ||
    event.key === 'Spacebar'
  );
}

export function resolveHorseshoeOpenRequest(
  context: HorseshoeOpenContext,
): HorseshoeOpenResolution {
  if (!context.canEdit) {
    return {
      action: 'blocked',
      reason: 'not_editable',
    };
  }

  if (!context.connectionDragActive) {
    return {
      action: 'blocked',
      reason: 'no_active_drag',
    };
  }

  if (!context.supportsInsert) {
    return {
      action: 'blocked',
      reason: 'insert_not_supported',
    };
  }

  if (!context.hasConnectionIntent) {
    return {
      action: 'queue',
      reason: 'candidates_pending',
    };
  }

  if (context.insertableCount <= 0) {
    return {
      action: 'blocked',
      reason: 'no_insertable_nodes',
    };
  }

  if (!context.anchorPosition) {
    return {
      action: 'blocked',
      reason: 'missing_anchor_position',
    };
  }

  return {
    action: 'open',
    reason: null,
  };
}

export function formatHorseshoeBlockedReason(reason: HorseshoeBlockedReason): string {
  switch (reason) {
    case 'not_editable':
      return 'graph is not editable';
    case 'no_active_drag':
      return 'no active connection drag';
    case 'insert_not_supported':
      return 'insert from reconnect is not supported; start a new drag from the output handle';
    case 'candidates_pending':
      return 'compatible insert candidates are still loading';
    case 'no_insertable_nodes':
      return 'no compatible node types can be inserted from this anchor';
    case 'missing_anchor_position':
      return 'cursor anchor position is unavailable';
  }
}
