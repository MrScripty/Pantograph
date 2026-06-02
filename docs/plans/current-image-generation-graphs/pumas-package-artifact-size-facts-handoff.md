# Pumas Package Artifact Size Facts Handoff Plan

## Objective

Add canonical logical package size facts to Pumas Library package facts so
Pantograph and other consumers can make backend-owned resource estimates without
asking Pumas to load models, choose runtimes, or decide scheduler admission.

This handoff is for:

```text
/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library
```

Pantograph should consume the resulting Pumas contract only after the Pumas
implementation is released or pinned to a specific commit.

## Source Findings

Read-only inspection of the current Pumas-Library checkout found that this
change fits Pumas' existing direction:

- `rust/crates/pumas-core/src/models/package_facts.rs` already defines
  versioned, consumer-facing package-facts DTOs. Its module comment says these
  facts are bounded serializable evidence about local model packages and do not
  select a runtime or execute Python/Transformers code.
- `rust/crates/pumas-core/src/model_library/package_facts/README.md` already
  says package facts are package-file evidence, selector and summary paths must
  stay non-hydrating, and machine-readable semantics belong in typed DTO fields.
- `rust/crates/pumas-core/src/model_library/package_facts/manifest.rs` already
  builds a bounded package inspection manifest from selected artifact files and
  standard package files. It already reads file metadata length for source
  fingerprinting, so exposing logical file sizes does not add a new kind of
  filesystem behavior.
- `ResolvedModelPackageFacts` already has `artifact`, `components`,
  `transformers`, `diffusers`, `gguf`, and optional `inspection_manifest`
  fields. The missing piece is a typed detail fact for logical artifact size.
- Compact summaries are intentionally projected by
  `rust/crates/pumas-core/src/model_library/package_facts/summary.rs` and
  should not receive heavy detail fields unless Pumas separately decides summary
  consumers need them.
- `ModelMetadata.size_bytes`, `ModelMetadata.files`, Hugging Face search
  `total_size_bytes`, and download-option sizes exist, but those are broad
  catalog/import metadata. They can be used as source material only when
  source-tagged; they should not replace selected-artifact package facts.
- Diffusers/image bundles may have `selected_files` containing only
  `model_index.json`, while the logical artifact size is the executable bundle
  components and weights. A selected-file-only implementation would be
  incomplete for Pantograph image inference.

## Ownership Boundary

Pumas owns static package evidence:

- model identity and selected artifact identity
- artifact kind, storage kind, validation state, selected files, and entry path
- package layout and bounded component/file facts
- logical file, component, and artifact sizes from filesystem metadata or
  upstream metadata
- source tagging for every size value
- package-facts freshness, cache rows, and update events

Pantograph owns runtime/resource interpretation:

- loaded-model memory estimates
- context/KV/cache/temp-buffer estimates
- runtime and device overhead
- scheduler admission, queueing, batching, and reservation policy
- same-model residency reuse
- learned refinement from observed memory ledger facts

Pumas must not expose exact loaded memory, runtime fit, device placement,
Pantograph support verdicts, scheduler rankings, or admission decisions.

## Proposed Contract

Add source-tagged logical size facts to the versioned package-facts detail
contract. The names below are planning names; Pumas may choose equivalent names
that preserve the same semantics.

```rust
pub struct ResolvedArtifactFacts {
    // existing fields
    pub logical_size: Option<PackageLogicalSizeFacts>,
}

pub struct PackageLogicalSizeFacts {
    pub total_size_bytes: Option<u64>,
    pub value_source: PackageFactValueSource,
    pub files: Vec<PackageFileSizeFact>,
    pub diagnostics: Vec<ModelPackageDiagnostic>,
}

pub struct PackageFileSizeFact {
    pub relative_path: String,
    pub size_bytes: Option<u64>,
    pub status: PackageFactStatus,
    pub value_source: PackageFactValueSource,
    pub role: Option<PackageSizeRole>,
}

pub enum PackageSizeRole {
    SelectedArtifact,
    Weight,
    Shard,
    ComponentConfig,
    Tokenizer,
    DependencyManifest,
    CompanionArtifact,
    Other,
}
```

Recommended value-source semantics:

- `filesystem_metadata`: local file metadata from the validated package
  directory.
- `upstream_metadata`: trusted upstream/catalog metadata such as Hugging Face
  file sizes when local files are not available or package is incomplete.
- `component_layout`: size derived by summing component files discovered from a
  bounded package layout.
- `unavailable`: size cannot be known without unbounded scanning or execution.

If Pumas reuses the existing `PackageFactValueSource` enum, add only the source
variants needed for size facts and keep old variants stable.

## Detail Versus Summary

