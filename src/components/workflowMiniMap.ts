interface WorkflowMiniMapNodeLike {
  type?: string | null;
  data?: {
    isGroup?: unknown;
    definition?: {
      category?: string;
    };
  } | null;
}

/*
 * The minimap draws node colours as inline `fill`, so CSS cannot restyle them;
 * they are token references here (docs/design/mockups/boards/shell.css
 * `.minimap i`). Categories are not colour-coded; groups read lighter so they
 * stay distinguishable from ordinary nodes.
 */
export const WORKFLOW_MINIMAP_NODE_COLOR = 'var(--pg-border-idle)';
export const WORKFLOW_MINIMAP_GROUP_COLOR = 'var(--pg-text-muted)';

export function getWorkflowMiniMapNodeColor(node: WorkflowMiniMapNodeLike): string {
  if (node.type === 'node-group' || node.data?.isGroup) {
    return WORKFLOW_MINIMAP_GROUP_COLOR;
  }

  return WORKFLOW_MINIMAP_NODE_COLOR;
}
