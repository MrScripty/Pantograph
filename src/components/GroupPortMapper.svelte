<script lang="ts">
  import type { PortMapping, NodeGroup } from '../services/workflow/groupTypes';
  import type { NodeDefinition, PortDefinition } from '../services/workflow/types';
  import { nodeDefinitions } from '../stores/workflowStore';

  interface Props {
    group: NodeGroup;
    onUpdate: (inputs: PortMapping[], outputs: PortMapping[]) => void;
    onClose: () => void;
  }

  let { group, onUpdate, onClose }: Props = $props();

  // Create a local copy for editing
  let exposedInputs = $state<PortMapping[]>([...group.exposed_inputs]);
  let exposedOutputs = $state<PortMapping[]>([...group.exposed_outputs]);

  // Get available ports from internal nodes
  let availableInputPorts = $derived(() => {
    const ports: Array<{ nodeId: string; nodeName: string; port: PortDefinition }> = [];

    for (const node of group.nodes) {
      const def = $nodeDefinitions.find((d) => d.node_type === node.node_type);
      if (def) {
        for (const input of def.inputs) {
          // Check if this port is not already exposed
          const isExposed = exposedInputs.some(
            (p) => p.internal_node_id === node.id && p.internal_port_id === input.id
          );
          if (!isExposed) {
            ports.push({
              nodeId: node.id,
              nodeName: def.label,
              port: input,
            });
          }
        }
      }
    }

    return ports;
  });

  let availableOutputPorts = $derived(() => {
    const ports: Array<{ nodeId: string; nodeName: string; port: PortDefinition }> = [];

    for (const node of group.nodes) {
      const def = $nodeDefinitions.find((d) => d.node_type === node.node_type);
      if (def) {
        for (const output of def.outputs) {
          // Check if this port is not already exposed
          const isExposed = exposedOutputs.some(
            (p) => p.internal_node_id === node.id && p.internal_port_id === output.id
          );
          if (!isExposed) {
            ports.push({
              nodeId: node.id,
              nodeName: def.label,
              port: output,
            });
          }
        }
      }
    }

    return ports;
  });

  function addInput(nodeId: string, port: PortDefinition) {
    const newMapping: PortMapping = {
      internal_node_id: nodeId,
      internal_port_id: port.id,
      group_port_id: `in-${nodeId}-${port.id}`,
      group_port_label: port.label,
      data_type: port.data_type,
    };
    exposedInputs = [...exposedInputs, newMapping];
  }

  function addOutput(nodeId: string, port: PortDefinition) {
    const newMapping: PortMapping = {
      internal_node_id: nodeId,
      internal_port_id: port.id,
      group_port_id: `out-${nodeId}-${port.id}`,
      group_port_label: port.label,
      data_type: port.data_type,
    };
    exposedOutputs = [...exposedOutputs, newMapping];
  }

  function removeInput(index: number) {
    exposedInputs = exposedInputs.filter((_, i) => i !== index);
  }

  function removeOutput(index: number) {
    exposedOutputs = exposedOutputs.filter((_, i) => i !== index);
  }

  function updateInputLabel(index: number, label: string) {
    exposedInputs = exposedInputs.map((p, i) =>
      i === index ? { ...p, group_port_label: label } : p
    );
  }

  function updateOutputLabel(index: number, label: string) {
    exposedOutputs = exposedOutputs.map((p, i) =>
      i === index ? { ...p, group_port_label: label } : p
    );
  }

  function handleSave() {
    onUpdate(exposedInputs, exposedOutputs);
  }

  const typeColors: Record<string, string> = {
    string: 'bg-green-500',
    prompt: 'bg-blue-500',
    number: 'bg-amber-500',
    boolean: 'bg-red-500',
    image: 'bg-violet-500',
    audio: 'bg-pink-500',
    stream: 'bg-cyan-500',
    json: 'bg-orange-500',
    component: 'bg-pink-600',
    document: 'bg-teal-500',
    tools: 'bg-amber-600',
    embedding: 'bg-indigo-500',
    vector_db: 'bg-purple-500',
    any: 'bg-neutral-500',
  };

  function getTypeColor(dataType: string): string {
    return typeColors[dataType] || typeColors.any;
  }
