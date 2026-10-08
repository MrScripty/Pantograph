<script lang="ts">
  import { untrack } from 'svelte';
  import JsonFilterPathEditor from './JsonFilterPathEditor.svelte';
  import { updateGroupNodeData } from '../../../stores/workflowStore';
  import type { GraphNode } from '../../../services/workflow/types';
  interface Props { groupId: string; node: GraphNode; sessionId: string | null }
  let { groupId, node, sessionId }: Props = $props();
  // This editor belongs to its opening session, including the interval before
  // the parent processes a session-switch render and destroys its keyed child.
  const ownerSessionId = untrack(() => sessionId);
  let invalidData = $derived(!node.data || typeof node.data !== 'object' || Array.isArray(node.data));
</script>

<JsonFilterPathEditor id={node.id} data={invalidData ? {} : node.data} {invalidData}
  inputPrefix={encodeURIComponent(JSON.stringify([groupId, node.id]))}
  applyNodeData={(nodeId, patch) => updateGroupNodeData(groupId, nodeId, node.node_type,
    node.data === undefined ? null : node.data, patch, ownerSessionId)} />
