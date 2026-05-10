# Pumas Library Image Generation Facts Plan

## Objective

Define the Pumas Library changes needed by the current Pantograph image-generation
plan. Pumas remains the canonical model source and factual package producer.
Pantograph remains responsible for scheduler policy, backend/runtime/device
selection, inference execution, diagnostics ledger events, and graph semantics.

The implementation target is:

```text
/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library
```

This plan replaces the older Pumas work referenced by
`inference-execution-boundary-contracts` for the current image-generation graph
effort. It is scoped to the package facts Pantograph needs for SD/SDXL, FLUX,
FLUX.2, Qwen Image, Lumina Image, GLM Image, Z-Image, GGUF, and future
Transformers-compatible model packages.

## Scope

In scope:

- Refactor Pumas package-facts extraction out of `library.rs` into focused
  modules before adding new extraction behavior.
- Add structured package-facts DTOs for Diffusers bundles, image-generation
  family evidence, GGUF metadata, generation defaults, custom-code evidence, and
  value sources.
- Make package-facts detail and summary cache rows selected-artifact-aware so
  multi-artifact repositories can expose facts for the artifact being inspected.
- Define a standards-oriented package inspection manifest that is shared by
  extraction, cache fingerprinting, and summary projection.
- Preserve fast selector snapshots through summary rows, explicit missing/stale
  states, and update cursors.
- Add a package-facts cache migration/backfill path so existing Pumas SQLite
  rows are upgraded to the new facts contract rather than served as stale facts.
- Add synthetic and small real fixtures that prove the produced facts are stable
  for Pantograph.
- Align Pantograph and Pumas through shared JSON fixture expectations.

Out of scope:

- Pantograph scheduler policy, backend selection, device allocation, queueing,
  learned throughput, and diagnostics-ledger writes.
- Pumas execution, PyTorch/Diffusers loading, llama.cpp loading, or runtime
  process management.
- Consumer support decisions, runtime support matrices, or "can execute"
  verdicts for Pantograph or any other host application.
- Name-based image-family inference from display names, workflow names, or
  directory names.
- Model-specific lookup tables that classify individual repositories by known
  names instead of package-standard evidence.
- Compatibility shims for old Pantograph graph shapes or old Pumas consumer
  assumptions that conflict with this contract.

## Affected Contracts And Artifacts

Structured contracts:

- `ResolvedModelPackageFacts` or successor package-facts DTOs.
- Package-facts summary DTOs used by selector snapshots.
- Model-library update events and cursors.
- Pantograph fixture JSON consumed by the image-family planner.

Persisted artifacts:

- Pumas package-facts detail cache rows.
- Pumas selector-summary cache rows.
- Pumas update cursor/event rows.
- Pumas package-facts cache migration reports and checkpoints.
- Pantograph checked-in fixture copies used for contract tests.

Any structured contract change must be implemented with serde round-trip tests
and fixture updates in the same logical slice.

## Standards Review Inputs

This plan was iterated against the standards in:

- `PLAN-STANDARDS.md`
- `CODING-STANDARDS.md`
- `DOCUMENTATION-STANDARDS.md`
- `TESTING-STANDARDS.md`
- `SECURITY-STANDARDS.md`
- `INTEROP-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`
- `TOOLING-STANDARDS.md`
- `languages/rust/RUST-STANDARDS.md`
- `languages/rust/RUST-API-STANDARDS.md`
- `languages/rust/RUST-SECURITY-STANDARDS.md`
- `languages/rust/RUST-INTEROP-STANDARDS.md`
- `languages/rust/RUST-DEPENDENCY-STANDARDS.md`

The explicit exception for this plan is concurrency-standard adherence. Pumas
already has an established package-facts locking, SQLite connection, snapshot,
and update-feed model. The implementation should preserve that design unless a
change makes it simpler or more efficient. This exception does not waive the
requirements to avoid parsing under SQLite locks, to keep blocking filesystem
work out of async request paths, or to document lifecycle changes if the
existing concurrency model must be modified.

## Boundary

Pumas should expose durable facts that can be derived from downloaded or
registered model artifacts:

- stable model identity and selected artifact identity
- artifact kind, executable entry path, storage kind, validation state, and
  selected files
- Transformers-compatible model/config/task/generation metadata
- Diffusers bundle metadata and component layout
- GGUF header metadata
- dependency and custom-code evidence
- package-fact summaries and update cursors for fast startup snapshots
- package-facts cache migration/backfill state for existing indexed models

Pumas should be a standards-aware fact producer. It may understand package
standards such as Transformers config files, Diffusers `model_index.json`, GGUF
headers, safetensors/index files, dependency manifests, and custom-code markers.
It must not become a model-specific oracle that recognizes individual model
repositories through hardcoded lookup tables.

Pumas must not expose Pantograph-specific runtime policy:

- no scheduler admission decisions
- no backend ranking
- no RAM/VRAM placement decisions
- no learned throughput policy
- no Pantograph workflow/node ids
- no implicit runtime recommendations that replace missing facts
- no diagnostics-ledger writes
- no consumer-specific support verdicts

Backend hints from Pumas remain advisory facts. Pantograph may map those facts to
runtime candidates, but Pumas does not decide which backend, device, or runtime
variant executes a workflow.

Package-fact family labels are acceptable only when they are backed by explicit
source-tagged package evidence. For example, Pumas may emit `flux` when
`model_index.json` identifies a FLUX pipeline or component layout, but it must
emit `unknown` or `ambiguous` when the only signal is a model name, repository
name, directory name, or host workflow name.

## Current Source Findings

Pumas already has the correct high-level surfaces:

- `ResolvedModelPackageFacts` carries artifact, component, task, generation,
  custom-code, and backend-hint facts.
- Selector snapshots are SQLite-backed and return model rows plus update cursors
  without full package hydration.
- Diffusers directories are detected through `model_index.json` and component
  presence validation.
- GGUF artifacts are first-class package artifacts, but current facts are mostly
  selected-file and filename-derived quantization evidence.

The current gap is depth. Pantograph's image-generation family planner needs
structured family/component facts that are not currently available in a direct,
consumer-stable shape.

## Implementation Architecture

Pumas should implement this as a package-facts module split before adding new
image-generation extraction behavior. The public `ModelLibrary` API remains the
facade, but `library.rs` must not keep growing as the extraction owner.

Target Pumas module shape:

```text
rust/crates/pumas-core/src/model_library/package_facts/
  mod.rs
  artifact.rs
  manifest.rs
  transformers.rs
  diffusers.rs
  gguf.rs
  generation.rs
  summary.rs
  README.md
```

Ownership boundaries:

- `library.rs`: orchestration facade for public APIs such as
  `resolve_model_package_facts`, selector snapshots, and cache/update entry
  points. It delegates package inspection and projection to focused modules.
- `package_facts/artifact.rs`: selected artifact kind, storage kind, entry path,
  validation state, and selected-file projection.
- `package_facts/manifest.rs`: deterministic package inspection manifests used
  by extraction and cache fingerprinting. The manifest lists only bounded
  standard files that may contribute facts for the selected artifact.
- `package_facts/transformers.rs`: Transformers-compatible config, task,
  tokenizer/config evidence, and custom-code evidence from Transformers files.
- `package_facts/diffusers.rs`: `model_index.json` and nested component config
  extraction for Diffusers-style packages.
- `package_facts/gguf.rs`: bounded GGUF header/metadata extraction and mmproj
  companion evidence.
- `package_facts/generation.rs`: model-provided generation defaults and source
  tagging for normalized defaults.
- `package_facts/summary.rs`: compact selector-summary projection from full
  package facts without owning selector query execution.

Existing Pumas modules should keep their current narrower jobs:

- `external_assets.rs` remains import/validation metadata and should not become
  the full package-facts extraction owner.
- `identifier.rs` remains broad file identification. Any GGUF parser added for
  package facts may share low-level bounded parsing helpers, but it should live
  behind the package-facts extraction contract.

