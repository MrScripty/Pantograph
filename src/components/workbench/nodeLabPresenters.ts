/**
 * Node Editor projection (docs/design/workbench-redesign-spec.md, "Node Editor").
 *
 * Node authoring has no backend in this build. The page reads the built-in node
 * definitions the engine already reports (the `nodeDefinitions` store the Graph
 * palette lists) and shows them read-only. Everything the mockup shows that the
 * definitions do not carry (version, runtime, device, output retention) is
 * omitted rather than invented.
 */
import type {
  ExecutionMode,
  IoBindingOrigin,
  NodeCategory,
  NodeDefinition,
  PortDefinition,
} from '../../services/workflow/types';

/** The authoring facilities this build does not include. */
export const NODE_LAB_UNAVAILABLE_FACILITIES = [
  'Authoring API',
  'Local Agent',
  'Runtime Publishing',
] as const;

export type NodeLabFacility = (typeof NODE_LAB_UNAVAILABLE_FACILITIES)[number];

export interface NodeLabNotice {
  headline: string;
  sentence: string;
}

export function nodeLabAuthoringNotice(): NodeLabNotice {
  return {
    headline: 'Node authoring is not available in this build.',
    sentence:
      'You can read the built-in node definitions, but the Authoring API, the Local Agent and ' +
      'Runtime Publishing are not included, so creating, saving, publishing and trying nodes ' +
      'are off. They would be enabled in Settings; this build has no setting for them yet.',
  };
}

/** Every authoring control the board shows, each disabled with its own reason. */
export type NodeLabAuthoringControl =
  | 'new_node'
  | 'save_draft'
  | 'publish'
  | 'try'
  | 'edit_field'
  | 'add_port'
  | 'remove_port';

const DISABLED_REASON: Record<NodeLabAuthoringControl, string> = {
  new_node: 'Creating nodes needs the Authoring API, which this build does not include.',
  save_draft: 'Saving drafts needs the Authoring API, which this build does not include.',
  publish:
    'Publishing needs Runtime Publishing, which this build does not include. Built-in nodes are already available in Graph.',
  try: 'Trying a node outside a workflow needs the Local Agent, which this build does not include.',
  edit_field: 'Built-in nodes are read-only, and this build has no Authoring API to edit definitions.',
  add_port: 'Built-in nodes are read-only, and this build has no Authoring API to add ports.',
  remove_port: 'Built-in nodes are read-only, and this build has no Authoring API to remove ports.',
};

export function nodeLabDisabledReason(control: NodeLabAuthoringControl): string {
  return DISABLED_REASON[control];
}

/** Palette order (NodeCategory); categories the frontend does not know sort last. */
export const NODE_LAB_CATEGORIES: readonly NodeCategory[] = [
  'input',
  'processing',
  'tool',
  'output',
  'control',
];

export function nodeLabCategoryLabel(category: string): string {
  return category.length === 0 ? 'Uncategorised' : category.charAt(0).toUpperCase() + category.slice(1);
}

const EXECUTION_MODE_LABEL: Record<ExecutionMode, string> = {
  reactive: 'Reactive',
  manual: 'Manual',
  stream: 'Stream',
};

const IO_BINDING_LABEL: Record<IoBindingOrigin, string> = {
  client_session: 'Client session',
  integrated: 'Integrated',
};

export const NODE_LAB_EXECUTION_MODES = Object.keys(EXECUTION_MODE_LABEL) as ExecutionMode[];
export const NODE_LAB_IO_BINDINGS = Object.keys(IO_BINDING_LABEL) as IoBindingOrigin[];

export function nodeLabExecutionModeLabel(mode: string): string {
  return EXECUTION_MODE_LABEL[mode as ExecutionMode] ?? mode;
}

export function nodeLabIoBindingLabel(origin: string): string {
  return IO_BINDING_LABEL[origin as IoBindingOrigin] ?? origin;
}

export const NODE_LAB_IMPLEMENTATION_TEXT = 'built-in, source not editable';

function categoryRank(category: string): number {
  const index = NODE_LAB_CATEGORIES.indexOf(category as NodeCategory);
  return index === -1 ? NODE_LAB_CATEGORIES.length : index;
}

/** Definitions in palette category order, then by label. Does not mutate the input. */
export function sortNodeLabDefinitions(definitions: readonly NodeDefinition[]): NodeDefinition[] {
  return [...definitions].sort(
    (a, b) =>
      categoryRank(a.category) - categoryRank(b.category) ||
      a.category.localeCompare(b.category) ||
      a.label.localeCompare(b.label) ||
      a.node_type.localeCompare(b.node_type),
  );
}

function portsOf(ports: readonly PortDefinition[] | undefined): readonly PortDefinition[] {
  return Array.isArray(ports) ? ports : [];
}

function countText(count: number, one: string, many: string): string {
  return `${count} ${count === 1 ? one : many}`;
}

export interface NodeLabListRow {
  nodeType: string;
  label: string;
  categoryLabel: string;
  /** Text after the category, for example ", 3 inputs, 2 outputs". */
  context: string;
}

export interface NodeLabListGroup {
  id: 'built_in';
  label: string;
  rows: NodeLabListRow[];
}

