# Runtime owner integration checkpoint

The execution shell remained usable after the disconnect notification. This bounded
checkpoint combines the published owner features with approved main using ordinary
merges; no feature commits are rewritten, no further feature is started and no PR
or hosted merge action is performed. Parent retains review/publication decisions.

## Exact history and conflict resolution

- Timing correlation repair: `6bb96e26f9549323d12a54127baf0d29dc57dd25`, tree
  `c4272f648ed77e0504fb8963fe9b95dac6d8d178`.
- CUDA owner observations: `4f189dd9e506205c0e5d8dca5fd403f57f4dfa32`, tree
  `6a41be4ae98502f7d396fe6dc1266cf7a71d0220`.
- Approved main: `4153772634269e342a8b0cca797f1cd6716f18a5`, tree
  `8c63e110f650e83a62ece7cc302671c5ab2b6374`.
- Feature merge: `38818b48b93ff78cdbdbf08651b7c16c9b37ee1f`, ordered parents
  `6bb96e26f9549323d12a54127baf0d29dc57dd25` then
  `4f189dd9e506205c0e5d8dca5fd403f57f4dfa32`.
- Qualified source merge: `23b3428eaf6952e89b5c0b07be736cb882c95c6d`, tree
  `df9564c47b34b58192ada5c5a3016aa0675f1df5`, ordered parents
  `38818b48b93ff78cdbdbf08651b7c16c9b37ee1f` then approved main.

The feature merge conflicts only in the plan's adjacent progress paragraphs, which
are retained together. The main merge conflicts in the plan and three Pumas pin
references. Manifest, lockfile and CI retain main's accepted
`26a84e323cae566a46a8f76bef48fa1010aed48b` pin. The BLAKE3 inference dependency
edge remains. Approved main's explicit dynamic/no-download ORT contract is
preserved. Existing descriptor/sampler changes arrive through main ancestry;
none are cherry-picked or patched a second time. The plan records the combined
checkpoint and preserves historical qualification evidence.

The final publication adds only this report and its plan link after the qualified
source merge. Its exact tip/tree and remote verification are recorded in the local
qualification manifest.

## Executed affected qualification

Builds use Rust 1.92, locked/offline dependencies, the actual Python 3.12 library,
unset ORT variables and `backend-llamacpp,backend-pytorch` without default ONNX
features. Local registry/inference/timing core artifacts are cleaned first.

- All 139 runtime-registry tests and six timing-contract tests pass.
- Inference library: 727 pass, one fails. All ten timing and three controlled CUDA
  inventory tests pass. The failure is the previously documented fresh-backend
  lifecycle fixture: registration retains both production `PyTorch` and fixture
  `pytorch`; canonical HashMap lookup can select the production factory and attempt
  a load against a Transformers stub lacking AutoModelForCausalLM. That registry
  and fixture issue remains outside this integration; it is not reported green.
- An explicitly invoked separate native test passes through the actual gateway
  with installed `2.14.1+cpu`, yielding `unavailable/cuda_unavailable` and preserving
  CPU candidates/current backend. Positive CUDA UUID cases are controlled only.
- Full embedded runtime: 501 pass, one fails. All selected-text host, resident,
  dispatch, padded-ID timing and full-peak custody regressions pass. The descriptor
  assertion passes after integrating main's repair. The remaining baseline test
  `test_session_runtime_load_releases_reservation_after_warmup_timeout` still
  receives success; no assertion is weakened or timeout increased.
- Mixed-backend warning-deny all-target Clippy covers registry, timing contracts,
  inference and embedded runtime. Formatting, critical/accessibility/traceability
  gates, 28 traceability tests and nine ORT no-build-download graphs are checked.

## Readiness and remaining limits

This is ready for bounded integration review, not a claim of a green full suite
or permission to merge a PR. Review scope is the two history-preserving merges,
Pumas pin agreement/ORT feature union, owner capability and admission composition,
bounded timing correlation schema, and the preserved qualification limits. The
standalone source reviews and native evidence remain separately identifiable.
The two recorded suite failures require an explicit parent disposition.

Full peak task claims, resident uncertainty, source/generation fences and known-zero
semantics remain conservative. Missing timing/capacity facts stay unknown; UUIDs
are not inferred from configured labels, class counts or monitor aggregates. No
automatic GPU admission or new ranking policy is introduced. Positive GPU UUIDs,
GPU execution/capacity/backing, native exact-model timings, GTK/WebKit desktop,
real model quality and cross-platform native builds remain unqualified. Native RAM
still has no positive qualification for the unsupported `/..` cgroup mapping.
Actual ONNX execution is not rerun here; prior separately provisioned CPU evidence
and the checked no-download contract remain distinct.

Work stops at this checkpoint as requested under the weekly usage ceiling; no
subsequent feature, unrelated failure repair or repeated broad qualification is
started.
