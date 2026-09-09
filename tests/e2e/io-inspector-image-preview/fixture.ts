import { mount, tick, unmount } from 'svelte';
import IoInspectorPage from '../../../src/components/workbench/IoInspectorPage.svelte';
import { workflowService } from '../../../src/services/workflow/WorkflowService';
import {
  resetWorkbenchState,
  selectActiveWorkflowRun,
} from '../../../src/stores/workbenchStore';

const RUN_ID = 'run-d02-webkit';
const NODE_ID = 'image-output';
const ARTIFACT_IDS = [
  'valid-image',
  'malformed-image',
  'partial-image',
  'partial-stream-image',
  'failed-image',
  'stale-image',
  'unmount-pending-image',
  'bounded-text',
] as const;

type ArtifactId = (typeof ARTIFACT_IDS)[number];
type ReadRequest = {
  artifact_id: string;
  byte_range_start?: number | null;
  byte_range_end_exclusive?: number | null;
};

interface HarnessState {
  png: number[];
  artifacts: ArtifactId[];
  readRequests: ReadRequest[];
  streamRequests: ReadRequest[];
  createdUrls: string[];
  revokedUrls: string[];
  pendingStaleRead: ((value: unknown) => void) | null;
  pendingUnmountRead: ((value: unknown) => void) | null;
  pendingValidRefreshReject: ((reason: unknown) => void) | null;
  deferNextValidRefresh: boolean;
  rejectNextInspection: boolean;
  component: ReturnType<typeof mount> | null;
}

type HarnessSnapshot = Pick<
  HarnessState,
  'artifacts' | 'readRequests' | 'streamRequests' | 'createdUrls' | 'revokedUrls'
>;

declare global {
  interface Window {
    d02: {
      ready: boolean;
      pngByteLength: number;
      snapshot: () => HarnessSnapshot;
      removeArtifact: (artifactId: ArtifactId) => void;
      rejectInspectionOnce: () => void;
      deferValidRefresh: () => void;
      rejectValidRefresh: () => void;
      resolveStaleRead: () => void;
      resolveUnmountRead: () => void;
      unmount: () => Promise<void>;
    };
  }
}

function projectionState(name: string) {
  return {
    projection_name: name,
    projection_version: 1,
    last_applied_event_seq: 1,
    status: 'current',
    rebuilt_at_ms: null,
    updated_at_ms: Date.now(),
  };
}

function artifact(artifactId: ArtifactId, pngByteLength: number) {
  const isText = artifactId === 'bounded-text';
  const isStream = artifactId === 'partial-stream-image';
  return {
    event_seq: ARTIFACT_IDS.indexOf(artifactId) + 1,
    event_id: `event-${artifactId}`,
    occurred_at_ms: 1_700_000_000_000,
    recorded_at_ms: 1_700_000_000_000,
    workflow_run_id: RUN_ID,
    workflow_id: 'workflow-d02',
    workflow_version_id: 'version-d02',
    workflow_semantic_version: '1.0.0',
    node_id: NODE_ID,
    node_type: 'image_output',
    artifact_fact_id: `fact-${artifactId}`,
    payload_artifact_id: `payload-${artifactId}`,
    artifact_id: artifactId,
    artifact_role: 'output',
    producer_node_id: NODE_ID,
    producer_port_id: artifactId,
    consumer_node_id: null,
    consumer_port_id: null,
    media_type: isText ? 'text/plain' : 'image/png',
    size_bytes: isText ? 90_000 : pngByteLength,
    content_hash: `sha256-${artifactId}`,
    payload_ref: `fixture://${artifactId}`,
    retention_state: 'retained',
    payload_kind: isText ? 'text' : 'image',
    lifecycle_state: 'retained',
    access_modes: isStream ? ['stream'] : ['read', 'download'],
    read_handle: isStream ? null : `read-${artifactId}`,
    stream_handle: isStream ? `stream-${artifactId}` : null,
    format: {
      format_id: isText ? 'text' : 'png',
      media_type: isText ? 'text/plain' : 'image/png',
    },
  };
}

