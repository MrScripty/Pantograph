export type DependencyState =
  | 'unresolved'
  | 'invalid'
  | 'resolved'
  | 'checking'
  | 'missing'
  | 'installing'
  | 'ready'
  | 'failed'
  | string;

export type DependencyValidationState =
  | 'resolved'
  | 'unknown_profile'
  | 'invalid_profile'
  | 'profile_conflict';

export interface DependencyValidationError {
  code: string;
  scope: 'top_level' | 'binding';
  binding_id?: string;
  field?: string;
  message: string;
}

export interface ModelDependencyRequirement {
  kind: string;
  name: string;
  exact_pin: string;
}

export interface ModelDependencyBinding {
  binding_id: string;
  profile_id: string;
  profile_version: number;
  profile_hash?: string;
  backend_key?: string;
  platform_selector?: string;
  environment_kind?: string;
  env_id?: string;
  validation_state: DependencyValidationState;
  validation_errors: DependencyValidationError[];
  requirements: ModelDependencyRequirement[];
}

export interface ModelDependencyRequirements {
  model_id: string;
  platform_key: string;
  backend_key?: string;
  dependency_contract_version: number;
  validation_state: DependencyValidationState;
  validation_errors: DependencyValidationError[];
  bindings: ModelDependencyBinding[];
  selected_binding_ids: string[];
}

export interface ModelDependencyBindingStatus {
  binding_id: string;
  env_id?: string;
  state: DependencyState;
  code?: string;
  message?: string;
  missing_requirements?: string[];
  installed_requirements?: string[];
  failed_requirements?: string[];
}

export interface ModelDependencyStatus {
  state: DependencyState;
  code?: string;
  message?: string;
  requirements: ModelDependencyRequirements;
  bindings: ModelDependencyBindingStatus[];
  checked_at?: string;
}

export interface EnvironmentRef {
  contract_version: number;
  environment_key?: string;
  environment_kind?: string;
  env_id?: string;
  env_ids?: string[];
  python_executable?: string;
  state: string;
  requirements_fingerprint?: string;
  platform_key?: string;
  backend_key?: string;
  manifest_path?: string;
}

export interface DependencyOverrideFieldsV1 {
  python_executable?: string;
  index_url?: string;
  extra_index_urls?: string[];
  wheel_source_path?: string;
  package_source_override?: string;
}

export interface DependencyOverridePatchV1 {
  contract_version: number;
  binding_id: string;
  scope: 'binding' | 'requirement';
  requirement_name?: string;
  fields: DependencyOverrideFieldsV1;
  source?: string;
  updated_at?: string;
}

export interface DependencyActivityEvent {
  timestamp: string;
  node_type: string;
  model_path: string;
  phase: string;
  message: string;
  binding_id?: string;
  requirement_name?: string;
  stream?: string;
}

export type StringOverrideField =
  | 'python_executable'
  | 'index_url'
  | 'wheel_source_path'
  | 'package_source_override';

export interface DependencyEnvironmentActionRequest {
  action: 'resolve' | 'check' | 'install' | 'run';
  mode?: 'auto' | 'manual';
  modelPath: string;
  modelId?: string;
  modelType?: string;
  taskTypePrimary?: string;
  backendKey?: string;
  platformContext?: Record<string, string>;
  selectedBindingIds?: string[];
  dependencyRequirements?: ModelDependencyRequirements;
  dependencyOverridePatches?: DependencyOverridePatchV1[];
}

export interface DependencyEnvironmentActionResponse {
  nodeData: Record<string, unknown>;
}

export interface DependencyEnvironmentActionPayloadInput {
  action: DependencyEnvironmentActionRequest['action'];
  mode: 'auto' | 'manual';
  upstreamModelPath: string | null;
  upstreamModelId: string | null;
  upstreamModelType: string | null;
  upstreamTaskType: string | null;
  upstreamBackendKey: string | null;
  upstreamPlatformContext: Record<string, string> | null;
  selectedBindingIds: string[];
  upstreamRequirements: ModelDependencyRequirements | null;
  dependencyRequirements: ModelDependencyRequirements | null;
  effectiveManualOverrides: DependencyOverridePatchV1[];
}

