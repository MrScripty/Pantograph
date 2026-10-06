# Requested embedding revision evidence

This separate successor starts from frozen embedding source
`2961b1480c103ec242c6359a45ceee6bb5d999f1`, tree
`fff2f10bb15d396c698e88ce01de283f6ad4b3b6`. It contains no PR60 stop repair
or separate evidence-publication commits. Original source and archive remain
unchanged. `qualification.json` binds the final changed source, deciding logs and
source patch. The final candidate commit/tree come from the published Git ref.

`reproduction.log.gz` records the exact four requested-identity changes against
the unchanged original projection. Structural request validation passes, but
actual Candle CPU inference incorrectly completes with width-8 vectors, token
usage and selected `untrained-seed-179` metadata despite an explicit different
requested revision. The intentionally failing regression records that successful
wrong-revision execution before the repair.

Four new regression methods cover the original case, mismatched/missing selected
revisions before resolver calls on single and two-member envelope routes, and
omitted-request refinement with actual Candle CPU golden output. Resolver
sentinels panic if invalid requests reach package/target resolution. The original
width-8/12 matching-revision, public saved/reopened graph, member identity,
cancellation and malformed-result tests remain in the full suite.

`features.txt.gz` is the complete 4,705-line five-package/all-target feature
superset inspected before the focused reproduction and final builds.
`reproduction-features.txt.gz` separately confirms the 4,603-line focused package
selection. Both have current Pumas `26a84e32`, ORT `load-dynamic`, ort-sys
`disable-linking` and no `download-binaries`. Cargo manifests/lockfile are
unchanged. All builds set `ORT_SKIP_DOWNLOAD=1`; Cargo offline alone is not a
build-script guard. Offline model flags, the installed Python library path and
an isolated writable XDG config directory are used. No weights or binaries are
acquired and privileges/authentication are unchanged.

Reproduce with existing installed dependencies and the original embedding
report's environment, package and feature scope:

```bash
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
embedding_packages=(-p inference -p pantograph-embedded-runtime
  -p pantograph-inference-interface-contracts -p pantograph-runtime-host-contracts
  -p pantograph-workflow-service)
embedding_features=inference/backend-pytorch,inference/backend-candle,inference/std-process,pantograph-embedded-runtime/backend-pytorch
cargo tree --locked --offline "${embedding_packages[@]}" --features "$embedding_features" --target all -e features
# Inspect dynamic ORT/disabled linking and absence of download-binaries first.
cargo test --locked --offline "${embedding_packages[@]}" --features "$embedding_features"
cargo clippy --locked --offline "${embedding_packages[@]}" --features "$embedding_features" --all-targets -- -D warnings
```

The correction is local to embedding host validation/projection. Generic
scheduler policy and unrelated text/image execution are unchanged. Synthetic,
untrained Candle CPU fixtures and controlled Pumas/readiness/dispatch facts do
not qualify live Pumas, production loading, pretrained quality, GPU, throughput
or desktop execution. Independent parent verification and fresh hosted checks
remain pending; no main changes, PR creation, merges or resident work occur.