function inspection(state: HarnessState) {
  const ioArtifacts = state.artifacts.map((id) => artifact(id, state.png.length));
  return {
    run_graph: {
      workflow_run_id: RUN_ID,
      workflow_id: 'workflow-d02',
      workflow_version_id: 'version-d02',
      workflow_presentation_revision_id: 'presentation-d02',
      workflow_semantic_version: '1.0.0',
      workflow_execution_fingerprint: 'fixture-only',
      snapshot_created_at_ms: 1_700_000_000_000,
      workflow_version_created_at_ms: 1_700_000_000_000,
      presentation_revision_created_at_ms: 1_700_000_000_000,
      graph: {
        nodes: [{ id: NODE_ID, node_type: 'image_output', position: { x: 0, y: 0 }, data: {} }],
        edges: [],
      },
      graph_diagnostics: [],
      executable_topology: { schema_version: 1, nodes: [], edges: [] },
      presentation_metadata: { schema_version: 1, nodes: [], edges: [] },
      graph_settings: { schema_version: 1, nodes: [] },
    },
    run: null,
    node_statuses: [],
    io_artifacts: ioArtifacts,
    resolved_node_io: ioArtifacts.map((item) => ({
      node_id: NODE_ID,
      port_id: item.producer_port_id,
      direction: 'output',
      resolution: 'produced_output',
      provenance_kind: 'produced_output',
      artifact_fact_id: item.artifact_fact_id,
      payload_artifact_id: item.payload_artifact_id,
      artifact_id: item.artifact_id,
      artifact_role: item.artifact_role,
      media_type: item.media_type,
      retention_state: item.retention_state,
    })),
    retention_summary: [{ retention_state: 'retained', artifact_count: ioArtifacts.length }],
    run_projection_state: projectionState('run_detail'),
    node_projection_state: projectionState('node_status'),
    io_projection_state: projectionState('io_artifact'),
  };
}

function completeRead(artifactId: string, body: number[], complete = true) {
  return {
    response: {
      artifact_id: artifactId,
      media_type: artifactId.includes('bounded-text')
        ? 'text/plain'
        : artifactId.includes('partial-')
          ? 'application/octet-stream'
          : 'image/png',
      body_transport: 'binary_body',
      read_handle: `read-${artifactId}`,
      byte_length: body.length,
      content_hash: `sha256-${artifactId}`,
      complete,
    },
    body,
  };
}

async function noisyPng(): Promise<number[]> {
  const canvas = document.createElement('canvas');
  canvas.width = 320;
  canvas.height = 320;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('2D canvas is unavailable');
  const pixels = context.createImageData(canvas.width, canvas.height);
  let value = 0x5a17c9e3;
  for (let index = 0; index < pixels.data.length; index += 4) {
    value ^= value << 13;
    value ^= value >>> 17;
    value ^= value << 5;
    pixels.data[index] = value & 0xff;
    pixels.data[index + 1] = (value >>> 8) & 0xff;
    pixels.data[index + 2] = (value >>> 16) & 0xff;
    pixels.data[index + 3] = 255;
  }
  context.putImageData(pixels, 0, 0);
  const blob = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((result) => result ? resolve(result) : reject(new Error('PNG encoding failed')), 'image/png'),
  );
  return Array.from(new Uint8Array(await blob.arrayBuffer()));
}

