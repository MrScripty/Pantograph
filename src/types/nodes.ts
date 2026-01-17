// Node types for the agent workflow graph

export interface UserInputNodeData {
  label: string;
  imagePreview?: string;
  promptText?: string;
}

export interface SystemPromptNodeData {
  label: string;
  promptPreview?: string;
  isEditing?: boolean;
  onEdit?: () => void;
}

export interface ToolsNodeData {
  label: string;
  tools?: Array<{
    name: string;
    description: string;
    enabled: boolean;
  }>;
}

export interface AgentNodeData {
  label: string;
  modelName?: string;
  maxTurns?: number;
  status?: 'idle' | 'running' | 'success' | 'error';
}

export interface OutputNodeData {
  label: string;
  lastOutput?: string;
  componentPath?: string;
}

export type NodeData =
  | UserInputNodeData
  | SystemPromptNodeData
  | ToolsNodeData
  | AgentNodeData
  | OutputNodeData;
