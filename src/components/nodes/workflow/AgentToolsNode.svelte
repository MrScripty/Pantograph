<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
  import NodeHeaderTitle from './NodeHeaderTitle.svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      enabledTools?: string[];
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  const availableTools = [
    { id: 'read_file', label: 'Read File', description: 'Read file contents' },
    { id: 'write_file', label: 'Write File', description: 'Write to files' },
    { id: 'web_search', label: 'Web Search', description: 'Search the web' },
    { id: 'code_exec', label: 'Code Execution', description: 'Execute code snippets' },
    { id: 'shell', label: 'Shell Command', description: 'Run shell commands' },
  ];

  let enabledTools = $derived(data.enabledTools || []);

  function toggleTool(toolId: string) {
    const nextEnabledTools = enabledTools.includes(toolId)
      ? enabledTools.filter((t) => t !== toolId)
      : [...enabledTools, toolId];
    updateNodeData(id, { enabledTools: nextEnabledTools });
  }
</script>

<div class="agent-tools-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <NodeHeaderTitle label={data.label || 'Agent Tools'} />
    {/snippet}

      <div class="space-y-1">
        {#each availableTools as tool (tool.id)}
          <label
            class="flex items-center gap-2 cursor-pointer hover:bg-pg-panel-raised rounded px-1 py-0.5"
            title={tool.description}
          >
            <input
              type="checkbox"
              checked={enabledTools.includes(tool.id)}
              onchange={() => toggleTool(tool.id)}
              class="w-3 h-3 accent-pg-text-soft"
            />
            <span class="text-xs text-pg-text-soft">{tool.label}</span>
          </label>
        {/each}
      </div>
  </BaseNode>
</div>
