# PR49 summary-cache fixture settlement

Branch: `fix/pr49-summary-cache-fixture`.
Exact base: `9074cdc9d334934dd9e30bb80137ddf13ea38acd`.
Base tree: `75ce55555e91782457071f00d5e62ea030d33e1e`.
Accepted consumer and embedding chains, including full PR44 ancestry, remain
intact. Pooling repair `495efea606620153fe2f0363b104e56518436a06` is excluded.

## Observed failure and established semantics

Exact-base hosted Quality run `37202921241`, job `111438109920`, executed the
model-library suite: **208 passed, 1 failed, 0 ignored**. The retrieved job log
confirms public authentication refusal and real full-facts/guarded-target IPC
passed. Only the final warm-client/owner equality in
`local_client_selected_model_detail_uses_batch_detail_methods` failed:
client status `Regenerated`, owner status `Fresh`, with identical complete
summary payloads and identity. The preceding cold `Regenerated`, warm owner
`Fresh`, descriptor, selector-row and full-payload comparisons all passed.
Embedded-host tests were not reached. Other Quality jobs, including strict
Clippy, passed according to the parent.

At producer pin `5be6d967dbd5c0ff7449f342e77f05f9cd645a8e`,
`api/models.rs::resolve_model_package_facts_summaries` and the authenticated
plural dispatch in `api/state.rs` call the same
`PrimaryState.model_library` resolver. IPC validates the token and serializes
the result; it does not intentionally replace freshness. The resolver returns
`Fresh` only for matching contract-version/source-fingerprint cache rows,
`DetailDerived` for a usable detail cache, otherwise `Regenerated`.

The fingerprint in `model_library/package_facts/manifest.rs` includes serialized
metadata, descriptor, dependency bindings, selected file lengths and mtimes;
these exceed the returned summary's fields. Equal payloads alone cannot establish
unchanged source state. The original log has no cache/fingerprint snapshots,
so it cannot distinguish source mutation from a producer cache defect.

The builder marks the catalog dirty and starts retained-intent inspection and
the real filesystem watcher even with HF/process-manager initialization disabled.
Retained-intent startup reconciliation is conditional on declarations; it must
not be described as an unconditional completed full-catalog pass. The watcher
debounces events for 100 ms and can schedule model reconciliation. That path can
reindex/reclassify an unsettled imported model and update metadata/updated_date,
which affects freshness without necessarily changing the compact summary.
Import returns after its own publication, not a global reconciliation fence.
**Background invalidation is a source-backed hypothesis, not an observed
before/after mutation proven by the original CI log.**

## Bounded fixture repair and stronger evidence

Only the existing selected-detail test changes. Before its first descriptor,
selector or summary lookup, it awaits public `rebuild_model_index()` completion.
The source-backed forced full-scope API refuses active full/model passes with
`ModelIndexRefreshInProgress`; the fixture retries only that typed conflict
with a 10 ms backoff and a 10-second timeout. Other errors and timeout fail.
The completed pass must retain the single imported model. This is an awaited
operation barrier, not a fixed sleep or a retry-until-status-matches loop.

All existing assertions remain, including cold `Regenerated`, owner `Fresh`,
exact model identities, complete selector/descriptor/payload comparisons and
the entire final warm-client/owner result equality. Freshness is never relabelled,
ignored or accepted as an arbitrary status.

A public `ModelIndex::open_read_only` captures the durable summary cache row,
including fingerprint and selected-artifact identity, around the final client
request. Canonical metadata is captured alongside it. Failure output includes
both cache rows and both metadata snapshots. Complete cache-row and metadata
equality must also pass, so a cache regeneration or hidden source change cannot
silently satisfy the fixture. The watcher remains enabled. Production code,
Pumas pins/default features, fixture model bytes, all 14 setup test names and
attributes, and audit/traceability/workflow gates are unchanged.

## Actual verification and limits

The changed model-library test targets compile with locked offline dependencies:

```sh
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo check --locked --offline -p workflow-nodes --features model-library --tests
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo clippy --locked --offline -p workflow-nodes --all-targets --all-features
```

Both pass with the inherited inference unused-field warning. Formatting,
whitespace and the full no-new-debt/traceability gate also pass.
Compiler/Clippy results do not execute the fixture or prove the hypothesis.

A normal targeted native test attempt on the unchanged base failed downloading
the pinned ONNX Runtime 1.24.2 from the approved CDN: proxy CONNECT 403, exit 101.
No alternate download route, older installed runtime, feature reduction or
network/credential change was used. The changed-fixture targeted attempt was:

```sh
ORT_SKIP_DOWNLOAD=1 CARGO_BUILD_JOBS=1 cargo test --locked --offline -p workflow-nodes --features model-library --lib setup::tests::local_client_selected_model_detail_uses_batch_detail_methods -- --exact --nocapture
```

It exits **101 at linking**, including missing `OrtGetApiBase`. **Zero revised
IPC tests executed locally.** Missing native linking is not a test pass.
The revised barrier and before/after assertions require actual hosted execution
after independent review. This report does not claim a runtime fix has passed,
or that the original source mutation has been measured. No model inference,
pretrained model, GUI or GPU acceptance is established by these checks.

No PR49 head advancement, main merge or external review request is part of this
handoff. The separate descendant is for independent source review before the
parent coordinates hosted qualification.
