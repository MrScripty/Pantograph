# CPU embedding editor host

This feature integrates PR71's verified browser transport and workbench at
`7fc3584cca8bef647b60ebf303c4b5d6b37d98bd` with Pantograph's real embedded runtime.
The ordinary workspace dependency is pinned to public Pumas correction
`aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`. No Pumas source patch, unpublished
override, private catalog, model download, or paid inference is used.

**Qualified:** actual Chromium Graph edit/Save/Resolve/Submit → scheduler →
real Candle CPU inference → visible Vector Output and exact retained readback.
The same capture also qualifies the integrated text Graph. See the
[inspected screenshots and provenance](../../docs/images/real-inference-demo-2026-10-11/README.md).
Settings, the separate NodeEditor, native GTK/WebKit, batch inference and ONNX
inference are outside this qualification.

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
output. This exact procedure passed in Chromium 151.0.7922.173.

The host supports this fixture workflow, prompt and dependency UI state edits,
layout updates, Resolve/Check, execution sessions, live diagnostic queries, and
artifact reads. Unsupported commands, other workflow paths, other runtime/model
bindings, model writes/search and dependency installation fail closed. The
deterministic host remains a separate process with its own `editor-demo`; this host's
new catalog contains only the explicitly registered committed fixture.

## Actual model and evidence

