# Text stop-string evidence

The frozen base is `fae339bdb1eb681a01ea983f1abe0f2f48916349`, tree
`6c8c4bb1cbed0527b58c1048ebf49fa0e243d56b`, including main `d61b86fc` through
the accepted precision successor. `qualification.json` binds the final source
files to SHA-256 hashes and records exact commands' package/feature scopes,
tool versions, routine gate results and qualification limits. `SHA256SUMS`
hashes this archive. The final candidate commit/tree are supplied by its Git ref;
the evidence does not embed a self-referential commit hash.

`python-tests.log.gz` records all 73 actual CPU methods, including the retained
59 regressions and 14 stop methods. `rust-tests.log.gz` records all five selected
packages, including public graph JSON roundtrip/host forwarding with a controlled
backend. `features.txt.gz` is the complete effective all-target graph inspected
before any build: dynamic ORT/disabled linking, current Pumas and no
`download-binaries`. The initial CPU fixture/native-config and generic oversized
host fixture failures are preserved separately; they were corrected without
weakening production validation. Routine static/frontend gates are summarized
in `qualification.json` rather than duplicated as transcripts.

Reproduce with existing installed dependencies, from this checkout, using Bash:

```bash
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
stop_packages=(-p inference -p pantograph-embedded-runtime
  -p pantograph-inference-interface-contracts -p pantograph-runtime-host-contracts
  -p pantograph-workflow-service)
stop_features=inference/backend-pytorch,inference/backend-candle,inference/std-process,pantograph-embedded-runtime/backend-pytorch
# Inspect the complete graph and confirm no download-binaries before builds.
cargo tree --locked --offline "${stop_packages[@]}" --features "$stop_features" --target all -e features
python -m unittest discover -s crates/inference/torch/tests -p 'test_*.py'
cargo test --locked --offline "${stop_packages[@]}" --features "$stop_features"
cargo clippy --locked --offline "${stop_packages[@]}" --features "$stop_features" --all-targets -- -D warnings
cargo fmt --all -- --check
npm run test:frontend
npm run typecheck
npm run build
TRACEABILITY_STAGED_ONLY=1 npm run lint:no-new
npm run lint
npm run test:scheduler-only-workflow
```

For PyO3 set `LD_LIBRARY_PATH` to the installed Python library directory and
use an isolated writable `XDG_CONFIG_HOME`. This worktree uses local workspace
package links to existing installed npm dependencies. No dependency or model
installation is part of these reproduction commands. Cargo offline alone does
not prevent build-script downloads; the verified effective graph and defensive
ORT environment variable are both required.

Independent review is source-only. Its legacy-default detection finding is
closed by matching the installed native refresh conditions and regressions for
empty-list refresh and edited-None ownership on native/manual/SDAR routes.
Seeded native/manual stopping consumes four request-owned draws in the controlled
fixture and preserves ambient RNG. A marker before the applicable authored floor
refuses; native inherited minimum/forced-EOS semantics are preserved separately.

This evidence does not qualify pretrained/custom models, production loading,
GPU, full worker import, ONNX execution or complete Tauri/desktop execution.
Six existing optional native/doctest cases remain ignored. Parent candidate
review/publication and hosted CI remain separate.
