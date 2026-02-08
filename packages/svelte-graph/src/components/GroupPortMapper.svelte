<script lang="ts">
  import { get } from 'svelte/store';
  import { useGraphContext } from '../context/useGraphContext.js';
  import type { PortMapping, NodeGroup } from '../types/groups.js';
  import type { PortDefinition } from '../types/workflow.js';
  import { getPortColor } from '../constants/portColors.js';

  const { stores } = useGraphContext();

  interface Props {
    group: NodeGroup;
    onUpdate: (inputs: PortMapping[], outputs: PortMapping[]) => void;
    onClose: () => void;
  }

  let { group, onUpdate, onClose }: Props = $props();

  const { nodeDefinitions: nodeDefsStore } = stores.workflow;

  let exposedInputs = $state<PortMapping[]>([...group.exposed_inputs]);
  let exposedOutputs = $state<PortMapping[]>([...group.exposed_outputs]);

  let availableInputPorts = $derived(() => {
    const defs = get(nodeDefsStore);
    const ports: Array<{ nodeId: string; nodeName: string; port: PortDefinition }> = [];

    for (const node of group.nodes) {
      const def = defs.find((d) => d.node_type === node.node_type);
      if (def) {
        for (const input of def.inputs) {
          const isExposed = exposedInputs.some(
            (p) => p.internal_node_id === node.id && p.internal_port_id === input.id
          );
          if (!isExposed) {
            ports.push({ nodeId: node.id, nodeName: def.label, port: input });
          }
        }
      }
    }

    return ports;
  });

  let availableOutputPorts = $derived(() => {
    const defs = get(nodeDefsStore);
    const ports: Array<{ nodeId: string; nodeName: string; port: PortDefinition }> = [];

    for (const node of group.nodes) {
      const def = defs.find((d) => d.node_type === node.node_type);
      if (def) {
        for (const output of def.outputs) {
          const isExposed = exposedOutputs.some(
            (p) => p.internal_node_id === node.id && p.internal_port_id === output.id
          );
          if (!isExposed) {
            ports.push({ nodeId: node.id, nodeName: def.label, port: output });
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
</script>

<div class="port-mapper-overlay">
  <div class="port-mapper">
    <!-- Header -->
    <div class="mapper-header">
      <div class="mapper-title-row">
        <svg class="header-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
        </svg>
        <span class="mapper-title">Configure Group Ports</span>
      </div>
      <button class="close-btn" onclick={onClose}>
        <svg width="20" height="20" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="mapper-content">
      <!-- Exposed Inputs -->
      <div class="port-section">
        <h3 class="section-title">Exposed Input Ports</h3>
        <div class="port-list">
          {#each exposedInputs as input, i}
            <div class="port-row">
              <span class="type-dot" style="background-color: {getPortColor(input.data_type)}"></span>
              <input
                type="text"
                class="port-label-input"
                value={input.group_port_label}
                oninput={(e) => updateInputLabel(i, (e.target as HTMLInputElement).value)}
              />
              <span class="port-node-id">{input.internal_node_id}</span>
              <button class="remove-btn" onclick={() => removeInput(i)}>
                <svg width="16" height="16" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          {/each}

          {#if availableInputPorts().length > 0}
            <div class="add-port-section">
              <span class="add-label">Add input:</span>
              <div class="add-port-list">
                {#each availableInputPorts() as { nodeId, nodeName, port }}
                  <button class="add-port-btn" onclick={() => addInput(nodeId, port)}>
                    {nodeName} / {port.label}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>

      <!-- Exposed Outputs -->
      <div class="port-section">
        <h3 class="section-title">Exposed Output Ports</h3>
        <div class="port-list">
          {#each exposedOutputs as output, i}
            <div class="port-row">
              <span class="type-dot" style="background-color: {getPortColor(output.data_type)}"></span>
              <input
                type="text"
                class="port-label-input"
                value={output.group_port_label}
                oninput={(e) => updateOutputLabel(i, (e.target as HTMLInputElement).value)}
              />
              <span class="port-node-id">{output.internal_node_id}</span>
              <button class="remove-btn" onclick={() => removeOutput(i)}>
                <svg width="16" height="16" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          {/each}

          {#if availableOutputPorts().length > 0}
            <div class="add-port-section">
              <span class="add-label">Add output:</span>
              <div class="add-port-list">
                {#each availableOutputPorts() as { nodeId, nodeName, port }}
                  <button class="add-port-btn" onclick={() => addOutput(nodeId, port)}>
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
    <div class="mapper-footer">
      <button class="cancel-btn" onclick={onClose}>Cancel</button>
      <button class="save-btn" onclick={handleSave}>Save Changes</button>
    </div>
  </div>
</div>

<style>
  .port-mapper-overlay {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }

  .port-mapper {
    background-color: #262626;
    border-radius: 0.5rem;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
    width: 600px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }

  /* --- Header --- */
  .mapper-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #404040;
  }

  .mapper-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .header-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: #c084fc;
  }

  .mapper-title {
    font-size: 1.125rem;
    font-weight: 500;
    color: #e5e5e5;
  }

  .close-btn {
    background: none;
    border: none;
    color: #a3a3a3;
    cursor: pointer;
    padding: 0;
    transition: color 150ms;
  }

  .close-btn:hover {
    color: #e5e5e5;
  }

  /* --- Content --- */
  .mapper-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .port-section {
    display: flex;
    flex-direction: column;
  }

  .section-title {
    font-size: 0.875rem;
    font-weight: 500;
    color: #d8b4fe;
    margin: 0 0 0.5rem 0;
  }

  .port-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .port-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background-color: rgba(64, 64, 64, 0.5);
    border-radius: 0.25rem;
    padding: 0.5rem 0.75rem;
  }

  .type-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 9999px;
    flex-shrink: 0;
  }

  .port-label-input {
    flex: 1;
    background: transparent;
    border: none;
    font-size: 0.875rem;
    color: #e5e5e5;
    outline: none;
  }

  .port-node-id {
    font-size: 0.75rem;
    color: #737373;
  }

  .remove-btn {
    background: none;
    border: none;
    color: #a3a3a3;
    cursor: pointer;
    padding: 0;
    transition: color 150ms;
  }

  .remove-btn:hover {
    color: #ef4444;
  }

  /* --- Add Port --- */
  .add-port-section {
    margin-top: 0.5rem;
  }

  .add-label {
    font-size: 0.75rem;
    color: #a3a3a3;
  }

  .add-port-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.25rem;
  }

  .add-port-btn {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    background-color: #404040;
    border: none;
    border-radius: 0.25rem;
    color: #d4d4d4;
    cursor: pointer;
    transition: background-color 150ms;
  }

  .add-port-btn:hover {
    background-color: rgba(147, 51, 234, 0.5);
  }

  /* --- Footer --- */
  .mapper-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-top: 1px solid #404040;
  }

  .cancel-btn {
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    background: none;
    border: none;
    color: #d4d4d4;
    cursor: pointer;
    transition: color 150ms;
  }

  .cancel-btn:hover {
    color: #f5f5f5;
  }

  .save-btn {
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    background-color: #9333ea;
    border: none;
    border-radius: 0.25rem;
    color: white;
    cursor: pointer;
    transition: background-color 150ms;
  }

  .save-btn:hover {
    background-color: #a855f7;
  }
</style>
