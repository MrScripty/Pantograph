import type { DrawingBounds, ComponentPosition } from '../DrawingAnalyzer';

// Types matching the Rust backend
export interface AgentRequest {
  prompt: string;
  image_base64: string;
  drawing_bounds: DrawingBounds | null;
  component_tree: ComponentInfo[];
  target_element_id: string | null;
  /** Path to the component being edited (for edit mode) */
  target_component_path: string | null;

  // Fix mode fields - skip vision analysis and use pre-loaded context
  /** Skip vision analysis, go directly to agent with error context */
  fix_mode?: boolean;
  /** Pre-loaded file content (avoids read tool call) */
  file_content?: string;
  /** Error that triggered fix mode (enriched with docs) */
  error_context?: string;
  /** Explicit target file path for write */
  target_file_path?: string;
}

export interface ComponentInfo {
  id: string;
  name: string;
  path: string;
  bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

export interface AgentResponse {
  file_changes: FileChange[];
  component_updates: ComponentUpdate[];
  message: string;
}

export interface FileChange {
  path: string;
  action: 'create' | 'update' | 'delete';
  content: string | null;
}

export interface ComponentUpdate {
  id: string;
  path: string;
  position: { x: number; y: number };
  size: { width: number; height: number };
  source: string;
}

export interface AgentEvent {
  event_type: 'tool_call' | 'tool_call_delta' | 'tool_result' | 'content' | 'component_created' | 'done' | 'error';
  data: unknown;
}

// Detailed event data types from the backend
export interface ContentEventData {
  type?: 'system_prompt' | 'text_chunk' | 'reasoning' | 'reasoning_delta';
  message?: string;
  prompt?: string;
  chunk?: string;
  text?: string;
  id?: string;
}

export interface ToolCallDeltaEventData {
  id: string;
  content: {
    type: 'name' | 'delta';
    value: string;
  };
}

export interface ToolCallEventData {
  name: string;
  arguments: string;
}

export interface ToolResultEventData {
  tool_id: string;
  output: string;
}

// Activity item for UI display
export interface AgentActivityItem {
  id: string;
  type: 'system_prompt' | 'text' | 'tool_call' | 'tool_call_streaming' | 'tool_result' | 'reasoning' | 'reasoning_streaming' | 'status' | 'error';
  timestamp: number;
  content: string;
  metadata?: {
    toolName?: string;
    toolArgs?: string;
    toolId?: string;
    status?: 'pending' | 'success' | 'error' | 'streaming';
    errorMessage?: string;
    streamingId?: string; // ID for tracking streaming tool calls
  };
}

// Track streaming tool call state
export interface StreamingToolCall {
  id: string;
  name: string;
  arguments: string;
  activityId: string; // Reference to the activity log item
}

export interface AgentState {
  isRunning: boolean;
  currentMessage: string;
  streamingText: string;
  streamingReasoning: string;
  activityLog: AgentActivityItem[];
  error: string | null;
  lastResponse: AgentResponse | null;
}

export type AgentStateListener = (state: AgentState) => void;
export type AgentEventListener = (event: AgentEvent) => void;
