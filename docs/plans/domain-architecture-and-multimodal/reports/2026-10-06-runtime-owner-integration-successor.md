# Runtime owner integration successor and draft review packet

This succeeds the [bounded integration checkpoint](2026-10-06-runtime-owner-integration-readiness.md)
on the same `integration/runtime-owner-qualified-main` branch. The owner authorized
resumption and replacement of the two recorded failures. Parent owns PR creation,
review and merge; no PR mutation is performed by this task.

## Exact tested source

- Source successor: `9a4f80267150f57a61ea7c0e460b63f899a3ac26`.
- Tree: `9a7135374b38d80e3634db70ac8cab6120d6b447`.
- Parent: `d823fa9c14d5c862b857dd163655d01ae1658fd8`.
- First repair parent: `a3a0bd3d3d81bf67bbf2f0c0ed49e3a1ad96070d`.
- Freshly fetched main: `4153772634269e342a8b0cca797f1cd6716f18a5`, already
  an ancestor. The accepted Pumas `26a84e323cae566a46a8f76bef48fa1010aed48b`
  pin and explicit dynamic/no-build-download ORT contract remain intact.

The final documentation publication is a direct successor of the tested source.
Its exact commit/tree/parent and fresh remote equality are recorded in
`/workspace/qualification-evidence/owner-successor-qualification.json`.

## Diagnoses and smallest repairs

The backend-switch failure exposed a production identity defect: registration
stored raw names while lookup matched canonical aliases by HashMap iteration.
Registering `pytorch` therefore did not replace `PyTorch`. Registration now keeps
one factory per canonical identity; the most recent registration replaces its
aliases and preserves its display name. A portable factory-identity regression
fails on the old implementation, and the existing real gateway switch test passes.

The old timeout fixture invented registry `Warming` while supplying a stopped
owner. Reconciliation correctly superseded that projection. The replacement fixture
enters an actual gateway start, waits for the backend to enter, then abandons the
caller with its unfinished owner marker intact. This exposed production defects:
inactive unfinished starts were classified `Stopped`, and legacy reclaim skipped
shutdown before readiness. Unfinished owner starts now remain `Warming` and are
eligible for owned shutdown after the last reservation ends. Acknowledged stop
clears pending warmup markers without fabricating completion or duration; failed
stop still preserves uncertainty. The host regression retains its timeout,
reservation-cleanup and stopped assertions and additionally requires exactly one
acknowledged backend stop and absent success timing. Neither production nor test
timeout is increased.

## Executed qualification

Rust 1.92, locked/offline dependencies, Python 3.12.3 and actual PyTorch 2.14.1+cpu;
ORT variables unset; mixed native builds use `--no-default-features --features
backend-llamacpp,backend-pytorch`. Independent portable tests retain their default
feature graph.

- Runtime registry: 140 pass; timing contracts: six pass.
- Inference library: all 729 pass with normal parallel execution.
- Full embedded runtime: all 502 pass, including the real-host warmup timeout,
  selected-text dispatch, full-peak custody, retained resident estimate and failed
  owned-load cleanup regressions. The cold-load shared-capacity regression still
  preserves resident 40 plus remaining peak task 50 against capacity 100 and
  rejects a further 20-byte claim.
- Public contracts: device seven pass, managed media ten pass, managed
  redistributables twelve pass, runtime load one pass.
- Model contracts: 39 pass, one existing failure. The unchanged
  `pumas_artifact_load_target_decodes_existing_pumas_wire_shape` fixture sends
  retired `model_ref_contract_version`, rejected by accepted Pumas. This also
  reproduced on the earlier CPU baseline; no unrelated wire fixture is repaired
  here. The full inference package is therefore not reported wholly green.
- Explicit native CUDA inventory test: one pass through the actual CPU-only
  embedded runtime; CUDA unavailable and uninitialized. Controlled positive
  UUID fixtures remain distinct from native GPU qualification.
- Mixed-backend warning-deny all-target Clippy passes for registry, timing
  contracts, inference and embedded runtime.
- Formatting/diff, critical/accessibility gates, 28 traceability tests, staged and
  main-to-successor traceability, and all nine ONNX no-build-download graphs pass.

## Draft integration PR packet for parent

Suggested title: **Integrate conservative runtime owner admission and timing**.

The integrated stack accounts for loaded producers across selected-text task lease
release and retain, refuses admission during unresolved allocation uncertainty,
and reaches owned shutdown after failed loads or abandoned warmups. Backend alias
registration deterministically selects one owner. CPU/RAM capacity facts,
conservative shared backing, bounded service timing correlation and explicit
CUDA owner observations are integrated with approved main through preserved
history; missing placement, capacity and exact-model facts remain unknown.

Review the main/feature merge resolutions and accepted Pumas/ORT agreement, owner
publication before custody release, full peak claim retention, source/generation
fences, known-zero semantics, bounded timing identity, canonical factory replacement,
and abandoned-start timeout/shutdown sequencing. Validation and the one unchanged
wire-fixture failure are listed above. Parent should attach the exact final commit
and its qualification manifest to a draft PR after coordinating publication.

Positive GPU UUID/execution/capacity/backing, native exact-model timing, real-model
quality, GTK/WebKit desktop and cross-platform native execution remain unqualified.
The unsupported `/..` native RAM mapping has no positive capacity qualification.
ONNX execution is not rerun; separately provisioned earlier CPU evidence and
current no-download Cargo graph checks remain distinct. No new admission ranking,
automatic GPU policy, permission change or access-denial workaround is introduced.