The new `package_facts/README.md` should document the API consumer contract and
structured producer contract because these facts are consumed by Pantograph and
persisted in selector/cache rows.

Implementation invariants:

- One selected-artifact identity source: `PackageInspectionContext`.
- One package file source: `PackageInspectionManifest`.
- One cache freshness classifier or query wrapper for package-facts cache rows.
- One summary projection owner: `package_facts/summary.rs`.
- One package-facts migration report family or typed report-envelope payload.
- Selector snapshots and summary snapshots never hydrate package facts.
- Package-facts extraction never infers family from display names, workflow
  names, directory names, basenames, or model-specific lookup tables.
- Old package-facts contract rows are stale rows, not fallback facts.
- Pumas facts remain source-tagged, factual, and consumer-agnostic.

Package inspection context:

- P0 should introduce an internal `PackageInspectionContext` before adding new
  extraction behavior. It should carry the normalized model id, model directory,
  validated metadata, descriptor facts, selected artifact id/path, selected
  artifact file set, and dependency bindings needed by package-facts extraction.
- The context is the only place where selected artifact identity is resolved.
  Resolver locks, cache reads, cache writes, `PumasModelRef`, update events,
  manifest construction, and summary projection must all receive the same
  selected artifact identity from this context.
- The current default-empty selected-artifact row may remain only for packages
  that genuinely have no selected artifact. If a selected artifact exists, Pumas
  must not read, write, or project facts through `None` or an empty artifact id.
- The context must be constructed from Pumas-owned factual metadata and package
  evidence only. It must not contain Pantograph workflow ids, scheduler choices,
  backend/device decisions, or runtime support verdicts.

Selected-artifact behavior:

- Detail and summary facts describe the selected artifact when a selected
  artifact exists.
- Cache keys use model id, selected artifact id, package-facts contract version,
  and source fingerprint.
- `PumasModelRef.selected_artifact_id` and `selected_artifact_path` should be
  populated when the inspected package has selected-artifact metadata.
- Repositories with multiple GGUF quantizations, mmproj companions, LoRAs,
  adapters, or multiple weight formats must not collapse those facts into a
  model-level-only cache row.

Package inspection manifest behavior:

- The manifest is constructed from validated metadata, selected artifact files,
  and package-standard file locations.
- The manifest replaces filename-only selected-file fallback behavior for
  package-fact extraction and fingerprinting. Relative paths must be preserved
  so nested files such as `scheduler/scheduler_config.json`,
  `transformer/config.json`, and tokenizer component files can be attributed and
  invalidated correctly.
- The same manifest feeds extraction and source fingerprinting so changing a
  file that can affect facts invalidates the corresponding cache row.
- Diffusers manifests include `model_index.json` plus bounded component config
  files such as `scheduler/scheduler_config.json`, `transformer/config.json`,
  `unet/config.json`, `vae/config.json`, text-encoder configs, tokenizer
  configs, processor configs, and image-processor configs when present.
- GGUF manifests include the selected GGUF artifact plus selected mmproj or
  companion files needed for factual companion evidence.
- Manifest construction must stay independent of host runtime policy and must
  not scan arbitrary package depth during selector snapshots.

## DTO Design Addendum

The implementation should add explicit DTOs rather than overloading existing
human-readable diagnostic fields.

Expected DTO additions or equivalents:

- `DiffusersPackageEvidence`
- `DiffusersComponentFacts`
- `DiffusersComponentRole`
- `ImageGenerationFamilyEvidence`
- `ImageGenerationFamilyEvidenceSource`
- `GgufPackageEvidence`
- `PackageFactValueSource`
- `PackageInspectionManifest`
- `PackageInspectionManifestEntry`

Structured fields should carry machine-consumed facts. Any existing `message`
field remains human diagnostics only and must not become the transport for
family, component, quantization, task, scheduler, or custom-code semantics.

Value-source fields should distinguish:

- header-derived facts
- config-derived facts
- upstream metadata facts
- component-layout facts
- filename-derived weak evidence
- ambiguous or unavailable evidence

Typed fields should replace machine use of `ProcessorComponentFacts.message`.
Existing human diagnostic messages can remain for display and debugging, but
new consumers must not need to parse `message` text to recover tokenizer
internals, shard provenance, quantization, image-family evidence, scheduler
facts, or component semantics.

## Required Pumas Facts

### Diffusers Bundle Evidence

Add or extend a package-facts section for diffusers bundles. It should be factual
and independent of Pantograph runtime policy.

Required fields:

- `pipeline_class`: value from `model_index.json` `_class_name`.
- `diffusers_version`: value from `model_index.json` when present.
- `name_or_path`: value from `model_index.json` when present.
- `task`: upstream task or pipeline tag plus normalized input/output modalities.
- `family_evidence`: bounded labels derived from package facts, such as
  `stable_diffusion`, `stable_diffusion_xl`, `flux`, `flux2`, `qwen_image`,
  `lumina_image`, `glm_image`, `z_image`, or `unknown`.
- `family_evidence_source`: one of `pipeline_class`, `model_index_component`,
  `component_config`, `repo_metadata`, or `ambiguous`.
- `components`: stable component-role records with role, relative path,
  source library, class name, status, and optional config path.

Initial component roles:

- `pipeline_index`
- `scheduler`
- `tokenizer`
- `tokenizer_2`
- `text_encoder`
- `text_encoder_2`
- `text_encoder_3`
- `image_processor`
- `processor`
- `unet`
- `transformer`
- `vae`
- `controlnet`
- `adapter`
- `weights`
- `generation_config`

Nested component config parsing should read only files inside the approved bundle
root. Useful files include:

- `scheduler/scheduler_config.json`
- `unet/config.json`
- `transformer/config.json`
- `vae/config.json`
- `text_encoder/config.json`
- `text_encoder_2/config.json`
- `text_encoder_3/config.json`
- tokenizer and processor configs under their component directories

The facts should preserve class names and config `model_type` values, not load
Python classes or instantiate Diffusers pipelines.

Diffusers package-fact extraction may reuse existing Pumas path normalization
and component path validation helpers from `external_assets.rs`, but it must not
reuse import-time support filters as package-fact support policy. Import
validation can decide whether a bundle is suitable for a specific Pumas import
flow; package facts should preserve explicit pipeline and component evidence for
supported, unsupported, unknown, and ambiguous families alike.

### Image Family Requirements

Pumas does not need to validate whether Pantograph supports a family, but it
should provide enough facts for Pantograph to validate deterministically.

Minimum family evidence:

| Family | Pumas Facts Needed |
| ------ | ------------------ |
| SD / SDXL | Pipeline class, UNet, VAE, scheduler, tokenizer/text encoder roles, SD vs SDXL evidence, optional refiner/inpaint/controlnet evidence. |
| FLUX | Pipeline class or component evidence, transformer role, VAE role, tokenizer/text encoder roles, dtype/config evidence, scheduler evidence. |
| FLUX.2 | FLUX.2-specific family evidence, transformer role, VAE role, Qwen/Mistral-style encoder evidence where present, dtype/config evidence. |
| Qwen Image | Qwen image pipeline or component evidence, transformer role, VAE role, tokenizer/text encoder facts, processor facts when present. |
| Lumina Image | Lumina pipeline or component evidence, transformer role, VAE role, Gemma-style or other text encoder evidence, scheduler facts. |
| GLM Image | GLM image package evidence when available; if package files do not identify the family, expose `unknown` with diagnostics rather than guessing from names. |
| Z-Image | Z-Image pipeline or component evidence, transformer role, VAE or explicit pixel-space variant evidence, Qwen3/tokenizer/text-encoder facts. |

If evidence is ambiguous, Pumas should expose the ambiguity. It should not infer
family from display name, saved workflow name, or directory name.

### GGUF Evidence

Add GGUF header metadata extraction for selected GGUF artifacts. This supports
Pantograph's llama.cpp planning without making Pumas select a runtime.

Useful facts:

