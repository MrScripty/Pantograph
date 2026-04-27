import { invoke } from '@tauri-apps/api/core';
import type {
  WorkflowRunDetailQueryRequest,
  WorkflowRunDetailQueryResponse,
  WorkflowRunListQueryRequest,
  WorkflowRunListQueryResponse,
} from '../diagnostics/types.ts';
import { WorkflowGraphMutationService } from './WorkflowGraphMutationService.ts';
import { USE_WORKFLOW_MOCKS } from './workflowServiceConfig.ts';

export class WorkflowRunProjectionService extends WorkflowGraphMutationService {
  async queryRunList(
    request: WorkflowRunListQueryRequest = {},
  ): Promise<WorkflowRunListQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        runs: [],
        facets: [],
        projection_state: {
          projection_name: 'run_list',
          projection_version: 1,
          last_applied_event_seq: 0,
          status: 'current',
          rebuilt_at_ms: null,
          updated_at_ms: Date.now(),
        },
      };
    }

    return invoke<WorkflowRunListQueryResponse>('workflow_run_list_query', {
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

    return invoke<WorkflowRunDetailQueryResponse>('workflow_run_detail_query', {
      request,
    });
  }
}
