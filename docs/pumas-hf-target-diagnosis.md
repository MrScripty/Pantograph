# Pinned Pumas HF load-target diagnosis

Original diagnosis on 2026-10-11 at Pantograph commit `5cd83456`, with Pumas pin
`26a84e323cae566a46a8f76bef48fa1010aed48b` and the committed untrained BERT-8
fixture. The three selected-artifact cases return a ready directory target whose
path is an existing weight **file**. This contradiction is present in direct
Pumas API responses, before the Pantograph adapter or inference loader. The
separate public-importer case stops at a missing artifact identity and must not
be counted as a fourth reproduction of the directory mismatch.

## Reproduce without an existing catalog

From Pantograph commit `5cd834560333991827dd6406cf61ebf559bfc3eb`, with ordinary
Rust build dependencies available (the current branch now selects the public fix):

```sh
probe_parent="$(mktemp -d)"
probe_root="$probe_parent/run"
PUMAS_REGISTRY_DB_PATH="$probe_root/registry.db" \
HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
cargo run --locked -p pantograph-embedded-runtime \
  --features host-dependency-inventory \
  --example cpu_embedding_workflow_probe -- "$probe_root" --inspect-targets
```

The root must be a new absolute directory and the registry must be inside it.
Pumas is built with its HF client and process manager disabled. The inspection
mode copies only the repository fixture, creates the launcher directories and a
new catalog, and invokes published owner APIs. It neither opens the user's
catalog nor changes Pumas code. It downloads zero model bytes.

The output `pumas-target-observations.json` records the compile-time lockfile
source, exact serialized requests and responses, indexed metadata, and filesystem
observations. Absolute paths under the new root are replaced recursively with
`<probe_root>`; field names, values outside those paths, and absent fields are
preserved. The [recorded output](evidence/pumas-hf-targets-26a84e32.json) is from
this procedure. This command succeeds when observation completes; it does not
assert that inference or the editor passed.

## Fixture and selection matrix

Every package contains the same real 6,464-byte `model.safetensors`, `config.json`,
`tokenizer.json`, `modules.json`, `sentence_bert_config.json`, and
`1_Pooling/config.json`. The package root exists as an absolute directory and all
six components exist as files. The fixture hashes and numerical-forward evidence
are described in the [workflow guide](real-inference-workflow-probe.md).

The first three cases seed isolated metadata with `storage_kind: library_owned`,
`import_state: ready`, `validation_state: valid`, directory `entry_path`,
`selected_artifact_id: main`, embedding/feature-extraction task hints and Candle
backend hints, then call `rebuild_model_index`. Only the file selection differs:

| Case | Input selection | Owner result |
| --- | --- | --- |
| `weights-only` | `model.safetensors` | Ready directory target pointing to the weight file |
| `complete-files` | All six component paths | Same directory/file contradiction |
| `owner-inferred` | No selected file list | Same directory/file contradiction |
| `owner-imported` | Public `import_model_in_place`, no seeded metadata, all six expected files, hashing enabled | Import succeeds; target remains ambiguous because owner metadata has no selected artifact ID |

The three selected-artifact cases have identical target responses across
`owner_fresh`, `read_only_indexed`, and owner-fresh with explicit expected kind
`hf_compatible_directory`. Changing the selected file list or specifying the
expected kind therefore does not correct the path on this pin. The importer
result is retained without inventing an artifact ID or a fallback target.

## Exact API response and directory contract

For `weights-only`, the owner-fresh request contains model ID
`embedding/qualification/weights-only`, selected artifact ID `main`, and consumer
`pantograph-embedded-runtime` / task `embedding` / runtime family `candle.cpu`.
Expected kind, caller-observed path and caller-observed package-facts version are
unset. The serialized response is:

```json
{
  "artifact_state": "ready",
  "entry_path_state": "ready",
  "target": {
    "artifact_kind": "hf_compatible_directory",
    "load_path_kind": "directory",
    "local_load_path": "<probe_root>/shared-resources/models/embedding/qualification/weights-only/model.safetensors",
    "model_ref": {
      "model_id": "embedding/qualification/weights-only",
      "model_ref_contract_version": 1,
      "selected_artifact_id": "main",
      "selected_artifact_path": "<probe_root>/shared-resources/models/embedding/qualification/weights-only/model.safetensors"
    },
    "package_facts_contract_version": 3,
    "storage_kind": "library_owned",
    "validation_state": "valid"
  }
}
```

Empty diagnostics and unset optional fields are absent in this serialization.
The returned path is absolute, exists, is a file, is not a directory, and has
length 6,464 bytes. Both `execution_descriptor_response.entry_path` and
`package_facts_response.artifact.entry_path` name this same file. The expected
executable directory is its parent package root, containing the weights and
sidecars. No consumer-side parent-path substitution is permitted: Pumas owns the
resolved identity, kind and load path.

