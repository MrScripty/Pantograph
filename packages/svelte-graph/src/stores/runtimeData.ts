export interface RuntimeDataCleanupResult {
  changed: boolean;
  data: Record<string, unknown>;
}

export function removeNodeDataKeys(
  data: Record<string, unknown>,
  keys: Iterable<string>
): RuntimeDataCleanupResult {
  const nextData = { ...data };
  let changed = false;

  for (const key of keys) {
    if (!(key in nextData)) continue;
    delete nextData[key];
    changed = true;
  }

  return { changed, data: nextData };
}
