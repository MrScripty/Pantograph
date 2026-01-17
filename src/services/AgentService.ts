import { invoke, Channel } from '@tauri-apps/api/core';
import { engine } from './DrawingEngine';
import { canvasExport } from './CanvasExport';
import { Logger } from './Logger';
import { LLMService } from './LLMService';
import {
  calculateBounds,
  findTargetComponent,
  type DrawingBounds,
  type ComponentPosition,
} from './DrawingAnalyzer';

// Types matching the Rust backend
export interface AgentRequest {
  prompt: string;
  image_base64: string;
  drawing_bounds: DrawingBounds | null;
  component_tree: ComponentInfo[];
  target_element_id: string | null;
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
  event_type: 'tool_call' | 'tool_result' | 'content' | 'component_created' | 'done' | 'error';
  data: unknown;
}

// Detailed event data types from the backend
export interface ContentEventData {
  type?: 'system_prompt' | 'text_chunk' | 'reasoning';
  message?: string;
  prompt?: string;
  chunk?: string;
  text?: string;
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
  type: 'system_prompt' | 'text' | 'tool_call' | 'tool_result' | 'reasoning' | 'status' | 'error';
  timestamp: number;
  content: string;
  metadata?: {
    toolName?: string;
    toolArgs?: string;
    toolId?: string;
    status?: 'pending' | 'success' | 'error';
    errorMessage?: string;
  };
}

interface AgentState {
  isRunning: boolean;
  currentMessage: string;
  streamingText: string;
  activityLog: AgentActivityItem[];
  error: string | null;
  lastResponse: AgentResponse | null;
}

type AgentStateListener = (state: AgentState) => void;
type AgentEventListener = (event: AgentEvent) => void;

class AgentServiceClass {
  private state: AgentState = {
    isRunning: false,
    currentMessage: '',
    streamingText: '',
    activityLog: [],
    error: null,
    lastResponse: null,
  };

  private activityIdCounter = 0;

  private generateActivityId(): string {
    return `activity-${Date.now()}-${this.activityIdCounter++}`;
  }

  private addActivityItem(
    type: AgentActivityItem['type'],
    content: string,
    metadata?: AgentActivityItem['metadata']
  ): void {
    const item: AgentActivityItem = {
      id: this.generateActivityId(),
      type,
      timestamp: Date.now(),
      content,
      metadata,
    };
    this.state.activityLog = [...this.state.activityLog, item];
    this.notifyState();
  }

  private stateListeners: AgentStateListener[] = [];
  private eventListeners: AgentEventListener[] = [];
  private componentRegistry: ComponentPosition[] = [];

  /**
   * Subscribe to state changes
   */
  public subscribeState(callback: AgentStateListener): () => void {
    this.stateListeners.push(callback);
    callback({ ...this.state });
    return () => {
      this.stateListeners = this.stateListeners.filter((l) => l !== callback);
    };
  }

  /**
   * Subscribe to agent events (tool calls, content, etc.)
   */
  public subscribeEvents(callback: AgentEventListener): () => void {
    this.eventListeners.push(callback);
    return () => {
      this.eventListeners = this.eventListeners.filter((l) => l !== callback);
    };
  }

  private notifyState() {
    this.stateListeners.forEach((l) => l({ ...this.state }));
  }

  private notifyEvent(event: AgentEvent) {
    this.eventListeners.forEach((l) => l(event));
  }

  /**
   * Register a component's position for target detection
   */
  public registerComponent(component: ComponentPosition) {
    const existing = this.componentRegistry.findIndex((c) => c.id === component.id);
    if (existing >= 0) {
      this.componentRegistry[existing] = component;
    } else {
      this.componentRegistry.push(component);
    }
  }

  /**
   * Unregister a component
   */
  public unregisterComponent(id: string) {
    this.componentRegistry = this.componentRegistry.filter((c) => c.id !== id);
  }

  /**
   * Get current component registry
   */
  public getComponentRegistry(): ComponentPosition[] {
    return [...this.componentRegistry];
  }

