export const WORKFLOW_NODE_DOUBLE_CLICK_THRESHOLD_MS = 300;

export interface WorkflowNodeClickState {
  lastClickTime: number;
  lastClickNodeId: string | null;
}

export interface WorkflowNodeActivationLike {
  id: string;
  type?: string | null;
  position: {
    x: number;
    y: number;
  };
  measured?: {
    width?: number | null;
    height?: number | null;
  } | null;
  width?: number | null;
  height?: number | null;
  data?: Record<string, unknown> | null;
}

export interface WorkflowNodeClickDecision {
  state: WorkflowNodeClickState;
  isDoubleClick: boolean;
}

export interface WorkflowGroupZoomTarget {
  nodeId: string;
  position: {
    x: number;
    y: number;
  };
  bounds: {
    width: number;
    height: number;
  };
}

export function resolveWorkflowNodeClick(
  state: WorkflowNodeClickState,
  nodeId: string,
  now: number,
  thresholdMs = WORKFLOW_NODE_DOUBLE_CLICK_THRESHOLD_MS,
): WorkflowNodeClickDecision {
  return {
    state: {
      lastClickTime: now,
      lastClickNodeId: nodeId,
    },
    isDoubleClick: state.lastClickNodeId === nodeId && now - state.lastClickTime < thresholdMs,
  };
}

export function isWorkflowGroupNode(node: WorkflowNodeActivationLike): boolean {
  return node.data?.isGroup === true || node.type === 'node-group';
}

export function resolveWorkflowGroupZoomTarget(
  node: WorkflowNodeActivationLike,
): WorkflowGroupZoomTarget | null {
  if (!isWorkflowGroupNode(node)) {
    return null;
  }

  return {
    nodeId: node.id,
    position: node.position,
    bounds: {
      width: node.measured?.width || node.width || 200,
      height: node.measured?.height || node.height || 100,
    },
  };
}