- GGUF architecture, such as `general.architecture`.
- file type / quantization from GGUF metadata.
- tokenizer model and chat template fields when present.
- context length and embedding length when present.
- block/layer count and attention-head facts when present.
- multimodal projector companion evidence, such as `mmproj` files.
- whether the artifact appears to be text generation, embedding, reranking, VLM,
  or unknown based on package facts and upstream task metadata.

Filename-derived quantization may remain as weak source-tagged evidence only
when GGUF metadata does not provide a stronger value. It must not be treated as
a substitute for header metadata or as an execution decision.

### Generation Defaults

Pumas should expose model-provided defaults without treating them as user
overrides or runtime policy.

For text/LLM packages:

- parse `generation_config.json`
- preserve older upstream config-carried generation keys with a diagnostic

For diffusers packages:

- parse scheduler defaults from `scheduler/scheduler_config.json`
- preserve pipeline/package defaults from `model_index.json` only when present
- expose defaults as raw JSON plus normalized known keys where stable

Candidate normalized keys:

- `num_inference_steps`
- `guidance_scale`
- `negative_prompt_supported`
- `height`
- `width`
- `scheduler_class`
- `prediction_type`
- `timestep_spacing`
- `clip_sample`

Pantograph may merge these with workflow/run options, but Pumas should only
publish the model/package-provided facts and their source paths.

### Custom Code And Trust Evidence

Pumas should preserve custom-code facts without executing code:

- `auto_map` entries
- custom pipeline declarations
- custom component class references
- local `custom_pipeline.py` or custom module files when identifiable
- `requirements.txt` and component-specific dependency manifests
- remote-code/trust-required diagnostics

Pantograph can then enforce its trust policy before PyTorch/Transformers or
Diffusers execution.

## Snapshot And Cache Behavior

Fast selector snapshots must remain fast and SQLite-backed.

- Startup model lists should not parse every model package.
- Selector rows may include cached package-fact summaries.
- Missing or stale summaries should be explicit states, not hidden full scans.
- Full package facts should be lazily regenerated for selected models or during
  bounded import/index refresh.
- Summary and detail cache rows should be keyed by model id, selected artifact id,
  contract version, and source fingerprint.
- Source fingerprints should be derived from the package inspection manifest,
  not from a separate hardcoded file list.
- Any package-facts change should advance the model-library update cursor.
- Existing cache rows from older package-facts contract versions are stale cache
  rows, not fallback facts. They must be invalidated or regenerated before being
  exposed as fresh facts to clients.
- Package parsing must not happen while holding SQLite connection locks.
  Acquire database connections only for cache reads, cache writes, summary
  projection persistence, and update-event publication.

Pantograph should be able to load a selector snapshot, keep the returned cursor,
and then consume Pumas update events without missing changes between startup and
subscription.

## Performance And Lifecycle Constraints

The package-facts additions must preserve Pumas' fast selector path.

- Selector snapshots must not hydrate all package facts.
- Startup model lists must not parse all package files.
- Missing, stale, or invalid summary rows are explicit states, not hidden
  full-package scans.
- Full diffusers component parsing happens only for selected-model hydration or
  bounded import/index refresh.
- Selector snapshots may read cached manifest-derived summaries, but must never
  construct or hydrate the full package inspection manifest inline.
- GGUF parsing reads only the header and metadata section. It must never read
  tensor data for package facts.
- File reads are bounded and run through existing Pumas path validation.
- Long or blocking filesystem work must be isolated from async request paths
  according to the Rust async standards used by Pumas.
- Cache keys include model id, selected artifact id, contract version, and source
  fingerprint so changed packages cannot reuse stale facts silently.
- Changing a manifest-listed nested Diffusers config or selected GGUF/mmproj file
  must invalidate the affected detail and summary cache without requiring a full
  startup package scan.
- Package-facts contract upgrades must be handled by an explicit migration or
  backfill path. Selector snapshots may report stale/missing states during the
  upgrade window, but must not hydrate packages or reuse old-contract summary
  rows to hide migration work.

## Package-Facts Cache Migration And Backfill Design

Pumas already has migration reports, checkpointed execution, package-facts cache
rows, and model-library update events. The image-generation facts work should
extend that existing migration model rather than adding a separate refresh
mechanism.

The migration target is all indexed models and selected artifacts whose cached
package facts are missing, invalid, fingerprint-stale, or produced by an older
`PACKAGE_FACTS_CONTRACT_VERSION`.

Required migration behavior:

- Bump `PACKAGE_FACTS_CONTRACT_VERSION` whenever persisted package-facts JSON or
  public package-facts semantics change.
- Treat old-contract detail and summary rows as stale. They are never fallback
  facts for Pantograph or any other client.
- Enumerate the selected artifacts for each indexed model through
  `PackageInspectionContext`.
- Regenerate detail and summary rows per `model_id + selected_artifact_id +
  contract_version` using the same extraction path used by normal selected-model
  hydration.
- Delete or invalidate obsolete empty-selected-artifact rows when the model has
  concrete selected artifacts.
- Preserve empty-selected-artifact rows only for packages that genuinely have no
  selected artifact.
- Emit `PackageFactsModified` update events for changed detail or summary rows,
  including selected artifact id when present.
- Keep selector snapshots SQLite-backed and non-hydrating while migration runs.

Dry-run report requirements:

- Report per model and selected artifact.
- Include current detail and summary row state: `fresh`, `missing`,
  `stale_contract`, `stale_fingerprint`, `invalid_json`,
  `wrong_selected_artifact`, `blocked_partial_download`, or `error`.
- Include selected artifact id/path and source fingerprint inputs when
  available.
- Report whether detail will be regenerated, summary will be regenerated, or
  obsolete rows will be deleted.
- Preserve parse, manifest, and validation diagnostics without making consumer
  support decisions.

Checkpointed execution requirements:

- Persist pending work by `model_id`, `selected_artifact_id`, target
  package-facts contract version, and source fingerprint.
- Resume without repeating completed package-facts regeneration work.
- Recompute source fingerprints before writing rows so interrupted migrations do
  not publish stale facts after files change.
- Record per-row execution results in machine-readable and human-readable
  migration artifacts.
- Publish update-feed events after durable cache writes, not before.

Post-migration validation requirements:

- Count missing, stale-contract, stale-fingerprint, invalid-json, and
  wrong-selected-artifact cache rows.
- Verify every completed selected artifact has matching detail and summary rows
  at the target package-facts contract version.
- Verify selector snapshot and summary snapshot APIs expose fresh rows for
  completed artifacts and explicit stale/missing states for incomplete artifacts.
- Verify obsolete old-contract rows are absent or ignored by all public summary
  paths.

Cache freshness classification:

- Add one shared cache-row classification helper for package facts. It should
  classify row state as `fresh`, `missing`, `stale_contract`,
  `stale_fingerprint`, `invalid_json`, `wrong_selected_artifact`, or `error`.
- Detail resolution, summary resolution, selector snapshots, summary snapshots,
  migration dry-run, and post-migration validation must use this shared
  classification helper or a single query wrapper that owns the same semantics.
- Raw SQL call sites must not independently decide whether a row is usable by
  checking only JSON validity or row presence.
- Public summary/selector DTOs should expose enough status information for
  clients to distinguish stale contract rows from missing rows and invalid JSON.
- Cache lookup APIs should prefer a `PackageInspectionContext` or
  selected-artifact-aware request object over `(model_id, Option<&str>)` so
  empty-string/default-artifact behavior is not accidentally reused for concrete
  selected artifacts.

Package-facts migration report shape:

- Reuse the existing migration report directory, report index, checkpoint file
  safety patterns, and UI/report listing infrastructure.
- Do not overload metadata v2 move-oriented report DTOs with package-facts cache
  states. Add distinct package-facts migration dry-run, execution, item, and
  checkpoint DTOs or a typed migration-report envelope with separate payloads.
- Package-facts migration report rows should be keyed by model id, selected
  artifact id, target package-facts contract version, and source fingerprint.
