# Inference dependency sidecar rendering candidate

An authored inference snapshot replaces the frontend node's task ports. The
snapshot does not contain the schema-owned dependency association input, so the
frontend previously dropped `dependency_environment_sidecar`. Saved graphs kept
the `deps-to-infer` edge, but SvelteFlow had no matching target handle and omitted
its path. This is a production frontend projection defect. Backend effective
contracts already merge static and authored ports; changing that merge or waiting
longer in QA does not repair the frontend handle.

## Source and scope

The isolated `fix/inference-sidecar-render-main-2026-10-07` candidate starts at
freshly fetched main `a8483e511dcec4f36e269e6e4debf181a318222f`, tree
`e0d26d872866cfeec4244cdbc72bb1475efd47a3`. The exact candidate commit/tree and
independent review are recorded in the accompanying qualification evidence.

The production change is confined to
`packages/svelte-graph/src/stores/definitionOverlay.ts`: append the registered
dependency sidecar input when it is absent from the authored snapshot. Retain
its existing label, type, optionality and metadata; do not duplicate an authored
port or append unrelated bootstrap inputs. Authored task inputs and outputs stay
in their authored order. The source blob matches the narrow frontend change in
`77b9704920697ff01c9fa5b0bb49dd5b585c885f` and qualified source
`2590a5337fe30688c8cd29dc60aa2cb518ad9422`. Broader dependency/bootstrap/runtime
changes from those histories are excluded.

The other two candidate paths are the existing overlay test file and this report.
No backend, manifests, lockfiles, native QA assertions or fixtures change. This
slice remains separate from inspector PR64 (`2a748f11`) and CPU owner PR65
(`6338b71a`).

## Diagnosis and acceptance evidence

The preserved native failure
[37593069408](https://github.com/MrScripty/Pantograph/actions/runs/37593069408)
ran exact QA source `1f679b5a2888a07188378fde788f6c2f35af2847`, tree
`758e2aa1e2f4a1a86f8361dd8401c0eec92155bf`. Actual save/reopen preserved four
nodes and three edges, but line90 found only two rendered paths. The complete
failure DOM includes the dependency source handle and lacks the inference target
handle. All18 original artifact members and the full masked log are preserved.

Two added regressions fail on main before the production change: registered
sidecar preservation and saved-edge target preservation after definitions arrive.
A third checks authored-port deduplication. Tests also verify selective control
retention, unchanged registered metadata, no input mutation, definition refresh
and runtime overlays. All21 focused store/materialization tests and all673
aggregate frontend tests pass with the change.

A controlled Chromium fixture uses the actual production store materializer,
`BaseNode` handles, SvelteFlow and production `ReconnectableEdge`, with the saved
four-node/three-edge graph from the failed run. The host definitions and graph
shell are controlled. Main reproduces two paths and zero inference sidecar
handles across initial load, definition/runtime refresh and save/reopen replay,
even after settling. The candidate produces three paths and one matching handle
in all three stages, without changing stored edge IDs. Baseline and repaired
JSON, DOM and screenshots are preserved; styled captures use actual production
CSS. The existing local Chromium launch recipe disables GPU and uses
`--no-sandbox`; this is component rendering evidence, not native WebKit or browser
sandbox qualification.

Typechecking, production frontend build, full ESLint, critical anti-pattern and
accessibility gates pass. The complete effective all-target normal/build/dev
desktop Cargo feature graph was inspected before builds: dynamic ORT,
`ort-sys` disable-linking, pinned Pumas
`26a84e323cae566a46a8f76bef48fa1010aed48b`, and no download-binaries/fetch-models/
copy-dylibs/tls-native features. `ORT_SKIP_DOWNLOAD=1` is set defensively. No Rust
build, model download or new native attempt is needed for this frontend slice.

## Remaining qualification boundary

This fixes the missing sidecar handle/path. It does not establish current native
submission, dependency resolution, CPU execution, retained artifact Read or
inspector vector display. The existing production edge component still records
URL glow filters; the native geometry/glow loop after line90 expects drop-shadow
and remains a separate unresolved gate. Later validation/bootstrap/loader gates
are also untested on this candidate. No full production loader, real-user model
discovery, pretrained model, GPU or end-to-end native acceptance is implied.

Any later native qualification must use a separate composition of these three
independent production slices plus the unchanged QA recipe, inspect its complete
effective feature graph before building, preserve all previous failures, and
record exact source identities. No native retry is performed by this candidate.
