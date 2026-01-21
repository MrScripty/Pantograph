<script lang="ts">
  import BaseNode from '../BaseNode.svelte';
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

  let enabledTools = $state<string[]>(data.enabledTools || []);

  function toggleTool(toolId: string) {
    if (enabledTools.includes(toolId)) {
      enabledTools = enabledTools.filter((t) => t !== toolId);
    } else {
      enabledTools = [...enabledTools, toolId];
    }
    updateNodeData(id, { enabledTools });
  }
</script>

<div class="agent-tools-node-wrapper">
  <BaseNode {id} {data} {selected}>
    {#snippet header()}
      <div class="flex items-center gap-2">
        <div class="w-5 h-5 rounded bg-amber-600 flex items-center justify-center flex-shrink-0">
          <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.42 15.17L17.25 21A2.652 2.652 0 0021 17.25l-5.877-5.877M11.42 15.17l2.496-3.03c.317-.384.74-.626 1.208-.766M11.42 15.17l-4.655 5.653a2.548 2.548 0 11-3.586-3.586l6.837-5.63m5.108-.233c.55-.164 1.163-.188 1.743-.14a4.5 4.5 0 004.486-6.336l-3.276 3.277a3.004 3.004 0 01-2.25-2.25l3.276-3.276a4.5 4.5 0 00-6.336 4.486c.091 1.076-.071 2.264-.904 2.95l-.102.085m-1.745 1.437L5.909 7.5H4.5L2.25 3.75l1.5-1.5L7.5 4.5v1.409l4.26 4.26m-1.745 1.437l1.745-1.437m6.615 8.206L15.75 15.75M4.867 19.125h.008v.008h-.008v-.008z" />
          </svg>
        </div>
        <span class="text-sm font-medium text-neutral-200">{data.label || 'Agent Tools'}</span>
      </div>
    {/snippet}

    {#snippet children()}
      <div class="space-y-1">
        {#each availableTools as tool}
          <label
            class="flex items-center gap-2 cursor-pointer hover:bg-neutral-700/50 rounded px-1 py-0.5"
            title={tool.description}
          >
            <input
              type="checkbox"
              checked={enabledTools.includes(tool.id)}
              onchange={() => toggleTool(tool.id)}
              class="w-3 h-3 rounded border-neutral-500 bg-neutral-800 text-amber-500 focus:ring-amber-500 focus:ring-offset-0"
            />
            <span class="text-xs text-neutral-300">{tool.label}</span>
          </label>
        {/each}
      </div>
    {/snippet}
  </BaseNode>
</div>

<style>
  .agent-tools-node-wrapper :global(.base-node) {
    border-color: rgba(217, 119, 6, 0.5);
  }

  .agent-tools-node-wrapper :global(.node-header) {
    background-color: rgba(217, 119, 6, 0.2);
    border-color: rgba(217, 119, 6, 0.3);
  }
</style>
