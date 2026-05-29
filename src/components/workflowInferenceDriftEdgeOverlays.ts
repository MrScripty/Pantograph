import type { Edge, Node } from '@xyflow/svelte';
import type {
  InferenceInterfaceAffectedEdge,
  InferenceInterfaceGraphPatchOperation,
  InferenceInterfaceUpdateProposal,
} from '../services/workflow/types';

export const INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY =
  'inferenceInterfaceDriftAffected';
export const INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY =
  'inferenceInterfaceDriftTitle';

interface InferenceDriftEdgeOverlay {
  affected: boolean;
  title: string;
}

export function applyInferenceDriftEdgeOverlays(
  edges: readonly Edge[],
  nodes: readonly Node[],
): { edges: Edge[]; changed: boolean } {
  const overlays = collectInferenceDriftEdgeOverlays(nodes);
  let changed = false;

  const nextEdges = edges.map((edge) => {
    const overlay = overlays.get(edge.id) ?? null;
    const edgeData = edge.data ?? {};
    const hasOverlay =
      edgeData[INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY] === true;
    const currentTitle = edgeData[INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY];

    if (
      overlay &&
      hasOverlay &&
      currentTitle === overlay.title
    ) {
      return edge;
    }

    if (!overlay && !hasOverlay && currentTitle === undefined) {
      return edge;
    }

    changed = true;
    const nextData = { ...edgeData };
    if (overlay) {
      nextData[INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY] = true;
      nextData[INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY] = overlay.title;
    } else {
      delete nextData[INFERENCE_INTERFACE_DRIFT_EDGE_ACTIVE_KEY];
      delete nextData[INFERENCE_INTERFACE_DRIFT_EDGE_TITLE_KEY];
    }

    return {
      ...edge,
      data: nextData,
    };
  });

  return {
    edges: nextEdges,
    changed,
  };
}

export function inferenceProposalAffectsPort(params: {
  data: Record<string, unknown>;
  nodeId: string;
  portId: string;
}): boolean {
  const proposal = readInferenceInterfaceUpdateProposal(
    params.data.inference_interface_update_proposal,
  );
  if (!proposal) {
    return false;
  }

  return collectAffectedEdges(proposal).some(
    (edge) =>
      (
        edge.source_node_id === params.nodeId &&
        edge.source_port_id === params.portId
      ) ||
      (
        edge.target_node_id === params.nodeId &&
        edge.target_port_id === params.portId
      ),
  );
}

function collectInferenceDriftEdgeOverlays(
  nodes: readonly Node[],
): Map<string, InferenceDriftEdgeOverlay> {
  const overlays = new Map<string, InferenceDriftEdgeOverlay>();

  for (const node of nodes) {
    const proposal = readInferenceInterfaceUpdateProposal(
      node.data?.inference_interface_update_proposal,
    );
    if (!proposal) {
      continue;
    }

    for (const edge of collectAffectedEdges(proposal)) {
      overlays.set(edge.edge_id, {
        affected: true,
        title: driftEdgeTitle(proposal, edge),
      });
    }
  }

  return overlays;
}

function collectAffectedEdges(
  proposal: InferenceInterfaceUpdateProposal,
): InferenceInterfaceAffectedEdge[] {
  const affectedEdges = new Map<string, InferenceInterfaceAffectedEdge>();

  for (const edge of proposal.affected_edges ?? []) {
    affectedEdges.set(edge.edge_id, edge);
  }

  for (const operation of proposal.operations ?? []) {
    const affectedEdge = operationAffectedEdge(operation);
    if (affectedEdge) {
      affectedEdges.set(affectedEdge.edge_id, affectedEdge);
    }
  }

  return [...affectedEdges.values()];
}

function operationAffectedEdge(
  operation: InferenceInterfaceGraphPatchOperation,
): InferenceInterfaceAffectedEdge | null {
  if (operation.operation !== 'remove_invalid_edge') {
    return null;
  }

  return operation.value.edge;
}

function driftEdgeTitle(
  proposal: InferenceInterfaceUpdateProposal,
  edge: InferenceInterfaceAffectedEdge,
): string {
  const summary = proposal.drift_report?.blocking
    ? 'Blocking inference interface drift'
    : 'Inference interface drift';
  return `${summary}: ${edge.source_node_id}.${edge.source_port_id} -> ${edge.target_node_id}.${edge.target_port_id}`;
}

function readInferenceInterfaceUpdateProposal(
  value: unknown,
): InferenceInterfaceUpdateProposal | null {
  if (!value || typeof value !== 'object') {
    return null;
  }

  const candidate = value as Partial<InferenceInterfaceUpdateProposal>;
  if (
    typeof candidate.proposal_id !== 'string' ||
    typeof candidate.node_id !== 'string' ||
    typeof candidate.current_descriptor_fingerprint !== 'string'
  ) {
    return null;
  }

  return candidate as InferenceInterfaceUpdateProposal;
}
