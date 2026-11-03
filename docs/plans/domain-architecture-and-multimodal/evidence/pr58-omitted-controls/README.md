# PR58 omitted-control evidence

The source base is PR58 `1b9fd0bdaf201c92a54165ffab08cc6c6863a640`; the exact main
oracle is `763e8d4b13ba311ff245c75a370e56dafb417d8e`. Logs are verbatim and gzip
compressed. `SHA256SUMS` hashes the archived bytes. The repair report records scope
and limitations; no model or hardware qualification is inferred from these logs.

`original-failures.log.gz` is the six-method regression run against untouched
PR58 production source: 13 failed subtests, eight errors. The same exact-main
oracle script passed all six methods after the final repair
(`repaired-exact-main-tests.log.gz`). To rerun the comparison from a repository
with the recorded Git object and installed CPU dependencies:

```sh
PYTHONPATH=crates/inference/torch/tests HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
python docs/plans/domain-architecture-and-multimodal/evidence/pr58-omitted-controls/exact-main-regressions.py
```

`original-kv-failure.log.gz` replays the exact original PR58
`_generate_dllm_autoregressive_safe` function selected by AST into the controlled
worker fixture. Both explicit minimum refusal paths retained the empty KV snapshot.
It is a focused original-function replay, not a second whole-package baseline.
`internal-floor-failures.log.gz` records two errors in the intermediate repair:
strict checking mistakenly promised its internal retry floor rather than the
actual authored value. Both failure logs remain separate from final passes.

The committed regression suite uses deterministic expected main behavior and a
native `GenerationMixin.generate` oracle, without requiring Git history.
`python-tests.log.gz` records the final 40-method suite:

```sh
HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
python -m unittest discover -s crates/inference/torch/tests -p 'test_*.py' -v
```

Before any Cargo build, all-target feature trees were recorded with `--locked
--offline -e features`. `combined-features.txt.gz` covers inference, embedded
runtime, interface/host contracts and workflow service with the PyTorch features.
`inference-features.txt.gz` covers the exact inference-only test/Clippy feature
selection. The first has current Pumas, dynamic ORT, disable-linking and no
`download-binaries`; the second has no ORT or Pumas dependency.

Final Rust commands used `ORT_SKIP_DOWNLOAD=1` and the installed Python library in
`LD_LIBRARY_PATH`:

```sh
cargo test --locked --offline -p inference --features backend-pytorch --lib
cargo clippy --locked --offline -p inference --features backend-pytorch --all-targets -- -D warnings
```

`inference-tests.log.gz`: 747 passes. `clippy.log.gz`: warning-deny check passes.
These are library/embedding checks, not the broader 821-test composition command
or a fresh native Diffusers qualification. No dependency was installed and no
build-time binary download was attempted.