- Report fields should name package-facts operations directly, such as
  regenerated detail rows, regenerated summary rows, deleted obsolete rows,
  skipped partial downloads, stale contract rows, stale fingerprint rows, invalid
  JSON rows, and wrong selected-artifact rows.
- If the Pumas UI continues to render both migration families in one panel, the
  frontend should switch on report kind and render package-facts-specific fields
  rather than hiding them behind metadata migration wording.

Batch write and update-feed behavior:

- Add a batch cache upsert/delete path for migration backfill where practical.
  It should write cache rows and obsolete-row deletions in a bounded transaction,
  collect event ids, commit, then publish update notifications.
- The existing single-row upsert path can remain for selected-model hydration,
  but migration backfill should avoid per-row event storms and avoid holding
  SQLite locks across package parsing.
- A backfill batch must not publish an update event for a row that failed to
  persist.

## Required Fixtures

Pumas should add package-facts fixtures for:

- Tiny SD Turbo or another small SD text-to-image bundle.
- Juggernaut X v10 SDXL or a representative SDXL bundle.
- FLUX text-to-image bundle.
- FLUX.2 bundle.
- Qwen Image bundle.
- Lumina Image bundle.
- GLM Image bundle, or an explicit unsupported/unknown fixture if local package
  evidence is insufficient.
- Z-Image bundle.
- GGUF text-generation artifact with header-derived context/tokenizer facts.
- GGUF embedding or reranker artifact.
- GGUF VLM artifact with multimodal projector companion.
- Missing `model_index.json`.
- Invalid `model_index.json`.
- Missing required component directory.
- Ambiguous family evidence.
- Custom-code-required diffusers bundle.
- Existing SQLite cache fixture with old package-facts contract detail and
  summary rows.
- Existing SQLite cache fixture with invalid package-facts JSON.
- Existing SQLite cache fixture with stale source fingerprints.
- Existing SQLite cache fixture with multi-artifact rows and obsolete
  empty-selected-artifact rows.
- Existing SQLite cache fixture with partial-download rows that migration must
  report but not hydrate.

Fixtures should be small and synthetic where possible. They should encode the
package shape and metadata needed for contract tests without requiring large
model weights.

## Pantograph Consumption Contract

Pantograph expects Pumas facts to support this deterministic flow:

1. The `puma-lib` node provides a stable Pumas model ref plus cached summary
   facts for model lists.
2. On selected-model hydration, Pantograph gets full package facts.
3. The inference planner validates task, artifact kind, family evidence,
   component roles, generation defaults, custom-code facts, dependency facts,
   and backend hints.
4. The scheduler chooses backend/runtime/device from Pantograph-owned candidate
   facts.
5. The inference backend executes exactly the scheduler-selected decision.

If Pumas facts are missing or ambiguous, Pantograph should fail planning with a
typed diagnostic. It must not guess from names or substitute another family,
backend, scheduler, device, or model interpretation.

## Standards Compliance And Blast Radius

| Standard Area | Plan Constraint |
| ------------- | --------------- |
| Plan structure | Objective, scope, milestones, verification, risks, re-plan triggers, and completion criteria remain explicit in this file. |
| Architecture | Pumas stays the factual producer; Pantograph stays the scheduler, runtime, diagnostics-ledger, and graph-semantics owner. |
| File organization | New extraction behavior is blocked behind a `library.rs` decomposition milestone and focused `package_facts` modules. |
| Documentation | The new Pumas `package_facts/` source directory gets a README with API consumer and structured producer contracts. |
| Testing | Each milestone requires fixture or unit tests, and cross-repo fixture alignment proves Pantograph can consume the facts without name guessing. |
| Interop | DTOs carry structured fields with stable serde shapes; human `message` text is not used as a machine contract. |
| Security | All package file reads stay inside validated package roots; path escape cases are tested. |
| Concurrency | Existing Pumas concurrency design is preserved by exception; SQLite locks are not held across filesystem parsing, and blocking parsing remains isolated from async paths. |
| Performance | Selector snapshots remain SQLite-backed and non-hydrating; package parsing is lazy or bounded import/index work. |
| Dependencies | Any new parser dependency must be justified by bounded parsing needs and rejected if it requires unsafe, tensor-loading, or oversized behavior. |
| Rust API | Public DTOs use typed enums/newtypes where they encode expensive invariants; fallible public APIs return structured errors instead of `Result<T, String>` or panics. |
| Tooling | Every implementation slice runs the Pumas repo's owned format, check, and targeted test commands before commit. |

## Code-Area Findings And Required Constraints

These findings turn the standards review into concrete implementation
constraints:

- `rust/crates/pumas-core/src/model_library/library.rs` is currently the largest
  blast-radius risk. It is over the standards decomposition threshold by a wide
  margin and already owns package-facts resolution, summary repair, cache writes,
  fingerprinting, and tests. P0 must move package-facts-specific logic out of
  this file before P1-P6 add behavior.
- `rust/crates/pumas-core/src/models/package_facts.rs` is already a public wire
  DTO owner. It may remain the DTO facade, but new nested DTO groups should be
  split only when the module gains separate responsibilities that cannot be
  reviewed clearly. Any split must preserve public re-exports and serde wire
  shape.
- `rust/crates/pumas-core/src/model_library/package_facts/` is a new source
  directory. It must include a README that satisfies the directory
  documentation template, including API consumer contract and structured
  producer contract sections.
- `rust/crates/pumas-core/src/index/model_index/model_selector_snapshot.rs`
  should remain a cached-summary reader. It must not construct package
  inspection manifests, parse package files, or repair summaries inline.
- `rust/crates/pumas-core/src/index/model_index/package_facts_cache.rs` and the
  package-facts cache schema are persisted-artifact owners. Selected-artifact
  keys, contract-version changes, and source-fingerprint semantics must be
  verified with persistence tests, not only DTO tests.
- Package-facts cache freshness classification must have one code owner. Detail
  resolve, summary resolve, selector snapshots, summary snapshots, migration
  dry-run, and post-migration validation must not duplicate row usability logic.
- Existing metadata migration DTOs are move/reclassification contracts. They may
  share artifact storage and UI listing infrastructure with package-facts
  migration reports, but package-facts cache migration must use distinct typed
  payloads or a typed report envelope.
- Pumas frontend, Electron, UniFFI, C# binding, and any generated or manually
  maintained host-language contract surfaces are interop blast radius. If the
  changed DTOs or APIs cross those surfaces, the host-language type, validation,
  and smoke checks must change in the same logical slice.
- Pantograph fixture copies are consumer contract artifacts, not implementation
  details. Pumas and Pantograph fixture changes must remain traceable to the
  same contract version and be verified by both repositories before the Pumas
  contract is treated as stable.
- Existing `model_library/external_assets.rs` path validation and package-root
  helpers may be reused, but import support filters must not become package-fact
  policy. Support decisions remain a consumer concern.
- No implementation slice may add model-specific repository lookup tables,
  consumer-specific support verdicts, runtime/device selection, or fallback
  guessing to make a fixture pass.

## Cross-Layer Contract Update Requirements

When a package-facts DTO, selector snapshot field, cache row shape, update event,
or public API changes, the same logical slice must update every affected layer:

- Rust DTOs and serde fixtures.
- SQLite cache/schema code and cache round-trip tests.
- Public facade methods on `ModelLibrary`.
- UniFFI or other binding surfaces if the changed API is exported.
- Pumas frontend or Electron client types if they consume the shape directly.
- Pantograph fixture copies and fixture-consumption tests when the changed facts
  are required by Pantograph.

If an existing surface is intentionally unaffected, the implementation report or
commit message should state why. No stale handwritten mirror type should remain
after a DTO change.

Breaking contract changes are allowed when they are the clean canonical design,
but they must be explicit semver/release-note items for Pumas. Do not add
compatibility shims or fallbacks for old Pantograph graph shapes.

## Worker Coordination

