import type { GraphNode, PortDefinition } from '../../../services/workflow/types';

export interface GroupDisplayDiagnostic {
  code: 'invalid_group_display_data';
  field: string;
  message: string;
  hint: string;
}

export interface GroupDisplay {
  label: string;
  nodeCount: number;
  nodes: GraphNode[];
  inputs: PortDefinition[];
  outputs: PortDefinition[];
  diagnostics: GroupDisplayDiagnostic[];
}

function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/** Check only the fields consumed by the group card. Execution validity belongs to the backend. */
export function groupDisplay(data: unknown): GroupDisplay {
  const group = record(data) && record(data.group) ? data.group : null;
  const label = record(data) && typeof data.label === 'string' && data.label
    ? data.label
    : group && typeof group.name === 'string' && group.name ? group.name : 'Group';
  const result: GroupDisplay = {
    label,
    nodeCount: group && Array.isArray(group.nodes) ? group.nodes.length : 0,
    nodes: [], inputs: [], outputs: [], diagnostics: [],
  };
  const report = (field: string, message: string): void => {
    result.diagnostics.push({
      code: 'invalid_group_display_data', field, message,
      hint: 'Restore a valid saved group or repair this field in the saved workflow, then reopen it. The editor has kept the original data.',
    });
  };
  if (!group) {
    report('data.group', 'The saved group must be an object.');
    return result;
  }
  if (!Array.isArray(group.nodes)) {
    report('data.group.nodes', 'The saved group nodes must be a list.');
  } else {
    const ids = new Set<string>();
    for (const [index, node] of group.nodes.entries()) {
      const field = `data.group.nodes[${index}]`;
      if (!record(node) || typeof node.id !== 'string' || !node.id ||
          typeof node.node_type !== 'string' || !node.node_type) {
        report(field, 'Each displayed node needs a nonempty ID and node type.');
      } else if (ids.has(node.id)) {
        report(`${field}.id`, `The displayed node ID '${node.id}' is repeated.`);
      } else {
        ids.add(node.id);
        // Node data is deliberately retained, including malformed data the path editor can repair.
        result.nodes.push(node as unknown as GraphNode);
      }
    }
  }
  for (const [field, ports] of [
    ['exposed_inputs', result.inputs], ['exposed_outputs', result.outputs],
  ] as const) {
    const mappings = group[field];
    if (!Array.isArray(mappings)) {
      report(`data.group.${field}`, 'The saved group ports must be a list.');
      continue;
    }
    const mappingsById = new Map<string, Record<string, unknown>>();
    for (const [index, mapping] of mappings.entries()) {
      const path = `data.group.${field}[${index}]`;
      if (!record(mapping) || typeof mapping.group_port_id !== 'string' || !mapping.group_port_id ||
          typeof mapping.group_port_label !== 'string' || typeof mapping.data_type !== 'string') {
        report(path, 'Each displayed port needs an ID, text label and data type.');
      } else if (mappingsById.has(mapping.group_port_id)) {
        const prior = mappingsById.get(mapping.group_port_id)!;
        // The public group producer repeats identical mappings for fan-out edges.
        // Draw one handle, while retaining every original mapping in saved data.
        if (['internal_node_id', 'internal_port_id', 'group_port_label', 'data_type']
            .some(key => prior[key] !== mapping[key])) {
          report(`${path}.group_port_id`, `The displayed port ID '${mapping.group_port_id}' has conflicting mappings.`);
        }
      } else {
        mappingsById.set(mapping.group_port_id, mapping);
        ports.push({
          id: mapping.group_port_id, label: mapping.group_port_label,
          data_type: mapping.data_type as PortDefinition['data_type'], required: false, multiple: false,
        });
      }
    }
  }
  if (result.diagnostics.length) {
    // A partial display could invite editing or connecting to the wrong saved identity.
    result.nodes = [];
    result.inputs = [];
    result.outputs = [];
  }
  return result;
}
