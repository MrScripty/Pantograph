import type {
  DependencyOverrideFieldsV1,
  DependencyOverridePatchV1,
  StringOverrideField,
} from './dependencyEnvironmentTypes.ts';

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

export function countDependencyBindingPatches(
  patches: DependencyOverridePatchV1[],
  bindingId: string,
): number {
  return patches.filter((patch) => patch.binding_id === bindingId).length;
}

export function countDependencyRequirementPatches(
  patches: DependencyOverridePatchV1[],
  bindingId: string,
  requirementName: string,
): number {
  return patches.filter((patch) =>
    isPatchTarget(patch, bindingId, 'requirement', requirementName)
  ).length;
}

export function hasDependencyBindingOverrideFields(
  patches: DependencyOverridePatchV1[],
  bindingId: string,
): boolean {
  const patch = getPatchFrom(patches, bindingId, 'binding');
  return patch ? hasOverrideFields(patch.fields) : false;
}

export function hasDependencyRequirementOverrideFields(
  patches: DependencyOverridePatchV1[],
  bindingId: string,
  requirementName: string,
): boolean {
  const patch = getPatchFrom(patches, bindingId, 'requirement', requirementName);
  return patch ? hasOverrideFields(patch.fields) : false;
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