Add size facts to `ResolvedModelPackageFacts` detail first.

Do not add file-level size facts to selector summaries by default. The summary
path is intentionally SQLite-backed and non-hydrating. If Pumas wants summary
size facts for list views, add a separate compact field such as
`artifact_total_size_bytes` after evaluating cache size and stale-summary
behavior. Pantograph should use detail facts for executable resource estimates.

## Implementation Slices

1. Contract DTO slice
   - Add logical size DTOs to `models/package_facts.rs`.
   - Advance `PACKAGE_FACTS_CONTRACT_VERSION`.
   - Add serde round-trip tests and update package-facts fixtures.
   - Keep `serde` behavior stable for absent optional fields.

2. Manifest/file-size producer slice
   - Extend `model_library/package_facts/manifest.rs` so manifest entries can
     carry `size_bytes`, `status`, and `value_source`.
   - Keep filesystem reads bounded to selected artifact files and standard
     package paths.
   - Preserve source fingerprint behavior.

3. Artifact logical-size projection slice
   - Populate `ResolvedArtifactFacts.logical_size` from manifest entries and
     selected artifact/package layout facts.
   - For GGUF/safetensors/ONNX selected files, include selected artifact file
     sizes directly.
   - For sharded safetensors/bin packages, include all declared shard sizes
     when available and report missing shards as diagnostics.
   - For Diffusers bundles, include bounded component config files plus known
     weight files from selected/expected/package layout evidence. Do not stop at
     `model_index.json`.

4. Metadata/upstream source-tag slice
   - When local filesystem facts are unavailable or incomplete, source-tag
     upstream/catalog sizes from `ModelMetadata.files`, `ModelMetadata.size_bytes`,
     Hugging Face download details, or equivalent Pumas-owned metadata.
   - Do not present upstream metadata as filesystem proof.
   - Emit diagnostics when local and upstream sizes disagree.

5. Cache and migration slice
   - Ensure detail cache rows are invalidated by the new package-facts contract
     version.
   - Ensure summary rows remain compact and correctly report stale contract
     status.
   - Add or update migration/backfill tests for old rows.

6. API/RPC fixture slice
   - Verify `resolve_model_package_facts` returns logical size facts through
     owner API/RPC.
   - Update package-facts fixture JSON for GGUF, sharded safetensors, and
     Diffusers image bundles.
   - Keep local-client/read-only behavior fail-closed if full detail facts are
     not available there.

## Standards Guardrails

- Keep package-size facts in Pumas package-facts modules, not in selector UI,
  runtime profile code, Pantograph-specific adapters, or model-specific lookup
  tables.
- Do not infer family, backend support, or runtime fit from display names,
  repository names, workflow names, or directory names.
- Do not load model weights, import Python, initialize Diffusers, run llama.cpp,
  or simulate runtime memory.
- Use checked arithmetic for byte sums and return typed diagnostics on overflow
  or contradictory facts.
- Keep blocking filesystem scans bounded and outside SQLite locks.
- Preserve one public contract owner for package facts and update fixtures in
  the same slice as DTO changes.
- Keep Pumas summaries non-hydrating unless a later Pumas plan explicitly
  justifies a compact summary size field.

## Acceptance Criteria

The Pumas implementation is ready for Pantograph when:

- `ResolvedModelPackageFacts` detail facts expose logical artifact size with
  source-tagged total and file/component facts.
- The package-facts contract version has advanced and stale rows fail or refresh
  through existing cache semantics.
- Tests prove GGUF, sharded safetensors/bin, ONNX, and Diffusers bundle sizes
  are produced without loading models.
- Diffusers image fixtures include enough logical size evidence for the
  executable bundle, not only `model_index.json`.
- Owner API/RPC returns the new detail facts.
- Local-client/read-only modes either return current detail facts or fail
  closed with typed stale/missing diagnostics.
- Pumas documentation states that these are logical package sizes, not loaded
  memory requirements or runtime fit decisions.

## Pantograph Handoff

After Pumas publishes or pins this contract, Pantograph should:

1. Update its Pumas dependency and inference package-facts DTOs to the new
   contract version.
2. Extend the embedded-runtime Pumas package-facts bridge to project logical
   size facts and reject stale or missing size contracts with typed diagnostics.
3. Implement the production `InferenceInterfaceFactsProvider` through backend
   composition.
4. Add Pantograph-owned conservative model/load and execution/context estimates
   from Pumas logical sizes plus runtime/device/task-shape/residency facts.
5. Keep missing, stale, ambiguous, unsupported, or insufficient size facts as
   blocking diagnostics. Do not synthesize zero-resource or graph-path-derived
   estimates.
