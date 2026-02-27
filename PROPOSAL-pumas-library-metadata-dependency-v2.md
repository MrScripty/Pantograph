# Proposal: Pumas Library Metadata v2 + Model Dependency System

## Status
Proposed

## Audience
Pumas Library team and Pantograph team

## Problem Statement
Pantograph depends on Pumas metadata for model selection and execution routing. Current metadata has gaps that block reliable multi-model inference:

1. Taxonomy can be inconsistent with directory placement (example: Stable Audio model stored under `llm/...` but `model_type=audio`).
2. `ModelMetadata` does not provide a first-class model dependency contract.
3. Dependency install/check exists in `pumas-app-manager` at app/version scope, not model scope.
4. Inference settings defaults are strong for LLM/diffusion but do not currently define audio defaults.

This proposal introduces a metadata and dependency architecture that is programmatic-first, migration-safe, and reviewable for ambiguous cases.

## Goals

1. Make model taxonomy deterministic and path-consistent.
2. Add explicit model-level dependency metadata and reusable dependency profiles.
3. Add model-level dependency APIs in `pumas-core`.
4. Support programmatic metadata extraction with optional human/agent review for hard cases.
5. Migrate existing model library contents to the new system with auditable reports.

## Non-Goals

1. Preserve backward compatibility for old metadata schema.
2. Solve all ecosystem-specific package management in a single first release.
3. Infer perfect metadata from arbitrary custom repos without a review path.

## Current-State Evidence

1. Import path is derived from `model_type/family/name`.
2. `ModelMetadata` has no model dependency fields.
3. Dependency manager in app-manager is version-scoped.
4. Existing stable-audio record demonstrates taxonomy drift.

## Proposal Summary

1. Introduce `ModelMetadata` schema v2.
2. Introduce reusable `DependencyProfile` resources.
3. Add model dependency APIs in `pumas-core`.
4. Add deterministic classification pipeline + review queue.
5. Ship one-time migration + idempotent repair/re-run support.

## ModelMetadata v2

### New Required Fields

1. `schema_version: u32`
2. `task_type_primary: String` (example: `text-to-audio`)
3. `input_modalities: Vec<String>`
4. `output_modalities: Vec<String>`
5. `classification_source: String` (example: `pipeline_tag`, `config`, `heuristic`)
6. `classification_confidence: f32` (`0.0..=1.0`)

### New Optional Fields

1. `task_type_secondary: Vec<String>`
2. `runtime_engine_hints: Vec<String>` (example: `pytorch`, `stable_audio`)
3. `dependency_profile_id: Option<String>`
4. `requires_custom_code: bool`
5. `custom_code_sources: Vec<CustomCodeSource>`
6. `model_card_local_path: Option<String>`
7. `model_card_source_url: Option<String>`
8. `model_card_hash: Option<String>`
9. `metadata_needs_review: bool`
10. `review_reason: Option<String>`
11. `review_status: Option<String>` (`pending`, `approved`, `rejected`)
12. `reviewed_by: Option<String>`
13. `reviewed_at: Option<String>`

### Validation Rules (Boundary-Level)

1. `model_type` must match known enum.
2. `task_type_primary` must be a known task tag.
3. `input_modalities` and `output_modalities` must be non-empty.
4. `classification_confidence` must be bounded.
5. If `requires_custom_code=true`, `custom_code_sources` cannot be empty.
6. If `dependency_profile_id` is set, referenced profile must exist.

## Task Taxonomy

### Strategy

1. Keep broad `model_type` (`llm`, `diffusion`, `audio`, `vision`, `embedding`).
2. Add task-level semantics for routing and UX:
   - examples: `text-to-audio`, `audio-to-text`, `text-generation`, `text-to-image`.
3. Keep `pipeline_tag` as provenance, not sole source of truth.

### Classification Precedence

1. HuggingFace `pipeline_tag` (if present).
2. Explicit model spec type from import request.
3. On-disk config/model file heuristics.
4. Unknown fallback + review flag.

