import { invoke, Channel } from '@tauri-apps/api/core';
import { engine } from '../DrawingEngine';
import { canvasExport } from '../CanvasExport';
import { Logger } from '../Logger';
import { LLMService } from '../LLMService';
import { calculateBounds, findTargetComponent, type ComponentPosition } from '../DrawingAnalyzer';
import { ActivityLogger } from './ActivityLogger';
import { StreamHandler } from './StreamHandler';
import type {
  AgentRequest,
  AgentResponse,
  AgentEvent,
  AgentState,
  AgentStateListener,
  AgentEventListener,
} from './types';

// Re-export types for consumers
export type {
  AgentRequest,
  AgentResponse,
  AgentEvent,
  AgentState,
  AgentActivityItem,
  ComponentUpdate,
  FileChange,
  ComponentInfo,
} from './types';

class AgentServiceClass {
  private state: AgentState = {
    isRunning: false,
    currentMessage: '',
    streamingText: '',
    streamingReasoning: '',
    activityLog: [],
    error: null,
    lastResponse: null,
  };

  private abortRequested = false;
  private stateListeners: AgentStateListener[] = [];
  private componentRegistry: ComponentPosition[] = [];

  private activityLogger = new ActivityLogger();
  private streamHandler = new StreamHandler(this.activityLogger);

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
    return this.streamHandler.subscribeEvents(callback);
  }

  private notifyState() {
    this.stateListeners.forEach((l) => l({ ...this.state }));
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

    // Reset abort flag
    this.abortRequested = false;

    // Clear streaming tool calls tracker
    this.activityLogger.clearStreamingToolCalls();

    this.state = {
      isRunning: true,
      currentMessage: '',
      streamingText: '',
      streamingReasoning: '',
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
        this.state = this.streamHandler.handleEvent(event, this.state);
        this.notifyState();
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

      this.state = this.activityLogger.addActivityItem(this.state, 'error', errorMessage);
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

  /**
   * Clear the activity log
   */
  public clearActivityLog(): void {
    this.state = this.activityLogger.clearActivityLog(this.state);
    this.notifyState();
  }

  /**
   * Stop the currently running agent
   * Preserves partial results in activity log
   */
  public stop(): void {
    if (!this.state.isRunning) {
      return;
    }

    Logger.log('agent_stop_requested', {});
    this.abortRequested = true;

    // Add status message to activity log
    this.state = this.activityLogger.addActivityItem(this.state, 'status', 'Agent stopped by user');

    // Preserve any streaming text as a partial result
    if (this.state.streamingText) {
      this.state = this.activityLogger.addActivityItem(
        this.state,
        'text',
        this.state.streamingText + ' [interrupted]'
      );
      this.state = { ...this.state, streamingText: '' };
    }

    // Preserve any streaming reasoning as a partial result
    if (this.state.streamingReasoning) {
      this.state = this.activityLogger.addActivityItem(
        this.state,
        'reasoning',
        this.state.streamingReasoning + ' [interrupted]'
      );
      this.state = { ...this.state, streamingReasoning: '' };
    }

    // Clear streaming tool calls
    this.activityLogger.clearStreamingToolCalls();

    // Update state to stopped
    this.state = {
      ...this.state,
      isRunning: false,
      currentMessage: 'Stopped',
      error: null,
    };
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
