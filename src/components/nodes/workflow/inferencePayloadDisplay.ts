import type {
  InferencePortPayloadContract,
  InferencePortPayloadRole,
  InferenceTaskId,
  NodeDefinition,
  PortDefinition,
} from '../../../services/workflow/types';

export interface InferencePayloadDisplayRow {
  label: string;
  value: string;
}

export interface InferencePayloadDisplay {
  tasks: string[];
  rows: InferencePayloadDisplayRow[];
}

type PortDirection = 'input' | 'output';

const TASK_LABELS: Record<InferenceTaskId, string> = {
  text_generation: 'Text',
  chat_completion: 'Chat',
  embedding: 'Embedding',
  rerank: 'Rerank',
  image_generation: 'Image',
  image_understanding: 'Vision',
  audio_transcription: 'Audio',
  video_understanding: 'Video',
  multimodal_generation: 'Multimodal',
  unknown: 'Unknown',
};

const ROLE_LABELS: Record<InferencePortPayloadRole, string> = {
  task_input: 'Task inputs',
  task_output: 'Task outputs',
  model_reference: 'Model facts',
  options: 'Options',
  diagnostics: 'Diagnostics',
  usage: 'Usage',
};

const ROLE_ORDER: InferencePortPayloadRole[] = [
  'model_reference',
  'options',
  'diagnostics',
  'usage',
  'task_input',
  'task_output',
];

export function buildInferencePayloadDisplay(
  definition: NodeDefinition | undefined,
): InferencePayloadDisplay | null {
  if (!definition) {
    return null;
  }

  const tasks = new Set<string>();
  const rolePorts = new Map<InferencePortPayloadRole, Set<string>>();

  collectPorts(definition.inputs, 'input', tasks, rolePorts);
  collectPorts(definition.outputs, 'output', tasks, rolePorts);

  const taskLabels = [...tasks].sort(compareLabels);
  const rows = ROLE_ORDER.flatMap((role) => {
    const ports = rolePorts.get(role);
    if (!ports || ports.size === 0) {
      return [];
    }

    return [{ label: ROLE_LABELS[role], value: [...ports].sort(compareLabels).join(', ') }];
  });

  if (taskLabels.length === 0 && rows.length === 0) {
    return null;
  }

  return { tasks: taskLabels, rows };
}

function collectPorts(
  ports: PortDefinition[],
  direction: PortDirection,
  tasks: Set<string>,
  rolePorts: Map<InferencePortPayloadRole, Set<string>>,
): void {
  for (const port of ports) {
    const payloads = port.inference_payloads ?? [];
    for (const payload of payloads) {
      const taskLabel = taskLabelFor(payload.task_id);
      if (taskLabel) {
        tasks.add(taskLabel);
      }

      if (isDisplayRole(payload.role, direction)) {
        const portsForRole = rolePorts.get(payload.role) ?? new Set<string>();
        portsForRole.add(port.label || port.id);
        rolePorts.set(payload.role, portsForRole);
      }
    }
  }
}

function isDisplayRole(role: InferencePortPayloadContract['role'], direction: PortDirection): boolean {
  if (direction === 'input') {
    return role === 'model_reference' || role === 'options';
  }

  return role === 'model_reference' || role === 'diagnostics' || role === 'usage';
}

function taskLabelFor(taskId: InferenceTaskId): string | null {
  const label = TASK_LABELS[taskId];
  if (!label || label === TASK_LABELS.unknown) {
    return null;
  }

  return label;
}

function compareLabels(left: string, right: string): number {
  return left.localeCompare(right, undefined, { sensitivity: 'base' });
}
