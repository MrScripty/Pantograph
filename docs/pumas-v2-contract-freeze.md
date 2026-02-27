# Pumas v2 Dependency Contract Freeze

Date: 2026-02-27
Scope: Pantograph workflow dependency preflight and Puma-Lib dependency UI commands.

## Authority and Layering

- Source of truth for dependency planning/check/install is `pumas-library` (`PumasApi`).
- Layering is fixed as:
  - UI (`src/`) -> Tauri commands (`src-tauri/src/workflow/commands.rs`) -> resolver/service (`src-tauri/src/workflow/model_dependencies.rs`) -> infrastructure API client (`pumas-library::PumasApi`).
- The resolver may only return conservative non-ready states when the API or model identity is unavailable.
- Resolver must not perform speculative local dependency authority logic.
- Pantograph must not mutate `pumas-library` dependency tables directly; dependency writes belong to `pumas-library` APIs.

## Frozen Commands and Request Context

Pantograph command surface:

- `resolve_model_dependency_plan`
- `check_model_dependencies`
- `install_model_dependencies`
- `get_model_dependency_status`
- `list_models_needing_review`
- `submit_model_review`
- `reset_model_review`
- `get_effective_model_metadata`

Request context fields:

- `node_type`
- `model_path`
- optional `model_id`
- optional `model_type`
- optional `task_type_primary`
- optional `backend_key`
- optional `platform_context`
- optional `selected_binding_ids`

`platform_context` is normalized to a stable `platform_key` string in resolver logic.

## Frozen Result States and Codes

Pantograph dependency state enum includes:

- `ready`
- `missing`
- `installing`
- `failed`
- `unknown_profile`
- `manual_intervention_required`
- `profile_conflict`
- `required_binding_omitted`

Canonical non-ready codes:

- `unknown_profile`
- `manual_intervention_required`
- `profile_conflict`
- `required_binding_omitted`

Mapping rule:

- Pumas `error_code=required_binding_omitted` maps to Pantograph `state=required_binding_omitted`.

## Frozen DTO Shapes (Pantograph)

Plan:

- top-level: `state`, `code`, `message`, `review_reasons`, `plan_id`
- binding list: `binding_id`, `profile_id`, `profile_version`, `profile_hash`, `binding_kind`, `backend_key`, `platform_selector`, `env_id`
- selection data: `selected_binding_ids`, `required_binding_ids`

Status/install:

- top-level: `state`, `code`, `message`, `review_reasons`, `plan_id`, timestamp (`checked_at`/`installed_at`)
- per-binding rows include state and component buckets (`missing_components`, `installed_components`, `failed_components`) for UI rendering.

Model ref contract:

- `contract_version=2`
- `engine`, `model_id`, `model_path`, `task_type_primary`
- `dependency_bindings[]`
- optional `dependency_plan_id`

## Determinism Rules

- Resolver output ordering for bindings is deterministic (priority then `binding_id`).
- Cache keys include model identity, backend key, platform key, and selected binding set.
- `plan_id` format is deterministic:
  - `{model_id}:{backend_key|unspecified}:{platform_key}:{selected_binding_ids_csv}`
