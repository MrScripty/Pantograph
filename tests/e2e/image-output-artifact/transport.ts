import type {
  WorkflowArtifactBodyRead,
  WorkflowArtifactDescriptorQueryRequest,
  WorkflowArtifactDescriptorQueryResponse,
} from '../../../src/services/workflow/types';

export const workflowService = {
  artifactDescriptor: async (_request: WorkflowArtifactDescriptorQueryRequest): Promise<WorkflowArtifactDescriptorQueryResponse> => {
    throw new Error('Fixture transport is not ready');
  },
  readArtifactBody: async (_request: WorkflowArtifactDescriptorQueryRequest): Promise<WorkflowArtifactBodyRead> => {
    throw new Error('Fixture transport is not ready');
  },
  artifactFormatSettings: async () => ({ settings: { image: { format_id: 'png', quality_percent: 75, color_profile_id: 'srgb' } } }),
  artifactFormatCapabilities: async () => ({ image_formats: [
    { format_id: 'png', display_name: 'PNG', media_type: 'image/png', codec_ids: [], quality_min_percent: null,
      quality_max_percent: null, bitrate_min_kbps: null, bitrate_max_kbps: null, crf_min: null, crf_max: null,
      bit_depths: [], color_profile_ids: ['srgb'], provided_by_dependency_id: 'fixture', provided_by_version: null },
  ] }),
};
