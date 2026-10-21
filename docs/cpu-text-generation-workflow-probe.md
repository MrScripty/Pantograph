# Public CPU text workflow probe

`cpu_text_generation_workflow_probe` composes an isolated public Pumas catalog,
obtains the current inference descriptor, authors its snapshot, invokes graph
Resolve and Check, publishes executable validation, and submits one workflow to
the real scheduler. It resumes only that existing run when dependency inventory
becomes available. The fixed token cap and temperature are wired through normal source nodes;
request-level inputs target only the prompt and control source nodes. Generated completion text
is collected through `text-output`.
It does not insert readiness snapshots or call the Python worker directly.

This is a native/headless CPU integration procedure. The committed untrained
GPT-2 fixture proves routing and execution only; it cannot qualify trained-model
T3 in [the readiness plan](plans/text-generation-readiness/plan.md). Neither a
successful build nor package presence proves model compatibility or text quality.
The report deliberately leaves `t3_qualified` false: an operator must review the
trained input's provenance/license, exact bytes and actual output together.

Build with the repository's supported default features:

```bash
cargo build --locked -p pantograph-embedded-runtime \
  --features backend-pytorch,host-dependency-inventory \
  --example cpu_text_generation_workflow_probe
```

The linked PyO3 interpreter must import NumPy, SoundFile, Torch, Transformers,
Accelerate, Safetensors and Tokenizers. Use the same Python library and site-packages
for build and execution. A host executable override cannot select a different
inventory interpreter. Environment setup is explicit; the probe installs nothing
and downloads nothing. On the saved development environment:

```bash
source /workspace/pantograph-tools/activate.sh
export LD_LIBRARY_PATH=/workspace/pantograph-tools/python/cpython-3.12.3-linux-x86_64-gnu/lib
export PYTHONPATH=/workspace/Pantograph/.venv/lib/python3.12/site-packages
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 OMP_NUM_THREADS=2 MKL_NUM_THREADS=2
```

Supply a new absolute evidence directory and an absolute local model directory.
The example copies only an allowlist of ordinary Transformers config/tokenizer
files and `model.safetensors`, with a 600 MiB total limit; it rejects symlinks.
There is no remote model code. Set dedicated writable XDG roots and the registry
path before starting, so existing catalogs and previous evidence are untouched:

```bash
probe_root=/tmp/pantograph-text-real-001
export XDG_CONFIG_HOME="$probe_root/config"
export XDG_DATA_HOME="$probe_root/data"
export XDG_CACHE_HOME="$probe_root/cache"
export PUMAS_REGISTRY_DB_PATH="$probe_root/registry.db"
timeout 600 /workspace/pantograph-cache/target/debug/examples/cpu_text_generation_workflow_probe \
  "$probe_root" /absolute/path/to/verified-local-model generate
```

Do not pre-create `probe_root` for an initial scenario; only the documented
`reopen` mode accepts the successful generation's existing directory.
The model must fit the standard local Transformers causal-LM loader without
custom Python code or additional Pumas dependency declarations. Verify and retain
upstream URL, immutable revision, authoritative license terms and acquisition
checksums before loading a downloaded model. Keep weights and license material
outside Git. The probe records BLAKE3 and size of every copied input; these bind
the observation to bytes but do not establish training history or licensing.

The selected trained candidate remains DistilGPT2 revision
`2290a62682d06624634c1f46a6ad5be0f47f38aa`; access to the official Hugging Face
endpoint is blocked in the saved environment. Its missing input is the verified
local safetensors/config/tokenizer asset set and authoritative license evidence.
An equivalent trained, licensed local Transformers causal LM meeting the same
CPU, local-only, no-custom-code and size bounds can satisfy the behavioral T3
criterion with its own recorded identity. Randomly initialized GPT-2 files,
mock gateways, synthetic strings or a direct Python-only call change the required
proof and cannot substitute for T3.

The separate `CPU Text Workflow Integration` CI workflow invokes
`scripts/qualify-cpu-text-workflow.sh EXPECTED_SOURCE_SHA NEW_ABSOLUTE_EVIDENCE_ROOT`
with a prepared Python environment. It requires a clean checkout, records the exact source SHA, source
status and relevant source hashes, then builds and runs all three synthetic
scenarios and a fourth fresh-process cold reopen of the successful generation.
This job provides integration coverage, never trained-model T3.