export function buildDependencyEnvironmentActionPayload({
  action,
  mode,
  upstreamModelPath,
  upstreamModelId,
  upstreamModelType,
  upstreamTaskType,
  upstreamBackendKey,
  upstreamPlatformContext,
  selectedBindingIds,
  upstreamRequirements,
  dependencyRequirements,
  effectiveManualOverrides,
}: DependencyEnvironmentActionPayloadInput): DependencyEnvironmentActionRequest | null {
  const modelPath = (upstreamModelPath ?? '').trim();
  if (!modelPath) return null;
  return {
    action,
    mode,
    modelPath,
    modelId: upstreamModelId ?? dependencyRequirements?.model_id ?? undefined,
    modelType: upstreamModelType ?? undefined,
    taskTypePrimary: upstreamTaskType ?? undefined,
    backendKey: upstreamBackendKey ?? dependencyRequirements?.backend_key ?? undefined,
    platformContext: upstreamPlatformContext ?? undefined,
    selectedBindingIds,
    dependencyRequirements: upstreamRequirements ?? dependencyRequirements ?? undefined,
    dependencyOverridePatches: effectiveManualOverrides.length > 0 ? effectiveManualOverrides : undefined,
  };
}

export function normalizeOverridePatch(raw: unknown): DependencyOverridePatchV1 | null {
  if (!raw || typeof raw !== 'object') return null;
  const value = raw as Record<string, unknown>;
  const contract_version = Number(value.contract_version ?? 1);
  const binding_id = String(value.binding_id ?? '').trim();
  const scopeRaw = String(value.scope ?? '').trim().toLowerCase();
  const scope = scopeRaw === 'binding' || scopeRaw === 'requirement' ? scopeRaw : '';
  const requirement_name = value.requirement_name ? String(value.requirement_name) : undefined;
  const rawFields = value.fields as Record<string, unknown> | undefined;
  if (
    !Number.isFinite(contract_version) ||
    binding_id.length === 0 ||
    scope.length === 0 ||
    !rawFields
  ) {
    return null;
  }

  const fields: DependencyOverrideFieldsV1 = {};
  if (typeof rawFields.python_executable === 'string') {
    fields.python_executable = rawFields.python_executable;
  }
  if (typeof rawFields.index_url === 'string') {
    fields.index_url = rawFields.index_url;
  }
  if (Array.isArray(rawFields.extra_index_urls)) {
    fields.extra_index_urls = rawFields.extra_index_urls
      .map((entry) => String(entry).trim())
      .filter((entry) => entry.length > 0);
  }
  if (typeof rawFields.wheel_source_path === 'string') {
    fields.wheel_source_path = rawFields.wheel_source_path;
  }
  if (typeof rawFields.package_source_override === 'string') {
    fields.package_source_override = rawFields.package_source_override;
  }

  return {
    contract_version,
    binding_id,
    scope: scope as 'binding' | 'requirement',
    requirement_name,
    fields,
    source: typeof value.source === 'string' ? value.source : undefined,
    updated_at: typeof value.updated_at === 'string' ? value.updated_at : undefined,
  };
}

export function parseOverridePatches(raw: unknown): DependencyOverridePatchV1[] {
  const parseArray = (value: unknown): DependencyOverridePatchV1[] => {
    if (!Array.isArray(value)) return [];
    return value
      .map((entry) => normalizeOverridePatch(entry))
      .filter((entry): entry is DependencyOverridePatchV1 => entry !== null);
  };

  if (typeof raw === 'string') {
    try {
      return parseArray(JSON.parse(raw));
    } catch {
      return [];
    }
  }
  return parseArray(raw);
}

export function mergeOverridePatches(
  base: DependencyOverridePatchV1[],
  overlay: DependencyOverridePatchV1[]
): DependencyOverridePatchV1[] {
  const byKey = new Map<string, DependencyOverridePatchV1>();
  const patchKey = (patch: DependencyOverridePatchV1): string =>
    `${patch.binding_id}|${patch.scope}|${(patch.requirement_name ?? '').toLowerCase()}`;

  for (const patch of base) {
    byKey.set(patchKey(patch), patch);
  }
  for (const patch of overlay) {
    byKey.set(patchKey(patch), patch);
  }
  return [...byKey.values()];
}

export function dependencyTokenLabel(value: string): string {
  return value.replaceAll('_', ' ');
}

