# CPU embedding editor host

This feature integrates PR71's verified browser transport and workbench at
`7fc3584cca8bef647b60ebf303c4b5d6b37d98bd` with Pantograph's real embedded runtime.
The ordinary workspace dependency is pinned to public Pumas correction
`aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`. No Pumas source patch, unpublished
override, private catalog, model download, or paid inference is used.

**Qualified:** the native workflow and the live HTTP editor-command lifecycle,
including graph edit/save, backend validation, dependency Resolve, publication,
scheduler dispatch, Candle CPU inference, retained vector readback and cleanup.
**Pending:** actual browser interaction and visible-output screenshots. The local
Chromium sandbox cannot start; this is not a completed editor-visible demo.

## Run

Use the repository's Rust/Node toolchains and installed dependencies:

```sh
evidence_parent="$(mktemp -d)"
python3 examples/cpu-embedding-demo/serve.py --output-dir "$evidence_parent/run"
# In another terminal:
npm run dev -- --host 127.0.0.1
```

Open `http://127.0.0.1:8766`, then follow its workbench link. The new output
directory retains the isolated fixture catalog, workflow, diagnostic ledger and
artifact bodies. An existing directory is refused. Ctrl+C closes the native
process through EOF and records `browser-shutdown.json`.

In the actual workbench, open **Graph**, select **cpu-embedding-probe**, edit its
text input to `hello`, then use **Resolve** on the dependency node. Save using
the exact name `cpu-embedding-probe` and click **Submit** after backend validation
allows it. Open **I/O Inspector**, select `vectors`, and **Read** the retained
output. This interactive procedure awaits verification on a supported browser.

The host supports this fixture workflow, prompt and dependency UI state edits,
layout updates, Resolve/Check, execution sessions, live diagnostic queries, and
artifact reads. Unsupported commands, other workflow paths, other runtime/model
bindings, model writes/search and dependency installation fail closed. The
original deterministic host remains separate and initially empty; this host's
new catalog contains only the explicitly registered committed fixture.

## Actual model and evidence

The model is the repository's **untrained BERT-8 numerical fixture**, with
6,464-byte safetensors weights and zero model acquisition bytes. It produces
eight-dimensional normalized embeddings on Candle `0.9.2-alpha.2`, locked at
`88ed7911de9e88196b1f55b199145d22647f415e`, using CPU F32. It does not demonstrate
useful semantic embedding quality. See the [model hashes and license scope](../../docs/real-inference-workflow-probe.md#model-and-acquisition).

The [recorded live HTTP receipt](../../docs/evidence/cpu-embedding-http-aa1d79ae.json)
contains run `run_b88f6784-46fc-4ce0-92aa-45cd20632e0d`, input `hello`, its actual
eight-dimensional output, scheduler timeline and exact retained artifact bytes.
Maximum absolute error against the independent Transformers reference is
`5.960464477539063e-8`. The backend run reports 117 ms and the host's complete
submit/resume/response observation is 169.58 ms. Cleanup succeeds. These are
single observations, not benchmarks. This acceptance covers scalar text; batch
and ONNX execution are not exercised.

Tests:

```sh
cargo build --locked -p pantograph-embedded-runtime \
  --features host-dependency-inventory --example cpu_embedding_workflow_probe
python3 -m unittest discover -s examples/cpu-embedding-demo -p test_host.py
status_registry_root="$(mktemp -d)"
PUMAS_REGISTRY_DB_PATH="$status_registry_root/registry.db" \
cargo test --locked -p pantograph-embedded-runtime \
  --features host-dependency-inventory --lib
cargo test --locked -p pantograph-diagnostics-ledger --lib node_status_projection
```

Set `PANTOGRAPH_HTTP_EVIDENCE_PATH` to a new file path to retain the real HTTP test
receipt; existing files are refused. The tests use the real native service and
the unchanged PR71 HTTP transport. They verify expected numerical output,
readback equality, successful scheduler runtime identity, completed `infer` and
`vectors` node statuses, and cleanup, plus
forbidden origins, path traversal, changed runtime bindings, installation,
unsupported commands and invalid publication. They do not render a browser.

## Browser capture gate

On a host where Chromium's sandbox is supported, with the two servers running:

```sh
chromium --headless --remote-debugging-port=9222 \
  --remote-debugging-address=127.0.0.1 \
  --user-data-dir=/tmp/pantograph-cpu-capture about:blank
node examples/cpu-embedding-demo/capture.mjs /tmp/pantograph-cpu-captures
```

The capture script edits through the DOM, clicks the actual Save/Submit controls,
observes real HTTP responses, checks the numerical reference and retained body,
requires the vector to be visible, and saves full viewport screenshots. It does
not inject stores, statuses, IPC, or recorded outputs. Its syntax is checked, but
the complete script has **not run successfully** in this execution environment.

The attempted default Chromium startup failed because
`/usr/lib/chromium/chrome-sandbox` appears owned by `nobody:nogroup`, rather than
root, inside this environment. Selecting the user-namespace sandbox instead
also failed with `No usable sandbox!`. Browser sandboxing was not disabled, no
system permission was escalated, and no substitute screenshot was created.

## Runtime status ownership

The real single-request runtime-host boundary now records the existing
`NodeExecutionStatus` event after a terminal result and requests projection
refresh. The live receipt contains both `infer` and `vectors` as completed.
The `infer` owner status records Candle and the actual fixture model, with 28 ms
of host-boundary elapsed time; the scheduler attempt records 30 ms. Host duration
includes resolution, gateway waiting, model loading and output publication; it
is not an isolated model forward benchmark.

The ledger still refuses to substitute scheduler runtime-attempt events for
node-owner status. Its existing non-runtime retry/reset, cache, runtime settings
and negative runtime-attempt tests all pass unchanged. The host observer checks
response identity and validity: mismatched/invalid responses become failed
observations, never completed statuses. Acknowledged cancellation/shutdown maps
to cancelled; accepted responses and abandoned futures emit no terminal status.
Execution responses and strict capability checks are unchanged. Writes retain
the existing bounded, best-effort diagnostics recording policy; overload or a
ledger failure is logged and cannot authorize execution or invent completion.
This change covers the single-request host path, not batch member status.

The new regression exercises nine response/request cases, including projection
refresh and response preservation. Full embedded runtime tests pass (590 passed,
2 ignored); five node-status ledger regressions, two real inference HTTP tests
and nine existing PR71 transport tests pass. The first full-suite attempt used
an incorrect registry environment variable and failed on the read-only home
registry path; rerunning with `PUMAS_REGISTRY_DB_PATH` in a fresh workspace test
directory passed without changing filesystem permissions.

## Browser environment inspection

No browser, Playwright, or computer automation tool is exposed in this selected
environment. PR71's capture script expects an already running CDP browser; its
README does not supply a separate supported browser service. The repository's
previously passing desktop seed precision harness launches Chromium using
`--no-sandbox` at `scripts/check-desktop-seed-precision-browser.mjs:161`.
That launch mode was not used for this task. The two sandboxed startup failures
above remain the blocker under the instruction to preserve security boundaries.
See the [recorded browser inspection](../../docs/evidence/cpu-embedding-browser-environment.json).

`capture.mjs` is syntax checked and now also requires actual completed statuses
for both inference and output nodes. It remains unexecuted here. No actual
editor-visible model-output screenshot or completed integrated demo is claimed.