For `owner-imported`, the exact distinct response is:

```json
{
  "artifact_state": "ambiguous",
  "diagnostics": [{
    "code": "missing_selected_artifact",
    "field_path": "model_ref.selected_artifact_id",
    "message": "model_ref must identify one selected artifact before load-target resolution"
  }],
  "entry_path_state": "ambiguous"
}
```

## Attribution and public correction

The pinned Pumas source explains the observation:

1. [`resolve_model_execution_descriptor_read_only`](https://github.com/MrScripty/Pumas-Library/blob/26a84e323cae566a46a8f76bef48fa1010aed48b/rust/crates/pumas-core/src/model_library/library.rs#L3074)
   uses a directory root for Diffusers, but otherwise selects the primary model
   file. It does not preserve a directory target for this managed HF package.
2. [Artifact classification](https://github.com/MrScripty/Pumas-Library/blob/26a84e323cae566a46a8f76bef48fa1010aed48b/rust/crates/pumas-core/src/model_library/package_facts/artifact.rs)
   recognizes the safetensors-plus-config package as `hf_compatible_directory`.
   Package facts retain the descriptor's file entry path.
3. [Load-target production](https://github.com/MrScripty/Pumas-Library/blob/26a84e323cae566a46a8f76bef48fa1010aed48b/rust/crates/pumas-core/src/model_library/artifact_load_target.rs#L342)
   copies that entry path while deriving `load_path_kind: directory` from the
   artifact kind.
4. Pantograph's [projection](../crates/pantograph-embedded-runtime/src/runtime_host_image_execution.rs)
   copies `local_load_path` and the path-kind enum unchanged. Its
   [selected-embedding guard](../crates/inference/src/selected_embedding_execution.rs)
   correctly requires an existing absolute directory and rejects this file.

This identifies a provider inconsistency for the selected managed fixture,
rather than a Pantograph path projection error or an incomplete selected-file
list. It does not qualify every import/acquisition workflow.

Public Pumas commit
[`aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`, “Produce coherent managed HF package load targets”](https://github.com/MrScripty/Pumas-Library/commit/aa1d79ae2d53f0acb088bc85223ecc2ff46af12b)
already changes managed HF packages to use the package directory and adds owner
coherence checks and tests. The public GitHub comparison confirms this commit is
an ancestor of observed public main
[`ad31e391dcd94c204b3fcd748cc98d27b3281e65`](https://github.com/MrScripty/Pumas-Library/compare/aa1d79ae2d53f0acb088bc85223ecc2ff46af12b...ad31e391dcd94c204b3fcd748cc98d27b3281e65)
(ahead 134, behind 0). At the diagnosis checkpoint its source and tests were
inspected read-only; neither public revision was built at that point. The fix changes
observation/coherence contracts, so changing the dependency requires coordinated
consumer integration rather than assuming a one-line pin bump is sufficient.

The only setup correction made during diagnosis was creating the isolated
launcher metadata/cache/log directories before calling the public importer.
After that correction import succeeds and the separate identity rejection above
is observable. No provider, adapter or runtime validation was patched.

## Original diagnosis validation

The probe's redaction and filesystem distinction tests pass. The inspection run
captures three directory/file contradictions and the separate import ambiguity.
A fresh native workflow run still reaches the real scheduler and Candle CPU
dispatch, then fails the unchanged existing-directory guard; this repeat attempt
took 22 ms and is **failure latency**, not model inference latency. Session close
and runtime shutdown succeeded.

## Public dependency integration

The branch now pins the exact public correction
`aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`, the earliest revision containing that
commit, rather than current Pumas main. It is 70 commits ahead of the previous
pin. The enabled dependency graph requires no additional packages, consumer API
adaptation, or guard changes. The target now supplies the existing package
directory and a `pumas-package-observation-v1:sha256:` observation token through
the already supported `content_fingerprint` field. This token observes package
metadata and file size/mtime; it is not a complete package byte digest.

The [direct API rerun](evidence/pumas-hf-targets-aa1d79ae.json) confirms all three
selected-artifact variants return coherent directory targets in both resolution
modes and with the explicit expected kind. The importer case retains the distinct
missing-artifact-identity rejection. The [native scheduler rerun](evidence/cpu-embedding-public-aa1d79ae.json)
also passes: actual Candle CPU inference matches the independent reference, the
inference task completes in 28 ms, and the session and runtime close cleanly.
No Pumas source or unpublished override is used.

The editor-to-model demo remains pending browser qualification using the
separately owned PR71 Rust browser host. One successful editor run must connect
Submit, scheduler run identity, actual model output and visible I/O Inspector
content, with screenshot evidence. Separate numerical forwards and this API
diagnosis do not satisfy that gate.