export function dependencyCodeLabel(code?: string): string | null {
  switch (code) {
    case 'requirements_missing':
      return 'requirements missing';
    case 'dependency_install_failed':
    case 'dependency_check_failed':
      return 'dependency check failed';
    case 'profile_conflict':
      return 'profile conflict';
    case 'unknown_profile':
      return 'unknown profile';
    case 'invalid_profile':
      return 'invalid profile';
    default:
      return code ? dependencyTokenLabel(code) : null;
  }
}

export function isPatchTarget(
  patch: DependencyOverridePatchV1,
  bindingId: string,
  scope: 'binding' | 'requirement',
  requirementName?: string
): boolean {
  if (patch.binding_id !== bindingId) return false;
  if (patch.scope !== scope) return false;
  if (scope === 'requirement') {
    return (
      (patch.requirement_name ?? '').trim().toLowerCase() ===
      (requirementName ?? '').trim().toLowerCase()
    );
  }
  return true;
}

export function hasOverrideFields(fields: DependencyOverrideFieldsV1): boolean {
  return (
    (fields.python_executable?.trim().length ?? 0) > 0 ||
    (fields.index_url?.trim().length ?? 0) > 0 ||
    (fields.wheel_source_path?.trim().length ?? 0) > 0 ||
    (fields.package_source_override?.trim().length ?? 0) > 0 ||
    (fields.extra_index_urls?.length ?? 0) > 0
  );
}

export function getPatchFrom(
  patches: DependencyOverridePatchV1[],
  bindingId: string,
  scope: 'binding' | 'requirement',
  requirementName?: string
): DependencyOverridePatchV1 | undefined {
  return patches.find((patch) => isPatchTarget(patch, bindingId, scope, requirementName));
}

export function upsertStringOverrideField(
  patches: DependencyOverridePatchV1[],
  bindingId: string,
  scope: 'binding' | 'requirement',
  requirementName: string | undefined,
  field: StringOverrideField,
  rawValue: string,
  updatedAt: string
): DependencyOverridePatchV1[] {
  const value = rawValue.trim();
  const next = [...patches];
  const idx = next.findIndex((patch) => isPatchTarget(patch, bindingId, scope, requirementName));
  const patch: DependencyOverridePatchV1 =
    idx >= 0
      ? {
          ...next[idx],
          fields: { ...next[idx].fields },
        }
      : {
          contract_version: 1,
          binding_id: bindingId,
          scope,
          requirement_name: scope === 'requirement' ? requirementName : undefined,
          fields: {},
          source: 'user',
        };

  if (value.length === 0) {
    delete patch.fields[field];
  } else {
    patch.fields[field] = value;
  }
  patch.source = 'user';
  patch.updated_at = updatedAt;

  if (!hasOverrideFields(patch.fields)) {
    if (idx >= 0) {
      next.splice(idx, 1);
    }
  } else if (idx >= 0) {
    next[idx] = patch;
  } else {
    next.push(patch);
  }

  return next;
}

export function upsertExtraIndexUrls(
  patches: DependencyOverridePatchV1[],
  bindingId: string,
  requirementName: string,
  rawValue: string,
  updatedAt: string
): DependencyOverridePatchV1[] {
  const deduped = Array.from(
    new Set(
      rawValue
        .split(',')
        .map((part) => part.trim())
        .filter((part) => part.length > 0)
    )
  );

  const next = [...patches];
  const idx = next.findIndex((patch) => isPatchTarget(patch, bindingId, 'requirement', requirementName));
  const patch: DependencyOverridePatchV1 =
    idx >= 0
      ? {
          ...next[idx],
          fields: { ...next[idx].fields },
        }
      : {
          contract_version: 1,
          binding_id: bindingId,
          scope: 'requirement',
          requirement_name: requirementName,
          fields: {},
          source: 'user',
        };

  if (deduped.length === 0) {
    delete patch.fields.extra_index_urls;
  } else {
    patch.fields.extra_index_urls = deduped;
  }
  patch.source = 'user';
  patch.updated_at = updatedAt;

  if (!hasOverrideFields(patch.fields)) {
    if (idx >= 0) {
      next.splice(idx, 1);
    }
  } else if (idx >= 0) {
    next[idx] = patch;
  } else {
    next.push(patch);
  }

  return next;
}
