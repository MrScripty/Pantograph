import type {
  WorkflowLibraryUsageQueryRequest,
  WorkflowLibraryUsageQueryResponse,
  WorkflowIoArtifactQueryRequest,
  WorkflowIoArtifactQueryResponse,
  WorkflowRunDetailQueryRequest,
  WorkflowRunDetailQueryResponse,
  WorkflowRunInspectionQueryRequest,
  WorkflowRunInspectionQueryResponse,
  WorkflowRunListQueryRequest,
  WorkflowRunListQueryResponse,
  WorkflowSchedulerEstimateQueryRequest,
  WorkflowSchedulerEstimateQueryResponse,
  WorkflowSchedulerTimelineQueryRequest,
  WorkflowSchedulerTimelineQueryResponse,
} from '../diagnostics/types.ts';
import type {
  WorkflowLocalNetworkStatusQueryRequest,
  WorkflowLocalNetworkStatusQueryResponse,
} from './types.ts';
import { mockProjectionState } from '../diagnostics/projectionState.ts';
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
        projection_state: mockProjectionState({
          projection_name: 'scheduler_timeline',
          projection_version: 1,
        }),
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
        projection_state: mockProjectionState({
          projection_name: 'run_list',
          projection_version: 7,
        }),
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
        node_statuses: [],
        projection_state: mockProjectionState({
          projection_name: 'run_detail',
          projection_version: 6,
        }),
        node_projection_state: mockProjectionState({
          projection_name: 'node_status',
          projection_version: 5,
        }),
      };
    }

    return invokeWorkflowCommand<WorkflowRunDetailQueryResponse>('workflow_run_detail_query', {
      request,
    });
  }

  async queryRunInspection(
    request: WorkflowRunInspectionQueryRequest,
  ): Promise<WorkflowRunInspectionQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        run_graph: null,
        run: null,
        node_statuses: [],
        io_artifacts: [],
        retention_summary: [],
        run_projection_state: mockProjectionState({
          projection_name: 'run_detail',
          projection_version: 6,
        }),
        node_projection_state: mockProjectionState({
          projection_name: 'node_status',
          projection_version: 6,
        }),
        io_projection_state: mockProjectionState({
          projection_name: 'io_artifact',
          projection_version: 6,
        }),
      };
    }

    return invokeWorkflowCommand<WorkflowRunInspectionQueryResponse>('workflow_run_inspection_query', {
      request,
    });
  }

  async querySchedulerEstimate(
    request: WorkflowSchedulerEstimateQueryRequest,
  ): Promise<WorkflowSchedulerEstimateQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        estimate: null,
        projection_state: mockProjectionState({
          projection_name: 'run_detail',
          projection_version: 6,
        }),
      };
    }

    return invokeWorkflowCommand<WorkflowSchedulerEstimateQueryResponse>('workflow_scheduler_estimate_query', {
      request,
    });
  }

  async queryLibraryUsage(
    request: WorkflowLibraryUsageQueryRequest = {},
  ): Promise<WorkflowLibraryUsageQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        assets: [],
        projection_state: mockProjectionState({
          projection_name: 'library_usage',
          projection_version: 1,
        }),
      };
    }

    return invokeWorkflowCommand<WorkflowLibraryUsageQueryResponse>('workflow_library_usage_query', {
      request,
    });
  }

  async queryIoArtifacts(
    request: WorkflowIoArtifactQueryRequest,
  ): Promise<WorkflowIoArtifactQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        artifacts: [],
        retention_summary: [],
        projection_state: mockProjectionState({
          projection_name: 'io_artifact',
          projection_version: 4,
        }),
      };
    }

    return invokeWorkflowCommand<WorkflowIoArtifactQueryResponse>('workflow_io_artifact_query', {
      request,
    });
  }

  async queryLocalNetworkStatus(
    request: WorkflowLocalNetworkStatusQueryRequest = {},
  ): Promise<WorkflowLocalNetworkStatusQueryResponse> {
    if (USE_WORKFLOW_MOCKS) {
      return {
        local_node: {
          node_id: 'local',
          display_name: 'Local Pantograph',
          captured_at_ms: Date.now(),
          transport_state: 'local_only',
          system: {
            hostname: 'mock-host',
            os_name: 'mock-os',
            os_version: null,
            kernel_version: null,
            cpu: {
              logical_core_count: 1,
              average_usage_percent: 0,
            },
            memory: {
              total_bytes: 0,
              used_bytes: 0,
              available_bytes: 0,
            },
            disks: [],
            network_interfaces: [],
            gpu: {
              available: false,
              reason: 'GPU metrics are not available in mock workflow service',
            },
          },
          scheduler_load: {
            max_sessions: 0,
            active_session_count: 0,
            max_loaded_sessions: 0,
            loaded_session_count: 0,
            active_run_count: 0,
            queued_run_count: 0,
            active_workflow_run_ids: [],
            queued_workflow_run_ids: [],
            run_placements: [],
          },
          degradation_warnings: ['GPU metrics are not available in mock workflow service'],
        },
        peer_nodes: [],
      };
    }

    return invokeWorkflowCommand<WorkflowLocalNetworkStatusQueryResponse>(
      'workflow_local_network_status_query',
      {
        request,
      },
    );
  }
}
