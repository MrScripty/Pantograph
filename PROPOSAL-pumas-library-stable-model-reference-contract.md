# Proposal: Pumas-Library Stable Model Reference Contract (No Raw Path Ownership in Clients)

## Status
Proposed

## Audience
Pumas-Library maintainers, Pantograph maintainers

## Problem Statement
Pantograph should not own filesystem model paths. Model paths are infrastructure detail owned by Pumas-Library and may change due to reclassification/migration.

Observed behavior in this workspace:

1. Pantograph config persisted an embedding model path under `models/llm/...`:
   - `/home/jeremy/.local/share/com.pantograph.app/config.json`
2. Pumas-Library later reindexed the same family under `models/embedding/...`:
   - log evidence in `/home/jeremy/.config/pumas-library-electron/logs/main.log`
   - entries show earlier `llm/Qwen/...` then later `embedding/Qwen/...` on February 27, 2026.
3. Runtime startup failed because stale path no longer existed:
   - llama.cpp failed to open GGUF path under `models/llm/...`

This is a contract bug: clients are coupled to mutable storage paths.

## Goals

1. Eliminate raw model-path ownership in Pantograph and other clients.
2. Make model identity stable across reindex/migration.
3. Keep path migration transparent to clients.
4. Preserve backward compatibility for path-based callers during transition.

## Non-Goals

1. Redesign Pantograph workflow graph schema in this proposal.
2. Remove filesystem paths from Pumas internals.
3. Break current APIs without migration.

## Proposed Contract Changes in Pumas-Library

1. Stable model handle (required):
   - Add immutable `model_id` as the primary contract key.
   - `model_id` must survive taxonomy/path moves.

2. Locator resolution API (required):
   - Add API: `resolve_model_locator(model_id, runtime_kind)` returning:
     - `model_id`
     - `canonical_path` (if file-backed)
     - `artifact_kind` (`gguf_file`, `directory`, `repo_ref`, etc.)
     - `revision`/`generation`
     - `aliases` (optional old paths/ids)
   - Runtime consumers request locator at execution time, not from cached path.

3. Legacy path compatibility (required):
   - Add API: `resolve_legacy_path(path)` -> `model_id` + canonical locator.
   - Maintain alias map when models move (`old_path -> model_id`).

4. Change notification (recommended):
   - Emit `model_relocated` and `model_reindexed` events with old/new locator.
   - Clients can invalidate local caches.

5. Metadata indexing invariant (required):
   - If taxonomy changes (e.g., `llm` -> `embedding`), alias map must be updated atomically with reindex.

## Pantograph Integration Plan (Follow-Up)

1. Store `model_id` (and optional display name), not absolute path.
2. Resolve locator via Pumas immediately before startup/execution.
3. Keep path fields only as temporary compatibility fallback.
4. Remove path persistence after one migration cycle.

## Backward Compatibility

1. Existing path-based calls continue to work through `resolve_legacy_path`.
2. Deprecate direct path usage with warnings.
3. Publish removal date once model-id flow is fully adopted.

## Migration Plan

1. Pumas builds alias map from current index + migration history.
2. Pantograph reads current persisted paths and converts to `model_id` using `resolve_legacy_path`.
3. Pantograph writes back `model_id` and drops path fields.
4. Add telemetry counters:
   - `% requests using model_id`
   - `% requests resolved through legacy path`
   - unresolved legacy paths count

## Acceptance Criteria

1. Moving a model between taxonomy directories does not break client execution.
2. Pantograph can start embedding/inference with only `model_id` and zero path persistence.
3. Legacy path from pre-migration config resolves correctly via alias map.
4. No client-facing runtime failures of the form “file not found” due solely to taxonomy relocation.

## Suggested Work Breakdown

1. Pumas-Library:
   - implement locator and legacy-resolution APIs
   - implement alias table persistence
   - emit relocation events
2. Pantograph:
   - switch config storage to `model_id`
   - resolve locator on-demand before launching backend
   - add migration routine for old configs

## Rationale
Path-based contracts are brittle in any system that supports model reclassification, normalization, or migration. Stable IDs + resolver APIs are the durable boundary.
