import type { AgentActivityItem, AgentState, StreamingToolCall } from './types';

/**
 * Manages the activity log for agent operations.
 * Handles adding, updating, and clearing activity items.
 */
export class ActivityLogger {
  private activityIdCounter = 0;
  private streamingToolCalls: Map<string, StreamingToolCall> = new Map();

  /**
   * Generate a unique activity ID
   */
  public generateActivityId(): string {
    return `activity-${Date.now()}-${this.activityIdCounter++}`;
  }

  /**
   * Add a new activity item to the log
   */
  public addActivityItem(
    state: AgentState,
    type: AgentActivityItem['type'],
    content: string,
    metadata?: AgentActivityItem['metadata']
  ): AgentState {
    const item: AgentActivityItem = {
      id: this.generateActivityId(),
      type,
      timestamp: Date.now(),
      content,
      metadata,
    };
    return {
      ...state,
      activityLog: [...state.activityLog, item],
    };
  }

  /**
   * Get or create a streaming tool call entry
   */
  public getOrCreateStreamingToolCall(
    id: string,
    state: AgentState
  ): { streamingTool: StreamingToolCall; state: AgentState; isNew: boolean } {
    let streamingTool = this.streamingToolCalls.get(id);
    let isNew = false;

    if (!streamingTool) {
      isNew = true;
      const activityId = this.generateActivityId();
      streamingTool = {
        id,
        name: '',
        arguments: '',
        activityId,
      };
      this.streamingToolCalls.set(id, streamingTool);

      // Add a streaming tool call activity item
      const item: AgentActivityItem = {
        id: activityId,
        type: 'tool_call_streaming',
        timestamp: Date.now(),
        content: 'Generating tool call...',
        metadata: {
          toolName: '',
          toolArgs: '',
          status: 'streaming',
          streamingId: id,
        },
      };
      state = {
        ...state,
        activityLog: [...state.activityLog, item],
      };
    }

    return { streamingTool, state, isNew };
  }

  /**
   * Update a streaming tool call with delta content
   */
  public updateStreamingToolCall(
    streamingTool: StreamingToolCall,
    contentType: 'name' | 'delta',
    value: string,
    state: AgentState
  ): AgentState {
    if (contentType === 'name') {
      streamingTool.name += value;
    } else if (contentType === 'delta') {
      streamingTool.arguments += value;
    }

    // Update the activity item
    const activityIndex = state.activityLog.findIndex(
      (item) => item.id === streamingTool.activityId
    );
    if (activityIndex !== -1) {
      const newActivityLog = [...state.activityLog];
      newActivityLog[activityIndex] = {
        ...newActivityLog[activityIndex],
        content: streamingTool.name ? `Calling ${streamingTool.name}...` : 'Generating tool call...',
        metadata: {
          ...newActivityLog[activityIndex].metadata,
          toolName: streamingTool.name,
          toolArgs: streamingTool.arguments,
        },
      };
      return { ...state, activityLog: newActivityLog };
    }

    return state;
  }

  /**
   * Find and finalize a streaming tool call by name
   */
  public finalizeStreamingToolCall(
    toolName: string,
    toolArgs: string,
    state: AgentState
  ): AgentState {
    // Find streaming tool call by name
    let foundStreamingTool: StreamingToolCall | undefined;
    for (const [, tool] of this.streamingToolCalls) {
      if (tool.name === toolName) {
        foundStreamingTool = tool;
        break;
      }
    }

    if (foundStreamingTool) {
      // Update the existing streaming item to be a complete tool call
      const activityIndex = state.activityLog.findIndex(
        (item) => item.id === foundStreamingTool!.activityId
      );
      if (activityIndex !== -1) {
        const newActivityLog = [...state.activityLog];
        newActivityLog[activityIndex] = {
          ...newActivityLog[activityIndex],
          type: 'tool_call',
          content: `Calling ${toolName}`,
          metadata: {
            toolName,
            toolArgs,
            status: 'pending',
          },
        };
        // Remove from streaming map
        this.streamingToolCalls.delete(foundStreamingTool.id);
        return { ...state, activityLog: newActivityLog };
      }
    }

    // No streaming tool call found, add a new one
    return this.addActivityItem(state, 'tool_call', `Calling ${toolName}`, {
      toolName,
      toolArgs,
      status: 'pending',
    });
  }

  /**
   * Update a tool call's status based on tool result
   */
  public updateToolCallStatus(
    output: string,
    state: AgentState
  ): AgentState {
    // Find the most recent pending tool_call (search from end)
    let toolCallIndex = -1;
    for (let i = state.activityLog.length - 1; i >= 0; i--) {
      const item = state.activityLog[i];
      if (item.type === 'tool_call' && item.metadata?.status === 'pending') {
        toolCallIndex = i;
        break;
      }
    }

    if (toolCallIndex !== -1) {
      const isSuccess = output === 'true';
      const newActivityLog = [...state.activityLog];
      newActivityLog[toolCallIndex] = {
        ...newActivityLog[toolCallIndex],
        metadata: {
          ...newActivityLog[toolCallIndex].metadata,
          status: isSuccess ? 'success' : 'error',
          errorMessage: isSuccess ? undefined : output,
        },
      };
      return { ...state, activityLog: newActivityLog };
    }

    return state;
  }

  /**
   * Clear all streaming tool calls
   */
  public clearStreamingToolCalls(): void {
    this.streamingToolCalls.clear();
  }

  /**
   * Clear activity log and reset state
   */
  public clearActivityLog(state: AgentState): AgentState {
    this.streamingToolCalls.clear();
    return {
      ...state,
      activityLog: [],
      streamingText: '',
      streamingReasoning: '',
    };
  }

  /**
   * Flush streaming text to activity log
   */
  public flushStreamingText(state: AgentState): AgentState {
    if (state.streamingText) {
      state = this.addActivityItem(state, 'text', state.streamingText);
      return { ...state, streamingText: '' };
    }
    return state;
  }

  /**
   * Flush streaming reasoning to activity log
   */
  public flushStreamingReasoning(state: AgentState): AgentState {
    if (state.streamingReasoning) {
      state = this.addActivityItem(state, 'reasoning', state.streamingReasoning);
      return { ...state, streamingReasoning: '' };
    }
    return state;
  }
}
