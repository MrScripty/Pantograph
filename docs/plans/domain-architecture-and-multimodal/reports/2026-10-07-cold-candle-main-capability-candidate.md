# Cold Candle CPU capability candidate on frozen main

This separate local production slice addresses the first prerequisite failure in
native run `37586061519`: the compiled Candle CPU owner was absent from the cold
runtime registry. It starts at frozen main
`a8483e511dcec4f36e269e6e4debf181a318222f`, tree
`e0d26d872866cfeec4244cdbc72bb1475efd47a3`. The inspector normalization remains
the independent three-file candidate `2a748f113cbedbeffb0c218d8b2a6936628f49ba`
and draft PR #64. No QA or inspector changes are part of this slice.

## Production provenance and scope

Four Rust paths change under `crates/pantograph-embedded-runtime/src/`:

- `runtime_registry.rs`: reuse the production enrollment function from
  `42155032ef88c009f5fcd7680a3f10ea89c0bbd1`. Require the gateway owner's exact
  `candle` / `candle.cpu` / `cpu` candidate and register a stopped owner with
  its dispatch identity. Return without modifying any existing Candle record.
- `inference_interface_facts_provider.rs`: reuse the final qualified projection
  from `2590a5337fe30688c8cd29dc60aa2cb518ad9422`. CPU-device preservation
  originated in `7fcd990bf6800defcb4f1eae5e07a2d30cadd799`; cold availability
  originated in `42155032ef88c009f5fcd7680a3f10ea89c0bbd1`. Advertise a stopped
  Candle owner only with matching family and owner CPU evidence; retain failure
  gates and project actual device IDs with deduplication.
- `workflow_service_composition.rs`: carry the two production constructor calls
  and original focused hosted/real-Pumas descriptor tests from `42155032`.
  Add a cold-owner identity/resource assertion to the existing bundle test.
- `runtime_registry_tests.rs`: carry the qualified cold-owner and existing-failed
  owner regressions. Add assertions for idempotence, absent admission/reservation
  authority, and no registration when Candle is not compiled.

This report is the fifth changed path. No manifests, lockfiles, selected model
loader, backend factory, application startup adapter, dependency producer,
scheduler, frontend, QA fixture or workflow is changed. The older configured
embedding preload workaround and later native execution repairs are excluded.

## Existing dependencies and callers

The existing Cargo `backend-candle` feature forwards to inference and provides
the Candle CPU backend factory. The gateway already exposes owner-advertised CPU
candidates; its candidate producer accepts only available CPU variants with the
owner's canonical variant ID and derives no GPU device identity.

`RuntimeDispatchCapabilityFactsSource` already joins registered backend keys to
those gateway candidates and reads actual lifecycle, model, reservation and
admission facts. `RuntimeRegistration::new` has no admission budget and creates
a stopped registry record. Enrollment sets only the existing dispatch identity.
Resource accounting, host RAM ceilings, residency and admission remain owned by
their existing registry/scheduler boundaries. Descriptor availability expresses
load capability; it does not establish runtime readiness, a dependency proof or
permission to execute a package.

The two constructor callers are `resource_backed_hosted` and
`resource_backed_hosted_bundle`. Both validate owner/authenticated-client Pumas
selector access first. The desktop adapter at `src-tauri/src/app_setup.rs` calls
`resource_backed_hosted_startup`, which reaches the bundle constructor. Existing
hosted startup adapters therefore receive the same production capability;
qualification does not inject a registration or choose a loaded backend.

The existing `InferenceGateway::new()` is compiled with `backend-llamacpp`.
Hosted local tests consequently use the existing combined
`backend-candle,backend-llamacpp` matrix. The feature-disabled regression uses
only `backend-llamacpp`. Neither scope changes the repository's feature graph.

## Acceptance and evidence

Acceptance covers both hosted constructors; actual Pumas model metadata and
graph descriptor resolution without loading; exact stopped `candle.cpu` identity;
no model, instance, reservation or admission budget; repeat-registration identity
preservation; preservation of an existing failed owner; owner-evidence and failure
availability gates; and absence of fabricated Candle capability without its feature.

Each complete effective Cargo graph is inspected before its build. ORT must use
`load-dynamic`, `ort-sys` must use `disable-linking`, and download/copy/model-fetch
features must be absent. `ORT_SKIP_DOWNLOAD=1`, `HF_HUB_OFFLINE=1` and
`TRANSFORMERS_OFFLINE=1` are set defensively. Pumas remains pinned to
`26a84e323cae566a46a8f76bef48fa1010aed48b`.

The first Candle-only test invocation failed to compile because the existing
gateway constructor requires `backend-llamacpp`; its full log is preserved. No
production code changed in response. The first corrected hosted CPU run had 505
passing tests and 32 failures caused by the read-only home-directory Pumas test
registry. Its complete log is also preserved. With a temporary writable
`XDG_CONFIG_HOME` supplied only to the test process, all 537 hosted CPU tests pass,
including real-Pumas cold descriptor resolution and both hosted constructors.
The feature-disabled regression also passes with only `backend-llamacpp`: no
Candle runtime or reservation is fabricated. Formatting, traceability and
independent review results are recorded in the separate local evidence directory
`/workspace/qualification-evidence/cold-candle-main-candidate`.

Formatting, whitespace, critical anti-pattern, scheduler-only and five-path
staged traceability checks pass. The traceability tool uses the already cached
`commonmark` package; its initial missing-package error is preserved. Warning-deny
Clippy passes for the changed embedded-runtime crate with `--no-deps`. The broader
dependency-inclusive warning-deny invocation is blocked by pre-existing unused
fields in unchanged `crates/inference/src/selected_text_execution.rs` under this
CPU feature scope; its log is preserved. No lint exemption or unrelated source
repair is introduced, and no full-workspace warning-deny pass is claimed.

## Unpublished combined qualification

After local tests and independent review, prepare an integration checkout with
the exact inspector commit as parent and this CPU commit applied as a separate
commit. Verify that both production path sets are disjoint and that the service
and regression blobs still match PR #64. Keep the CPU candidate branch based
directly on frozen main for independent review.

The separate existing-workflow QA overlay can then be applied from the preserved
`8717dc69e294ca6dbc89046b13403eba3ad926c7` and
`4f2d7090543c5ec4619b6801cca4daa09aa2cb8d` qualification commits. Enumerate every
QA-only difference, update only its branch and exact combined-source guard, and
retain all ordinary pointer interaction, cold-registration, graph save/reopen,
runtime/admission identity, scoped artifact, Read and eight-vector display checks.
Do not publish or launch the workflow as part of this local preparation.

The recipe must inspect the complete desktop/driver feature graphs before any
future build, preserve existing no-download guards and original failed artifacts,
and retain every failure from a subsequent native run. No native qualification
has been performed on this CPU candidate or combination. Later qualified-tree
dependency/execution repairs are not implicitly included; a future native attempt
may identify additional capability gaps. Controlled CPU descriptor evidence does
not qualify GPU, pretrained models, real-user discovery or the full production loader.
