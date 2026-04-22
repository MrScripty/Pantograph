interface WorkflowMiniMapNodeLike {
  type?: string | null;
  data?: {
    isGroup?: unknown;
    definition?: {
      category?: string;
    };
  } | null;
}

export function getWorkflowMiniMapNodeColor(node: WorkflowMiniMapNodeLike): string {
  if (node.type === 'node-group' || node.data?.isGroup) {
    return '#7c3aed';
  }

  switch (node.data?.definition?.category) {
    case 'input':
      return '#2563eb';
    case 'processing':
      return '#16a34a';
    case 'tool':
      return '#d97706';
    case 'output':
      return '#0891b2';
    case 'control':
      return '#9333ea';
    default:
      return '#525252';
  }
}
