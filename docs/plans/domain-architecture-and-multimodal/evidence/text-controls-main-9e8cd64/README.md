# Frozen composition qualification evidence

This archive preserves the exact qualification JSON and every file it references
for source `9e8cd64cc64fa7ccb8dfd6cd85d37bff2282dfb6`, tree
`10b3b11a064823055a3a8da9987a67ec3aabb88f`. Original absolute workspace paths and
hashes inside the JSON and original checksum file remain unchanged. Every referenced
file is packaged verbatim in `qualification-logs.tar.gz`, with the same basenames
and its own portable checksum file. The JSON and original failure log are also
readable directly. This preserves raw trailing blank lines without changing
source/style gates. Verify and unpack with:

```sh
sha256sum --check SHA256SUMS
mkdir extracted
tar -xzf qualification-logs.tar.gz -C extracted
(cd extracted && sha256sum --check SHA256SUMS)
```

No production, test or dependency source changes accompany
this evidence-only successor.

The original parallel command was:

```sh
cargo test --locked --offline -p inference --no-default-features --features backend-llamacpp,backend-pytorch --test diffusers_guidance_native -- --ignored
```

Its unedited [failure log](text-main-composition-native-diffusers.log) records two
passes and the scheduler oracle's `AssertionError` at Rust source line 214. That
original PyO3 rendering includes a traceback object, not the Python frame contents.
The original `text-main-composition-native-diffusers-serial.log` in the archive
records the same three tests with `--test-threads=1`. The serial pass is isolated
CPU evidence, not closure of the parallel failure. The JSON preserves this limit.

Dependency versions are Python 3.12.3, Torch 2.14.1+cpu, Transformers 4.53.3 and
Diffusers 0.39.0, as captured by both native logs and the Python environment log.
No dependency installation was performed for this qualification. The scheduler
test in the frozen source replaces `DDIMScheduler.step` globally while the other
native tests call it from different threads. Its exact-two-calls observation can
therefore include unrelated calls after Torch releases the GIL. Parent requested
a separate deterministic reproduction and isolation repair under the unchanged
parallel command; those remain required and are not claimed by this archive.

Full worker import lacks soundfile; actual worker-function tests use AST-selected
source. GPU, pretrained/custom models, native desktop and ONNX execution remain
unqualified. Parent owns PR/review/merge. Frozen source and original evidence are
unchanged.