</script>

<div class="port-mapper-overlay fixed inset-0 bg-black/50 flex items-center justify-center z-50">
  <div class="port-mapper bg-neutral-800 rounded-lg shadow-xl w-[600px] max-h-[80vh] flex flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-3 border-b border-neutral-700">
      <div class="flex items-center gap-2">
        <svg class="w-5 h-5 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
        </svg>
        <span class="text-lg font-medium text-neutral-200">Configure Group Ports</span>
      </div>
      <button
        class="text-neutral-400 hover:text-neutral-200 transition-colors"
        onclick={onClose}
      >
        <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-4 space-y-6">
      <!-- Exposed Inputs -->
      <div>
        <h3 class="text-sm font-medium text-purple-300 mb-2">Exposed Input Ports</h3>
        <div class="space-y-2">
          {#each exposedInputs as input, i}
            <div class="flex items-center gap-2 bg-neutral-700/50 rounded px-3 py-2">
              <span class="w-2 h-2 rounded-full {getTypeColor(input.data_type)}"></span>
              <input
                type="text"
                class="flex-1 bg-transparent text-sm text-neutral-200 outline-none"
                value={input.group_port_label}
                oninput={(e) => updateInputLabel(i, (e.target as HTMLInputElement).value)}
              />
              <span class="text-xs text-neutral-500">{input.internal_node_id}</span>
              <button
                class="text-neutral-400 hover:text-red-400 transition-colors"
                onclick={() => removeInput(i)}
              >
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          {/each}

          {#if availableInputPorts().length > 0}
            <div class="mt-2">
              <span class="text-xs text-neutral-400">Add input:</span>
              <div class="flex flex-wrap gap-1 mt-1">
                {#each availableInputPorts() as { nodeId, nodeName, port }}
                  <button
                    class="text-xs px-2 py-1 bg-neutral-700 hover:bg-purple-600/50 rounded text-neutral-300 transition-colors"
                    onclick={() => addInput(nodeId, port)}
                  >
                    {nodeName} / {port.label}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>

      <!-- Exposed Outputs -->
      <div>
        <h3 class="text-sm font-medium text-purple-300 mb-2">Exposed Output Ports</h3>
        <div class="space-y-2">
          {#each exposedOutputs as output, i}
            <div class="flex items-center gap-2 bg-neutral-700/50 rounded px-3 py-2">
              <span class="w-2 h-2 rounded-full {getTypeColor(output.data_type)}"></span>
              <input
                type="text"
                class="flex-1 bg-transparent text-sm text-neutral-200 outline-none"
                value={output.group_port_label}
                oninput={(e) => updateOutputLabel(i, (e.target as HTMLInputElement).value)}
              />
              <span class="text-xs text-neutral-500">{output.internal_node_id}</span>
              <button
                class="text-neutral-400 hover:text-red-400 transition-colors"
                onclick={() => removeOutput(i)}
              >
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          {/each}

          {#if availableOutputPorts().length > 0}
            <div class="mt-2">
              <span class="text-xs text-neutral-400">Add output:</span>
              <div class="flex flex-wrap gap-1 mt-1">
                {#each availableOutputPorts() as { nodeId, nodeName, port }}
                  <button
                    class="text-xs px-2 py-1 bg-neutral-700 hover:bg-purple-600/50 rounded text-neutral-300 transition-colors"
                    onclick={() => addOutput(nodeId, port)}
                  >
                    {nodeName} / {port.label}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-end gap-2 px-4 py-3 border-t border-neutral-700">
      <button
        class="px-4 py-2 text-sm text-neutral-300 hover:text-neutral-100 transition-colors"
        onclick={onClose}
      >
        Cancel
      </button>
      <button
        class="px-4 py-2 text-sm bg-purple-600 hover:bg-purple-500 text-white rounded transition-colors"
        onclick={handleSave}
      >
        Save Changes
      </button>
    </div>
  </div>
</div>