The model is the repository's **untrained BERT-8 numerical fixture**, with
6,464-byte safetensors weights and zero model acquisition bytes. It produces
eight-dimensional normalized embeddings on Candle `0.9.2-alpha.2`, locked at
`88ed7911de9e88196b1f55b199145d22647f415e`, using CPU F32. It does not demonstrate
useful semantic embedding quality. See the [model hashes and license scope](../../docs/real-inference-workflow-probe.md#model-and-acquisition).

The [browser receipt](../../docs/images/real-inference-demo-2026-10-11/inference/receipt.json)
records source commit `12fa09a685a4ccc44edf992ba117648f8fa367ef`, a clean worktree,
and run `run_92d972df-730e-4412-88d0-b568b5d37c49`. The actual `hello` vector
matches the independent Transformers reference within `5.960464477539063e-8`.
The owner node duration is 29 ms, workflow duration 121 ms, and browser-observed
HTTP Submit response 191.28 ms. These are single observations including runtime,
scheduler and transport overhead, not isolated model-forward benchmarks.
The session close and runtime shutdown receipts both report success.

The earlier [live HTTP-only receipt](../../docs/evidence/cpu-embedding-http-aa1d79ae.json)
remains separate historical evidence and correctly retains `editor_qualified: false`.

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

## Reproduce the browser capture

Start the two servers above with a **fresh** inference output directory. Each
capture is one new editor validation/publication lifecycle; reusing a previously
published fixture version can correctly fail immutable validation-snapshot checks.
Do not disable those checks or blindly bump the version for an unchanged graph.

On a host with a usable Chromium sandbox:

```sh
browser_root="$(mktemp -d)"
chromium --headless=new --disable-gpu --disable-dev-shm-usage \
  --disable-background-networking --disable-component-update --disable-sync \
  --disable-extensions --no-first-run --no-default-browser-check \
  --proxy-server=http://127.0.0.1:9 \
  '--proxy-bypass-list=127.0.0.1;localhost;[::1]' \
  --remote-debugging-port=9222 --remote-debugging-address=127.0.0.1 \
  --user-data-dir="$browser_root/profile" about:blank
node examples/cpu-embedding-demo/capture.mjs /tmp/pantograph-cpu-captures
```

This selected cloud environment lacked a usable Chromium sandbox. After explicit
user approval, the successful run added `--no-sandbox` and used a new isolated
profile and XDG config/cache directories, with no personal profile or credentials.
The loopback proxy bypass and capture's CDP request filter restricted navigation
to the local application. No OS permission or system configuration was changed.
The [earlier sandbox failures](../../docs/evidence/cpu-embedding-browser-environment.json)
are preserved as historical evidence, not current blockers.

For the text Graph check, start `python3 examples/scheduler-demo/serve.py` on its
default port 8765, then run the same script with a new capture path and `--text`.
Both captures use real DOM controls, observe actual command responses, and assert
visible output, completed owner statuses, Scheduler, Network and retained bytes.
No stores, IPC, status events, model outputs or screenshots are synthesized.
Receipts include source SHA, worktree state, browser version, timings, command
bodies, screenshot hashes and visible text. Failed capture attempts retain their
error and actual screenshot and exit nonzero.

## Browser integration corrections

Browser startup loads only a workflow advertised by the selected host and scopes
last-graph storage by host; native component-workspace loading stays on the desktop
host. Dependency activity lines use runtime display state, preventing a log entry
after Resolve from mutating the authored graph and invalidating the owner's proof.
Semantic dependency edits still go through backend graph mutation and validation.
The fixture layout avoids node overlap and labels the model as untrained BERT-8.

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
and twelve integrated PR71 transport tests pass. The frontend suite passes all
722 tests; typecheck, production build, full lint and critical lint pass. The first full-suite attempt used
an incorrect registry environment variable and failed on the read-only home
registry path; rerunning with `PUMAS_REGISTRY_DB_PATH` in a fresh workspace test
directory passed without changing filesystem permissions.

## Local CI pin and ONNX feature qualification

Commit `424783bf6fc722cb1c9dc5fbc0658ee957883eae` aligns the CI checkout with
Cargo's exact public Pumas pin `aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`.
The unchanged guard completed with exit status 0 on 2026-10-11 after permitted
Cargo registry fetching:

```sh
CARGO_NET_OFFLINE=false ORT_SKIP_DOWNLOAD=1 \
python3 scripts/check-onnx-no-build-download.py
```

All nine combinations passed: Linux, Windows and macOS target triples, each with
workspace defaults, workspace all-features, and the reduced embedded-runtime
`backend-llamacpp,backend-pytorch` profile. The guard verifies manifest/lock/CI
pin consistency, rejects forbidden ONNX download features, and requires dynamic
loading/disabled linking. See the [complete local log](../../docs/evidence/onnx-feature-guard-424783bf.txt).
No manifest, lockfile, dependency pin, or workflow trigger was changed by this
verification. This checks Cargo feature graphs; it does not qualify ONNX model
inference or cross-platform builds. Hosted CI runs on the separately approved
[draft PR #72](https://github.com/MrScripty/Pantograph/pull/72).

## Multi-branch CPU workflow

Run `python3 examples/cpu-embedding-demo/serve.py --rich --output-dir /tmp/pantograph-rich-new`
with a **new** absolute output directory, alongside the Vite server above. Select
`cpu-multibranch-probe`, edit Prompt to `hello`, Resolve both dependency nodes,
Save under the same name, and Submit. The normal editor issues every command.

The 13-node graph composes Context (`different world`) and Prompt into text, sends
that text through the untrained BERT-8 fixture, independently embeds Context with
untrained BERT-12, extracts Candle's actual runtime identity, and joins text,
capability limitations and that identity into a visible report. The scheduler
sorts Merge inputs by source task ID, so composed text is `different world\nhello`.
It retains both vectors and the report under one run ID. Two models may remain
loaded; the catalog, graph topology, model bindings and supported commands are fixed.

Both models use Candle CPU F32 at the same pinned version as the original demo.
Weights are 6,464 and 10,992 bytes (17,456 total), already committed. No model is
downloaded and no paid inference is used. They are repository-generated untrained
fixtures under the repository license material described above, not pretrained
models or evidence of semantic quality.

`python3 examples/cpu-embedding-demo/reference.py` independently computes both
vectors with actual local Transformers BERT forwards. It requires installed Torch,
Transformers and Tokenizers, uses local files only, and never regenerates fixtures.
`python3 -m unittest discover -s examples/cpu-embedding-demo -p 'test*host.py'`
checks both HTTP workflows, exact retained readback and rejected scope changes.
The browser procedure is `node examples/cpu-embedding-demo/capture.mjs /tmp/rich-captures --rich`
with a fresh host/catalog and capture directory. Browser sandbox permission must
be established for the specific host; the prior isolated capture approval is not
a blanket permission for future environments.

Text generation remains **unqualified** in this host. A real GPT-2 attempt reached
public owner model and dependency-profile resolution but stopped at scheduler
readiness: `Dependency requirements payload is missing`, followed by the bounded
20-second deadline. The editor facts provider's dependency handoff currently only
implements native Candle. No Python Ready state was injected and no GPT-2 output
is claimed. The attempted 23,024-byte untrained GPT-2 weights were already committed;
only the ordinary SoundFile dependency (1.3 MiB) was installed during investigation.

Image generation is **unavailable** without diffusion weights. External image
transport is also unqualified: image-input is not a supported scheduler source,
selection strings are bounded at 2 KB, and image-output requires a typed media
artifact reference rather than a base64 string. These checks remain intact. No
image branch, generated image, or image-passthrough success is claimed. The actual
report shows these limitations explicitly. No Pumas source, private catalog or
user-local environment was changed.

Merge now uses the existing generic workflow renderer, exposing its real ports
and connections. Toolbar submission also materializes the scheduler's supported
integer, boolean and selection sources; unsafe/fractional integer values are
rejected before submission and structured selection values are captured by value.

## Opt-in bounded CPU service opportunities

`WorkflowSerialReadyConfig::with_fair_service_opportunities()` enables M3 on the
constructor-owned serial CPU service. Existing ordinary construction and M1/M2
configuration keep their prior behavior. This mode requires sessions created
through the public authenticated attribution API. It shares a service unit by
validated client/bucket, including multiple sessions belonging to that unit.
Missing attribution or the 64-waiter/32-unit limit returns an explicit refusal
before runtime admission. After two other-unit grants, a waiting unit receives
a protected opportunity before ordinary priority; otherwise active-run priority
and FIFO apply. Existing per-session starvation boosts remain intact. The
promise counts opportunities, not milliseconds or interruption of native work.

The isolated `cpu_embedding_workflow_probe <new absolute directory> --fair-service`
mode executes a fixed cohort with five sessions across two authenticated units.
Qualification controls arrival using transparent native barriers, then forwards
real results and cleanup receipts unchanged. The cancellation request is after
native drain while the workflow still reports Running; it does not demonstrate
interrupted compute. The existing public cancellation API requires Running and
cannot cancel a waiting Ready task. No default activation, GPU, browser, native
batching, model-quality or speedup qualification follows from this mode.

The [M3 contract](../../docs/plans/cpu-scheduler/plan.md#m3-bounded-service-contract-2026-10-19-independently-admitted)
owns the fixed protocol and acceptance gates. Reproduce only in a fresh isolated
project with the existing BERT fixture and CPU build features; retain failed
outcomes, source/binary hashes, ledger events and teardown records.

Saved M3 evidence can be checked with
`cpu_embedding_workflow_probe <absolute existing run directory> --reanalyze-fair-service`.
This synchronous mode reads saved JSON and committed golden bytes, creates a
separate `reanalysis.json` in the run directory's parent with create-new semantics,
and starts no host, runtime or model. It preserves the original failed oracle and
exit status. Aggregate training checks use frozen-source assistance; local sample
task names do not independently identify run/attempt timings. See the M3 report
for the original checker defect and exact-source evidence.

## Opt-in serial CPU residency producer

The M4 producer is selected explicitly with `--serial-residency`. It uses the
owning `EmbeddedSerialCpuResidency` facade, authenticated ephemeral sessions,
and the committed untrained BERT-8/BERT-12 fixtures. Construction performs no
native bootstrap. The constructor owns its gateway and runtime registry; no
writable runtime or owner capability is exposed to the producer.

The build command and invocation shape below require independent
source/producer review, source/binary/environment freeze, and separate
authorization of a future protocol:

```sh
ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
  cargo build --locked -p pantograph-embedded-runtime --no-default-features \
  --features backend-candle,host-dependency-inventory --release \
  --example cpu_embedding_workflow_probe
# Execution requires separate prospective protocol admission:
PUMAS_REGISTRY_DB_PATH=/tmp/pantograph-m4-new/registry.db \
  "${CARGO_TARGET_DIR:-target}/release/examples/cpu_embedding_workflow_probe" \
  /tmp/pantograph-m4-new --serial-residency
```

The retained protocol makes exactly five public embedding calls in `A,A,B,B,A`
order through one fresh facade. Its count oracle requires three cold loads,
two retains, two switch stops and one final acknowledged idle stop. Comparison
uses five fresh facades, one call and acknowledged shutdown each, with the same
model declarations, inputs and CPU calibration. Each previous facade is dropped
before the next is constructed. This compares native counts, vectors and
ownership; it establishes no startup, session, history or latency equivalence.
The process-wide Pumas registry SQLite file is shared metadata only. Each facade
has a distinct project, model-library and application-data root and owns a new
native gateway and runtime registry.

A separate fresh pressure owner attempts one A call with 80 MiB declared RAM
capacity and must refuse before any load, runtime Start or reservation. The
fitting protocols declare 1,104 MiB RAM capacity, 1,024 MiB VRAM admission
capacity, 32 MiB resident RAM and a 48 MiB cold guard, with explicit CPU VRAM
zero. Actual task RAM/VRAM maxima remain unchanged and must fit additively;
these declarations are neither allocator measurements nor GPU qualification.

The producer first verifies all fourteen committed fixture hashes using the
read-only `m4_fixture_contract.py` helper. It saves every public result/error,
successful-run readiness resumes, retained vector readback, ledger Start and
Completed observations, native
identity, resource claim, phase observation and teardown outcome. Load,
publication, forward-and-drain, physical stop and registry ACK are distinct
observations. The registry task ACK is not labelled a scheduler cleanup ACK.
Pressure requires its exact headroom refusal and failed terminal ledger run.
Outer request time and shutdown watchdog timeouts are labelled separately;
an uncertain shutdown ends the cohort without constructing another owner.
Failed-run resume history is not returned by the helper; public errors and
matching preparation/terminal identities remain in raw native/ledger evidence.
Readback explicitly refreshes the three diagnostic projections with a 500-event
page bound, checks each cursor against its relevant visible raw events, and
saves inspection before artifact assertions. Start identifies run/node/task/
attempt; Completed supplies the matching selected lease, runtime and device.
Existing directories,
fixture overrides, missing evidence and log overflow refuse; failed cohorts
are preserved and never restarted to obtain a desired result.

A successful exit covers only `main-counts-v2+pressure`. Full M4-C4 requires
independent review together with the separately preserved cancellation and
controlled fault evidence. The two once-only cancellation cases at `96691fd7`
are not invoked by this producer. No missing phase is
assigned zero, and no general speedup, useful embedding quality, default
activation or M4 acceptance follows from this invocation.

The original frozen `7139b2ad` main attempt exited after its first A result
because diagnostic projections had not been advanced. Its source, binaries and
failed evidence remain preserved. Parent subsequently authorized exactly one new
`m4-serial-residency-v2-readback` main cohort after independent causal-fix review
and a deterministic saved-event/artifact replay proves all required projections
are populated. See the [prospective protocol](../../docs/plans/cpu-scheduler/reports/2026-10-20-m4-main-v2.md).
All original retention/switching/pressure/3-versus-5 criteria and budgets remain
binding. This admission does not restart or continue the original failed owner,
and allows no winner retry or new cancellation execution. Publication stays held.