Parallel implementation is allowed only after P1 freezes the shared DTO and
fixture shape.

Serial owner:

- P0 module split and `library.rs` refactor.
- P1 DTO/serde/cache schema contract.
- P4 summary cache and update-feed projection.
- P5 package-facts cache migration/backfill.
- P6 cross-repo fixture alignment and final integration.

Parallel worker wave after P1:

| Worker | Primary Write Set | Allowed Adjacent Write Set | Forbidden Shared Files |
| ------ | ----------------- | -------------------------- | ---------------------- |
| Diffusers facts | `rust/crates/pumas-core/src/model_library/package_facts/diffusers.rs`, diffusers fixture data, diffusers-focused tests | `package_facts/manifest.rs` only through documented extension hooks | `models/package_facts.rs`, cache schema, selector snapshot SQL, public API wiring, lockfiles |
| GGUF facts | `rust/crates/pumas-core/src/model_library/package_facts/gguf.rs`, GGUF fixture data, GGUF-focused tests | `package_facts/manifest.rs` only through documented extension hooks | `models/package_facts.rs`, cache schema, selector snapshot SQL, public API wiring, lockfiles |

Each worker may read broadly but must record required changes outside its write
set in a worker report instead of editing shared contracts. Integration then
lands shared-contract or schema changes serially and reruns the wave
verification.

Suggested worker report paths in the Pumas implementation branch:

- `docs/plans/current-image-generation-graphs/reports/diffusers-facts-worker.md`
- `docs/plans/current-image-generation-graphs/reports/gguf-facts-worker.md`

Coordination ledger path in the Pumas implementation branch:

- `docs/plans/current-image-generation-graphs/coordination-ledger.md`

Worker integration sequence:

1. Start workers from the same clean P1 integration commit.
2. Review each worker report before merging code.
3. Verify changed files match the assigned write set.
4. Integrate one worker branch at a time.
5. Resolve shared-contract or fixture conflicts in a separate integration
   commit owned by the serial integrator.
6. Run the wave verification matrix before starting P4.
7. Delete worker worktrees or temporary clones only after confirming they have
   no uncommitted changes and their commits are reachable from the integration
   branch.

## Verification Matrix

Each milestone should use the Pumas repository's owned commands where available.
The minimum verification intent is:

| Slice | Required Verification |
| ----- | --------------------- |
| P0 | Format/check; existing package-facts tests; selector snapshot tests; manifest fingerprint parity tests. |
| P1 | DTO serde round trips; cache schema round trips; binding or host-language contract smoke checks when exported; Pantograph fixture-shape check. |
| P2 | Diffusers unit tests for every family fixture; path escape tests; unsupported/unknown/ambiguous evidence tests; no name-derived classification tests. |
| P3 | GGUF header tests; corrupt/invalid GGUF diagnostics; multi-quant selected-artifact cache tests; no tensor-read checks. |
| P4 | Warm selector snapshot performance check; cursor handoff test; stale/missing/invalid summary tests; manifest invalidation tests. |
| P5 | Shared cache-classifier tests; package-facts migration dry-run tests; checkpoint resume tests; old-contract/stale-fingerprint/invalid-json backfill tests; batch update-feed event tests; selector snapshot stale/fresh state tests. |
| P6 | Pumas fixture suite; Pantograph fixture-consumption suite; cross-layer acceptance proving Pantograph can plan/reject from Pumas facts alone. |

Any performance claim, such as selector snapshots staying within the existing
fast-startup target, must be measured with the same benchmark or timing harness
used by Pumas for the selector snapshot path.

## Implementation Findings

- 2026-05-10 P0 module split finding: fixed in P1/P3 follow-up. Package-facts detail and summary cache paths now use `PackageInspectionContext` selected-artifact identity when `metadata.selected_artifact_id` is present, and manifests prefer `selected_artifact_files` over all indexed sibling files.
- 2026-05-10 P1 DTO finding: current fixtures and PACKAGE_FACTS_CONTRACT_VERSION = 1 are sensitive to wire-shape changes. Image-generation facts require an explicit contract-version bump and fixture updates when diffusers/GGUF DTOs are added.
- 2026-05-10 large-file rationale: library.rs remains over the decomposition threshold at 11637 lines after package-facts extraction. The remaining size is legacy ModelLibrary facade, migration, import, projection, and test ownership outside this image-generation facts slice; further reductions should proceed through separate facade/migration/import decomposition rather than blocking the DTO and extractor work.
- 2026-05-10 P2 extractor finding: the initial Diffusers extractor now reads `model_index.json` and known nested component configs through an explicit bounded UTF-8 JSON reader, and it now emits component missing/invalid and ambiguous-family diagnostics. Remaining P2 fixture breadth still needs family-specific fixtures and malformed large JSON coverage beyond the unit-level oversized file case.
- 2026-05-10 P3 integration finding: the initial GGUF extractor reads bounded header metadata and preserves corrupt legacy placeholder files as invalid evidence, but resolver wiring still selects the first non-mmproj GGUF from the selected files. Multi-quant selected-artifact-aware cache semantics remain open until `PackageInspectionContext` owns the concrete selected artifact id/path end to end.
- 2026-05-10 P4 summary owner finding: selector and summary snapshots now share one package-facts summary cache classifier for missing, invalid, stale-contract, and wrong-selected-artifact states. Compact summary projection ownership moved into `package_facts/summary.rs` while preserving the existing public `ResolvedModelPackageFactsSummary::from(&facts)` conversion.
- 2026-05-10 DTO decomposition review: `rust/crates/pumas-core/src/models/package_facts.rs` is 684 lines after the DTO additions. It remains a single readable wire-contract module because extraction, projection, cache classification, and parsing behavior are owned elsewhere; split it when it crosses roughly 800 lines or when a DTO group needs a separate version/lifecycle boundary.
- 2026-05-10 P5 dry-run finding: Pumas commit `9da50b8d` adds a non-mutating package-facts cache migration dry-run report and crate-internal row-state classifier shared by summary snapshots and dry-run inventory. The report inventories indexed models by current selected artifact, reports detail/summary row states, flags regenerate decisions, and identifies obsolete empty-selected-artifact rows without writing checkpoints, regenerating facts, deleting rows, or publishing events.
- 2026-05-10 P5 partial-download follow-up: the dry-run slice computes the partial-download flag from the index row, but metadata-less partial rows still need a dedicated fixture before the skipped/blocked behavior is complete. A later P5 slice must ensure partial downloads report `blocked_partial_download` without attempting missing package-file hydration.
- 2026-05-10 P5 report artifact finding: Pumas commit `93a4fae1` persists package-facts dry-run reports as package-facts-specific JSON and Markdown artifacts in the existing migration report directory and index. This covers dry-run machine/human report output for planned regenerate/delete-obsolete/skipped/error fields, but execution/checkpoint DTOs and actual regenerated/deleted row result fields remain open.

## Implementation Sequencing

Execution should proceed in validated thin slices:

1. P0 is a behavior-preserving refactor. It must move existing package-facts
   helpers into focused modules before any new fact depth is added.
2. P1 freezes DTO shape with serde and fixture tests, including selected-artifact
   identity and source-tagged evidence semantics.
3. P2 starts with one smallest useful Diffusers vertical slice, preferably a
   synthetic SD or SDXL fixture, then expands adjacent family fixtures through
   the same extraction path.
4. P3 adds GGUF header extraction as a separate slice because it has different
   file parsing and performance risks.
5. P4 projects summaries and update events only after detail facts have stable
   source-tagged semantics.
6. P5 upgrades existing SQLite package-facts cache rows after summary
   projection, update events, selected-artifact identity, and source
   fingerprint semantics are stable.
7. P6 validates Pantograph consumption after Pumas fixtures and summaries are
   stable.

Implementation work should follow the planning standard's worktree hygiene and
commit cadence: inspect status before starting, keep each slice reviewable, run
the slice verification before commit, and commit code, tests, and documentation
that belong to the same slice together. Dirty implementation files from one
slice must not be carried into the next slice without an explicit ownership
decision.

