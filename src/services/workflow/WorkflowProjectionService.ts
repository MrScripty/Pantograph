import type {
  WorkflowLibraryUsageQueryRequest,
  WorkflowLibraryUsageQueryResponse,
  WorkflowRunDetailQueryRequest,
  WorkflowRunDetailQueryResponse,
  WorkflowRunListQueryRequest,
  WorkflowRunListQueryResponse,
  WorkflowSchedulerTimelineQueryRequest,
  WorkflowSchedulerTimelineQueryResponse,
} from '../diagnostics/types.ts';
import { WorkflowGraphMutationService } from './WorkflowGraphMutationService.ts';
import { USE_WORKFLOW_MOCKS } from './workflowServiceConfig.ts';
import { invokeWorkflowCommand } from './workflowServiceErrors.ts';

export class WorkflowProjectionService extends WorkflowGraphMutationService {
  async querySchedulerTimeline(
    request: WorkflowSchedulerTimelineQueryRequest = {},
  ): Promise<WorkflowSchedulerTimelineQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        events: [],
        projection_state: {
          projection_name: 'scheduler_timeline',
          projection_version: 1,
          last_applied_event_seq: 0,
          status: 'current',
          rebuilt_at_ms: null,
          updated_at_ms: Date.now(),
        },
      };
    }

    return invokeWorkflowCommand<WorkflowSchedulerTimelineQueryResponse>('workflow_scheduler_timeline_query', {
      request,
    });
  }

  async queryRunList(
    request: WorkflowRunListQueryRequest = {},
  ): Promise<WorkflowRunListQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        runs: [],
        facets: [],
        projection_state: {
          projection_name: 'run_list',
          projection_version: 2,
          last_applied_event_seq: 0,
          status: 'current',
          rebuilt_at_ms: null,
          updated_at_ms: Date.now(),
        },
      };
    }

    return invokeWorkflowCommand<WorkflowRunListQueryResponse>('workflow_run_list_query', {
      request,
    });
  }

  async queryRunDetail(
    request: WorkflowRunDetailQueryRequest,
  ): Promise<WorkflowRunDetailQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        run: null,
        projection_state: {
          projection_name: 'run_detail',
          projection_version: 1,
          last_applied_event_seq: 0,
          status: 'current',
          rebuilt_at_ms: null,
          updated_at_ms: Date.now(),
        },
      };
    }

    return invokeWorkflowCommand<WorkflowRunDetailQueryResponse>('workflow_run_detail_query', {
      request,
    });
  }

  async queryLibraryUsage(
    request: WorkflowLibraryUsageQueryRequest = {},
  ): Promise<WorkflowLibraryUsageQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        assets: [],
        projection_state: {
          projection_name: 'library_usage',
          projection_version: 1,
          last_applied_event_seq: 0,
          status: 'current',
          rebuilt_at_ms: null,
          updated_at_ms: Date.now(),
        },
      };
    }

    return invokeWorkflowCommand<WorkflowLibraryUsageQueryResponse>('workflow_library_usage_query', {
      request,
    });
  }
}
