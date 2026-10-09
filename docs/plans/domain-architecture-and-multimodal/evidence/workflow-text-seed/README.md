# Graph-authored text seed qualification

This candidate incorporates implementation repair `796d84cd` and documentation
successor `d0c9788c` normally. `qualification.json` records exact integration
identities, tool versions and SHA256 of each changed crate file. `SHA256SUMS`
hashes the archived bytes; logs are verbatim and gzip compressed.

The complete effective Cargo graph (`combined-features.txt.gz`) was recorded
before the first build with the exact five-package selection, all targets and
`-e features`. It contains current Pumas `26a84e32`, ORT `load-dynamic` and ort-sys
`disable-linking`, and no `download-binaries`. Manifests and Cargo.lock are
unchanged. Every build used `ORT_SKIP_DOWNLOAD=1`; offline mode supplemented it.

Reproduce with the recorded installed dependencies, an isolated test
`XDG_CONFIG_HOME`, and the installed Python library directory in `LD_LIBRARY_PATH`:

```sh
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
python -m unittest discover -s crates/inference/torch/tests -p 'test_*.py' -v
cargo test --locked --offline \
  -p inference -p pantograph-embedded-runtime \
  -p pantograph-inference-interface-contracts -p pantograph-runtime-host-contracts \
  -p pantograph-workflow-service \
  --features inference/backend-pytorch,inference/backend-candle,inference/std-process,pantograph-embedded-runtime/backend-pytorch
cargo clippy --locked --offline \
  -p inference -p pantograph-embedded-runtime \
  -p pantograph-inference-interface-contracts -p pantograph-runtime-host-contracts \
  -p pantograph-workflow-service \
  --features inference/backend-pytorch,inference/backend-candle,inference/std-process,pantograph-embedded-runtime/backend-pytorch \
  --all-targets -- -D warnings
npm run test:frontend
npm run typecheck
```

`python-tests.log.gz`: 59 methods pass. `combined-tests.log.gz`: final Rust suite
passes, with group counts in the report. `clippy.log.gz`: final warning-deny pass.
The two earlier static failures are retained separately: redundant struct updates
once all sampling fields were explicit, then Iterator::last in a new scope test.
Both were repaired without suppression; final Rust tests followed the cleanup.
Frontend and TypeScript logs record 662 passes and successful type checking.
`paused-seed-import-failures.log.gz` preserves the earlier seed worktree's two
missing-import failures at the pre-repair `1b9fd0bd` line. It is historical seed
evidence, not a failing run against this integration base; both fixtures pass now.

Tests qualify fixed CPU token selection, host/graph forwarding and controlled
worker lifecycle boundaries. Existing native Diffusers/CUDA and two optional
doctest ignores retain their status. Production loaders, GPUs, pretrained/custom
models, full worker import and full-u64 desktop entry remain unqualified.
PR58's main merge and parent publication/hosted CI remain release dependencies.