For independent integration evidence, replace the model directory with
`--synthetic`. Run each scenario in a separate new evidence root:

- `generate`: require a nonempty completion with `max_new_tokens=24`,
  `temperature=0.0`, explicit CPU and one loaded session.
- `failure`: corrupt only the copied weights after graph resolution and require
  the exact canonical PyTorch model-load failure and header-deserialization
  diagnostic from the same run/node/attempt; retain public error and cleanup.
- `cancel`: keep the existing resume future owned while polling the public
  bootstrap report for an actual Running runtime task. Request active task
  cancellation through the public service, then require matching runtime node
  cancellation and terminal attempt diagnostics, an empty queue and cleanup.
  Acknowledgment alone cannot pass. This observes early active cancellation;
  it does not prove interruption after generation begins.

After `generate` has exited successfully, invoke the same executable with the
same probe root, XDG roots and registry path, changing only the final scenario
argument to `reopen`. This is the sole mode that accepts an existing probe root;
it requires a successful, cleaned-up generation report from a different process
and creates a new `cold-reopen/` evidence directory. It refuses an existing
`cold-reopen/` directory. It copies no model inputs, performs no explicit catalog
rebuild, and loads the retained workflow through the public graph-load API.
The pinned Pumas `ModelLibrary::new` performs its normal metadata-to-index
reconciliation during startup. This procedure uses that supported lifecycle;
it does not establish that the on-disk index remains untouched across reopening.
The saved graph bytes, retained model input hashes and workflow identity must
match the original observation. Process IDs and random process markers are
retained for both invocations.

The fresh `EmbeddedRuntime` must start without an active run, refresh executable
validation with a new validation identity, invoke new Resolve and Check actions,
and publish a new executable snapshot before submitting generation. Its own
readiness producer and normal readiness resume API establish availability; no
previous in-memory readiness or validation is inserted. The generation must have
a new workflow-run identity, nonempty output, an empty scheduler queue, and the
same bounded session/shutdown/resource cleanup checks. `process-boundary.json`,
`bootstrap-before-reopen.json`, `reopen-observation.json`, the startup registry,
validation/action files, response and final result retain these observations.
The qualification script runs the two invocations sequentially, establishing
that the authoring process exited before the reopen process started.

This proves clean-process reopening of the current saved graph/catalog and
renewed public CPU execution. It does not exercise crash recovery, schema
migration, desktop reopening, or trained-model quality. `t3_qualified` remains
false in both processes. Retained SQLite diagnostics include both runs; correlate
events with each response's workflow-run identity.

The probe requires public terminal semantics as well as canonical runtime
observations and cleanup. A loader failure must return `InternalError` retaining
`pytorch_worker_model_load_failed` and its header-deserialization cause; an
acknowledged runtime cancellation must return typed `Cancelled`. Missing,
truncated, unrelated or contradictory diagnostics fail observation. A generic
`InvalidRequest` / `TerminalFailed` result fails `probe_passed` and the process.
This qualifies the bounded T4 behavior in
[the terminal-outcome plan](plans/cpu-text-terminal-outcomes/plan.md), never T3.

The frozen PR80 source and reports in `plans/text-generation-readiness` retain
the earlier observational behavior and the demonstrated projection defect.
Those historical observations do not qualify the repaired public semantics;
new-source exact-checkpoint evidence belongs to the terminal-outcome plan.

The run timeout is 60 seconds, the probe watchdog is 120 seconds, and cleanup
and shutdown each have a 30-second observation bound. An expired watchdog is
failure, never cancellation success. Always inspect `result.json`,
`session-close.json`, `diagnostics.json`, graph Resolve/Check files, input hashes,
and `response.json` (generation). Evidence files may contain local paths.
The example retains its disposable catalog and outputs for inspection; delete
only that explicitly created evidence directory after review.

Full text-to-image qualification additionally requires a licensed supported
local image model, actual generated image artifacts from text passed through the
composed public graph, output presentation, and failure/cancellation/cleanup at
those real boundaries. CPU embedding, synthetic image presentation and this
text probe do not establish that outcome. GPU and desktop qualification remain
separate.