  /**
   * Run the agent with the current drawing and a prompt
   */
  public async run(prompt: string): Promise<AgentResponse> {
    Logger.log('agent_run_called', { promptLength: prompt.length });

    if (this.state.isRunning) {
      throw new Error('Agent is already running');
    }

    // Check if LLM is ready
    if (!LLMService.isReady) {
      Logger.log('agent_llm_not_ready', {}, 'error');
      throw new Error('LLM not connected. Please connect to an LLM server first.');
    }

    Logger.log('agent_starting', {});

    this.state = {
      isRunning: true,
      currentMessage: '',
      streamingText: '',
      activityLog: [],
      error: null,
      lastResponse: null,
    };
    this.notifyState();

    try {
      // Get the current drawing state
      const drawingState = engine.getState();
      Logger.log('agent_drawing_state', { strokeCount: drawingState.strokes.length });

      const imageBase64 = canvasExport.exportToBase64();
      Logger.log('agent_canvas_exported', { base64Length: imageBase64?.length ?? 0 });

      if (!imageBase64) {
        throw new Error('Failed to export canvas');
      }

      // Calculate drawing bounds
      const drawingBounds = calculateBounds(drawingState.strokes);

      // Find target element if drawing overlaps existing components
      const targetElementId = findTargetComponent(drawingState.strokes, this.componentRegistry);

      // Build the request
      const request: AgentRequest = {
        prompt,
        image_base64: imageBase64,
        drawing_bounds: drawingBounds,
        component_tree: this.componentRegistry.map((c) => ({
          id: c.id,
          name: c.name,
          path: c.path,
          bounds: c.bounds,
        })),
        target_element_id: targetElementId,
      };

      Logger.log('agent_request', {
        promptLength: prompt.length,
        hasDrawing: drawingState.strokes.length > 0,
        drawingBounds,
        targetElementId,
        imageBase64Length: imageBase64.length,
      });

      // Create channel for streaming events
      const channel = new Channel<AgentEvent>();

      channel.onmessage = (event: AgentEvent) => {
        this.handleAgentEvent(event);
      };

      // Invoke the backend agent
      Logger.log('agent_invoking_backend', {});
      const response = await invoke<AgentResponse>('run_agent', {
        request,
        channel,
      });
      Logger.log('agent_backend_response', {
        filesChanged: response.file_changes.length,
        componentsUpdated: response.component_updates.length
      });

      // Success - clear the drawing
      engine.clearStrokes();

      this.state = {
        ...this.state,
        isRunning: false,
        currentMessage: response.message,
        error: null,
        lastResponse: response,
      };
      this.notifyState();

      Logger.log('agent_success', {
        filesChanged: response.file_changes.length,
        componentsUpdated: response.component_updates.length,
      });

      return response;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);

      this.addActivityItem('error', errorMessage);
      this.state = {
        ...this.state,
        isRunning: false,
        currentMessage: '',
        error: errorMessage,
        lastResponse: null,
      };
      this.notifyState();

      Logger.log('agent_error', { error: errorMessage }, 'error');
      throw error;
    }
  }

  private handleAgentEvent(event: AgentEvent) {
    this.notifyEvent(event);
    const data = event.data as ContentEventData | ToolCallEventData | ToolResultEventData | null;

    switch (event.event_type) {
      case 'content':
        if (data && typeof data === 'object') {
          const contentData = data as ContentEventData;

          if (contentData.type === 'system_prompt' && contentData.prompt) {
            // Add system prompt to activity log
            this.addActivityItem('system_prompt', contentData.prompt);
          } else if (contentData.type === 'text_chunk' && contentData.chunk) {
            // Append streaming text
            this.state.streamingText += contentData.chunk;
            this.notifyState();
          } else if (contentData.type === 'reasoning' && contentData.text) {
            // Add reasoning to activity log
            this.addActivityItem('reasoning', contentData.text);
          } else if (contentData.message) {
            // Status message
            this.state.currentMessage = contentData.message;
            this.addActivityItem('status', contentData.message);
          }
        }
        break;

      case 'tool_call':
        // Flush any accumulated streaming text before the tool call
        // This shows the agent's response/reasoning leading up to this tool call
        if (this.state.streamingText) {
          this.addActivityItem('text', this.state.streamingText);
          this.state.streamingText = '';
        }
        if (data && typeof data === 'object') {
          const toolData = data as ToolCallEventData;
          this.addActivityItem('tool_call', `Calling ${toolData.name}`, {
            toolName: toolData.name,
            toolArgs: toolData.arguments,
            status: 'pending',
          });
        }
        break;

      case 'tool_result':
        // Instead of adding a separate item, update the corresponding tool call's status
        if (data && typeof data === 'object') {
          const resultData = data as ToolResultEventData;
          // Find the most recent pending tool_call and update its status (search from end)
          let toolCallIndex = -1;
          for (let i = this.state.activityLog.length - 1; i >= 0; i--) {
            const item = this.state.activityLog[i];
            if (item.type === 'tool_call' && item.metadata?.status === 'pending') {
              toolCallIndex = i;
              break;
            }
          }
          if (toolCallIndex !== -1) {
            const isSuccess = resultData.output === 'true';
            this.state.activityLog[toolCallIndex].metadata!.status = isSuccess ? 'success' : 'error';
            if (!isSuccess) {
              this.state.activityLog[toolCallIndex].metadata!.errorMessage = resultData.output;
            }
            this.notifyState();
          }
        }
        break;

      case 'error':
        if (data && typeof data === 'object' && 'error' in data) {
          const errorMsg = String((data as { error: string }).error);
          this.state.error = errorMsg;
          this.addActivityItem('error', errorMsg);
        }
        break;

      case 'done':
        // Finalize streaming text as the last text activity item
        if (this.state.streamingText) {
          this.addActivityItem('text', this.state.streamingText);
          this.state.streamingText = '';
        }
        Logger.log('agent_done', {});
        break;

      default:
        Logger.log('agent_event', { type: event.event_type });
    }
  }

  /**
   * Clear the activity log
   */
  public clearActivityLog(): void {
    this.state.activityLog = [];
    this.state.streamingText = '';
    this.notifyState();
  }

  /**
   * Get current state
   */
  public getState(): AgentState {
    return { ...this.state };
  }
}

export const AgentService = new AgentServiceClass();
