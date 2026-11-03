import type { WorkflowArtifactReadRequest } from '../../../services/workflow/types.ts';

export class ImageOutputError extends Error {
  readonly code: 'invalid' | 'unsupported' | 'unavailable';

  constructor(code: 'invalid' | 'unsupported' | 'unavailable', message: string) {
    super(message);
    this.code = code;
    this.name = 'ImageOutputError';
  }
}

export interface ImageArtifactReference {
  readonly artifact_id: string;
  readonly media_type?: string | null;
}

const TASK_RESULT_ID_MAX_BYTES = 256;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isTaskResultId(value: unknown): value is string {
  if (typeof value !== 'string') return false;
  // Match Rust str::trim/char::is_control in task_result_contracts.rs. Do not
  // normalize the actual ID sent back to the artifact owner.
  const trimmed = value.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');
  return trimmed.length > 0
    && trimmed.length <= TASK_RESULT_ID_MAX_BYTES
    && !/[\uD800-\uDFFF]/u.test(trimmed)
    && new TextEncoder().encode(trimmed).length <= TASK_RESULT_ID_MAX_BYTES
    && ![...trimmed].some(character => {
      const code = character.codePointAt(0)!;
      return code <= 0x1f || (code >= 0x7f && code <= 0x9f);
    });
}

/** Projection of Rust WorkflowSchedulerTaskMediaArtifactRef, not image bytes. */
export function decodeImageArtifactReference(value: unknown): ImageArtifactReference {
  if (typeof value === 'string' || Array.isArray(value)) {
    throw new ImageOutputError('unsupported', 'Image output requires one typed artifact reference.');
  }
  if (!isRecord(value)
    || Object.keys(value).some(key => key !== 'artifact_id' && key !== 'media_type')
    || !isTaskResultId(value.artifact_id)
    || (value.media_type !== undefined && value.media_type !== null && !isTaskResultId(value.media_type))) {
    throw new ImageOutputError('invalid', 'Invalid image artifact reference.');
  }
  return Object.freeze({
    artifact_id: value.artifact_id,
    ...(value.media_type === undefined ? {} : { media_type: value.media_type }),
  });
}

// Raster formats supported by the browser preview. SVG is intentionally not a
// raster result; unsupported formats retain their artifact in the I/O Inspector.
const IMAGE_EXTENSIONS: Readonly<Record<string, string>> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/webp': 'webp',
  'image/gif': 'gif',
  'image/bmp': 'bmp',
  'image/avif': 'avif',
};

function imageMediaType(value: unknown): string {
  if (typeof value !== 'string' || !Object.hasOwn(IMAGE_EXTENSIONS, value.toLowerCase())) {
    throw new ImageOutputError('unsupported', 'Unsupported image preview format.');
  }
  return value.toLowerCase();
}

function optionalByteLength(value: unknown): number | null {
  if (value === undefined || value === null) return null;
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) {
    throw new ImageOutputError('invalid', 'Invalid image artifact byte length.');
  }
  return value;
}

interface ImageArtifactDescriptor {
  readonly mediaType: string | null;
  readonly byteLength: number | null;
}

function decodeImageArtifactDescriptor(value: unknown, artifactId: string): ImageArtifactDescriptor {
  if (!isRecord(value) || value.artifact == null) {
    throw new ImageOutputError('unavailable', 'Image artifact is unavailable.');
  }
  const artifact = value.artifact;
  if (!isRecord(artifact) || artifact.artifact_id !== artifactId) {
    throw new ImageOutputError('invalid', 'Image artifact descriptor does not match the result.');
  }
  if (artifact.payload_kind !== 'image') {
    throw new ImageOutputError('unsupported', 'Artifact is not an image.');
  }
  if (artifact.lifecycle_state !== 'retained' || artifact.retention_state !== 'retained') {
    throw new ImageOutputError('unavailable', 'Image artifact is not retained.');
  }
  if (!Array.isArray(artifact.access_modes)
    || !artifact.access_modes.every(mode => mode === 'read' || mode === 'download' || mode === 'stream')) {
    throw new ImageOutputError('invalid', 'Invalid image artifact access modes.');
  }
  if (!artifact.access_modes.includes('read') && !artifact.access_modes.includes('download')
    && !(typeof artifact.read_handle === 'string' && artifact.read_handle.length > 0)) {
    throw new ImageOutputError('unavailable', 'Image artifact is not readable.');
  }
  const format = artifact.format;
  if (format != null && !isRecord(format)) {
    throw new ImageOutputError('invalid', 'Invalid image artifact format.');
  }
  return {
    mediaType: format == null ? null : imageMediaType(format.media_type),
    byteLength: optionalByteLength(artifact.byte_length),
  };
}

export interface ImageArtifactBody {
  readonly blob: Blob;
  readonly mediaType: string;
  readonly downloadName: string;
}

/** Validate the complete binary read before Uint8Array can coerce corrupt bytes. */
export function decodeImageArtifactBody(
  value: unknown,
  artifactId: string,
  descriptor: ImageArtifactDescriptor,
): ImageArtifactBody {
  if (!isRecord(value) || !isRecord(value.response)
    || value.response.artifact_id !== artifactId) {
    throw new ImageOutputError('invalid', 'Image artifact read does not match the result.');
  }
  const response = value.response;
  if (response.body_transport !== 'binary_body') {
    throw new ImageOutputError('unsupported', 'Image artifact binary body is unavailable.');
  }
  if (response.complete !== true) {
    throw new ImageOutputError('unavailable', 'Image artifact read is incomplete.');
  }
  const mediaType = imageMediaType(response.media_type);
  if (descriptor.mediaType !== null && descriptor.mediaType !== mediaType) {
    throw new ImageOutputError('invalid', 'Image artifact media type does not match its descriptor.');
  }
  const length = optionalByteLength(response.byte_length);
  if (!Array.isArray(value.body) || length === null || value.body.length !== length
    || (descriptor.byteLength !== null && descriptor.byteLength !== length)) {
    throw new ImageOutputError('invalid', 'Image artifact byte length does not match its body.');
  }
  // Array.from visits holes too, rather than letting sparse arrays pass every().
  const bytes = Array.from(value.body);
  if (!bytes.every(byte => typeof byte === 'number' && Number.isInteger(byte) && byte >= 0 && byte <= 255)) {
    throw new ImageOutputError('invalid', 'Invalid image artifact bytes.');
  }
  return {
    blob: new Blob([Uint8Array.from(bytes)], { type: mediaType }),
    mediaType,
    downloadName: `output.${IMAGE_EXTENSIONS[mediaType]}`,
  };
}

interface ImageArtifactReader {
  artifactDescriptor(request: { artifact_id: string }): Promise<unknown>;
  readArtifactBody(request: WorkflowArtifactReadRequest): Promise<unknown>;
}

export async function readImageArtifact(
  reference: ImageArtifactReference,
  reader: ImageArtifactReader,
): Promise<ImageArtifactBody> {
  const request = { artifact_id: reference.artifact_id };
  const descriptor = decodeImageArtifactDescriptor(await reader.artifactDescriptor(request), reference.artifact_id);
  const body = await reader.readArtifactBody(request);
  return decodeImageArtifactBody(body, reference.artifact_id, descriptor);
}
