/**
 * Agent Feature Module
 *
 * LLM agent orchestration, streaming, and activity tracking.
 */

// Services
export {
  AgentService,
  ActivityLogger,
  StreamHandler,
} from '../../services/agent';

// Types
export type {
  AgentRequest,
  AgentResponse,
  AgentEvent,
  AgentState,
  AgentActivityItem,
  ComponentUpdate,
  FileChange,
  ComponentInfo,
  AgentStateListener,
  AgentEventListener,
  ContentEventData,
  ToolCallEventData,
  ToolCallDeltaEventData,
  ToolResultEventData,
  StreamingToolCall,
} from '../../services/agent';

// Components
export { default as TopBar } from '../../components/TopBar.svelte';
export { default as ActivityLog } from '../../components/side-panel/ActivityLog.svelte';
export { default as FollowUpInput } from '../../components/side-panel/FollowUpInput.svelte';
