# PR60 held-text rewrite evidence

This narrow successor starts from frozen
`1911983715155fc3927d0ebdfd0b94b851a58536`, tree
`ffc25587ce0cf84f6565e08e36f050355aaa2b9c`. The original parent archive is
unchanged. `qualification.json` binds the repaired source, source patch, complete
feature graph and executed checks; `SHA256SUMS` covers this successor packet.
The final commit/tree are identified by the published Git ref without embedding
a self-referential commit hash.

`reproduction.log.gz` records the four added tests run against the unchanged
matcher: 18 methods ran with three errors and one failed safety-stage assertion.
The safety assertion failed because the original code refused too early, before
the completed marker observation. After the repair and fifth regression,
`python-tests.log.gz` records all 78 methods passing with real installed Torch
CPU forwards and controlled fixed logits/cumulative decoding. The new methods
exercise the actual streaming route, including silence on `ab`, subsequent
`ab!` success, complete-marker refusal across the emitted boundary, suppression
of a marker and tail after the boundary, EOS/budget flush, and changed/shortened
emitted-prefix refusal. The existing ordinary emitted-text rewrite test remains.

`rust-tests.log.gz` records 2,457 non-doctest cases and one doctest passing across
the same five packages as the original packet; six optional cases remain ignored.
`clippy.log.gz` records strict all-target checks with `-D warnings`.
`features.txt.gz` is the complete 4,705-line all-target effective graph inspected
before these builds: current Pumas, ORT `load-dynamic`, ort-sys `disable-linking`,
and no `download-binaries`. Manifests/lockfile are unchanged. Builds used
`ORT_SKIP_DOWNLOAD=1`, locked offline Cargo, offline Hugging Face flags, the
existing Python library path and an isolated writable XDG config directory.
Cargo offline alone does not guard build scripts.

Reproduce with installed dependencies using the original parent README's exact
package/feature and environment commands, then:

```bash
python -m unittest discover -s crates/inference/torch/tests -p 'test_*.py'
cargo test --locked --offline "${stop_packages[@]}" --features "$stop_features"
cargo clippy --locked --offline "${stop_packages[@]}" --features "$stop_features" --all-targets -- -D warnings
```

Final-flush, marker matching, token-floor and EOS semantics are retained. A
completed marker that begins in already emitted text still refuses: append
streaming cannot retract earlier output. The initial parent's 50,000 randomized
checks used monotonic cumulative concatenation and did not cover held rewrites.
Production loading, pretrained/custom quality, GPU, full worker import, ONNX
execution and desktop qualification remain unqualified. New hosted CI and
independent parent verification remain pending; no review-thread resolution,
extra CodeRabbit request or merge is performed.