Parallel workers are only safe after P1 freezes DTOs. At that point P2
Diffusers extraction and P3 GGUF extraction can be delegated independently if
their write sets stay limited to `package_facts/diffusers.rs`,
`package_facts/gguf.rs`, related tests, and fixtures. Summary projection,
shared DTOs, cache schema, and public API wiring remain serial integration work.

## Milestones

### Milestone P0: Package-Facts Module Split And `library.rs` Refactor

**Goal:** Make package-facts extraction standards-compliant before adding deeper
image-generation behavior.

**Tasks:**
- [x] Extract existing package-fact helper logic from `library.rs` into
      `model_library/package_facts/` modules without changing public API
      behavior.
- [x] Add `package_facts/manifest.rs` and move existing selected-file and
      fingerprint file-list logic behind a behavior-preserving package
      inspection manifest.
- [x] Move selected artifact, component, Transformers config, generation
      defaults, custom-code evidence, and summary projection helpers behind
      focused module functions.
- [x] Introduce `PackageInspectionContext` as the single internal source of
      selected artifact identity for resolver locks, cache reads/writes,
      manifest construction, `PumasModelRef`, update events, and summary
      projection.
- [x] Replace package-facts extraction and fingerprinting uses of filename-only
      selected-file fallback with manifest entries that preserve relative paths.
- [x] Preserve existing cache behavior while making the refactor boundary ready
      for selected-artifact-aware cache keys in P1.
- [x] Keep `ModelLibrary::resolve_model_package_facts` and selector APIs as thin
      orchestration facades.
- [x] Add `model_library/package_facts/README.md` with ownership, API consumer
      contract, structured producer contract, unsupported behavior, and revisit
      triggers.
- [x] Record any remaining large-file rationale if `library.rs` still exceeds the
      standards soft threshold after package-facts extraction.
- [x] Record a decomposition review for `models/package_facts.rs` if the DTO
      additions push it beyond a readable single-contract module.
- [x] Keep package-facts helpers `pub(crate)` unless they are intentionally part
      of the public crate contract.

**Verification:**
- Existing Pumas package-facts tests pass unchanged.
- Existing selector snapshot and update-feed tests pass unchanged.
- Public DTO serde fixtures remain byte-for-byte compatible unless a planned DTO
  contract addendum is applied in P1.
- Manifest-derived fingerprints match current fingerprints for existing
  root-level package-facts fixtures.
- A selected-artifact fixture proves resolver lock scope, cache lookup,
  `PumasModelRef`, update event metadata, and summary projection all use the
  same selected artifact id/path.
- A nested-file manifest fixture proves relative paths are preserved and
  filename-only discovery is not used for Diffusers component files.
- No new diffusers or GGUF behavior is introduced in this milestone.
- `library.rs` no longer owns package-facts extraction details, source
  fingerprint assembly, or summary projection logic except as a facade.
- The new source directory README includes concrete Pumas-specific rationale,
  invariants, dependency notes, API consumer contract, and structured producer
  contract sections.

### Milestone P1: Contract Addendum

**Goal:** Define versioned, consumer-stable DTO additions for richer diffusers
and GGUF facts.

**Tasks:**
- [x] Add or extend package-facts DTOs for diffusers bundle evidence.
- [x] Add or extend package-facts DTOs for GGUF metadata evidence.
- [x] Add explicit source fields for derived facts.
- [x] Add selected-artifact-aware detail and summary cache semantics to the DTO
      contract, including populated `PumasModelRef.selected_artifact_id` and
      `selected_artifact_path` when available.
- [x] Add or extend public summary/selector status DTOs so clients can
      distinguish fresh, missing, stale-contract, stale-fingerprint,
      invalid-json, wrong-selected-artifact, and error states.
- [x] Add `PackageInspectionManifest` DTOs or internal structs with documented
      semantics when manifests are not public.
- [x] Add a selected-artifact contract fixture before family-specific fixtures,
      covering a multi-artifact package with distinct selected artifact ids,
      paths, source fingerprints, detail rows, and summary rows.
- [ ] Replace machine use of `ProcessorComponentFacts.message` with typed fields
      for tokenizer details, shard provenance, quantization, family evidence,
      scheduler facts, and component semantics where those facts are consumed.
- [x] Add round-trip fixtures for the new DTO shape.
- [x] Document that Pumas facts are advisory/factual and not scheduler policy.
- [x] Document that package-family labels are source-tagged evidence, not support
      verdicts.
- [x] Update every exported host-language or frontend contract surface that
      consumes the changed DTO shape, or record why the surface is not affected.
- [ ] Prefer generated schema/type output for host-language mirrors when Pumas
      already has an owned generation path. If a mirror remains handwritten, add
      fixture-driven tests proving Rust serde output and the mirror type stay in
      sync.
- [x] Bump the package-facts contract version when persisted or public serde
      shape changes.
- [x] Add explicit serde `rename_all` or field renames for new public DTOs so
      the wire format does not rely on Rust identifier casing.

**Verification:**
- DTO serde round trips.
- Fixture compatibility against Pantograph expected package-facts shape.
- No Pantograph-specific workflow/runtime fields in Pumas DTOs.
- Rust DTOs, frontend types, binding types, and Pantograph fixture copies either
  derive from the same schema or are checked by the same canonical fixture JSON.
- Tests prove human diagnostic `message` text is not required to recover any
  machine-consumed package fact.
- Tests prove selected-artifact facts preserve selected artifact id/path.
- Cache rows using the new contract version do not deserialize as the previous
  contract by accident.
- Shared cache-classification fixtures prove old-contract, stale-fingerprint,
  invalid-json, wrong-selected-artifact, missing, and fresh rows map to distinct
  statuses.
- Exported binding/frontend smoke checks pass when affected.

### Milestone P2: Diffusers Fact Extraction

**Goal:** Extract nested diffusers package facts from local bundle files.

**Tasks:**
- [x] Parse `model_index.json` into pipeline and component facts.
- [x] Parse nested component configs for scheduler, transformer, UNet, VAE, text
      encoders, tokenizers, processors, and image processors.
- [x] Emit missing/invalid/ambiguous component diagnostics.
- [x] Preserve component source library and class names.
- [x] Keep all reads inside the validated bundle root.
- [x] Reuse Pumas path normalization/component path validation helpers, but keep
      import-time support filters out of package-fact extraction.
- [x] Split low-level Diffusers evidence parsing from import suitability
      validation so unsupported, unknown, and ambiguous bundles still produce
      source-tagged facts.
- [x] Emit `unknown` or `ambiguous` when family evidence cannot be derived from
      package-standard files, even if names appear suggestive.
- [x] Bound every JSON file read by manifest entry type and package-root
      validation before parsing.
- [x] Normalize family and component labels through typed enums or validated
      string constants, not ad hoc strings scattered across extractors.

**Verification:**
- Unit tests for each supported family fixture.
- Missing/invalid component tests.
- Path escape tests.
- Tests prove unsupported or unknown Diffusers pipelines still produce factual
  package evidence instead of being discarded as a consumer support decision.
- Tests prove no family is derived from display name, repo name, workflow name,
  or directory name.
- Malformed large JSON fixtures fail with bounded diagnostics instead of
  unbounded memory growth or panics.

### Milestone P3: GGUF Metadata Extraction

**Goal:** Expose header-derived GGUF facts for llama.cpp planning.

**Tasks:**
- [x] Parse selected GGUF header metadata without loading model tensors.
- [x] Extract or add a bounded low-level GGUF metadata reader that is separate
      from model identification and name/basename classification policy.
- [x] Expose architecture, quantization/file type, tokenizer/chat-template,
      context length, embedding length, layer/attention facts when present.
- [x] Preserve weaker filename-derived quantization only as source-tagged
      diagnostic evidence.
- [x] Preserve mmproj companion facts.
- [x] Scope GGUF evidence to the selected GGUF artifact and selected companion
      artifacts instead of model-level filename aggregation.