async function bootstrap() {
  const state: HarnessState = {
    png: await noisyPng(),
    artifacts: [...ARTIFACT_IDS],
    readRequests: [],
    streamRequests: [],
    createdUrls: [],
    revokedUrls: [],
    pendingStaleRead: null,
    pendingUnmountRead: null,
    pendingValidRefreshReject: null,
    deferNextValidRefresh: false,
    rejectNextInspection: false,
    component: null,
  };
  if (state.png.length <= 65_536) {
    throw new Error(`Fixture PNG must exceed 64 KiB; got ${state.png.length} bytes`);
  }

  const nativeCreate = URL.createObjectURL.bind(URL);
  const nativeRevoke = URL.revokeObjectURL.bind(URL);
  URL.createObjectURL = (blob) => {
    const url = nativeCreate(blob);
    state.createdUrls.push(url);
    return url;
  };
  URL.revokeObjectURL = (url) => {
    state.revokedUrls.push(url);
    nativeRevoke(url);
  };

  Object.assign(workflowService, {
    queryRunInspection: async () => {
      if (state.rejectNextInspection) {
        state.rejectNextInspection = false;
        throw new Error('fixture inspection refresh failed');
      }
      return inspection(state);
    },
    artifactDescriptor: async ({ artifact_id }: ReadRequest) => ({
      artifact: {
        artifact_id,
        payload_kind: artifact_id.includes('bounded-text') ? 'text' : 'image',
        lifecycle_state: 'retained',
        retention_state: 'retained',
        byte_length: artifact_id.includes('bounded-text') ? 90_000 : state.png.length,
        content_hash: `sha256-${artifact_id}`,
        format: null,
        attribution: {},
        access_modes: artifact_id.includes('partial-stream') ? ['stream'] : ['read', 'download'],
        read_handle: artifact_id.includes('partial-stream') ? null : `read-${artifact_id}`,
        stream_handle: artifact_id.includes('partial-stream') ? `stream-${artifact_id}` : null,
      },
    }),
    readArtifactBody: async (request: ReadRequest) => {
      state.readRequests.push({ ...request });
      if (request.artifact_id === 'payload-failed-image') throw new Error('fixture retained body read failed');
      if (request.artifact_id === 'payload-valid-image' && state.deferNextValidRefresh) {
        state.deferNextValidRefresh = false;
        return new Promise((_resolve, reject) => { state.pendingValidRefreshReject = reject; });
      }
      if (request.artifact_id === 'payload-stale-image') {
        return new Promise((resolve) => { state.pendingStaleRead = resolve; });
      }
      if (request.artifact_id === 'payload-unmount-pending-image') {
        return new Promise((resolve) => { state.pendingUnmountRead = resolve; });
      }
      if (request.artifact_id === 'payload-malformed-image') {
        return completeRead(request.artifact_id, [137, 80, 78, 71, 13, 10, 26, 10, 0, 1, 2, 3]);
      }
      if (request.artifact_id === 'payload-partial-image') {
        return completeRead(request.artifact_id, state.png.slice(0, 8_192), false);
      }
      if (request.artifact_id === 'payload-bounded-text') {
        return completeRead(request.artifact_id, Array.from(new TextEncoder().encode('x'.repeat(90_000))));
      }
      return completeRead(request.artifact_id, state.png);
    },
    readArtifactStream: async (request: ReadRequest) => {
      state.streamRequests.push({ ...request });
      return {
        response: {
          artifact_id: request.artifact_id,
          stream_handle: `stream-${request.artifact_id}`,
          media_type: 'application/octet-stream',
          body_transport: 'binary_body',
          byte_length: state.png.length,
          available_byte_length: 8_192,
          lifecycle_state: 'streaming',
          complete: false,
        },
        body: state.png.slice(0, 8_192),
      };
    },
  });

  resetWorkbenchState();
  selectActiveWorkflowRun({ workflow_run_id: RUN_ID, workflow_id: 'workflow-d02' });
  state.component = mount(IoInspectorPage, { target: document.getElementById('app')! });
  await tick();

  window.d02 = {
    ready: true,
    pngByteLength: state.png.length,
    snapshot: () => ({
      artifacts: [...state.artifacts],
      readRequests: state.readRequests.map((request) => ({ ...request })),
      streamRequests: state.streamRequests.map((request) => ({ ...request })),
      createdUrls: [...state.createdUrls],
      revokedUrls: [...state.revokedUrls],
    }),
    removeArtifact: (artifactId) => {
      state.artifacts = state.artifacts.filter((id) => id !== artifactId);
    },
    rejectInspectionOnce: () => {
      state.rejectNextInspection = true;
    },
    deferValidRefresh: () => {
      state.deferNextValidRefresh = true;
    },
    rejectValidRefresh: () => {
      state.pendingValidRefreshReject?.(new Error('fixture pending refresh failed'));
      state.pendingValidRefreshReject = null;
    },
    resolveStaleRead: () => {
      state.pendingStaleRead?.(completeRead('payload-stale-image', state.png));
      state.pendingStaleRead = null;
    },
    resolveUnmountRead: () => {
      state.pendingUnmountRead?.(completeRead('payload-unmount-pending-image', state.png));
      state.pendingUnmountRead = null;
    },
    unmount: async () => {
      if (state.component) await unmount(state.component);
      state.component = null;
    },
  };
}

void bootstrap().catch((error) => {
  document.body.dataset.fixtureError = error instanceof Error ? error.stack ?? error.message : String(error);
  console.error(error);
});