## Dependency Profiles

### Purpose
Avoid duplicating dependency lists across model records. Model records point to a reusable profile.

### Proposed Profile Shape

1. `profile_id`
2. `profile_version`
3. `environment_kind` (`python-venv`, `binary`, `mixed`)
4. `requirements_sources` (files, inline requirements, wheel URLs, git refs)
5. `platform_constraints` (os, arch, cuda/rocm)
6. `install_policy` (`on-demand`, `eager`, `manual-approval-required`)
7. `validation_probes` (import checks, executable checks)
8. `notes`

### Example
`stable-audio-open-1.0` profile points to Python deps including `stable_audio_tools`, `torch`, `torchaudio`, plus optional platform-specific constraints.

## New APIs in pumas-core

1. `get_model_dependency_profile(model_id)`
2. `check_model_dependencies(model_id, env_id, platform_context)`
3. `install_model_dependencies(model_id, env_id, platform_context)`
4. `list_models_needing_review(filter)`
5. `submit_model_review(model_id, patch, reviewer)`

Notes:
1. Keep app/version dependency APIs in `pumas-app-manager`; do not remove them.
2. Model APIs are additive and separate from version manager behavior.

## Model Card Ingestion

1. Copy model card/readme into the model directory as raw artifact.
2. Store path/hash/source URL in metadata.
3. Extract structured hints programmatically (task clues, custom code clues, dependency hints).
4. Flag uncertain extraction for review queue.

## Human/Agent Review Workflow

### Trigger Conditions

1. Conflicting classification signals.
2. Missing dependency profile match.
3. Custom code requirements detected.
4. Unsupported task taxonomy output.

### Process

1. Deterministic extractor writes baseline metadata.
2. Record `metadata_needs_review=true`.
3. Human/agent submits patch with provenance.
4. Review state updated in metadata.

## Migration Plan

### Scope
All existing model records and directories in the current library root.

### Phase A: Preflight

1. Snapshot index and metadata files.
2. Verify disk space and permissions.
3. Produce dry-run move plan and classification report.

### Phase B: Classification

1. Reclassify each model using precedence pipeline.
2. Generate target path (`model_type/family/name`).
3. Mark ambiguous models for review.

### Phase C: Relocation + Rewrite

1. Move directory to new canonical path if needed.
2. Rewrite metadata to schema v2.
3. Preserve old->new path mapping in migration report.

### Phase D: Reindex + Validate

1. Rebuild index.
2. Validate uniqueness and referential integrity.
3. Validate dependency profile references.

### Phase E: Dependency Attach

1. Auto-map known model families/tasks to dependency profiles.
2. Mark unmatched records `metadata_needs_review=true`.

### Migration Safety Rules

1. Append-only migration scripts.
2. Idempotent operations (`if exists`/`if not exists` style behavior).
3. Crash-safe checkpointing for resume.
4. Machine-readable and human-readable migration reports.

## Testing Plan

1. Unit tests for taxonomy classifier and validators.
2. Unit tests for dependency profile resolution.
3. Integration tests for model dependency check/install APIs.
4. Migration tests against realistic mixed libraries.
5. Recovery tests for interrupted migration.

## Acceptance Criteria

1. Stable Audio is classified and stored under `audio/...` with `task_type_primary=text-to-audio`.
2. Every model has schema version and valid modality/task metadata.
3. Model dependency APIs return actionable status for supported profiles.
4. Migration can be rerun without corruption.
5. Ambiguous models are explicitly marked for review, not silently guessed.

## Rollout

1. Release metadata/dependency changes behind feature flag in first cycle.
2. Run migration tool in dry-run on representative real library.
3. Resolve high-priority review queue entries.
4. Enable new schema by default after validation threshold is met.

## Deliverables

1. ADR: Metadata v2 and taxonomy decisions.
2. ADR: Dependency profile architecture.
3. Migration utility + report format spec.
4. API docs for model dependency endpoints.
5. Review queue operational guide.