export interface NodeLabList {
  total: number;
  matched: number;
  /** One "Built-in" group; "Your nodes" waits for a definition store. */
  groups: NodeLabListGroup[];
  /** Muted sentence when the list is empty, otherwise null. */
  emptyText: string | null;
}

function matchesQuery(definition: NodeDefinition, query: string): boolean {
  if (query.length === 0) {
    return true;
  }
  return [definition.label, definition.node_type, definition.category, definition.description]
    .some((value) => typeof value === 'string' && value.toLowerCase().includes(query));
}

export function buildNodeLabList(definitions: readonly NodeDefinition[], query: string): NodeLabList {
  const normalized = query.trim().toLowerCase();
  const sorted = sortNodeLabDefinitions(definitions);
  const rows = sorted
    .filter((definition) => matchesQuery(definition, normalized))
    .map((definition) => ({
      nodeType: definition.node_type,
      label: definition.label,
      categoryLabel: nodeLabCategoryLabel(definition.category),
      context: `, ${countText(portsOf(definition.inputs).length, 'input', 'inputs')}, ${countText(
        portsOf(definition.outputs).length,
        'output',
        'outputs',
      )}`,
    }));
  let emptyText: string | null = null;
  if (definitions.length === 0) {
    emptyText = 'No built-in node definitions are loaded.';
  } else if (rows.length === 0) {
    emptyText = `No built-in node matches “${query.trim()}”.`;
  }
  return {
    total: definitions.length,
    matched: rows.length,
    groups: rows.length > 0 ? [{ id: 'built_in', label: 'Built-in', rows }] : [],
    emptyText,
  };
}

/**
 * The definition the centre and right columns show: the selected type when the
 * engine still reports it, otherwise the first definition in list order, or
 * null when there are none. Selection is resolved against all definitions, so a
 * search that hides the selected row does not change the detail.
 */
export function resolveNodeLabSelection(
  definitions: readonly NodeDefinition[],
  selectedNodeType: string | null,
): NodeDefinition | null {
  if (selectedNodeType !== null) {
    const match = definitions.find((definition) => definition.node_type === selectedNodeType);
    if (match) {
      return match;
    }
  }
  return sortNodeLabDefinitions(definitions)[0] ?? null;
}

export function nodeLabSummaryLine(definitions: readonly NodeDefinition[]): string {
  if (definitions.length === 0) {
    return 'No node definitions loaded';
  }
  const categories = new Set(definitions.map((definition) => definition.category)).size;
  return `${countText(definitions.length, 'built-in node', 'built-in nodes')} in ${countText(
    categories,
    'category',
    'categories',
  )}, read-only`;
}

/** Display text for a port default; empty when the definition declares none. */
export function formatNodeLabDefault(value: unknown): string {
  if (value === undefined || value === null) {
    return '';
  }
  if (typeof value === 'string') {
    return value;
  }
  try {
    return JSON.stringify(value) ?? String(value);
  } catch {
    return String(value);
  }
}

export interface NodeLabPortRow {
  id: string;
  label: string;
  /** The port id when it reads differently from the label (wires use the id), otherwise null. */
  idNote: string | null;
  dataType: string;
  /** Inputs only: the declared default, empty when none. */
  defaultText: string;
  /** Inputs only: "required" or "optional", plus ", accepts many" for multi-connection ports. */
  requirement: string;
  description: string | null;
}

function comparable(text: string): string {
  return text.toLowerCase().replace(/[\s_-]+/g, '');
}

function projectPort(port: PortDefinition): NodeLabPortRow {
  const label = port.label || port.id;
  return {
    id: port.id,
    label,
    idNote: comparable(label) === comparable(port.id) ? null : port.id,
    dataType: port.data_type,
    defaultText: formatNodeLabDefault(port.default_value),
    requirement: `${port.required ? 'required' : 'optional'}${port.multiple ? ', accepts many' : ''}`,
    description: port.description?.trim() ? port.description.trim() : null,
  };
}

export interface NodeLabCheck {
  glyph: 'succeeded';
  text: string;
}

export interface NodeLabDefinitionView {
  nodeType: string;
  label: string;
  category: string;
  categoryLabel: string;
  description: string;
  executionMode: string;
  executionModeLabel: string;
  ioBindingOrigin: string;
  ioBindingLabel: string;
  inputs: NodeLabPortRow[];
  outputs: NodeLabPortRow[];
  implementationText: string;
  checks: NodeLabCheck[];
}

export function projectNodeLabDefinition(definition: NodeDefinition): NodeLabDefinitionView {
  return {
    nodeType: definition.node_type,
    label: definition.label,
    category: definition.category,
    categoryLabel: nodeLabCategoryLabel(definition.category),
    description: definition.description?.trim() ?? '',
    executionMode: definition.execution_mode,
    executionModeLabel: nodeLabExecutionModeLabel(definition.execution_mode),
    ioBindingOrigin: definition.io_binding_origin,
    ioBindingLabel: nodeLabIoBindingLabel(definition.io_binding_origin),
    inputs: portsOf(definition.inputs).map(projectPort),
    outputs: portsOf(definition.outputs).map(projectPort),
    implementationText: NODE_LAB_IMPLEMENTATION_TEXT,
    checks: [{ glyph: 'succeeded', text: 'Built-in node, always valid' }],
  };
}