- [x] Keep GGUF parsing in a small bounded module; if a parser dependency is
      added, record dependency cost, license, feature flags, and why in-house
      parsing is insufficient.
- [x] Use checked arithmetic for header offsets, lengths, and allocation sizes.
- [x] Keep raw GGUF header facts source-tagged and separate from any classifier
      interpretation that currently lives in broad file identification code.

**Verification:**
- Synthetic/minimal GGUF fixture tests where practical.
- Real small GGUF smoke fixture if available.
- Invalid/corrupt GGUF metadata diagnostic test.
- Multi-quantization fixture proves selected artifact A and selected artifact B
  produce distinct cached facts.
- Tests prove package facts do not rely on filename, basename, or repository
  name classifiers when GGUF header fields are available.
- Tests prove malformed GGUF lengths cannot cause panics, unchecked allocation,
  or tensor reads.

### Milestone P4: Summary Cache And Update Feed

**Goal:** Make richer facts usable without slowing Pantograph startup.

**Tasks:**
- [x] Project compact family/artifact/task/backend/custom-code statuses into
      summary rows.
- [x] Add the shared package-facts cache freshness classifier or query wrapper
      and use it for summary cache reads.
- [x] Make `package_facts/summary.rs` the single owner of summary projection
      from full package facts. Selector snapshots and package summary snapshots
      may query cached summaries, but must not maintain separate projection
      semantics.
- [x] Keep selector snapshot behavior SQLite-backed and non-hydrating.
- [x] Advance update cursors when package facts or summaries change.
- [x] Preserve explicit missing/stale/invalid summary states.
- [x] Use the package inspection manifest to compute source fingerprints for
      detail and summary cache rows.
- [x] Keep selector snapshots from constructing package inspection manifests or
      parsing nested package files.
- [x] Keep summary repair and update-event publication in the package-facts
      facade or summary module; selector SQL remains a read-only projection.
- [x] Ensure selector snapshots and summary snapshot APIs use the same
      selected-artifact-aware cached-summary query shape, including explicit
      default-artifact behavior only when no selected artifact exists.
- [x] Preserve the existing Pumas package-facts lock/update-feed model unless a
      simpler measured alternative is explicitly re-planned.

**Verification:**
- Warm selector snapshot remains within the existing fast-snapshot target.
- Cursor handoff test proves no missed update between snapshot and update feed.
- Stale package-facts cache test regenerates only the selected model detail.
- Nested Diffusers config change test proves `scheduler/scheduler_config.json`
  or `transformer/config.json` invalidates the selected model's cache.
- Selected GGUF/mmproj change test proves only the affected artifact facts are
  invalidated.
- Cache persistence tests cover missing, stale, invalid, selected-artifact,
  default-artifact, and contract-version-mismatch rows.
- Selector snapshot and summary snapshot tests prove both surfaces return the
  same summary state for selected artifacts, default artifacts, stale rows, and
  missing rows.
- Tests prove selector and summary snapshot code paths do not independently
  reimplement cache usability checks.

### Milestone P5: Package-Facts Cache Migration And Backfill

**Goal:** Upgrade existing Pumas SQLite package-facts cache rows to the new
contract without serving stale facts to Pantograph or other clients.

**Tasks:**
- [x] Add a package-facts migration dry-run mode that inventories every indexed
      model and selected artifact.
- [ ] Add distinct package-facts migration dry-run, execution, item, and
      checkpoint DTOs, or a typed migration-report envelope with separate
      package-facts payloads.
- [x] Report detail and summary cache row state per selected artifact:
      `fresh`, `missing`, `stale_contract`, `stale_fingerprint`,
      `invalid_json`, `wrong_selected_artifact`, `blocked_partial_download`, or
      `error`.
- [ ] Add checkpointed execution keyed by model id, selected artifact id, target
      package-facts contract version, and source fingerprint.
- [ ] Regenerate detail and summary cache rows through the canonical
      selected-model package-facts hydration path.
- [ ] Use the shared package-facts cache freshness classifier for dry-run
      inventory, execution decisions, and post-migration validation.
- [ ] Delete or invalidate obsolete empty-selected-artifact rows when the model
      has concrete selected artifacts.
- [ ] Preserve empty-selected-artifact rows only for packages that genuinely
      have no selected artifact.
- [ ] Recompute source fingerprints before durable writes so interrupted
      migrations do not publish stale rows after package files change.
- [ ] Emit `PackageFactsModified` update events after durable cache writes,
      including selected artifact id when present.
- [ ] Add a bounded batch cache upsert/delete path for backfill work where
      practical. It writes rows, deletes obsolete rows, collects event ids,
      commits, then publishes update notifications.
- [ ] Extend post-migration validation to count missing, stale-contract,
      stale-fingerprint, invalid-json, and wrong-selected-artifact rows.
- [ ] Keep selector snapshots and summary snapshots non-hydrating during and
      after migration.
- [ ] Add human-readable and machine-readable report fields for regenerated
      detail rows, regenerated summary rows, deleted obsolete rows, skipped
      partial downloads, and per-model errors.

**Verification:**
- Dry-run fixture with old-contract detail and summary rows reports
  `stale_contract` without mutating the database.
- Migration report fixtures prove package-facts migration uses package-facts
  report payloads, not metadata v2 move/reclassification DTO fields.
- Execution fixture regenerates old-contract rows to the target contract version
  and emits update-feed events.
- Invalid JSON fixture reports and repairs or invalidates the affected row
  without panics.
- Multi-artifact fixture proves each selected artifact gets distinct detail and
  summary rows.
- Interrupted migration fixture resumes without repeating completed rows.
- Source-file-change fixture recomputes the fingerprint before writing migrated
  facts.
- Partial download fixture reports blocked/skipped rows without parsing missing
  package files.
- Selector snapshot and summary snapshot tests prove old-contract rows are not
  exposed as fresh facts.
- Batch write fixture proves update events are published only after durable
  cache writes and are not emitted for failed rows.

### Milestone P6: Cross-Repo Fixture Alignment

**Goal:** Prove Pantograph can consume the new Pumas facts without local
guesswork.

**Tasks:**
- [ ] Export or document canonical package-facts fixtures.
- [ ] Add Pantograph fixture copies or contract tests that consume Pumas fixture
      JSON.
- [ ] Verify Pantograph image-family planner can reject missing/ambiguous facts
      and accept supported fixtures.
- [ ] Record deviations in this plan and the Pumas implementation plan.
- [ ] Verify Pumas remains useful to non-Pantograph consumers by keeping fixture
      field names factual and standards-oriented rather than Pantograph-specific.
- [ ] Record the Pumas release/version boundary that Pantograph should pin after
      the contract is implemented.

**Verification:**
- Pantograph package-facts fixture tests pass.
- Pumas package-facts fixture tests pass.
- No name-derived family inference is required in Pantograph.
- At least one cross-layer acceptance path proves a Pumas-produced fixture is
  consumed by Pantograph's planner without local patching or bridge conversion
  guesses.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Pumas facts become scheduler policy | High | Keep DTOs factual and host-agnostic; Pantograph derives runtime candidates. |
