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

interface AgentState {
  isRunning: boolean;
  currentMessage: string;
  error: string | null;
  lastResponse: AgentResponse | null;
}

type AgentStateListener = (state: AgentState) => void;
type AgentEventListener = (event: AgentEvent) => void;

class AgentServiceClass {
  private state: AgentState = {
    isRunning: false,
    currentMessage: '',
    error: null,
    lastResponse: null,
  };

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
    console.log('[AgentService] run() called with prompt:', prompt);

    if (this.state.isRunning) {
      throw new Error('Agent is already running');
    }

    // Check if LLM is ready
    if (!LLMService.isReady) {
      console.error('[AgentService] LLM not ready');
      throw new Error('LLM not connected. Please connect to an LLM server first.');
    }

    console.log('[AgentService] LLM is ready, starting agent...');

    this.state = {
      isRunning: true,
      currentMessage: '',
      error: null,
      lastResponse: null,
    };
    this.notifyState();

    try {
      // Get the current drawing state
      const drawingState = engine.getState();
      console.log('[AgentService] Drawing state:', { strokeCount: drawingState.strokes.length });

      const imageBase64 = canvasExport.exportToBase64();
      console.log('[AgentService] Canvas exported, base64 length:', imageBase64?.length ?? 0);

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

      Logger.log('AGENT_REQUEST', {
        prompt,
        hasDrawing: drawingState.strokes.length > 0,
        drawingBounds,
        targetElementId,
      });

      console.log('[AgentService] Request built:', {
        prompt,
        hasDrawing: drawingState.strokes.length > 0,
        drawingBounds,
        targetElementId,
        imageBase64Length: imageBase64.length,
      });

      // Create channel for streaming events
      const channel = new Channel<AgentEvent>();

      channel.onmessage = (event: AgentEvent) => {
        console.log('[AgentService] Received event:', event);
        this.handleAgentEvent(event);
      };

      // Invoke the backend agent
      console.log('[AgentService] Invoking run_agent command...');
      const response = await invoke<AgentResponse>('run_agent', {
        request,
        channel,
      });
      console.log('[AgentService] Backend response:', response);

      // Success - clear the drawing
      engine.clearStrokes();

      this.state = {
        isRunning: false,
        currentMessage: response.message,
        error: null,
        lastResponse: response,
      };
      this.notifyState();

      Logger.log('AGENT_SUCCESS', {
        filesChanged: response.file_changes.length,
        componentsUpdated: response.component_updates.length,
      });

      return response;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);

      this.state = {
        isRunning: false,
        currentMessage: '',
        error: errorMessage,
        lastResponse: null,
      };
      this.notifyState();

      Logger.log('AGENT_ERROR', { error: errorMessage }, 'error');
      throw error;
    }
  }

  private handleAgentEvent(event: AgentEvent) {
    this.notifyEvent(event);

    switch (event.event_type) {
      case 'content':
        if (event.data && typeof event.data === 'object' && 'message' in event.data) {
          this.state.currentMessage = String((event.data as { message: string }).message);
          this.notifyState();
        }
        break;
      case 'error':
        if (event.data && typeof event.data === 'object' && 'error' in event.data) {
          this.state.error = String((event.data as { error: string }).error);
          this.notifyState();
        }
        break;
      case 'done':
        Logger.log('AGENT_DONE', event.data);
        break;
      default:
        Logger.log('AGENT_EVENT', { type: event.event_type, data: event.data });
    }
  }

  /**
   * Get current state
   */
  public getState(): AgentState {
    return { ...this.state };
  }
}

export const AgentService = new AgentServiceClass();
