# Pumas wire and consumer projection qualification

The remaining model-contract failure is repaired on the existing integration
branch, preserving reviewed checkpoint `75bbd61f08fcf49057f22a772883fd08b024fb11`
as a parent. This is a fixture-boundary repair with executable producer/adapter
coverage; no production protocol, required field, dependency pin or runtime
behavior changes.

## Exact failure and corrected diagnosis

At checkpoint 75bbd61f, `pumas_artifact_load_target_decodes_existing_pumas_wire_shape`
fails with `unknown field model_ref_contract_version`. It sends Pumas producer
JSON directly to `inference::PumasArtifactLoadTarget`, re-exported from the strict
Pantograph dependency-planning consumer contract.

The earlier [integration report](2026-10-06-runtime-owner-integration-successor.md)
incorrectly describes that field as retired. It is **current producer metadata**:
accepted Pumas `26a84e323cae566a46a8f76bef48fa1010aed48b` declares
`PUMAS_MODEL_REF_CONTRACT_VERSION = 1` and serializes
`PumasModelRef.model_ref_contract_version` in
`rust/crates/pumas-core/src/models/package_facts.rs`. Its artifact-load-target DTO
uses that producer model ref. The fixture was introduced in
`ee7dc18412a36d16ea34b396e530383f7dbe7d50` when the consumer mirror ignored unknown
fields. `743f745c96ee54caff3b0137b892312f35b79888` made the consumer contract strict
without updating this old test's layer assumption.

Production selected-runtime execution already receives the typed pinned Pumas
DTO through `RuntimeHostPumasLoadTargetResolver`, then uses
`runtime_host_image_execution::project_pumas_artifact_load_target` for the
inference handoff, including text and embedding routes. That adapter deliberately
constructs the consumer identity/facts instead of decoding raw producer JSON into
the consumer mirror. Direct raw decoding is the wrong boundary. The supported
producer-to-host-to-consumer route has no demonstrated wire incompatibility and
requires no Pumas author/protocol change.

## Repair and regression

Two shared fixtures represent the declared versioned producer wire and its strict
consumer projection. The inference positive test decodes and validates the
projected fixture, retaining its original model, artifact, path, storage and
validation assertions and adding a complete roundtrip comparison. Negative tests
require raw producer metadata to be rejected and require model identity, model
ref, artifact kind, local load path, load path kind, storage and validation fields.

The new embedded regression decodes the wire with the actual pinned Pumas DTO,
asserts its version against the producer constant and compares producer
serialization with the complete wire fixture. It calls the existing production
adapter and compares every resulting consumer fact with the projected fixture:
revision, selected artifact id/path, migration diagnostics, fingerprint, package
version and all target fields. The result validates and roundtrips through the
strict consumer DTO. No `deny_unknown_fields`, defaults or supported wire version
semantics are weakened.

## Exact source and executed checks

- Tested source: `15aed04650955f7f271b97c145f6a26c0cd17f5a`.
- Tree: `bec0d4cbdea04b27bcaf7c4e13515a41f430d16d`.
- Parent: `75bbd61f08fcf49057f22a772883fd08b024fb11`.
- Current fetched main: `4153772634269e342a8b0cca797f1cd6716f18a5`, already an ancestor.

Locked/offline mixed-backend tests use `--no-default-features --features
backend-llamacpp,backend-pytorch`, the actual Python 3.12.3 library and unset ORT
variables. Executed qualification:

- Original exact failing test: one failure reproduced before repair.
- Full inference package: 729 library, 42 model-contract, seven device-contract,
  ten managed-media, twelve managed-redistributable, one runtime-load and one
  doc test pass; no failures. One explicit-native CUDA test and two doc examples
  remain ignored in this ordinary package run. The earlier separately invoked
  CPU-only native CUDA evidence remains distinct; no new GPU proof is claimed.
- Full embedded runtime: all 503 pass, including the new versioned-wire adapter
  regression and the reviewed warmup/resident custody regressions.
- Warning-deny all-target Clippy for inference/embedded, formatting, diff,
  critical/accessibility and staged/final-range traceability pass.
- Cargo manifest/lockfile, CI and the ONNX no-build-download guard are unchanged
  against 75bbd61f. Its checked nine dependency graphs and conservative owner,
  generation, full-peak and known-zero behavior remain intact.

The publication adds only this report and its plan link after the tested source.
Exact final commit/tree/parent, logs, checksums and fresh remote equality are in
`/workspace/qualification-evidence/owner-wire-qualification.json`.
Parent retains PR/review/merge coordination; this task performs no PR mutation.
Positive GPU execution/UUID/capacity/backing, exact-model timing, real model quality,
GTK/WebKit desktop, unsupported native `/..` RAM mapping and cross-platform native
execution remain unqualified. ONNX execution is not rerun. No further feature is
started.