| Diffusion family detection guesses from names | High | Require package evidence or emit `unknown`/ambiguous diagnostics. |
| Selector snapshot regresses startup latency | High | Keep snapshots SQLite-backed and non-hydrating; hydrate selected models only. |
| Pumas parses model files on async runtime threads | Medium | Use blocking-safe file parsing patterns already used in Pumas. |
| DTO drift between Pumas and Pantograph | High | Add shared fixture round trips and update both plan references in the same slice. |
| GGUF metadata parser reads large tensors | High | Read only header/metadata; test with bounded files. |
| Selected-artifact facts drift between resolver, cache, model ref, events, and summaries | High | Introduce `PackageInspectionContext` and selected-artifact contract fixtures before family fixtures. |
| New image-generation work grows `library.rs` further | High | Complete P0 before P1-P6 and keep new extraction code in `package_facts/` modules. |
| Machine facts get encoded in human diagnostics | Medium | Add explicit DTO fields and reserve `message` for human diagnostics only. |
| SQLite contention increases during package parsing | Medium | Do not hold SQLite locks across filesystem parsing; cache only bounded projections. |
| Multi-artifact repositories collapse into one model-level facts row | High | Make resolver, cache keys, summaries, and `PumasModelRef` selected-artifact-aware. |
| Fingerprint misses nested config changes | High | Use a shared package inspection manifest for extraction and fingerprinting. |
| Import support filters hide useful package facts | Medium | Reuse path validation helpers only; emit unsupported/unknown facts rather than support verdicts. |
| Backend hints become runtime recommendations | Medium | Keep hints advisory and source-tagged; do not add host support or runtime selection semantics. |
| Model-specific lookup tables creep into family classification | High | Derive labels only from package-standard files and source-tag every label. |
| Host-language or frontend contracts drift from Rust DTOs | High | Update exported bindings, generated/manual types, and fixture consumers in the same logical slice. |
| Selector and summary snapshot projections drift | High | Make `package_facts/summary.rs` the only summary projection owner and test both public surfaces from the same fixtures. |
| Cache freshness logic drifts between API paths | High | Add one shared cache-row classifier/query wrapper and require all read/migration paths to use it. |
| New parser dependency expands Pumas core too much | Medium | Prefer bounded in-house parsing; require written dependency review for any parser crate. |
| P0 refactor changes Pumas concurrency behavior accidentally | Medium | Preserve the existing lock/update-feed model by exception; treat lifecycle changes as re-plan triggers. |
| Summary cache schema change breaks startup snapshots | High | Add persistence tests for selected-artifact, default-artifact, stale, invalid, and version-mismatch rows. |
| Existing SQLite rows serve stale package facts after contract upgrade | High | Add checkpointed package-facts cache migration/backfill and treat old-contract rows as stale, never fallback facts. |
| Migration backfill hydrates packages during selector snapshots | High | Keep migration as owner/background work; selector and summary snapshots only read cached state and explicit stale/missing statuses. |
| Package-facts cache migration is hidden inside metadata migration fields | Medium | Use distinct package-facts report/checkpoint DTOs or a typed report envelope. |
| Migration publishes update events before durable writes | Medium | Emit update-feed events only after successful cache upserts/deletes. |
| Fixture suite becomes heavyweight | Medium | Prefer synthetic/small metadata fixtures; never require large model weights for contract tests. |

## Definition Of Done

- Pumas package-facts extraction has been split out of `library.rs` into focused
  modules with a directory README and thin public facades.
- Package-facts detail and summary rows are selected-artifact-aware and do not
  collapse multi-artifact repositories into one ambiguous model-level fact set.
- Selected artifact identity is resolved once per package inspection context and
  shared by resolver locks, cache reads/writes, model refs, update events,
  manifests, and summary projection.
- Extraction and fingerprinting share one bounded package inspection manifest.
- Package manifests preserve relative paths and replace filename-only selected
  file discovery for package-facts extraction and invalidation.
- Pumas exposes richer diffusers and GGUF package facts through versioned DTOs.
- Pumas selector snapshots remain fast and do not hydrate every package.
- Package-facts summaries and update cursors reflect changed package facts.
- Package-facts cache freshness is classified by one shared helper or query
  owner used by detail resolution, summary resolution, selector snapshots,
  summary snapshots, migration dry-run, and post-migration validation.
- Existing Pumas SQLite package-facts cache rows can be dry-run inventoried,
  checkpoint-backfilled, validated, and reported without selector-path
  hydration.
- Package-facts cache migration uses distinct package-facts report/checkpoint
  contracts or a typed report envelope, not overloaded metadata migration move
  fields.
- Migration backfill writes package-facts cache changes in bounded batches where
  practical and publishes update events only after durable writes.
- Old package-facts contract rows are stale rows, not fallback facts, and public
  snapshot/detail APIs do not expose them as fresh facts.
- Fixtures cover SD/SDXL, FLUX, FLUX.2, Qwen Image, Lumina Image, GLM Image,
  Z-Image, GGUF text, GGUF embedding/reranking, and key missing/invalid cases.
- Pantograph can plan or reject image-generation package facts without using
  model names, workflow names, directory names, or implicit alternate execution.
- Pumas remains consumer-agnostic and does not own Pantograph scheduling,
  runtime/device selection, or diagnostics-ledger writes.
- Pumas emits source-tagged family evidence or explicit `unknown`/`ambiguous`
  states without model-specific lookup tables or consumer support verdicts.
- All touched source directories under Pumas `src/` have current README files
  with concrete contracts and revisit triggers.
- Large-file decomposition reviews are recorded for `library.rs`,
  `models/package_facts.rs`, and any new module that crosses the soft threshold.
- Every public or persisted serde shape changed by this plan has round-trip
  tests, fixture coverage, and updated binding/frontend consumers where
  applicable.
- Handwritten host-language mirror types, if any remain, are guarded by
  canonical fixture tests against Rust serde output.
- Dependency additions, if any, include a recorded dependency review and are
  scoped to the crate that owns the parsing behavior.
- The implementation preserves Pumas' current concurrency model unless a
  measured simplification was explicitly re-planned and accepted.

## Re-Plan Triggers

- A target image family cannot be identified from local package files or upstream
  metadata without name guessing.
- Pumas cannot parse required GGUF metadata without pulling in an unsafe or
  oversized dependency.
- Selector snapshot latency regresses because package-fact hydration moved into
  list paths.
- Pantograph needs runtime/device facts that depend on live host state rather
  than static model/package facts.
- Pumas DTO additions require Pantograph-specific labels or workflow/runtime
  identifiers.
- Existing Pumas public APIs cannot expose the facts without a broader contract
  break than this plan describes.
- The P0 module split cannot preserve public Pumas API behavior.
- New facts cannot be represented as structured DTO fields without overloading
  human-readable diagnostics.
- Selected-artifact-aware cache keys cannot be added without conflicting with
  existing selector snapshot semantics.
- Selected-artifact identity cannot be centralized without changing public
  resolver/cache/update semantics beyond this plan's contract.
- A required image-family label cannot be derived from package-standard files
  and would require a model-specific lookup table.
- Cache invalidation cannot include nested Diffusers configs or selected GGUF
  companion files without slowing selector snapshots.
- Selector snapshot and package summary snapshot APIs cannot share one
  selected-artifact-aware summary projection owner.
- Existing Pumas migration tooling cannot support checkpointed package-facts
  cache backfill without a broader migration architecture change.
- Old-contract package-facts rows cannot be identified, invalidated, or ignored
  without changing public snapshot semantics beyond this plan.
- A shared cache freshness classifier cannot express all states required by
  detail resolve, summary resolve, snapshots, and migration without becoming a
  policy catch-all.
- Package-facts migration cannot use distinct report/checkpoint payloads without
  breaking existing migration report listing or cleanup infrastructure.
- A DTO, cache schema, or public API change crosses into UniFFI, frontend, or
  Pantograph fixture consumers but cannot be updated and verified in the same
  logical slice.
- A parser dependency is required that adds unsafe code, tensor loading,
  network access, excessive transitive dependencies, or broad framework
  behavior to Pumas core.
- A planned change requires replacing the established Pumas package-facts
  concurrency model rather than preserving it.
- New package-facts modules exceed the decomposition thresholds and cannot be
  split into coherent responsibilities without changing the architecture.

## Traceability

- Active Pantograph plan:
  `docs/plans/current-image-generation-graphs/plan.md`
- Image-family planner requirements:
  `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`
- Device/runtime selection boundary:
  `docs/plans/current-image-generation-graphs/06-device-runtime-selection.md`
- Pumas repository:
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library`
- Reference repositories:
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/transformers/`
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/ai-systems/ComfyUI/`
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/ai-systems/InvokeAI/`
- Standards:
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
