import { Logger } from '../Logger';
import { ActivityLogger } from './ActivityLogger';
import type {
  AgentEvent,
  AgentState,
  ContentEventData,
  ToolCallEventData,
  ToolCallDeltaEventData,
  ToolResultEventData,
  AgentEventListener,
} from './types';

/**
 * Handles streaming events from the agent backend.
 * Parses events and updates state accordingly.
 */
export class StreamHandler {
  private activityLogger: ActivityLogger;
  private eventListeners: AgentEventListener[] = [];

  constructor(activityLogger: ActivityLogger) {
    this.activityLogger = activityLogger;
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

  /**
   * Notify all event listeners
   */
  private notifyEvent(event: AgentEvent): void {
    this.eventListeners.forEach((l) => l(event));
  }

  /**
   * Handle an agent event and return updated state
   */
  public handleEvent(event: AgentEvent, state: AgentState): AgentState {
    this.notifyEvent(event);
    const data = event.data as ContentEventData | ToolCallEventData | ToolCallDeltaEventData | ToolResultEventData | null;

    switch (event.event_type) {
      case 'content':
        return this.handleContentEvent(data as ContentEventData, state);

      case 'tool_call_delta':
        return this.handleToolCallDeltaEvent(data as ToolCallDeltaEventData, state);

      case 'tool_call':
        return this.handleToolCallEvent(data as ToolCallEventData, state);

      case 'tool_result':
        return this.handleToolResultEvent(data as ToolResultEventData, state);

      case 'component_created':
        Logger.log('agent_component_created', { data });
        this.notifyEvent(event);
        return state;

      case 'error':
        return this.handleErrorEvent(data, state);

      case 'done':
        return this.handleDoneEvent(state);

      default:
        Logger.log('agent_event', { type: event.event_type });
        return state;
    }
  }

  private handleContentEvent(data: ContentEventData | null, state: AgentState): AgentState {
    if (!data || typeof data !== 'object') {
      return state;
    }

    if (data.type === 'system_prompt' && data.prompt) {
      return this.activityLogger.addActivityItem(state, 'system_prompt', data.prompt);
    }

    if (data.type === 'text_chunk' && data.chunk) {
      return {
        ...state,
        streamingText: state.streamingText + data.chunk,
      };
    }

    if (data.type === 'reasoning' && data.text) {
      // Complete reasoning block - flush any streaming reasoning first
      if (state.streamingReasoning) {
        state = this.activityLogger.addActivityItem(state, 'reasoning', state.streamingReasoning);
        state = { ...state, streamingReasoning: '' };
      }
      return this.activityLogger.addActivityItem(state, 'reasoning', data.text);
    }

    if (data.type === 'reasoning_delta' && data.text) {
      return {
        ...state,
        streamingReasoning: state.streamingReasoning + data.text,
      };
    }

    if (data.message) {
      state = { ...state, currentMessage: data.message };
      return this.activityLogger.addActivityItem(state, 'status', data.message);
    }

    return state;
  }

  private handleToolCallDeltaEvent(data: ToolCallDeltaEventData | null, state: AgentState): AgentState {
    if (!data || typeof data !== 'object') {
      return state;
    }

    const { id, content } = data;
    const { streamingTool, state: newState } = this.activityLogger.getOrCreateStreamingToolCall(id, state);

    return this.activityLogger.updateStreamingToolCall(
      streamingTool,
      content.type,
      content.value,
      newState
    );
  }

  private handleToolCallEvent(data: ToolCallEventData | null, state: AgentState): AgentState {
    // Flush any accumulated streaming text before the tool call
    state = this.activityLogger.flushStreamingText(state);
    state = this.activityLogger.flushStreamingReasoning(state);

    if (!data || typeof data !== 'object') {
      return state;
    }

    return this.activityLogger.finalizeStreamingToolCall(data.name, data.arguments, state);
  }

  private handleToolResultEvent(data: ToolResultEventData | null, state: AgentState): AgentState {
    if (!data || typeof data !== 'object') {
      return state;
    }

    return this.activityLogger.updateToolCallStatus(data.output, state);
  }

  private handleErrorEvent(data: unknown, state: AgentState): AgentState {
    if (data && typeof data === 'object' && 'error' in data) {
      const errorMsg = String((data as { error: string }).error);
      state = { ...state, error: errorMsg };
      return this.activityLogger.addActivityItem(state, 'error', errorMsg);
    }
    return state;
  }

  private handleDoneEvent(state: AgentState): AgentState {
    // Finalize streaming text as the last text activity item
    state = this.activityLogger.flushStreamingText(state);
    state = this.activityLogger.flushStreamingReasoning(state);
    this.activityLogger.clearStreamingToolCalls();
    Logger.log('agent_done', {});
    return state;
  }
}
