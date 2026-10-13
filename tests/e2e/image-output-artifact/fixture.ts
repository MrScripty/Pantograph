import { mount, tick, unmount } from 'svelte';
import Harness from './Harness.svelte';
import { nodeData } from './store';
import { workflowService } from './transport';
import type { WorkflowArtifactBodyRead, WorkflowArtifactDescriptorQueryResponse } from '../../../src/services/workflow/types';

type EncodedImage = { body: number[]; mediaType: string; width: number; height: number };
type Pending = { resolve: (value: unknown) => void; reject: (reason: Error) => void };

async function encode(mediaType: string, width: number, height: number): Promise<EncodedImage> {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('Canvas 2D is unavailable');
  context.fillStyle = '#8b5cf6';
  context.fillRect(0, 0, width, height);
  context.fillStyle = '#f97316';
  context.fillRect(2, 2, width / 2, height / 2);
  const blob = await new Promise<Blob>((resolve, reject) => canvas.toBlob(
    (value) => value ? resolve(value) : reject(new Error(`Could not encode ${mediaType}`)), mediaType, 0.9,
  ));
  if (blob.type !== mediaType) throw new Error(`Browser substituted ${blob.type} for ${mediaType}`);
  return { body: Array.from(new Uint8Array(await blob.arrayBuffer())), mediaType, width, height };
}

async function bootstrap() {
  const images = { png: await encode('image/png', 48, 32), jpeg: await encode('image/jpeg', 64, 40) };
  const descriptors: string[] = [];
  const reads: object[] = [];
  const created: { url: string; mediaType: string; byteLength: number }[] = [];
  const revoked: string[] = [];
  const downloads: { url: string; filename: string }[] = [];
  const parentEvents: string[] = [];
  for (const eventName of ['pointerdown', 'pointerup', 'mousedown', 'mouseup', 'click']) {
    document.querySelector('#app')!.addEventListener(eventName, () => parentEvents.push(eventName));
  }
  const pending = new Map<string, Pending>();
  const nativeCreate = URL.createObjectURL.bind(URL);
  const nativeRevoke = URL.revokeObjectURL.bind(URL);
  URL.createObjectURL = (blob) => {
    if (!(blob instanceof Blob)) throw new Error('Fixture expects an image Blob');
    const url = nativeCreate(blob);
    created.push({ url, mediaType: blob.type, byteLength: blob.size });
    return url;
  };
  URL.revokeObjectURL = (url) => { revoked.push(url); nativeRevoke(url); };
  const nativeClick = HTMLAnchorElement.prototype.click;
  HTMLAnchorElement.prototype.click = function () {
    if (this.download) downloads.push({ url: this.href, filename: this.download });
    nativeClick.call(this);
  };
  const source = (id: string) => id === 'jpeg' ? images.jpeg : images.png;
  const descriptor = (id: string): WorkflowArtifactDescriptorQueryResponse => ({ artifact: {
    artifact_id: id === 'descriptor-id-mismatch' ? 'wrong-id' : id,
    payload_kind: id === 'nonimage' ? 'text' : 'image',
    lifecycle_state: 'retained', retention_state: 'retained',
    byte_length: id === 'bad-bytes' ? 12 : source(id).body.length, content_hash: 'fixture-hash',
    format: { format_id: id === 'jpeg' ? 'jpeg' : 'png', media_type: source(id).mediaType },
    attribution: { workflow_run_id: 'image-output-fixture-run' }, access_modes: id === 'unreadable' ? [] : ['read'], read_handle: id === 'unreadable' ? null : `read-${id}`,
  } });
  const body = (id: string): WorkflowArtifactBodyRead => {
    const image = source(id);
    const bytes = id === 'bad-bytes' ? [137, 80, 78, 71, 13, 10, 26, 10, 0, 1, 2, 3] : image.body;
    return { response: {
      artifact_id: id === 'read-id-mismatch' ? 'wrong-id' : id,
      media_type: id === 'mime-mismatch' ? 'image/jpeg' : image.mediaType,
      body_transport: 'binary_body', read_handle: `read-${id}`,
      byte_length: bytes.length, complete: id !== 'partial', content_hash: 'fixture-hash',
    }, body: bytes };
  };
  const defer = <T>(key: string): Promise<T> => new Promise((resolve, reject) => {
    pending.set(key, { resolve: (value) => resolve(value as T), reject });
  });
  workflowService.artifactDescriptor = async ({ artifact_id }) => {
    descriptors.push(artifact_id);
    if (artifact_id.startsWith('defer-descriptor')) return defer<WorkflowArtifactDescriptorQueryResponse>(artifact_id);
    return descriptor(artifact_id);
  };
  workflowService.readArtifactBody = async (request) => {
    reads.push({ ...request });
    if (request.artifact_id === 'read-failure') throw new Error('fixture retained read failed');
    if (request.artifact_id.startsWith('defer-read')) return defer<WorkflowArtifactBodyRead>(request.artifact_id);
    return body(request.artifact_id);
  };
  let component: ReturnType<typeof mount> | null = mount(Harness, { target: document.querySelector('#app')! });
  await tick();
  Object.assign(window, { imageArtifactFixture: {
    ready: true,
    images,
    snapshot: () => ({ descriptors: [...descriptors], reads: [...reads], created: [...created], revoked: [...revoked], downloads: [...downloads], parentEvents: [...parentEvents], pending: [...pending.keys()] }),
    set: async (image: unknown) => { nodeData.set({ label: 'Image artifact fixture', image }); await tick(); },
    resolve: async (id: string) => {
      const request = pending.get(id);
      if (!request) throw new Error(`No pending request for ${id}`);
      pending.delete(id);
      request.resolve(id.startsWith('defer-descriptor') ? descriptor(id) : body(id));
      await tick();
    },
    reject: async (id: string) => {
      const request = pending.get(id);
      if (!request) throw new Error(`No pending request for ${id}`);
      pending.delete(id);
      request.reject(new Error('fixture stale request failed'));
      await tick();
    },
    unmount: async () => { if (component) await unmount(component); component = null; },
    remount: async () => {
      if (component) throw new Error('Unmount before starting a fresh component scope');
      nodeData.set({ label: 'Image artifact fixture' });
      component = mount(Harness, { target: document.querySelector('#app')! });
      await tick();
    },
  } });
}

void bootstrap().catch((error: unknown) => {
  document.body.dataset.fixtureError = error instanceof Error ? error.stack ?? error.message : String(error);
});
