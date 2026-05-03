# Plan: Pumas Library Model Package Facts

## Objective

Update the Pumas Library side of the model-package boundary so any host
application, including Pantograph, can treat Pumas as the canonical model source
while consuming Transformers-aligned package facts, task evidence, artifact
facts, dependency state, and provenance without taking ownership of Pumas
model-library policy.

Transformers is used here as the reference vocabulary for package layout,
component discovery, Auto-class metadata, processor composition, generation
defaults, task ids, and trust/custom-code evidence. It must not become the
model registry authority or replace Pumas stable model identity.

The implementation target is:

```text
/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library
```

This plan is split from the Pantograph inference boundary plan so Pumas-specific
metadata, validation, projection, and model-library event work can be designed
and implemented in the Pumas repository without making any consumer responsible
for Pumas library indexing, import, deduplication, migration, dependency
binding, or storage policy.

Relevant Transformers source was reviewed from the local checkout:

```text
/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/transformers
```

The source review reinforced that Pumas should expose bounded package facts
rather than Python objects. The relevant shape is `from_pretrained`-style local
package evidence: `config.json`, `generation_config.json`, tokenizer files,
processor/preprocessor/image/video/audio component configs, chat templates,
weight/index/shard files, Auto-class `model_type` and `auto_map` metadata,
pipeline/task tags, and explicit trust requirements for custom code.

## Scope

### In Scope

- Define a consumer-visible `PumasModelRef` contract around stable `model_id`,
  optional revision/generation, optional selected artifact, and migration
  diagnostics.
- Define validated domain types for legacy path inputs, direct Hugging Face repo
  ids, raw GGUF paths, backend hints, artifact kinds, and selected artifact ids
  before consumers or Pumas internals consume them.
- Extend or define Pumas package facts for artifact kind, executable entry path,
  companion artifacts, storage kind, validation state, dependency bindings,
  backend hints, and provenance.
- Add a separate, versioned `ResolvedModelPackageFacts` projection instead of
  broadening `ModelExecutionDescriptor` into the full package-facts contract.
  `ModelExecutionDescriptor` should remain a compact execution-facing summary.
- Preserve component-file presence facts, including `config.json`, tokenizer
  files, tokenizer config, processor config, preprocessor config, image
  processor config, video processor config, audio feature extractor config,
  chat template, generation config, model index, weight index, shards, adapters,
  and quantization metadata where available.
- Preserve Transformers-aligned evidence where available, including raw
  `pipeline_tag`, `config.model_type`, `architectures`, `dtype`/`torch_dtype`,
  `auto_map`, tokenizer/processor file presence, chat template presence,
  generation config presence, weight format, selected files, and source
  revision/commit evidence.
- Preserve Auto-class and processor discovery evidence without loading Python
  classes, including `AutoConfig`, `AutoModel*`, and `AutoProcessor` `auto_map`
  entries, `processor_class`, tokenizer config, preprocessor config, image
  processor config, video processor config, and audio feature extractor config.
- Preserve task evidence at two levels: exact upstream task or pipeline tag for
  ecosystem compatibility, and normalized Pumas modality signature for
  consumer routing and validation.
- Preserve generation defaults parsed from `generation_config.json` or legacy
  config-carried generation fields as model-provided defaults, not
  consumer-authored options and not Pumas UI/runtime settings.
- Preserve custom-code evidence such as `requires_custom_code` and
  `custom_code_sources` so inference can enforce explicit trust policy. This
  includes `auto_map` entries, custom config/model/processor classes, custom
  generation methods where detectable, upstream repo references, and dependency
  manifests such as `requirements.txt` when present.
- Define host-agnostic backend-hint vocabulary for Pumas projections:
  `transformers`, `llama.cpp`, `vllm`, `mlx`, `candle`, `diffusers`, and
  `onnx-runtime`.
- Preserve ecosystem hints such as `ollama` as facts without making them
  executable selections. Individual consumers decide whether a hint is
  supported, unsupported, or migration-only.
- Keep GGUF, safetensors, diffusers bundles, ONNX, HF-compatible directories,
  and future formats as first-class Pumas artifact kinds.
- Define unresolved behavior when a legacy path cannot resolve to Pumas:
  emit a migration diagnostic and require user selection instead of preserving
  the raw path as canonical identity.
- Expose host-agnostic model-library change notifications or equivalent update
  cursors for added models, removed models, modified metadata, modified package
  facts, stale fact invalidation, and dependency-binding changes so consumers
  can keep local caches current.

### Out of Scope

- Implementing consumer backend execution.
- Selecting which runtime should execute consumer work.
- Scheduler admission, queueing, reservation, retention, eviction, or batching
  policy.
- Making Transformers the model registry authority.
- Removing or changing Pumas indexing/import/storage policy unless required to
  expose the package facts above.
- Writing diagnostics ledger events directly from Pumas.
- Defining whether any particular consumer supports Ollama execution.

## Inputs

### Problem

Host applications want to standardize around Transformers-aligned model package
and task semantics while keeping Pumas as the canonical model source. Pumas
already stores model identity, metadata, artifact state, validation state,
dependency bindings, and provenance, but its current execution-facing
descriptor is too small to carry the full package evidence consumers need for
compatibility checks, migration diagnostics, task validation,
generation-default merging, and trust policy.

The work must expose richer facts without making Pumas responsible for
consumer runtime selection, inference execution, workflow policy, scheduler
policy, or diagnostics-ledger writes.

### Standards Reviewed

- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/COMMIT-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CONCURRENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/FRONTEND-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/SECURITY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DEPENDENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TOOLING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-API-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-ASYNC-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-SECURITY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-TOOLING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/templates/PLAN-TEMPLATE.md`

### Standards Compliance Findings

| Area | Finding | Plan Response |
| ---- | ------- | ------------- |
| Plan structure | The plan needs explicit problem, assumptions, dependencies, risks, and traceability, not only milestones. | Add this `Inputs` section, risk table, lifecycle/facade notes, and traceability requirements before implementation. |
| Immutable contracts | Pumas and its consumers can drift independently across repository and process boundaries. | Freeze `ResolvedModelPackageFacts` and fixture schemas before broad extraction work; keep changes append-only unless producer/consumer contracts synchronize. |
| Executable boundary contracts | Package facts are persisted/cross-process structured data, so plain Rust/TS types are insufficient. | Require decode/normalize/round-trip fixture tests and optional schema-backed fixtures for changed DTOs. |
| Structured producer contract | Pumas publishes machine-consumed metadata whose defaults, enum meanings, and ordering semantics matter. | Define default semantics, volatile fields, enum casing, ordering guarantees, and regeneration/migration rules in the projection contract. |
| Security | Legacy paths, repo ids, `auto_map`, custom code, and dependency manifests cross trust boundaries. | Parse once at Pumas boundaries into validated domain types; never execute Python or load Transformers Auto classes for fact extraction. |
| Interop | Rust, TypeScript, RPC/IPC, host fixtures, and saved references need matching wire casing and enum labels. | Add contract round-trip tests and consumer fixtures for Pumas package facts. |
| Rust API design | Raw strings for model refs, artifact kinds, backend hints, and trust states are high-risk across module boundaries. | Introduce validated domain types and enums for persisted and boundary-facing values before internal consumption. |
| Dependency ownership | Pumas should not gain heavy runtime dependencies just to inspect Transformers packages. | Prefer standard-library/serde JSON parsing and existing Pumas helpers; do not add Python/Transformers runtime dependencies to Pumas. |
| Testing | Unit tests alone do not prove consumers can use the facts. | Require fixture round trips plus at least one consumer acceptance fixture before widening support. |
| Documentation | Source or contract changes require README/ADR traceability. | Update affected Pumas module READMEs or add ADRs when implementation changes public DTOs or persistence semantics. |
| Durable storage cleanup | SQLite projection cleanup can corrupt consumer assumptions if treated as ad hoc field deletion. | Treat cleanup as a versioned projection migration with dry-run output, explicit field ownership, rollback/rebuild guidance, and source metadata preservation. |
| Lazy cache correctness | Lazy package facts introduce stale-cache and overlapping-regeneration risks. | Key cached facts by artifact signature and contract version, require idempotent regeneration, and verify cache-hit/stale/missing/recovery paths. |
| Worktree hygiene | Implementation spans Pantograph docs and Pumas code, and the plan artifact is currently untracked in Pantograph. | Add an implementation-readiness gate requiring clean implementation worktrees, tracked/committed plan state, and verified logical-slice commits. |
| Commit discipline | The plan needs explicit commit cadence aligned with conventional commits and history cleanup. | Keep storage additions, projection cleanup, GUI changes, and fixture sync in separate verified commits; record verification in the plan/PR, not commit messages. |

### Assumptions

- Pumas remains the source of stable model identity and artifact/provenance
  facts; each consumer remains the owner of inference compatibility checks and
  runtime execution.
- Existing Pumas metadata can be projected into richer package facts without an
  immediate persisted schema migration for every field.
- Some package facts are volatile inspection facts and may be recomputed at
  resolution time rather than persisted.
- Transformers package evidence can be extracted from files with bounded JSON
  parsing and filesystem inspection, without importing Python code.
- Consumers can consume fixtures or generated package facts while full Pumas
  extraction support is implemented incrementally.
- SQLite `metadata_json` cleanup only changes the indexed projection unless a
  later milestone explicitly approves source `metadata.json` migration.
- Full package facts can be regenerated from stable model/artifact metadata and
  bounded package-file inspection when the lazy cache is missing or stale.

### Dependencies

- Pumas `ModelLibrary`, metadata v2, import classification, dependency
  resolution, and external-asset validation APIs.
- Existing Pumas RPC/UniFFI/TypeScript boundary conventions for serialized DTOs.
- Local Transformers source reference for package evidence vocabulary.
- Parent Pantograph inference plan and future host consumer contracts.
- Existing Pumas testing layout under `rust/crates/pumas-core/tests/` and
  frontend/RPC contract tests where DTOs cross into TypeScript.
- Existing Pumas SQLite migration, reconciliation, metadata overlay/history,
  and dry-run reporting patterns.
- HF search cache behavior in `shared-resources/cache/search.sqlite` for remote
  compatibility hints such as MLX and vLLM.

### Risks And Mitigations

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| `ResolvedModelPackageFacts` grows into runtime selection policy | High | Keep backend hints advisory and leave compatibility/execution decisions in host applications. |
| `ModelExecutionDescriptor` becomes overloaded with package evidence | Medium | Keep it compact; expose richer evidence through a separate versioned projection. |
| Transformers evidence extraction starts executing Python or loading custom code | High | Restrict Pumas extraction to file parsing and mark trust requirements as facts for consumers to enforce. |
| Generation defaults are confused with Pumas `inference_settings` | High | Keep `GenerationDefaultFacts` separate and explicitly label Pumas `inference_settings` as UI/runtime parameter schemas. |
| Backend hint vocabulary breaks existing Pumas consumers | Medium | Add host-agnostic backend-hint facts without renaming global Pumas metadata semantics. |
| Legacy path resolution silently selects the wrong model | High | Emit unresolved migration diagnostics unless a stable Pumas model/artifact match is deterministic. |
| Fixture contracts drift between Pumas and consumers | High | Add shared fixture review and round-trip tests before implementation slices are considered done. |
| Component facts become unbounded filesystem scans | Medium | Bound scans to selected artifact roots and known Transformers/Pumas component patterns. |
| Persisted metadata changes require migration after implementation starts | Medium | Classify stable persisted fields versus volatile inspection facts during Milestone 1 and re-plan if migration is required. |
| Lazy package-fact cache serves stale facts after artifact changes | High | Include artifact signature and contract version in cache keys; invalidate on path/size/mtime/hash/revision/dependency changes. |
| Concurrent requests regenerate the same package facts and race writes | Medium | Make regeneration idempotent and write facts atomically through existing SQLite ownership patterns. |
| SQLite cleanup removes data still needed by API/frontend consumers | High | Define field ownership first, run dry-run reports, and update consumers to read dedicated fields before write-mode cleanup. |
| Cleanup is mistaken for source metadata deletion | Medium | Limit cleanup to indexed projection unless an explicit source metadata migration is separately approved. |
| HF search compatibility tags overstate MLX or vLLM support | Medium | Label tags as discovery hints derived from compact evidence, not executable backend guarantees. |
| Model-list consumers serve stale cached facts after library changes | Medium | Expose host-agnostic model-library change notifications or cursors and require consumers to refresh affected model/fact cache entries. |

### Public Facade And Lifecycle Notes

- Preserve existing public facade entry points where practical, especially
  `ModelExecutionDescriptor` and current model-library metadata/RPC accessors.
- Add new projection APIs instead of breaking existing descriptor consumers
  unless an explicit migration plan is approved.
- Treat `ModelExecutionDescriptor` as the current compact execution-facing
  summary, not as deprecated legacy API. It should remain useful for callers
  that only need entry path, model/task summary, validation state, storage
  state, backend hints, and dependency resolution.
- Fact extraction must be request-scoped or reconciliation-scoped. Pumas should
  not introduce long-lived background polling, timers, runtime processes, or
  Python workers for this work.
- Any filesystem refresh/revalidation must be idempotent and bounded to the
  relevant model/artifact root. Cancellation and cleanup should follow existing
  Pumas async/task ownership patterns if async work is added.

### Documentation And Traceability Requirements

- Update affected Pumas source READMEs when implementation changes module
  responsibilities, public DTOs, or structured producer contracts.
- Add or update an ADR if `ResolvedModelPackageFacts` changes persisted
  metadata semantics, introduces a new contract module, or changes model-library
  authority boundaries.
- Fixture files must document producer version, expected consumer semantics,
  stable enum labels, omitted-field defaults, and intentionally volatile fields.
- PR summaries for implementation work must link this plan, affected Pumas
  README/ADR updates, and host consumer fixture changes.

### Affected Contracts And Artifacts

- Rust DTOs and projections that describe model refs, package facts, artifact
  facts, task evidence, backend hints, generation defaults, custom-code facts,
  and migration diagnostics.
- Pumas RPC, UniFFI, TypeScript, or other serialized boundaries that expose
  model-library facts outside the Rust crate.
- Model-library change notification or update-cursor contracts that let
  consumers refresh cached model-list and package-fact details after model
  added, model removed, metadata modified, package facts modified, stale facts
  invalidated, or dependency binding modified events.
- Pumas fixture JSON or snapshot artifacts used to prove producer semantics and
  consumer compatibility.
- Pumas metadata and import/index outputs only where stable persisted fields are
  required. Volatile inspection facts should remain recomputable unless a
  milestone explicitly chooses persistence.
- SQLite `models.metadata_json` projection, `tags_json`, `hashes_json`, and any
  new package-fact summary/detail tables used by lazy loading.
- HF search-cache repo detail projections that provide compact remote
  compatibility hints for MLX and vLLM.
- Dry-run and migration report artifacts that document SQLite cleanup results,
  preserved exception fields, and before/after payload-size changes.
- Host consumer fixtures that consume Pumas facts without reaching into
  Pumas storage or indexing internals.

### Structured Producer Contract Rules

- Boundary-facing enum labels, field casing, omitted-field defaults, and
  producer-version semantics must be specified before implementation begins.
- Unknown, unsupported, missing, invalid, and uninspected states must remain
  distinct where consumers make different decisions from those states.
- Collections that influence matching or diagnostics must define ordering
  guarantees. If ordering has no semantic meaning, producers should emit stable
  deterministic order for fixture and review stability.
- Volatile fields must be labeled as such and excluded from saved consumer
  identity decisions.
- Regenerated facts must be deterministic for the same model artifact and
  source revision unless they include explicitly volatile inspection metadata.
- Projection cleanup must preserve a documented source of truth for every
  removed field. Dedicated columns, structured tables, source metadata files,
  or user metadata must be named before a field is removed from
  `models.metadata_json`.
- Migration reports must be deterministic and reviewable before write-mode
  cleanup is enabled.

### Concurrency And Race-Risk Review

- Contract definition and fixture authoring are document/test work and have no
  long-lived concurrency requirements.
- Fact extraction should be request-scoped or reconciliation-scoped, bounded to
  a selected model/artifact root, and safe to retry.
- Refresh or revalidation work must tolerate the model artifact changing or
  disappearing between directory listing, JSON parsing, and projection
  generation.
- Lazy package-fact generation must tolerate simultaneous readers and duplicate
  requests. Concurrent regeneration should either coalesce through existing
  ownership patterns or allow last-writer-same-value atomic replacement.
- SQLite cleanup must not run concurrently with index rebuild/reconciliation
  unless the operation is explicitly serialized through the same index owner.
- If asynchronous refresh work is introduced, cancellation and cleanup must use
  existing Pumas ownership patterns, and no task may continue mutating shared
  state after its owning request/import/reconciliation context has ended.
- Cross-repo fixture updates must be reviewed as a synchronized producer and
  consumer contract change, not as independent repository-local snapshots.

## Structured Contracts

- `PumasModelRef`: stable consumer reference to a Pumas model and
  optional selected artifact/revision.
- `ModelExecutionDescriptor`: existing Pumas execution-facing summary that
  should remain focused on resolved execution entry point, model id, model/task
  summary, validation state, storage state, backend hints, and dependency
  resolution.
- `ResolvedModelPackageFacts`: proposed consumer-facing projection containing
  artifact kind, entry path, component facts, task evidence, generation defaults,
  dependency state, backend hints, security/trust facts, and provenance.
- `ResolvedArtifactFacts`: artifact-specific facts for GGUF, HF-compatible
  directories, safetensors, diffusers bundles, ONNX, adapters, shards, and
  companion artifacts such as GGUF `mmproj`.
- `TransformersPackageEvidence`: HF/Transformers-compatible local package facts
  such as `config.json` presence/parse status, `config.model_type`,
  `architectures`, `dtype`/`torch_dtype`, `_commit_hash`, `auto_map`,
  tokenizer/preprocessor/processor/image/video/audio component config presence,
  chat template files, `generation_config.json`, weight/index/shard files, and
  selected/sibling file evidence.
- `ProcessorComponentFacts`: component-level facts for tokenizer, processor,
  image processor, video processor, audio feature extractor, feature extractor,
  chat template, and additional named sub-processors without serializing Python
  processor objects.
- `TaskEvidence`: raw upstream task/pipeline tag plus normalized Pumas modality
  signature.
- `GenerationDefaultFacts`: model-provided generation defaults parsed from model
  package metadata where available, especially `generation_config.json`, kept
  separate from Pumas `inference_settings`.
- `CustomCodeFacts`: `requires_custom_code`, custom code sources, review state,
  trust requirements, `auto_map` sources, custom processor/config/model class
  references, custom generation evidence, and dependency-manifest evidence.
- `BackendHintFacts`: ecosystem/runtime hints expressed in stable vocabulary
  without making Pumas choose the runtime.

## Constraints

- Pumas remains the canonical model source and owns stable model identity,
  artifact facts, dependency bindings, validation state, and provenance.
- Pumas supplies facts and hints; consumers perform compatibility checks and
  backend execution.
- Raw paths and direct repo ids are import/debug/compatibility inputs, not the
  preferred saved consumer contract.
- Legacy paths must go through centralized path validation/resolution.
- Package facts must be bounded, serializable, and fixture-testable.
- Package facts must not include prompt text, generated output, embeddings,
  tensors, Python objects, backend command flags, or scheduler policy.
- Transformers-aligned facts must be parsed as evidence from package files;
  Pumas must not import arbitrary Python modules or instantiate Transformers
  Auto classes to produce package facts.
- Pumas `inference_settings` remain UI/runtime parameter schemas. They must not
  be conflated with model-provided `GenerationConfig` defaults.
- New or changed Pumas DTOs crossing into host applications need decode/normalize or
  round-trip fixture tests.
- Host applications should consume versioned Pumas DTO/API output, not
  `models.metadata_json`, SQLite table layouts, or search-cache internals.
- Remote Hugging Face search tags for MLX, vLLM, or other engines are discovery
  hints only. Installed-model compatibility should use local package facts and
  consumer-side runtime compatibility checks.
- Breaking persisted metadata changes require a Pumas-side migration plan.

## Recommended Design Position

- Keep `ModelExecutionDescriptor` as the small execution summary it is today.
  Add `ResolvedModelPackageFacts` as a richer versioned projection for
  inference compatibility, migration, and trust decisions.
- Treat Transformers as a package vocabulary source, not a dependency or an
  authority. Pumas should inspect known files and preserve evidence; consumers
  can decide whether a backend may execute that package.
- Keep path/repo inputs at the boundary as validated domain values. Saved
  consumer references should move toward `PumasModelRef`, selected artifact ids,
  and explicit migration diagnostics.
- Add contract fixtures before broad implementation so enum labels, default
  behavior, and unsupported-state semantics are reviewable by both Pumas and
  consumers.

## GUI Impact And UX Boundaries

- Do not add new compatibility badges to the installed model-library row list.
  The local library should remain focused on model name, format, quantization,
  size, dependency presence, download state, integrity state, and existing row
  actions.
- Add MLX and vLLM compatibility tags to Hugging Face search results when
  Pumas has enough package or catalog evidence to report them. These tags
  should be search/discovery hints only, not saved consumer identity and not
  runtime selection.
- Add a read-only `Execution Facts` surface to the model metadata modal for
  `ResolvedModelPackageFacts`, `ModelExecutionDescriptor`,
  `TransformersPackageEvidence`, `TaskEvidence`, `GenerationDefaultFacts`,
  `CustomCodeFacts`, backend-hint facts, dependency state, and validation
  diagnostics.
- Keep editable Pumas `inference_settings` separate from model-provided
  generation defaults. The metadata modal may display `GenerationDefaultFacts`,
  but must not merge them into the existing editable inference-settings schema.
- Surface package evidence earlier in the import flow before final import where
  practical: artifact kind, detected package family, required components,
  missing/invalid tokenizer or processor files, chat-template presence,
  generation-config parse state, custom-code-required state, dependency
  manifests, backend-hint evidence, and unsupported-consumer diagnostics such
  as `ollama`.
- Avoid expensive package-fact extraction in frequently rendered list rows.
  Search/import/detail views can fetch or compute richer facts because the user
  has already narrowed the context.

## SQLite Index And Lazy Package Facts

Current Pumas persistence has two relevant SQLite stores:

- Installed model library index:
  `shared-resources/models/models.db`.
- Hugging Face search cache:
  `shared-resources/cache/search.sqlite`.

Observed installed-library index shape:

- The primary `models` table stores `id`, `path`, `cleaned_name`,
  `official_name`, `model_type`, `tags_json`, `hashes_json`, `metadata_json`,
  and `updated_at`.
- `metadata_json` carries the broad projected metadata payload. It is flexible,
  but expensive to query for structured compatibility facts if every consumer
  has to deserialize or `json_extract` large blobs.
- FTS5 indexes `id`, `official_name`, `cleaned_name`, `model_type`, `tags`,
  `family`, and `description`, with triggers populated from `models` and
  selected `metadata_json` fields.
- Metadata v2 governance is already structured in SQLite through
  `task_signature_mappings`, `model_type_arch_rules`,
  `model_type_config_rules`, immutable metadata baselines, overlays, history,
  dependency profiles, and model dependency bindings.
- This means Pumas already has the right pattern for stable facts that need
  queryability: keep contract-specific table families next to the index instead
  of relying only on a large metadata JSON projection.

Observed Hugging Face search-cache shape:

- `search_cache` stores normalized query windows as repo id lists.
- `repo_details` stores repo-level details such as `formats`, `quants`,
  `download_options`, downloads, size, URL, and timestamps.
- Compatible engines are currently derived from cached `formats` when results
  are converted back into `HuggingFaceModel`, not persisted as first-class
  search-cache facts.
- This is the right surface for adding MLX and vLLM search/discovery tags,
  because it affects remote discovery without changing installed-library rows.

Lazy-loading policy for package facts:

- Do not eagerly compute full `ResolvedModelPackageFacts` for every installed
  model during list/search/rebuild operations.
- Store a small, indexed package-fact summary only when needed for fast checks:
  contract version, model id, selected artifact id/path identity, artifact
  signature, artifact kind, package family, primary task evidence, backend-hint
  summary, component-summary state, custom-code-required state,
  generation-config parse state, validation summary, inspected timestamp, and
  facts hash.
- Store full package facts in a separate cache/table keyed by model id,
  selected artifact, contract version, and artifact signature. Full facts
  should be loaded or regenerated only for import review, metadata modal
  `Execution Facts`, host export/contract calls, or explicit validation.
- Treat package facts as stale when the selected artifact path, size, mtime,
  content hash, source revision, metadata schema version, dependency binding,
  or package-facts contract version changes.
- On detail requests, return fresh cached facts when available; otherwise run a
  bounded filesystem/package inspection, persist the result, and return it with
  parse diagnostics.
- On import, compute enough evidence for review before final import, then reuse
  that evidence to seed the lazy facts cache after the model id/artifact id is
  finalized.
- On library rebuild/reconciliation, refresh only cheap summary invalidation
  state unless a caller explicitly requests full package-fact regeneration.
- Keep the installed model-library list query on the existing `models` and FTS
  surfaces. It should not join or deserialize full package-fact blobs.

Host cache/update policy:

- Pumas should expose host-agnostic change notifications, update cursors, or
  monotonic revision facts so consumers can keep local model-list/detail caches
  current.
- Consumers may cache model-list rows and package-fact details during startup or
  page population, but should invalidate or refresh affected entries when Pumas
  reports model added, model removed, metadata modified, package facts modified,
  stale facts invalidated, or dependency binding modified events.
- Pumas remains the backend-owned source of truth. Consumer caches are
  projections for responsiveness and must not become alternate model-library
  state.
- Change events should identify the affected model id, selected artifact id
  where available, changed fact family, producer revision/update cursor, and
  whether consumers should drop detail facts or refresh summary rows.

SQLite alignment opportunities:

- Add a dedicated package-facts table family instead of expanding
  `metadata_json` with large nested package evidence.
- Add targeted indexes for stale-summary lookups, model/artifact lookup, and
  contract-version invalidation.
- Consider `CHECK (json_valid(...))` constraints for JSON summary/detail fields
  and immutable history rows for package-fact contract migrations, matching the
  existing metadata-baseline/overlay approach.
- Keep Transformers-aligned fields queryable only where Pumas needs routing or
  diagnostics. Examples: `config.model_type`, task signature, artifact kind,
  backend-hint labels, custom-code-required state, and component/generation
  parse states. Large arrays such as sibling files, shard lists, and raw config
  snippets can remain in lazy detail JSON.
- Add MLX and vLLM compatibility derivation to HF search/catalog details from
  file formats, tags, config/model-type evidence, and sibling files where
  available. Persist or cache only the compact remote compatibility summary
  needed for search rendering.

Metadata cleanup opportunities from the current `models.db` sample:

- The current live index sample contains 48 model rows. In that sample,
  `metadata_json.model_id`, `metadata_json.model_type`,
  `metadata_json.cleaned_name`, and `metadata_json.official_name` are fully
  redundant with dedicated `models` columns.
- `metadata_json.hashes` duplicates `hashes_json` for populated rows.
  `metadata_json.tags` overlaps `tags_json` and should be normalized to one
  owner.
- The following fields are present across rows but not currently meaningful in
  the sample and should be removed from the projected index payload unless a
  contract owner is identified: `compatible_apps`, `conversion_source`,
  `last_lookup_attempt`, `license_artifact`, `model_card_artifact`,
  `reviewed_at`, `reviewed_by`, `subtype`, and `validation_errors`.
- Preserve `license`, `license_status`, `model_card`, `notes`, and
  `preview_image` even when currently sparse or empty. These are user-facing or
  provenance-facing fields with clear future value and should not be removed as
  part of cleanup.
- Treat cleanup as a projection/schema migration, not as deletion of source
  metadata. If original metadata files retain extra fields for provenance, the
  SQLite index projection should still avoid duplicating column-owned or
  meaningless values.
- Add a dry-run report before mutation that lists affected fields, affected
  rows, before/after payload sizes, and any fields preserved by exception.

## Verification Strategy

- Unit and fixture tests should cover package parsing for GGUF, HF-compatible
  directories, safetensors, diffusers bundles, ONNX, adapters, shards, missing
  components, invalid JSON, and unsupported backend hints.
- Serialization tests should round-trip every changed boundary DTO and prove
  default/unknown/unsupported states survive Rust-to-wire-to-Rust or
  Rust-to-TypeScript boundaries.
- Security tests should cover traversal, symlink, non-existent path, allowed
  import root, untrusted `auto_map`, custom-code-required packages, and
  dependency-manifest evidence.
- Interop tests should include host-owned consumer fixtures for at least
  one GGUF text-generation case, one HF/Transformers text-generation case, one
  multimodal/processor case, and one unsupported/migration diagnostic case.
- Durable-state tests should isolate SQLite database files per test, cover
  dry-run versus write-mode cleanup, prove idempotent reruns, and verify
  recovery after stale or missing package-fact cache rows.
- Cache tests should cover fresh hit, stale hit, concurrent duplicate
  regeneration, artifact-signature changes, contract-version changes, and
  missing artifact behavior.
- Tooling verification should use the repository's normal Rust and frontend
  commands for the touched crates/packages. If the implementation changes only
  contracts and docs, fixture/serialization tests are still required.

## Required Fixture Set

The first producer/consumer fixture set should include stable, named JSON files
or equivalent schema-backed snapshots for:

- `gguf_text_generation_package_facts.json`: GGUF artifact with text-generation
  task evidence, llama.cpp-compatible hint facts, quantization facts, and no HF
  tokenizer/processor requirements.
- `gguf_embedding_package_facts.json`: GGUF artifact with embedding or
  feature-extraction task evidence and backend hint facts that prove embeddings
  are represented as normal model tasks.
- `hf_transformers_text_generation_package_facts.json`: HF-compatible package
  with `config.json`, tokenizer files, chat-template evidence, generation
  defaults, and Transformers/PyTorch-compatible package evidence.
- `hf_multimodal_processor_package_facts.json`: HF-compatible package with
  processor, image/audio/video component evidence, multimodal task evidence,
  and missing/required component diagnostics.
- `custom_code_required_package_facts.json`: package with `auto_map`, custom
  class evidence, dependency manifest evidence, and explicit trust/custom-code
  requirements.
- `unsupported_ollama_hint_package_facts.json`: package preserving an `ollama`
  ecosystem hint as evidence while making clear that executable support is a
  consumer decision.
- `stale_package_facts.json`: cached package-facts summary/detail with an
  outdated artifact signature or contract version and explicit stale-state
  semantics.
- `invalid_generation_config_package_facts.json`: package with invalid or
  unparsable generation config and parse diagnostics distinct from unsupported
  model execution.
- `missing_tokenizer_package_facts.json`: package whose task evidence requires
  tokenizer support but tokenizer component facts are missing or invalid.
- `remote_search_mlx_vllm_hint.json`: Hugging Face search/cache result carrying
  MLX/vLLM discovery hints that are not installed-model compatibility facts.

Fixture rules:

- Fixtures must not depend on Pumas SQLite layout or `models.metadata_json`
  internals.
- Fixtures must document producer contract version, omitted-field defaults,
  unknown/unsupported/stale states, stable enum labels, ordering guarantees, and
  intentionally volatile fields.
- Host/consumer tests should be able to decode the fixtures without importing
  Pumas storage or indexing internals.

## Implementation Readiness Gate

Before implementation starts:

- Confirm the Pantograph plan file is tracked or intentionally committed as a
  documentation-only setup slice before Pumas code changes begin.
- Inspect `git status --short` in both Pantograph and Pumas. Do not start code
  changes while unrelated source, test, config, generated binding, lockfile, or
  build files are dirty unless that dirty state is explicitly accepted.
- Record the intended first implementation slice in this plan. The first slice
  should be Milestone 1 contract/fixture work only, because later persistence,
  cleanup, and GUI work depend on frozen contract semantics.
- Confirm no generated bindings or cross-repo fixture files will be edited by
  multiple owners in the same slice.
- Identify the verification commands expected for the slice before changing
  code. At minimum, contract DTO changes need Rust tests; cross-boundary DTOs
  need serialization or binding tests; GUI changes need frontend typecheck and
  focused component/hook tests.
- Commit each verified logical slice before starting the next one. Use
  conventional commit messages and keep verification output in this plan,
  PR notes, or the completion summary rather than in commit messages.

Implementation should not begin if any of these gates fail. Update the plan or
re-plan before writing code.

## Milestones

### Milestone 1: Freeze Pumas-to-Inference Vocabulary

**Goal:** Define the exact Pumas facts host applications can consume.

**Tasks:**
- [x] Define `PumasModelRef`, resolved artifact id, artifact kind, backend hint,
  task evidence, component facts, generation defaults, custom-code facts, and
  migration diagnostic terms.
- [x] Define `ResolvedModelPackageFacts` as a separate versioned projection and
  keep `ModelExecutionDescriptor` as a compact execution-facing summary.
- [x] Define versioning and compatibility behavior for the projection.
- [x] Define which fields are stable persisted contract and which are volatile
  inspection facts.
- [x] Define `TransformersPackageEvidence`, `ProcessorComponentFacts`,
  `GenerationDefaultFacts`, and `CustomCodeFacts` field names, casing,
  parse-status values, and unsupported/unknown states.
- [x] Define how Pumas backend hints map into host-agnostic backend hint facts
  without renaming global Pumas metadata semantics.
- [x] Define host-agnostic model-library change event or update-cursor
  semantics for model added, model removed, metadata modified, package facts
  modified, stale facts invalidated, and dependency binding modified cases.

**Verification:**
- Review against host consumer contract requirements, including the parent
  Pantograph inference plan as one consumer.
- Fixture outline for GGUF, HF-compatible directory, safetensors, diffusers
  bundle, ONNX, missing tokenizer, missing processor, missing generation config,
  custom-code-required, and unsupported Ollama hint cases.
- Required fixture names from `Required Fixture Set` are accepted or revised
  before implementation begins.
- Review against local Transformers source for config, Auto-class,
  processor/tokenizer, generation config, task alias, and trust/custom-code
  evidence.

**Status:** Complete for the Milestone 1 contract baseline. Broader fixture
coverage continues under Milestone 5.

### Milestone 2: Expand Package Fact Extraction

**Goal:** Make Pumas able to expose enough package facts for inference
compatibility checks.

**Tasks:**
- [x] Extract artifact kind, entry path, storage kind, selected files, sibling
  files, shards, indexes, adapters, companion artifacts, and quantization facts.
  Basic artifact kind, entry path, storage kind, selected files, and `mmproj`
  companion detection are implemented. Present Transformers weight indexes,
  standalone weights, and sharded weight files are now exposed as component
  evidence. Transformers `config.quantization_config` and GGUF filename
  quantization labels are exposed as diagnostic component evidence.
  PEFT-style `adapter_config.json` packages are classified as adapter
  artifacts and exposed as adapter component evidence. Transformers weight
  index `weight_map` entries now surface missing declared shards as missing
  shard component evidence. Artifact facts now expose `sibling_files` from
  Hugging Face evidence separately from selected local files.
- [x] Extract component presence for tokenizer, processor, image processor,
  video processor, audio feature extractor, chat template, and generation
  config.
- [x] Parse `config.json` evidence for `model_type`, `architectures`,
  `dtype`/`torch_dtype`, `auto_map`, `processor_class`, and related
  Transformers package fields without importing Python code.
- [x] Parse `generation_config.json` into model-provided generation defaults,
  with parse diagnostics for missing, invalid, or legacy config-carried
  generation defaults.
  Missing/invalid/present `generation_config.json` states are implemented, and
  legacy config-carried generation defaults are extracted from `config.json`
  with source-path and diagnostic provenance when `generation_config.json` is
  absent.
- [x] Extract tokenizer and processor discovery evidence from
  `tokenizer_config.json`, `special_tokens_map.json`, `preprocessor_config.json`,
  image/video/audio processor configs, raw `chat_template.jinja`, and
  `chat_templates/*.jinja` when present.
  Tokenizer/processor/image/video/audio class-name discovery and
  `chat_templates/*.jinja` presence are implemented; `special_tokens_map.json`
  is now exposed as component file-presence evidence. Common tokenizer vocab,
  merge, and SentencePiece files are exposed as tokenizer components, with a
  missing-vocabulary diagnostic when `tokenizer_config.json` is present without
  a known vocabulary file. `tokenizer.json` now contributes bounded schema
  diagnostics for tokenizer version, model type, normalizer type, and
  pre-tokenizer type when present.
- [x] Preserve raw upstream task evidence and normalized Pumas modality
  signature.
- [x] Preserve generation defaults separately from consumer request overrides.
- [x] Preserve custom-code/security facts from `auto_map`,
  custom processor/config/model class references, custom generation evidence,
  upstream repo references, and dependency manifests such as `requirements.txt`.
  `auto_map` source references and dependency-manifest detection are
  implemented. Local `custom_generate/generate.py` and
  `custom_generate/requirements.txt` are now trust-relevant package facts;
  Transformers package evidence now preserves `source_repo_id` from metadata
  or Hugging Face evidence. Custom-code facts now include structured
  class-reference provenance from component metadata and Transformers
  architectures without treating class names alone as trust-required code.
- [x] Keep GGUF and llama.cpp companion facts distinct from HF-only package
  assumptions.
  Resolver coverage now proves GGUF `mmproj` companions remain artifact-level
  companion facts and do not create HF/Transformers evidence.

**Verification:**
- Unit tests or fixture tests for each artifact family and missing-component
  case.
- Round-trip tests for projection serialization and default semantics.
- Fixture tests for `auto_map` requiring trust, processor-only packages,
  tokenizer-only packages, chat-template variants, invalid generation config,
  and config-driven generation fallback.

**Status:** In progress. A read-only `ModelLibrary::resolve_model_package_facts`
slice now derives HF/Transformers-compatible package facts from existing
metadata plus bounded local package files without adding runtime-selection
policy. Present Transformers weight index, weight, and shard files now surface
as package components. Remaining Milestone 2 work includes deeper tokenizer
schema/normalizer diagnostics, custom processor/config/model class provenance
beyond `auto_map`, upstream repo references, broader artifact-family facts,
and full GGUF/llama.cpp companion modeling.

### Milestone 3: Normalize Backend Hints

**Goal:** Keep Pumas backend hints stable and advisory without making them
consumer runtime selections.

**Tasks:**
- [x] Define accepted host-agnostic backend-hint labels:
  `transformers`, `llama.cpp`, `vllm`, `mlx`, `candle`, `diffusers`, and
  `onnx-runtime`.
- [x] Map existing ecosystem hints into accepted labels where safe.
- [x] Preserve `ollama` and other ecosystem hints as facts while leaving support
  or migration interpretation to consumers.
- [x] Ensure hints are advisory facts, not runtime selection decisions.

**Verification:**
- Tests proving backend hints do not become executable backend selections.
- Tests proving accepted hints serialize with stable casing and labels.

**Status:** Complete. Package-fact DTOs and the read-only resolver normalize
accepted backend hints without selecting a runtime. HF search-compatible
engine derivation now includes `vllm` and `mlx` for HF-compatible
Transformers weight formats so the frontend can render those search tags.
Unsupported ecosystem hints such as `ollama` are preserved as raw/unsupported
facts for consumers to interpret.

### Milestone 4: Legacy Reference Resolution

**Goal:** Let old consumer references migrate toward Pumas model refs safely.

**Tasks:**
- [x] Define path-to-Pumas resolution behavior for legacy raw paths.
- [x] Route path handling through centralized validation/resolution helpers.
- [x] Emit migration diagnostics when a path/repo/artifact cannot resolve to a
  stable Pumas model ref.
  `ModelLibrary::resolve_pumas_model_ref` now resolves known model ids and
  indexed local model paths/files to stable `PumasModelRef` values and returns
  diagnostics for unknown ids, unindexed library paths, unresolved legacy paths,
  and outside-library paths.
- [x] Preserve graph intent by reporting unresolved model refs instead of
  silently choosing replacements.
  The resolver does this at the Rust `ModelLibrary`, API, IPC, Electron bridge,
  and frontend typing boundaries. Pumas now includes a `PumasModelRef` fixture
  proving unresolved legacy paths preserve migration diagnostics without
  guessed replacements.

**Verification:**
- Tests for traversal, symlink, non-existent path, allowed import root,
  already-canonical Pumas model ref, resolvable GGUF path, and unresolved legacy
  path cases.

**Status:** In progress. The Rust model-library boundary now resolves canonical
model ids and legacy local paths into `PumasModelRef` without selecting
replacement models for unresolved inputs, and the API/RPC bridge now exposes
that resolver to host consumers. Non-existent absolute paths under the library
root now have resolver coverage proving unresolved diagnostics are returned
without choosing replacements. Pumas model-ref fixtures now cover unresolved
legacy graph refs. Repo/artifact-specific resolution remains future extension
work if consumers need it.

### Milestone 5: Consumer Integration Fixtures

**Goal:** Prove host consumers can consume Pumas package facts without
depending on Pumas internals.

**Tasks:**
- [x] Add fixture contracts for representative consumer vertical slices:
  GGUF text generation, GGUF embeddings, HF/Transformers text generation,
  rerank, and multimodal validation.
  GGUF text-generation and GGUF embedding fixtures are now present alongside
  the existing HF/Transformers text-generation fixture. Rerank and multimodal
  processor fixtures are present.
- [x] Include package facts needed for compatibility, lifecycle diagnostics,
  task registry matching, generation-default merging, and backend hint
  reporting.
  Fixtures now include task evidence, backend hints, quantization facts,
  generation defaults, stale/invalid cache lifecycle diagnostics, unsupported
  hint diagnostics, missing/invalid component diagnostics, custom-code trust
  evidence, and stable artifact facts.
- [x] Ensure fixtures do not require consumers to inspect Pumas
  storage/index internals.
  Fixture tests assert consumer fixtures do not expose SQLite or
  `metadata_json` internals.
- [x] Include explicit Transformers package evidence fixtures for config/model
  type resolution, Auto-class `auto_map`, processor discovery, chat template
  discovery, generation defaults, custom-code-required, and invalid/missing
  component diagnostics.
  HF/Transformers fixtures now cover text generation, rerank, multimodal
  processor/chat-template evidence, custom code, invalid generation config,
  missing tokenizer vocabulary, and source/class/artifact provenance.

**Verification:**
- Cross-repo fixture review with consumers, including the parent Pantograph
  inference plan.
- Consumer-side contract tests once implementation begins.

**Status:** Complete for the current Pumas producer fixture scope. Pumas now has consumer-facing contract fixtures for
HF/Transformers text generation, GGUF text generation, and GGUF embeddings.
Diagnostic fixtures now cover unsupported Ollama hints, invalid generation
config, missing tokenizer vocabulary, and custom-code-required trust evidence.
Rerank and multimodal processor fixtures now cover representative non-LLM task
evidence. Stale package-facts cache semantics are now covered by a named
fixture. Deeper cross-repo consumer review remains a Pantograph integration
activity, not a Pumas producer implementation blocker.

### Milestone 6: Lazy Package-Facts Persistence

**Goal:** Keep package facts efficient by separating installed-library summary
queries from lazy detail inspection and cached package-fact blobs.

**Tasks:**
- [x] Confirm worktree and database fixture ownership before implementing
  durable SQLite changes.
- [x] Define a package-fact summary contract suitable for SQLite indexing and
  stale checks without carrying the full `ResolvedModelPackageFacts` payload.
  `ResolvedModelPackageFactsSummary` now carries compact stable summary
  fields for model ref, artifact kind/path/storage/validation, task evidence,
  backend hints, custom-code trust state, component statuses, generation
  statuses, and diagnostic codes.
- [x] Define a lazy package-facts cache/table keyed by model id, selected
  artifact, contract version, and artifact signature.
  The first durable storage slice adds `model_package_facts_cache` with
  model id, selected artifact id, summary/detail scope, package-facts contract
  version, producer revision, source fingerprint, JSON facts, and timestamps.
  The implementation keeps one current row per model/artifact/scope and stores
  contract version plus source fingerprint as required stale-check columns
  rather than as separate primary-key dimensions.
- [x] Define artifact-signature inputs for invalidation, including path
  identity, size/mtime/hash evidence where available, source revision,
  metadata schema version, dependency binding state, and facts contract
  version.
  The source fingerprint now covers the package-facts contract version, the
  execution descriptor, serialized metadata including schema/provenance/hash
  fields, selected package-file names plus file size/mtime state, chat-template
  files, and active dependency binding rows.
- [x] Add request flow semantics: fresh cache hit, stale cache regeneration,
  missing cache generation, parse-diagnostic return, and bounded inspection
  failure.
  Request-driven on-demand inspection is implemented through
  `resolve_model_package_facts`; durable detail cache hit, stale-fingerprint
  regeneration, missing-cache generation, invalid cached JSON bypass, and
  parse-diagnostic return are implemented. Duplicate request coalescing is
  implemented with per-model async locks around cache read/regenerate/write
  cycles. Recovery coverage now includes a fixture and resolver test for a
  fresh cache row whose detail payload is valid JSON but not a decodable
  `ResolvedModelPackageFacts`.
- [x] Keep library list/search/rebuild operations on cheap summary or
  invalidation paths rather than full package-fact generation.
  Coverage now stores a detail-cache payload that is valid JSON but invalid for
  `ResolvedModelPackageFacts`, then proves `list_models`, `search_models`, and
  `rebuild_index` succeed without deserializing or regenerating that detail
  payload.
- [x] Add or adjust HF search/cache compatibility derivation so MLX and vLLM
  tags can be rendered from compact remote evidence.

**Verification:**
- SQLite fixture tests prove package-fact summaries and detail blobs invalidate
  on artifact-signature and contract-version changes.
- Tests prove installed-library list/search does not deserialize full
  package-fact blobs.
- Tests prove metadata modal/import/host fact requests lazy-load or
  regenerate full facts when missing or stale.
- Tests prove duplicate package-fact requests are idempotent and do not corrupt
  cached summary/detail rows.
- Tests prove HF search can surface MLX and vLLM compatibility tags without
  modifying installed-library row behavior.

**Status:** In progress. Pumas now exposes a lazy package-facts request path
through `PumasApi`, JSON-RPC dispatch, Electron preload, and frontend bridge
types without adding facts to installed-library list/search rows. A durable
SQLite `model_package_facts_cache` table and low-level `ModelIndex`
upsert/get/delete helpers now exist for summary/detail JSON facts, with JSON
validity checks and model-owned cascade cleanup. `resolve_model_package_facts`
now uses the durable detail cache when the model is present in the index and
regenerates stale rows using a source fingerprint over the package-facts
contract version, execution descriptor, metadata, selected package files, and
chat-template files. Duplicate detail requests for the same model now coalesce
through a per-model async lock. `resolve_model_package_facts` now persists a
compact summary cache row beside the detail row and refreshes it from fresh
detail-cache hits. The source fingerprint now includes active dependency
binding state beside existing descriptor, metadata, selected-file, and
chat-template inputs. List/search/rebuild coverage now proves cheap library
queries do not deserialize or regenerate package-facts detail payloads.
Malformed fresh detail-cache recovery now has an executable fixture and
resolver test.

### Milestone 7: SQLite Metadata Projection Cleanup

**Goal:** Reduce redundant and meaningless model-index metadata while
preserving user-facing and provenance-facing fields that still have product
value.

**Tasks:**
- [x] Confirm cleanup scope is projection-only unless a separate source
  metadata migration is explicitly approved.
- [x] Define the owner for each duplicated field: dedicated `models` column,
  `tags_json`, `hashes_json`, structured package-fact tables, source
  `metadata.json`, or UI/user metadata.
- [x] Remove `model_id`, `model_type`, `cleaned_name`, `official_name`,
  `hashes`, and overlapping `tags` from the SQLite `metadata_json` projection
  when the dedicated columns already own those values.
- [x] Remove consistently non-meaningful projected fields unless a current
  contract owner is identified: `compatible_apps`, `conversion_source`,
  `last_lookup_attempt`, `license_artifact`, `model_card_artifact`,
  `reviewed_at`, `reviewed_by`, `subtype`, and empty validation-error arrays.
- [x] Preserve `license`, `license_status`, `model_card`, `notes`, and
  `preview_image` by explicit exception, even if the current sample is sparse
  or empty.
- [x] Add a migration/dry-run report that shows affected rows, removed fields,
  preserved exception fields, and metadata payload-size reduction.
  Pumas now exposes a non-mutating metadata projection cleanup dry-run report
  over existing SQLite index rows with affected row counts, removed fields,
  preserved exception fields, and before/after JSON byte counts.
- [x] Define rollback/rebuild behavior for cleanup: regenerate the index from
  source metadata or restore the pre-migration projection from backup/report
  artifacts.
  Pumas cleanup mutates only SQLite projection rows. Recovery is a normal
  `rebuild_index()` from source metadata, which regenerates the current cleaned
  projection and preserves source-backed exception fields. Restoring redundant
  legacy projection fields requires an external database backup rather than
  source metadata rollback.
- [x] Add write-mode cleanup over isolated SQLite fixtures and prove
  idempotent reruns.
  Pumas now applies cleanup to existing SQLite projection rows using the same
  plan as the dry-run report and returns an execution report with the planned
  changes and updated-row count.
- [x] Update frontend and API assumptions so consumers read column-owned fields
  from the existing dedicated response fields rather than from
  `metadata_json`.
  Frontend local model row mapping now uses dedicated `ModelRecord` fields for
  model type/name identity and no longer relies on cleaned `metadata_json`
  duplicates or removed `conversion_source` projection fields.

**Verification:**
- Migration dry-run fixture proves only column-owned duplicates and approved
  meaningless fields are removed from the projected index payload.
- Write-mode cleanup tests run against isolated SQLite fixtures and prove the
  cleanup is idempotent.
- Rebuild/recovery tests prove cleaned projections can be regenerated from
  source metadata without losing preserved exception fields.
- Regression tests prove model list/search/API responses still include model
  id, model type, official name, cleaned name, tags, and hashes from their
  dedicated owners.
- Tests prove license/model-card/notes/preview-image fields are preserved when
  present and remain allowed when empty.
- Size/report tests prove cleanup results are auditable before any write-mode
  migration is run.

**Status:** In progress. New and reindexed SQLite `models.metadata_json`
projections now remove column-owned duplicates and approved non-meaningful
fields while preserving exception fields. Existing rows can now be audited with
a non-mutating cleanup dry-run report and cleaned with an idempotent write-mode
SQLite projection cleanup. Rebuild/recovery behavior is documented and covered,
and frontend row mapping no longer depends on cleaned metadata duplicates.
Milestone 7 is complete for the current implementation scope.

### Milestone 8: GUI Surfaces For Package Evidence

**Goal:** Surface package facts where they help discovery, inspection, and
import decisions without cluttering the installed model-library list.

**Tasks:**
- [x] Preserve the installed model-library row layout without adding
  compatibility badges for the new backend-hint vocabulary.
- [x] Add MLX and vLLM compatibility tags to Hugging Face search results when
  the backend search/catalog response can support them.
- [x] Add a read-only `Execution Facts` view to the metadata modal that
  presents resolved package facts separately from editable inference settings.
- [x] Update the import review flow to surface package evidence and diagnostics
  before final import where the backend can provide them.
- [x] Keep package-fact detail loading scoped to search, import, and metadata
  modal paths rather than eagerly enriching every installed-library row.
  Backend/API access is now request-driven and the metadata modal lazy-loads
  execution facts only when its `Execution Facts` tab is selected. Import
  review surfaces existing backend-provided classification/HF evidence before
  final import; deeper pre-import package-fact scanning remains out of scope for
  that UI slice.

**Verification:**
- Frontend tests prove local library rows do not gain compatibility badges.
- Frontend tests prove HF search renders MLX and vLLM tags from compatible
  search/catalog data.
- Metadata modal tests prove `Execution Facts` are read-only and generation
  defaults do not modify editable inference settings.
- Import workflow tests prove package evidence and diagnostics appear before
  final import when present.

**Status:** Complete for the current GUI implementation scope. HF search result
tags for `mlx` and `vllm` are backed by backend-compatible engine derivation
and frontend rendering coverage. Package facts are available through an
on-demand bridge method consumed by the metadata modal, and frontend regression
coverage now proves installed-library rows do not render backend compatibility
badges or eagerly load package-fact detail.

## Definition of Done

- Pumas exposes stable package facts sufficient for host inference
  compatibility checks.
- Pumas keeps stable model identity and artifact/provenance ownership.
- Host saved references can prefer Pumas model refs over raw paths.
- Raw paths remain compatibility/import/debug inputs with explicit migration
  diagnostics.
- GGUF, HF-compatible directories, safetensors, diffusers bundles, ONNX, and
  future formats remain first-class artifact kinds.
- Transformers vocabulary is preserved as package/task evidence, not as model
  registry authority.
- `ResolvedModelPackageFacts` is a separate versioned projection from
  `ModelExecutionDescriptor`.
- Transformers package evidence includes config, Auto-class, processor,
  tokenizer, chat-template, generation-config, selected-file, and
  custom-code/trust facts without loading Python objects.
- Model-provided generation defaults are not conflated with Pumas
  `inference_settings`.
- Ollama hints are retained as ecosystem evidence; individual consumers decide
  whether they are supported, unsupported, or migration-only.
- Custom-code/security facts are explicit enough for inference to require trust
  policy before Python/Transformers execution.
- Projection DTOs have decode/normalize or round-trip fixture tests.
- Boundary-facing enum labels, omitted-field defaults, ordering semantics,
  volatile fields, and producer-version behavior are documented in contract
  fixtures or source docs.
- Full package facts are lazy-loaded and cached by artifact signature, while
  installed-library list/search paths use only cheap existing model rows or
  compact package-fact summaries.
- SQLite package-fact persistence uses dedicated summary/detail table families
  or equivalent structured storage instead of bloating `models.metadata_json`
  with large nested package evidence.
- SQLite metadata projection cleanup removes column-owned duplicates and
  approved non-meaningful fields while preserving license, model-card, notes,
  and preview-image fields by explicit exception.
- SQLite cleanup has dry-run, write-mode, idempotency, and rebuild/rollback
  verification before it is considered complete.
- Lazy package-fact regeneration is idempotent under duplicate requests and
  recovers from stale, missing, or contract-version-mismatched cache rows.
- Host-agnostic model-library change notifications or update cursors exist for
  model added, model removed, metadata modified, package facts modified, stale
  facts invalidated, and dependency binding modified cases.
- Consumer-visible update events identify the model id, selected artifact id
  when applicable, changed fact family, producer revision or cursor, and
  whether consumers should refresh summary rows, detail facts, or both.
- Required package-fact fixtures from `Required Fixture Set` exist and can be
  decoded by host consumers without Pumas SQLite layout, `models.metadata_json`,
  or search-cache internals.
- The Implementation Readiness Gate is recorded as passed before Milestone 1
  code work starts, including worktree status, tracked plan state, slice scope,
  fixture ownership, and expected verification commands.
- Pumas and at least one host consumer have a reviewed cross-repo fixture path that
  proves producer/consumer compatibility for the new package facts.
- Affected Pumas READMEs or ADRs are updated when public DTOs, structured
  producer behavior, or persisted metadata semantics change.
- Implementation PRs record the verification commands run and explain any
  intentionally skipped checks.
- GUI changes preserve the installed model-library row layout while adding MLX
  and vLLM compatibility tags to HF search, read-only Execution Facts in the
  metadata modal, and earlier package-evidence diagnostics in import review.
- No scheduler, runtime-selection, inference-execution, or diagnostics-ledger
  write policy moves into Pumas.

## Re-Plan Triggers

- A host consumer needs package facts that Pumas cannot produce without
  changing indexing/storage policy.
- Existing Pumas metadata cannot represent GGUF, HF-compatible, safetensors,
  diffusers, ONNX, adapter, or companion-artifact facts without a persisted
  schema migration.
- Backend hints start acting as runtime selection policy.
- Raw paths would need to remain canonical saved consumer identity.
- A consumer requires Pumas to define runtime support policy for an ecosystem
  hint such as Ollama.
- Custom-code/trust evidence cannot be represented without unsafe defaults.
- Transformers package facts would require executing Python or loading
  Transformers Auto classes inside Pumas.
- `GenerationConfig` defaults and Pumas `inference_settings` start to drift or
  become ambiguous to consumers.
- Cross-repo DTO fixtures drift between Pumas and consumers.
- A consumer-specific contract label would need to move into Pumas instead of
  staying in that consumer's adapter or compatibility layer.
- Model-library cache invalidation cannot be expressed with host-agnostic
  update events or cursors.
- Required fixtures would need to expose Pumas SQLite layout,
  `models.metadata_json`, or HF search-cache internals to be useful.
- Lazy package-fact cache invalidation cannot be expressed with stable artifact
  signatures or contract-version keys.
- SQLite cleanup would require deleting source metadata instead of only
  changing the index projection.
- A frontend/API consumer still depends on removed `metadata_json` duplicate
  fields after ownership has been assigned to dedicated columns.
- MLX or vLLM search tags require evidence that is too expensive or unreliable
  to compute within the HF search/cache path.

## Execution Notes

- 2026-05-02: Plan reviewed against the Coding Standards set and updated to
  make inputs, affected contracts, risks, security constraints, structured
  producer rules, concurrency considerations, verification, and traceability
  explicit.
- 2026-05-02: Follow-up standards pass added durable-storage cleanup controls,
  lazy-cache invalidation/race handling, dry-run/write-mode/recovery
  verification, and commit separation for storage-addition versus
  projection-cleanup work.
- 2026-05-02: Clarified that Pumas remains host-agnostic. It should expose
  consumer-visible model facts, compact execution summaries, feasible execution
  candidates, and model-library update events or cursors, while each consumer
  owns final generation behavior, runtime selection, and cache projection
  policy.
- 2026-05-02: Implementation-readiness standards pass added explicit worktree
  hygiene, tracked-plan, slice-scope, fixture ownership, expected verification,
  and conventional-commit gates before code work begins.
- 2026-05-02: Milestone 1 first thin slice implemented in Pumas with pure
  serde package-fact DTOs, one HF/Transformers text-generation fixture, focused
  contract round-trip tests, and model/fixture README updates. Verification:
  `cargo fmt --all` from the Rust workspace root and
  `cargo test -p pumas-library --test package_facts_contract_fixtures`.
  Tooling note: `cargo fmt --manifest-path rust/Cargo.toml` failed to find
  targets in this workspace layout, so formatting should run from `rust/`.
- 2026-05-02: Milestone 1 completed in Pumas by adding stable-versus-volatile
  package-fact classification docs and host-agnostic `ModelLibraryUpdateEvent`
  DTOs/fixtures for package-fact cache invalidation. Verification:
  `cargo fmt --all` from `rust/` and
  `cargo test -p pumas-library --test package_facts_contract_fixtures`.
- 2026-05-02: Milestone 2 first vertical slice implemented in Pumas with
  read-only package-fact resolution on `ModelLibrary`, bounded parsing for
  `config.json` and `generation_config.json`, component presence detection,
  task/modality preservation, dependency-manifest detection, and advisory
  backend hint normalization. Verification: `cargo fmt --all` from `rust/`,
  `cargo test -p pumas-library --test package_facts_resolution`, and
  `cargo test -p pumas-library --test package_facts_contract_fixtures`.
  Implementation finding: existing Hugging Face evidence stores capture time
  but not a source commit/revision, so `model_ref.revision` and
  `transformers.source_revision` remain absent until importer/search evidence
  captures a true revision value.
- 2026-05-02: Backend hint/HF search slice implemented in Pumas by adding
  `vllm` and `mlx` compatible-engine tags for HF-compatible safetensors/bin
  formats and frontend coverage proving remote Hugging Face results render
  those tags. Verification: `cargo fmt --all` from `rust/`,
  `cargo test -p pumas-library detect_compatible_engines_includes_hf_search_transformers_tags`,
  and `npm run -w frontend test:run -- RemoteModelSummary.test.tsx`.
- 2026-05-02: Milestone 2 custom-code refinement implemented in Pumas by
  extracting `auto_map` source references into `CustomCodeFacts` and treating
  `auto_map` presence as trust-relevant custom-code evidence even when older
  metadata lacks `requires_custom_code`. Verification: `cargo fmt --all` from
  `rust/`, `cargo test -p pumas-library --test package_facts_resolution`, and
  `cargo test -p pumas-library --test package_facts_contract_fixtures`.
- 2026-05-02: Lazy package-facts API slice implemented in Pumas with
  `PumasApi::resolve_model_package_facts`, JSON-RPC dispatch, Electron preload
  method registration, frontend bridge typing, and an API integration test.
  Verification: `cargo fmt --all` from `rust/`,
  `cargo test -p pumas-library --test api_tests test_resolve_model_package_facts_is_lazy_api_surface`,
  `cargo test -p pumas-library --test package_facts_resolution`,
  `npm run -w frontend check:types`, and `npm run -w electron validate`.
- 2026-05-02: Metadata modal Execution Facts slice implemented in Pumas with
  lazy frontend loading through `resolveModelPackageFacts` when the
  `Execution Facts` tab is selected, rendered through the existing read-only
  metadata grid. Verification: `npm run -w frontend test:run --
  ModelMetadataModal.test.tsx` and `npm run -w frontend check:types`.
- 2026-05-02: Import review evidence slice implemented in Pumas by surfacing
  package evidence already available from classification and Hugging Face
  lookup, including format and matched repo facts, before final import.
  Verification: `npm run -w frontend test:run -- ImportReviewStep.test.tsx`
  and `npm run -w frontend check:types`.
- 2026-05-02: SQLite metadata projection cleanup slice implemented in Pumas by
  removing column-owned duplicates and approved non-meaningful fields from new
  or reindexed `models.metadata_json` rows while preserving license status,
  model card, notes, and preview image exception fields. Verification:
  `cargo fmt --all` from `rust/`,
  `cargo test -p pumas-library metadata_projection_removes_column_owned_duplicates`,
  `cargo test -p pumas-library metadata_projection_removes_non_meaningful_fields_but_keeps_exceptions`,
  and
  `cargo test -p pumas-library test_list_models_projects_primary_format_and_quant_from_indexed_metadata`.
  Implementation finding: existing populated SQLite rows still require a
  dry-run/report and write-mode migration or explicit rebuild to receive the
  slimmer projection.
- 2026-05-02: Backend hint normalization completed in Pumas with explicit
  resolver coverage proving unsupported ecosystem hints such as `ollama` remain
  raw/unsupported facts while accepted hints such as `transformers` normalize
  to stable labels. Verification:
  `cargo test -p pumas-library --test package_facts_resolution preserves_unsupported_backend_hints_as_raw_package_facts`.
- 2026-05-02: Milestone 2 processor-component evidence slice implemented in
  Pumas by extracting class names from standard Transformers sidecar configs
  (`tokenizer_config.json`, `processor_config.json`, image/video/audio
  processor configs, and config architectures) and detecting
  `chat_templates/*.jinja` entries as chat-template components. Verification:
  `cargo fmt --all` from `rust/` and
  `cargo test -p pumas-library --test package_facts_resolution`.
- 2026-05-02: Milestone 6 durable cache schema slice implemented in Pumas with
  a `model_package_facts_cache` SQLite table, summary/detail scope DTOs,
  low-level `ModelIndex` upsert/get/delete helpers, JSON validity checks,
  changed-row upsert semantics, and model-owned cascade cleanup. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library package_facts_cache`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library test_metadata_v2_schema_tables_exist`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library test_list_foreign_key_violations`.
  Implementation finding: the cache storage is not yet wired into
  `resolve_model_package_facts`, so fresh/stale/missing cache behavior and
  artifact-signature construction remain follow-up Milestone 6 work.
- 2026-05-02: Milestone 6 resolver cache slice implemented in Pumas by wiring
  `resolve_model_package_facts` through the durable detail cache for indexed
  models, adding source fingerprints over contract version, execution
  descriptor, metadata, selected package files, and `chat_templates/*.jinja`,
  and preserving metadata-only package resolution when no index row exists.
  Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library package_facts_cache`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test api_tests test_resolve_model_package_facts_is_lazy_api_surface`.
  Implementation finding: SQLite foreign-key enforcement means cache writes
  must only occur for models already present in `ModelIndex`; metadata-only
  resolution remains uncached until the model is indexed.
- 2026-05-02: Milestone 6 duplicate request coalescing slice implemented in
  Pumas with per-model async locks around package-facts cache
  read/regenerate/write cycles and concurrent request coverage proving callers
  receive the same regenerated facts and persisted detail row. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library package_facts_cache`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test api_tests test_resolve_model_package_facts_is_lazy_api_surface`.
- 2026-05-02: Milestone 2 legacy generation-default slice implemented in Pumas
  by extracting Transformers generation defaults from `config.json` when
  `generation_config.json` is absent, preserving `config.json` as the source
  path and adding diagnostic provenance so consumers can distinguish the
  fallback from first-class generation-config files. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all` and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`.
- 2026-05-02: Milestone 2 special-tokens component slice implemented in Pumas
  by adding append-only `special_tokens_map` processor-component evidence for
  `special_tokens_map.json` presence. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`.
- 2026-05-02: Milestone 2 weight-index/shard component slice implemented in
  Pumas by adding append-only `shard` component evidence and resolving present
  Transformers weight index, standalone weight, and sharded weight files into
  package facts without selecting a runtime. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`.
- 2026-05-02: Milestone 2 quantization component slice implemented in Pumas by
  exposing Transformers `config.quantization_config` and GGUF filename quant
  labels as read-only package component evidence. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`.
- 2026-05-02: Milestone 2 adapter package slice implemented in Pumas by
  classifying PEFT-style `adapter_config.json` packages as adapter artifacts
  and exposing `peft_type` as adapter component provenance. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`.
- 2026-05-02: Milestone 2 tokenizer vocabulary slice implemented in Pumas by
  exposing common vocab, merges, and SentencePiece files as tokenizer
  components and emitting a missing-vocabulary diagnostic when
  `tokenizer_config.json` lacks a known vocabulary companion. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`.
- 2026-05-02: Milestone 2 custom generation trust-evidence slice implemented
  in Pumas by detecting local `custom_generate/generate.py` and
  `custom_generate/requirements.txt` as package facts without importing or
  executing Python. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`.
- 2026-05-02: Milestone 4 model-ref resolver slice implemented in Pumas with
  `ModelLibrary::resolve_pumas_model_ref`, covering canonical model ids,
  indexed model directory/file paths, unknown ids, and outside-library legacy
  paths while returning diagnostics instead of guessed replacements.
  Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 4 API/RPC model-ref resolver slice implemented in
  Pumas by exposing `resolve_pumas_model_ref` through the Pumas API facade, IPC
  dispatch, Electron method registry/preload bridge, and frontend model API
  typings. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test api_tests test_resolve_pumas_model_ref_api_surface`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution resolves_canonical_model_refs_and_reports_unresolved_legacy_paths`,
  `npm run -w frontend check:types`, and `npm run -w electron validate`.
- 2026-05-02: Milestone 4 canonicalized model-ref fixture slice implemented in
  Pumas with resolver coverage for traversal-normalized paths, symlinked legacy
  paths, and unindexed library paths. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution resolves_model_refs_`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 5 GGUF consumer fixture slice implemented in Pumas with
  `gguf_text_generation_package_facts.json` and
  `gguf_embedding_package_facts.json`, plus contract assertions that the
  fixtures carry advisory backend hints and quantization evidence without
  exposing SQLite or `metadata_json` internals. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 5 diagnostic consumer fixture slice implemented in
  Pumas with fixtures for unsupported Ollama ecosystem hints, invalid
  generation config diagnostics, and missing tokenizer vocabulary evidence.
  Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 5 custom-code-required fixture slice implemented in
  Pumas with `custom_code_required_package_facts.json`, combining `auto_map`,
  local custom generation code, and dependency manifest evidence for consumer
  trust-policy tests. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 5 remote search hint fixture slice implemented in
  Pumas with `remote_search_mlx_vllm_hint.json`, proving HF search
  Transformers/vLLM/MLX discovery hints remain separate from installed-model
  package facts. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 5 rerank/multimodal fixture slice implemented in
  Pumas with `hf_rerank_package_facts.json` and
  `hf_multimodal_processor_package_facts.json`, covering text-ranking task
  evidence and multimodal processor/image-processor/chat-template evidence.
  Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 5 stale package-facts cache fixture slice implemented
  in Pumas with `stale_package_facts.json`, covering stale contract-version
  and source-fingerprint semantics plus a decodable package-facts detail
  payload. Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures stale_package_facts_fixture_matches_cache_contract`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 6 compact package-facts summary slice implemented in
  Pumas with `ResolvedModelPackageFactsSummary` and summary-scope cache writes
  from `resolve_model_package_facts`. The resolver now stores compact stable
  facts beside detail facts using the same source fingerprint while leaving the
  fingerprint in cache metadata instead of the summary payload. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution persists_compact_package_facts_summary_cache`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 6 dependency-binding invalidation slice implemented
  in Pumas by adding active dependency binding rows to the package-facts source
  fingerprint. Focused coverage poisons a detail cache row, mutates dependency
  binding state, and proves regenerated detail plus matching summary/detail
  fingerprints. Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution dependency_binding_changes_refresh_package_fact_cache_fingerprints`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library package_facts_cache`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 6 cheap library-query guard slice implemented in
  Pumas with coverage proving `list_models`, `search_models`, and
  `rebuild_index` do not deserialize or regenerate package-facts detail cache
  rows. Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution list_search_and_rebuild_skip_package_facts_detail_cache`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 6 invalid detail-cache recovery fixture slice
  implemented in Pumas with `invalid_cached_package_facts.json` plus resolver
  coverage proving malformed fresh detail cache is bypassed and regenerated.
  Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures invalid_cached_package_facts_fixture_matches_recovery_contract`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution recovers_from_invalid_package_facts_detail_cache_payload`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 2 shard completeness slice implemented in Pumas by
  parsing Transformers weight-index `weight_map` entries and surfacing missing
  declared shard files as missing shard component evidence. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution extracts_weight_index_and_shard_component_evidence`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 4 unresolved library-path coverage slice implemented
  in Pumas by testing a non-existent absolute path under the library root and
  verifying the resolver returns `legacy_path_unresolved` diagnostics without
  selecting a replacement model. Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution resolves_model_refs_through_canonicalized_legacy_paths`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 2 GGUF companion coverage slice implemented in Pumas
  by proving `mmproj` companions remain artifact-level facts for GGUF packages
  without producing HF/Transformers package evidence. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution keeps_gguf_companion_facts_distinct_from_transformers_evidence`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 2 upstream source-repo evidence slice implemented in
  Pumas by adding `source_repo_id` to Transformers package evidence, projecting
  it from model metadata or Hugging Face evidence, and updating frontend DTO
  typing. Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution resolves_hf_transformers_package_facts_from_metadata_and_files`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures hf_text_generation_fixture_matches_contract`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  `cargo check --manifest-path rust/Cargo.toml -p pumas-library`, and
  `npm run -w frontend check:types`.
- 2026-05-02: Milestone 2 class-reference provenance slice implemented in
  Pumas by adding structured `class_references` to custom-code facts, populated
  from component metadata and Transformers architectures without changing
  trust-required policy. Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution resolves_hf_transformers_package_facts_from_metadata_and_files`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures hf_text_generation_fixture_matches_contract`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  `cargo check --manifest-path rust/Cargo.toml -p pumas-library`, and
  `npm run -w frontend check:types`.
- 2026-05-02: Milestone 2 sibling-file evidence slice implemented in Pumas by
  adding artifact-level `sibling_files`, sourced from Hugging Face evidence and
  kept distinct from selected local files. Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution resolves_hf_transformers_package_facts_from_metadata_and_files`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures hf_text_generation_fixture_matches_contract`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_contract_fixtures`,
  `cargo check --manifest-path rust/Cargo.toml -p pumas-library`, and
  `npm run -w frontend check:types`.
- 2026-05-02: Milestone 2 tokenizer diagnostics slice implemented in Pumas by
  parsing `tokenizer.json` for bounded schema evidence and reporting version,
  model, normalizer, and pre-tokenizer types on tokenizer component facts.
  Verification: `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution extracts_tokenizer_vocabulary_files_and_missing_diagnostics`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test package_facts_resolution`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 4 unresolved graph fixture slice implemented in Pumas
  with a `PumasModelRef` fixture for unresolved legacy paths that preserves
  migration diagnostics and omits guessed replacement ids. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library --test model_ref_contract_fixtures`,
  and `cargo check --manifest-path rust/Cargo.toml -p pumas-library`.
- 2026-05-02: Milestone 7 metadata projection cleanup dry-run slice
  implemented in Pumas with a non-mutating report over existing SQLite index
  rows. The report includes total rows, affected rows, removed field counts,
  preserved exception fields, and before/after JSON payload byte counts without
  mutating source metadata or index rows. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library test_metadata_projection_cleanup_dry_run_reports_legacy_index_rows`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library metadata_projection_removes_column_owned_duplicates`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library metadata_projection_removes_non_meaningful_fields_but_keeps_exceptions`.
- 2026-05-02: Milestone 7 metadata projection cleanup write-mode slice
  implemented in Pumas with `execute_metadata_projection_cleanup`, which uses
  the same plan as the dry-run report, mutates only SQLite projection rows, and
  returns an execution report with planned and updated row counts. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library test_metadata_projection_cleanup_execution_is_idempotent`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library test_metadata_projection_cleanup_dry_run_reports_legacy_index_rows`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library metadata_projection_`.
- 2026-05-02: Milestone 7 cleanup recovery slice implemented in Pumas by
  documenting that cleanup mutates only SQLite projection rows and recovery is
  a normal `rebuild_index()` from source metadata. Verification:
  `cargo fmt --manifest-path rust/Cargo.toml --all`,
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library test_metadata_projection_cleanup_recovers_from_source_rebuild`,
  and
  `cargo test --manifest-path rust/Cargo.toml -p pumas-library metadata_projection_`.
- 2026-05-02: Milestone 7 frontend/API assumption slice implemented in Pumas by
  removing local row fallbacks to cleaned `metadata_json` duplicates and the
  removed `conversion_source` projection field. Verification:
  `npm run -w frontend test:run -- libraryModels.test.ts` and
  `npm run -w frontend check:types`.
- 2026-05-02: Milestone 8 GUI boundary regression slice implemented in Pumas
  by proving installed local model rows do not render MLX/vLLM compatibility
  badges and the metadata modal does not resolve package facts until the
  `Execution Facts` tab is selected. Verification:
  `npm run -w frontend test:run -- LocalModelsList.test.tsx ModelMetadataModal.test.tsx`
  and `npm run -w frontend check:types`.
- Implementation should start with Milestone 1 contract/fixture design before
  production extraction logic. This keeps producer semantics reviewable before
  code depends on them.
- Each implementation slice should update task status in this plan or link to a
  follow-up implementation plan if the work is split across multiple PRs.

## Commit Cadence Notes

- Prefer separate commits for contract definitions, extraction logic,
  fixture/test additions, documentation/ADR updates, and consumer fixture
  synchronization.
- Use conventional commit messages. Record verification commands and outcomes
  in this plan, PR notes, or the completion summary, not in commit messages.
- Before each commit, inspect staged changes and unpushed history for
  regression/fix pairs according to `COMMIT-STANDARDS.md`.
- Keep lazy package-fact persistence and SQLite metadata cleanup in separate
  logical commits because one adds storage and the other removes projection
  redundancy.
- Do not mix persisted metadata migrations with broad extraction behavior in
  the same review unless the migration is required by the contract freeze and
  documented in that PR.
- If consumer fixtures change at the same time as Pumas producer output,
  summarize the producer/consumer compatibility evidence in both repositories.

## Optional Subagent Assignment

- No parallel subagent work is required for Milestone 1 because the first
  critical path is contract freezing.
- After Milestone 1, extraction work can be split by artifact family only if
  each worker owns a disjoint module and fixture set. Shared DTO and schema
  files should remain under one owner to avoid contract conflicts.
- SQLite schema/cache work, projection cleanup, and GUI surfaces should be
  separate implementation slices. Shared schema files, migration helpers,
  generated bindings, and cross-repo fixture contracts must be owned serially
  by one implementer at a time.

## Completion Summary

To be completed during implementation.

### Completed

- List completed milestones and the final affected contracts, schemas, fixtures,
  GUI surfaces, and host-consumer integration points.

### Deviations

- List deviations from this plan, why they were accepted, and which re-plan
  trigger or implementation finding caused them.

### Follow-Ups

- List remaining unsupported package families, backend hints, cleanup
  exclusions, or host-consumer work.

### Verification Summary

- List verification commands, fixture paths, dry-run/write-mode migration
  evidence, cache recovery checks, frontend checks, and any intentionally
  skipped checks with reasons.

### Traceability Links

- Link affected Pumas READMEs, ADRs, PR notes, fixture locations, and
  host-consumer fixture or integration changes.

## Traceability

- Parent Pantograph inference plan:
  `docs/plans/inference-execution-boundary-contracts/plan.md`
- Pumas repository:
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library`
- Transformers source reference:
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/transformers`
- Reference standards:
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
